"""Bounded source/native checks. Captures are evidence, never source assets."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import sys

HERE = Path(__file__).parent
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
WINDOW = [29, 47, 36, 53]  # half-open source-cell sample halo, not a whole-map admission
LABELS = ['landed-A', 'exterior-walk-down-settled', 'exterior-walk-settled']

def require(ok, message):
    if not ok: raise ValueError(message)

def sha(b): return hashlib.sha256(b).hexdigest()
def u(b, p):
    require(0 <= p and p+2 <= len(b), 'word extent')
    return int.from_bytes(b[p:p+2], 'little')
def words(b):
    require(len(b)%2 == 0, 'odd word plane')
    return [u(b,p) for p in range(0,len(b),2)]

def camera_bounds(record):
    require(len(record)==2 and all(v!=255 and v>>4 for v in record), 'camera record')
    a,b = record
    return [(a&15)*256, (b&15)*256, ((a&15)+(a>>4))*256, ((b&15)+(b>>4))*256]

def camera(position, bounds, height):
    left,top,right,bottom = bounds
    require(right-left>=256 and bottom-top>=height and height in (224,256), 'camera bounds')
    return [max(left,min(position[0]-128,right-256)), max(top,min(position[1]-112,bottom-height))]

def ring_byte(base, x, y):
    require(base in (0x3800,0x3c00) and x>=0 and y>=0, 'ring coordinate/base')
    return 2*(base+(y%32)*32+x%32)

def definition_word(cells, definitions, width, x, y):
    require(width>0 and len(cells)%width==0 and 0<=x<width*2 and 0<=y<len(cells)//width*2, 'map tile coordinate')
    require(len(definitions)==4096, 'definitions extent')
    cell = cells[(y//2)*width+x//2]
    return u(definitions,(cell&511)*8+((y%2)*2+x%2)*2)

def grid_comparison(base, native, width, window):
    require(len(base)==len(native) and width>0 and len(base)%width==0, 'grid extent')
    x0,y0,x1,y1 = window
    require(0<=x0<x1<=width and 0<=y0<y1<=len(base)//width, 'window extent')
    differences = [[i,a,b] for i,(a,b) in enumerate(zip(base,native)) if a!=b]
    require(all((a^b)==0x8000 for _,a,b in differences), 'unqualified low-word terrain mutation')
    require(all(not(x0<=i%width<x1 and y0<=i//width<y1) for i,_,_ in differences), 'landing window occupancy mutation')
    require(all((base[y*width+x]>>9) in (0,2,22) for y in range(y0,y1) for x in range(x0,x1)), 'unqualified landing material')
    return differences

def sample(graphics, word, x, y):
    require(0<=x<8 and 0<=y<8 and (word&1023)*32+32<=len(graphics), 'tile sample extent')
    if word&0x4000: x=7-x
    if word&0x8000: y=7-y
    at=(word&1023)*32
    color=sum(((graphics[at+(p//2)*16+y*2+p%2]>>(7-x))&1)<<p for p in range(4))
    return [color+((word>>10)&7)*16, bool(word&0x2000)] if color else [0,False]

def attributed_grid(cells, attributes):
    require(len(attributes)==512 and all(0<=c<512 for c in cells), 'source cell/attribute extent')
    return [c|((attributes[c]&127)<<9) for c in cells]

def visible_oam(oam, obj):
    """Inventory only the witnessed OBSEL=$02 size mode; not an actor decoder."""
    require(len(oam)==544 and obj==bytes([2,0]), 'qualified OAM size/base/rotation')
    result=[]
    for i in range(128):
        x,y,_,attributes=oam[i*4:i*4+4]
        high=(oam[512+i//4]>>((i%4)*2))&3
        x += (high&1)*256
        if x>=256: x-=512
        if y>=240: y-=256
        size=16 if high&2 else 8
        if x<256 and x+size>0 and y<224 and y+size>0:
            result.append([i,x,y,size,(attributes>>4)&3])
    return result

def source_contract(rom):
    require(sha(rom)==ROM_SHA, 'normalized JP ROM identity')
    require(u(rom,0x1801a)==0x8dfe and u(rom,0x18e02)==0xa, 'D first exit destination')
    require(u(rom,0x28014)==0 and u(rom,0x38014)==0x89a7, 'scene-list selection')
    require(rom[0x389a7:0x389a9]==bytes([0,8]), 'display selector')
    selector=rom[0x389a8]
    profile_at=0x160000+u(rom,0x16bb64+selector*2)
    profile=rom[profile_at:profile_at+9]
    require(profile==bytes.fromhex('16 01 82 33 64 c0 09 ed 13'), 'A hardware profile')
    bounds=camera_bounds(rom[0x16be44:0x16be46])
    return {'schema':1, 'rom_sha256':ROM_SHA, 'map':10, 'sheet_pixels':[1024,1280],
            'camera_bounds':bounds, 'camera_clamp_height':256, 'hardware_background':2,
            'tilemap_word_base':0x3800, 'bgmode':9, 'display_profile_source':profile_at,
            'display_profile':list(profile), 'sample_window_cells':WINDOW,
            'global_event_bits':[32,38,251], 'passive_action_mask_clear':0x50,
            'policy':'base first BG only; bounded sample window, not whole-map actors/material admission'}

def inspect(exported, root, label, contract):
    blobs={name:(root/f'{label}.{name}').read_bytes() for name in ('wram','vram','cgram','oam','obj','pixels')}
    w,v,c = (blobs[name] for name in ('wram','vram','cgram'))
    require([len(w),len(v),len(c),len(blobs['oam'])]==[131072,65536,512,544], 'native surface extents')
    require(u(w,0x47e)==10 and [u(w,p) for p in (0x826,0x82a)]==contract['sheet_pixels'], 'native map/dimensions')
    position=[u(w,p) for p in (0x1000,0x1002)]
    require(488<=position[0]<=552 and 768<=position[1]<=832, 'checkpoint outside bounded landing')
    expected_camera=camera(position,contract['camera_bounds'],256)
    require([u(w,p) for p in (0x80e,0x812)]==[u(w,p) for p in (0x81e,0x822)]==expected_camera, 'source/native camera follow')
    require([u(w,p) for p in (0x874,0x876,0x878,0x87a)]==contract['camera_bounds'] and u(w,0x866)==256, 'camera bounds/height')
    require(u(w,0x868)&0x83==0 and u(w,0x980)&0x50==0, 'ordinary camera/passive hooks')
    require(w[0x468:0x46f]==bytes.fromhex('16 01 82 33 00 3c 38'), 'hardware shadows')
    require([u(w,p) for p in (0x86a,0x86c,0x86e)]==[0xc0,0x2ff,0x201], 'secondary-scroll setup')
    flags=[i for i in range(512) if w[0x6c0+i//8]&(1<<(i%8))]
    require(flags==contract['global_event_bits'], 'progressed fresh flags')
    base=words(exported['static-grid']); native=words(w[0xa000:0xc800])
    differences=grid_comparison(base,native,64,WINDOW)
    require(exported['definitions']==w[0x2000:0x3000], 'all native definitions')
    require(exported['attributes']==w[0x10000:0x10200], 'all native attributes')
    # Natural base art is intentional. Report every differing animated tile/color,
    # without substituting a RAM frame or claiming full framebuffer equivalence.
    graphics_deltas=sorted({i//32 for i,(a,b) in enumerate(zip(exported['graphics'],v)) if a!=b})
    palette_deltas=[[i,a,b] for i,(a,b) in enumerate(zip(words(exported['palette']),words(c[:256]))) if a!=b]
    high=equal=bg1_equal=changed_color_pixels=0
    changed_colors={i for i,_,_ in palette_deltas}
    for y in range(expected_camera[1]//8+2,expected_camera[1]//8+26):
        for x in range(expected_camera[0]//8+2,expected_camera[0]//8+30):
            word=definition_word(base,exported['definitions'],64,x,y)
            require(u(v,ring_byte(0x3800,x,y))==word, 'BG2 full source tile word')
            bg1_equal+=u(v,ring_byte(0x3c00,x,y))==word
            high+=bool(word&0x2000)
            for py in range(8):
                for px in range(8):
                    pixel=sample(exported['graphics'],word,px,py)
                    equal+=pixel==sample(v,word,px,py)
                    changed_color_pixels+=pixel[0]!=0 and pixel[0] in changed_colors
    require(0<high<672 and bg1_equal<672, 'independent BG2 mapping/priority witness')
    return {'label':label,'position':position,'camera':expected_camera,'files':{n:sha(b) for n,b in blobs.items()},
            'grid_differences':differences, 'changed_graphics_tiles':graphics_deltas,
            'palette_differences':palette_deltas,'bg2_equal_tile_words':672,'bg1_equal_tile_words':bg1_equal,
            'high_words':high,'equal_base_index_priority_pixels':equal,'sampled_pixels':43008,
            'changed_palette_opaque_pixels':changed_color_pixels,
            'visible_oam':[{'slot':i,'position':[x,y],'size':size,'priority':priority}
                           for i,x,y,size,priority in visible_oam(blobs['oam'],blobs['obj'])]}

def main():
    require(len(sys.argv) in (3,4), 'check.py ROM SOURCE_EXPORT [CONVERSATION_CAPTURE_ROOT]')
    rom=Path(sys.argv[1]).read_bytes(); out=Path(sys.argv[2]); contract=source_contract(rom)
    reference=json.loads((HERE/'reference.json').read_text())
    manifest=json.loads((out/'export.json').read_text())
    require(manifest==reference['export'], 'source export pins')
    exported={name:(out/'a'/name).read_bytes() for name in manifest[0]['files']}
    require(all(sha(exported[name])==digest for name,digest in manifest[0]['files'].items()), 'export file identity')
    require(contract==reference['contract'], 'source contract pins')
    grid=words(exported['static-grid'])
    require(len(grid)==5120 and len(exported['graphics'])==24576, 'source extents')
    require(grid==attributed_grid(words(exported['cells']),exported['attributes']), 'independent ROM-only attributed grid')
    grid_comparison(grid,grid,64,WINDOW)
    report={'contract':contract,'source_material_counts':dict(sorted(Counter((v>>9)&63 for v in grid).items())),'native':[]}
    if len(sys.argv)==4:
        root=Path(sys.argv[3])
        require(sha((root/'route.jsonl').read_bytes())==reference['route_sha256'], 'fresh input itinerary identity')
        report['native']=[inspect(exported,root,label,contract) for label in LABELS]
        require(report['native']==reference['native'], 'native pins')
    print(json.dumps(report,indent=2))

if __name__=='__main__': main()
