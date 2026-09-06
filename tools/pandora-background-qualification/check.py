#!/usr/bin/env python3
"""Compare source exports to two existing native journeys; never initialize from capture."""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
POINTS = {
    'town-west-rest': 0xa, 'town-north-rest': 0xa, 'town-gap-rest': 0xa,
    'town-gap-up-rest': 0xa, 'town-door-align-rest': 0xa,
    'landed-13': 0x13, '13-west-rest': 0x13,
    'E-left-rest': 0xe, '20-left-rest': 0x20,
    '21-entry-closed': 0x21, '21-box-contact-rest': 0x21,
    '21-opening-wait': 0x21,
    'tutorial-020': 0x41, 'tutorial-034': 0x44,
    'tutorial-041': 0x42, 'tutorial-045': 0x43,
    'pandora-neutral-stable': 0x41,
}
def require(ok, message):
    if not ok: raise ValueError(message)

def sha(data): return hashlib.sha256(data).hexdigest()
def u(data, at):
    require(0 <= at <= len(data)-2, 'word extent')
    return int.from_bytes(data[at:at+2], 'little')
def words(data):
    require(len(data)%2 == 0, 'word plane extent')
    return [u(data, at) for at in range(0, len(data), 2)]
def differences(a, b):
    require(len(a) == len(b), 'comparison extent')
    return [[i, x, y] for i, (x, y) in enumerate(zip(a, b)) if x != y]
def attributed(cells, attributes):
    require(len(attributes) == 512 and all(0 <= v < 512 for v in cells), 'attribute extent/cell')
    return [v | ((attributes[v]&127)<<9) for v in cells]
def tileword(definitions, cells, width, x, y):
    require(len(definitions) == 4096 and width > 0 and len(cells)%width == 0, 'definition/grid extent')
    require(0 <= x < width*2 and 0 <= y < len(cells)//width*2, 'tile coordinate')
    return u(definitions, (cells[(y//2)*width+x//2]&511)*8+((y%2)*2+x%2)*2)
def sample(graphics, word, x, y):
    require(0 <= x < 8 and 0 <= y < 8 and len(graphics) >= (word&1023)*32+32, 'pixel extent')
    if word&0x4000: x=7-x
    if word&0x8000: y=7-y
    at=(word&1023)*32
    color=sum(((graphics[at+(p//2)*16+y*2+p%2]>>(7-x))&1)<<p for p in range(4))
    return (color+((word>>10)&7)*16, bool(word&0x2000)) if color else (0, False)
def origin(player, bounds):
    l,t,r,b=bounds
    require(r-l >= 256 and b-t >= 256, 'camera bounds')
    return [max(l, min(r-256, player[0]-128)), max(t, min(b-256, player[1]-112))]
def verify_point(point):
    require(not point['tileword_errors'], 'source/native tileword mismatch')
    require(not point['unexpected_pixels'], 'unexplained source/native indexed/priority mismatch')
    require(point['camera'] == point['source_camera'], 'source/native settled camera mismatch')
    require(point['visible_phase_changed_cells'] == 0, 'unqualified visible dynamic grid phase')

# Each audited scene selects an eight-byte animation table, not a timer model.
ANIMATIONS = {
    0xa: [(0x38a73, 0, 0x1b8000, 0x1b807e, (0x120,), 128),
          (0x38a78, 1, 0x1b8000, 0x1b84df, (0x3e00,0x3e80,0x3f00,0x3f80), 128)],
    0x13: [(0x38eed, 15, 0x1b8000, 0x1be5ed, (0x4a0,), 128)],
}
TOUR_ANIMATION = (0x39559, 60, 0x1c8000, 0x1ce240, (0x2aa0,), 32)

def frame_candidates(rom, table, bank_base, destinations, size):
    result = {d: [] for d in destinations}
    for at in range(table, table+8*256, 8):
        require(at+8 <= bank_base+0x8000, 'animation table bank bound')
        count = rom[at]
        if not count: return result
        source = bank_base+u(rom, at+1)
        destination = u(rom, at+3)*2
        require(0 < count <= 8 and destination in result and u(rom, at+5) == size, 'animation transfer shape')
        require(source+count*size <= bank_base+0x8000, 'animation source bank bound')
        result[destination].extend((source+i*size, size) for i in range(count))
    raise ValueError('unterminated animation table')

def reconstruct_graphics(rom, base, native, map_id):
    require(len(native) >= len(base), 'native graphics extent')
    reconstructed = bytearray(base)
    witnesses = []
    specs = [TOUR_ANIMATION] if 0x41 <= map_id <= 0x44 else ANIMATIONS.get(map_id, [])
    for scene, selector, bank, table, destinations, size in specs:
        require(rom[scene:scene+5] == bytes([0xfb,selector,0xe8,0x98,0x87]), 'source animation actor')
        require(bank+u(rom, bank+selector*2) == table, 'source animation selector')
        candidates = frame_candidates(rom, table, bank, destinations, size)
        for destination, frames in candidates.items():
            require(destination+size <= len(base), 'animation destination extent')
            if native[destination:destination+size] == base[destination:destination+size]: continue
            matches = [source for source,n in frames if rom[source:source+n] == native[destination:destination+n]]
            require(matches, 'native graphics span not a source-selected animation payload')
            source = matches[0]
            reconstructed[destination:destination+size] = rom[source:source+size]
            witnesses.append({'selector':selector,'table':table,'source':source,'destination':destination,'size':size,'sha256':sha(rom[source:source+size])})
    require(reconstructed == native[:len(base)], 'unexplained native graphics bytes')
    return reconstructed, witnesses

def inspect(rom, root, export, profile, label):
    map_id = profile['map']; width = profile['width']//16
    blobs = {s:(root/f'{label}.{s}').read_bytes() for s in ('wram','vram','cgram')}
    w,v,c = (blobs[s] for s in ('wram','vram','cgram'))
    require([len(w),len(v),len(c)] == [131072,65536,512], 'capture extents')
    require(u(w,0x47e) == map_id, 'native map identity')
    b = {name:(export/name).read_bytes() for name in profile['files']}
    require(all(sha(b[n]) == h for n,h in profile['files'].items()), 'export plane pins')
    for source in profile['sources']:
        require(sha(rom[source['start']:source['end']]) == source['sha256'], 'source resource pin')
    cells = words(b['cells']); grid = words(b['static-grid'])
    require(grid == attributed(cells,b['attributes']), 'independent full attributed grid')
    require(len(cells)*256 == profile['width']*profile['height'], 'source grid dimensions')
    native = words(w[0xa000:0xa000+len(cells)*2])
    grid_deltas = differences(grid,native)
    camera = [u(w,0x81e),u(w,0x822)]; player = [u(w,0x1000),u(w,0x1002)]
    contract = profile['camera']; bounds = contract['bounds']
    require(contract['record_offset'] == 0x16be30+map_id*2, 'camera record table linkage')
    require(u(rom,0x28000+map_id*2)==0, 'scene fallback')
    scene = 0x30000+u(rom,0x38000+map_id*2)
    require(contract['scene_offset']==scene and rom[scene]==0, 'camera scene linkage')
    display = 0x160000+u(rom,0x16bb64+rom[scene+1]*2)
    require(contract['display_offset']==display, 'scene display table linkage')
    require(contract['ring_word_base']==0x3800 and contract['bgmode']==rom[display+6]==9, 'first layer ring/BGMODE source')
    require(contract['hardware_background']==(2 if rom[display+5]&0x80 else 1), 'display swap source')
    require(rom[display+4]&0x40 and [u(w,0x826),u(w,0x82a)]==[profile['width'],profile['height']], 'source clamp and native dimensions')
    record = rom[contract['record_offset']:contract['record_offset']+2]
    require(list(record)==contract['record'], 'retained camera record')
    a,z = record
    source_bounds = [(a&15)*256,(z&15)*256,((a&15)+(a>>4))*256,((z&15)+(z>>4))*256]
    require(bounds == source_bounds == [u(w,at) for at in range(0x874,0x87c,2)], 'source/native bounds')
    require(u(w,0x866) == contract['vertical_extent'] == 256, 'vertical clamp extent')
    require([u(w,0x80e),u(w,0x812)] == camera, 'both native camera pairs')
    require(rom[contract['display_offset']:contract['display_offset']+9] == bytes(contract['display']), 'display source')
    require(w[0x46d:0x46f] == (bytes([0x3c,0x38]) if contract['hardware_background']==2 else bytes([0x38,0x3c])), 'hardware BG assignment')
    require(b['definitions'] == w[0x2000:0x3000], 'all native definitions')
    reconstructed, animations = reconstruct_graphics(rom,b['graphics'],v,map_id)
    tile_errors=[]; unexpected=[]; sampled_words=high=base_differences=changed_words=sampled_pixels=alias_words=alias_pixels=0
    visible_phase_cells=set()
    x0,y0 = camera[0]//8,camera[1]//8
    # Cover every visible pixel. A fine X scroll exposes a 33rd tile column;
    # hardware's 32-column ring aliases that edge to the leading resident column.
    # Compare that word too, but retain its difference from a natural full-sheet crop.
    for y in range(y0,(camera[1]+223)//8+1):
        for x in range(x0,(camera[0]+255)//8+1):
            source_word = tileword(b['definitions'],cells,width,x,y)
            resident_x = x0+(x-x0)%32
            expected = tileword(b['definitions'],cells,width,resident_x,y)
            for cell in {(y//2)*width+x//2,(y//2)*width+resident_x//2}:
                if (cells[cell]^native[cell])&511: visible_phase_cells.add(cell)
            phase_word = tileword(b['definitions'],native,width,resident_x,y)
            actual = u(v,2*(contract['ring_word_base']+(y%32)*32+x%32))
            if expected != actual: tile_errors.append([x,y,expected,actual])
            sampled_words += 1; high += bool(actual&0x2000); changed_words += expected != phase_word
            alias_words += resident_x!=x and source_word!=actual
            for py in range(max(0,camera[1]-y*8),min(8,camera[1]+224-y*8)):
                for px in range(max(0,camera[0]-x*8),min(8,camera[0]+256-x*8)):
                    natural = sample(b['graphics'],source_word,px,py)
                    compiled_at = (y*8+py)*profile['width']+x*8+px
                    require(natural == (b['indices'][compiled_at],bool(b['priorities'][compiled_at])), 'compiler indexed/priority sampling')
                    observed = sample(v,actual,px,py)
                    source_phase = sample(reconstructed,expected,px,py)
                    if observed != source_phase: unexpected.append([x*8+px,y*8+py])
                    base_differences += natural != observed
                    alias_pixels += resident_x!=x and natural!=observed
                    sampled_pixels += 1
    result = {'map':map_id,'camera':camera,'source_camera':origin(player,bounds),
        'capture_hashes':{n:sha(data) for n,data in blobs.items()},
        'tileword_errors':tile_errors,'unexpected_pixels':unexpected,
        'tile_words':sampled_words,'high_words':high,'phase_changed_words':changed_words,
        'source_phase_equal_pixels':sampled_pixels-len(unexpected),
        'visible_phase_changed_cells':len(visible_phase_cells),
        'ring_alias_different_words':alias_words,'ring_alias_different_pixels':alias_pixels,
        'viewport':[camera[0],camera[1],256,224],
        'natural_base_different_pixels':base_differences,'animation_frames':animations,
        'grid':{'total_differences':len(grid_deltas),
            'bit15_only':sum((a^z)==0x8000 for _,a,z in grid_deltas),
            'changed_tile':sum((a^z)&511 != 0 for _,a,z in grid_deltas),
            'changed_tile_coordinates':[[i%width,i//width] for i,a,z in grid_deltas if (a^z)&511],
            'other_attribute':sum((a^z)&0x7e00 != 0 for _,a,z in grid_deltas),
            'source_sha256':sha(b['static-grid']),'native_sha256':sha(w[0xa000:0xa000+len(cells)*2])},
        'attribute_differences':differences(b['attributes'],w[0x10000:0x10200]),
        'palette_different_indices':[i for i,a,z in differences(words(b['palette']),words(c[:256]))]}
    verify_point(result)
    return result

def typed_source_contract(rom, source):
    require(source['rom_sha256'] == sha(rom), 'typed source identity')
    for item in source['maps']:
        m = item['map']
        if m not in (10,14,19,32,33,65,66,67,68): continue
        require(u(rom,0x28000+m*2)==0 and 0x830000+u(rom,0x38000+m*2)==item['scene'], 'typed scene table')
        require(int.from_bytes(rom[0x6959c+m*3:0x6959f+m*3],'little')==item['map_script'], 'typed map script table')
    for layer in source['layers']:
        at = layer['source']&0x3fffff
        require(rom[at:at+2] == bytes([0x10,1]), 'typed first-layer opcode')
        packed = int.from_bytes(rom[at+2:at+5],'little')
        pointer = ((0x98+(packed>>15))<<16)|0x8000|(packed&0x7fff)
        require(pointer == layer['pointer'], 'typed first-layer pointer')
    setup = source['interior_setup']; at = setup['controller']&0x3fffff
    require((rom[at+12]<<16)|u(rom,at+7) == setup['graphics'], 'typed controller graphics')
    for transfer in setup['palettes']:
        at = transfer['source']&0x3fffff
        require(rom[at:at+2]==bytes([2,0x5a]), 'typed COP5A opcode')
        pointer = (rom[at+2]<<16)|u(rom,at+3)
        require(pointer==transfer['pointer'] and list(rom[at+5:at+7])==[transfer['destination'],transfer['count']], 'typed bank-first palette transfer')

SOURCE_RANGES = [
    (0x69371,0x693f8), (0x790a1,0x79106), (0x68cde,0x68ce7),
    (0x68d3f,0x68d73), (0x692ac,0x69316), (0x9d24e,0x9d2a1),
    (0xd93d8,0xd949d), (0x6a678,0x6a6c5),
]

def compare_journeys(rom, export, profiles, source, parent, points):
    lookup = {p['map']:p for p in profiles}
    captures = {name:{label:inspect(rom,root,export/f'{m:x}',lookup[m],label) for label,m in points.items()}
        for name,root in [('source',source),('parent',parent)]}
    differing = [label for label in points if captures['source'][label] != captures['parent'][label]]
    require(not differing, f'independent WRAM/VRAM/CGRAM background reports differ: {differing}')
    return captures

def report(rom_path, export_path, source_path, parent_path):
    rom = Path(rom_path).read_bytes(); require(sha(rom) == ROM_SHA,'Japanese ROM identity')
    typed_bytes = (HERE.parent/'pandora-qualification/source.json').read_bytes()
    typed_source_contract(rom,json.loads(typed_bytes))
    export = Path(export_path)
    profiles = json.loads((export/'export.json').read_text())
    require([p['map'] for p in profiles] == [10,14,19,32,33,65,66,67,68], 'source profile coverage')
    captures = compare_journeys(rom, export, profiles, Path(source_path), Path(parent_path), POINTS)
    differing = []
    return {'rom_sha256':ROM_SHA,'typed_source_sha256':sha(typed_bytes),'profiles':profiles,
        'source_consumers':[{'start':a,'end':b,'sha256':sha(rom[a:b])} for a,b in SOURCE_RANGES],
        'captures':captures['source'],
        'independent_report_hashes':{name:sha(json.dumps(data,sort_keys=True).encode()) for name,data in captures.items()},
        'source_parent_differing_points':differing}

if __name__ == '__main__':
    require(len(sys.argv) in (5,6), 'check.py ROM SOURCE_EXPORT SOURCE_JOURNEY PARENT_JOURNEY [--record]')
    actual = report(*sys.argv[1:5])
    if len(sys.argv)==6:
        require(sys.argv[5]=='--record','unknown mode')
        print(json.dumps(actual,indent=2))
    else:
        require(actual == json.loads((HERE/'reference.json').read_text()), 'background qualification metadata differs')
        print('Pandora source/native background comparison passed; parent route acceptance remains separate.')
