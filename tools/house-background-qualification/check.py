"""Bounded ROM/source/native qualification; reports exclusions, never imports RAM as assets."""
from pathlib import Path
import hashlib
import json
import sys

HERE = Path(__file__).parent
ROOT = HERE.parent.parent
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
MAPS = (0xb,0xc,0xd,0xf,0x10,0x11)
# Source identities and audited COP3B geometry, not native slots or dumped grids.
FIXED = {0xb:[0x838ba0,0x838ba7],0xc:[],0xd:[0x838cc8],0xf:[0x838d4f],0x10:[0x838da6],0x11:[0x838dec]}
RESIDENTS = {0xb:[0x838b96],0xc:[0x838c0a,0x838c14,0x838c1e,0x838c28],0xd:[0x838cb4],0xf:[],0x10:[0x838d7c,0x838d86],0x11:[0x838de2]}
CHECKPOINTS = {'boot':0xf,'room10':0x10,'settledC':0xc,'north-door-settled':0xb,'settledD':0xd,'settled11':0x11,'wait11':0x11,'D-again':0xd,'returnedC':0xc}
SEGMENTS = {'boot':0xf,'room10':0x10,'settledC':0xc,'north-door-push':0xb,'settledD':0xd,'settled11':0x11}

def require(ok, message):
    if not ok: raise ValueError(message)

def u(b,p): return int.from_bytes(b[p:p+2], 'little')
def sha(b): return hashlib.sha256(b).hexdigest()
def words(b):
    require(len(b)%2 == 0, 'odd word plane')
    return [u(b,p) for p in range(0,len(b),2)]

def attributed_grid(cells, attributes):
    require(len(attributes)==512, 'attribute extent')
    return [(c&511) | ((attributes[c&511]&127)<<9) for c in cells]

def stamp(grid, position, geometry):
    require(len(grid)==2048, 'house grid extent')
    x,y = position; dx,width,dy,height = geometry
    nx,ny = width>>4,height>>4
    if nx+ny<3: points=[(x-8,y-16)]
    else: points=[(x+dx+16*col,y+dy+16*row) for row in range(ny) for col in range(nx)]
    for px,py in points:
        require(0<=px<512 and 0<=py<1024, 'stamp outside house')
        grid[(py//16)*32+px//16] |= 0x8000

def compare_grid(expected, native, actor_mask):
    require(len(expected)==len(native), 'grid extent')
    require(all(0<=i<len(expected) for i in actor_mask), 'mask extent')
    return [(i,a,b) for i,(a,b) in enumerate(zip(expected,native)) if (a^b)&(0x7fff if i in actor_mask else 0xffff)]

def sample(graphics, word, x, y):
    require(0<=x<8 and 0<=y<8 and len(graphics)>=(word&1023)*32+32, 'tile coordinate/extent')
    if word&0x4000: x=7-x
    if word&0x8000: y=7-y
    at=(word&1023)*32
    color=sum(((graphics[at+(p//2)*16+y*2+p%2]>>(7-x))&1)<<p for p in range(4))
    return (color+((word>>10)&7)*16, bool(word&0x2000)) if color else (0,False)

def camera_bounds(record):
    require(len(record)==2 and all(v>>4 and v!=255 for v in record), 'unqualified camera record')
    a,b=record
    return [(a&15)<<8,(b&15)<<8,((a&15)+(a>>4))<<8,((b&15)+(b>>4))<<8]

def animation_frames(rom, selector):
    """Audit only house selectors $0F/$10/$11, not a compact-script VM/timer."""
    require(selector in (0xf,0x10,0x11) and len(rom)>=0x1c0000, 'animation source extent')
    offset=u(rom,0x1b8000+selector*2)
    require(0<offset<0x8000, 'unqualified animation bank selector')
    at=0x1b8000+offset; result=[]
    for _ in range(16):
        require(at+8<=0x1c0000, 'animation record extent')
        repeats=rom[at]
        if not repeats: return result
        source=0x1b8000+u(rom,at+1); destination=u(rom,at+3)*2; size=u(rom,at+5)
        require(0<repeats<=8 and size==128 and destination=={0xf:0x4a0,0x10:0x520,0x11:0x720}[selector], 'unqualified animation transfer')
        require(source+repeats*size<=0x1c0000 and rom[at+7]>0, 'animation payload/delay')
        result.extend((source+i*size,destination,size) for i in range(repeats))
        at+=8
    raise ValueError('unterminated animation table')

def source_profile(rom, map_id, cells, attributes, first_f=False):
    require(map_id in MAPS, 'gated map')
    require(u(rom,0x28000+2*map_id)==0, 'scene table fallback')
    start=0x30000+u(rom,0x38000+2*map_id)
    require(rom[start:start+2]==bytes([0,6]), 'scene display selector')
    require(u(rom,0x16bb70)==0xbc1d and rom[0x16bc1d:0x16bc26]==bytes([0x17,0x12,0x82,0x21,0x64,0x80,9,0x11,0x11]), 'hardware profile')
    bounds=camera_bounds(rom[0x16be30+2*map_id:0x16be32+2*map_id])
    require(bounds[2]-bounds[0]==bounds[3]-bounds[1]==256, 'fixed camera bounds')
    grid=attributed_grid(cells,attributes)
    for source in FIXED[map_id]+RESIDENTS[map_id]:
        at=source&0x3fffff
        position=(rom[at+1]*16+8,rom[at+2]*16)
        geometry=(-8,32 if source==0x838da6 else 16,-16,16)
        stamp(grid,position,geometry)
    if first_f:
        require(map_id==0xf, 'F history on another map')
        # $8897C1 final stamp survives $8897C5 COPA7 unlink; not a live actor.
        stamp(grid,(392,256),(-8,16,-16,16))
    return grid,bounds

def inspect(rom, root, export, label, map_id):
    blobs={s:(root/f'{label}.{s}').read_bytes() for s in ('wram','vram','cgram')}
    w,v,c=(blobs[s] for s in ('wram','vram','cgram'))
    require([len(w),len(v),len(c)]==[131072,65536,512], 'capture extents')
    b={name:(export/name).read_bytes() for name in ('cells','static-grid','graphics','definitions','attributes','palette')}
    cells=words(b['cells'])
    expected,bounds=source_profile(rom,map_id,cells,b['attributes'],label=='boot')
    require(u(w,0x47e)==map_id, 'native map')
    camera=bounds[:2]
    require([u(w,0x81e),u(w,0x822)]==camera and [u(w,p) for p in range(0x874,0x87c,2)]==bounds and u(w,0x866)==256, 'source/native camera')
    require(w[0x468:0x46f]==bytes([0x17,0x12,0x82,0x21,0,0x3c,0x38]), 'hardware register shadows')
    require(u(w,0x980)&0x50==0, 'passive action-hook gate')
    # Fresh global flags; local wooden-door state is deliberately not conflated with globals.
    flags=[i for i in range(0x200) if w[0x6c0+i//8]&(1<<(i%8))]
    require(flags==[0x20,0xfb], 'fresh global events')
    require(b['definitions']==w[0x2000:0x3000], 'all 512 native definitions')
    attribute_differences=[(i,a,w[0x10000+i]) for i,a in enumerate(b['attributes']) if a!=w[0x10000+i]]
    require(all(i==0 for i,_,_ in attribute_differences), 'unexpected attribute change')
    require(b['palette'][2:]==c[2:256] and u(c,0)==0x1d6b, 'natural colors 1..127 / runtime backdrop')
    require(attributed_grid(cells,b['attributes'])==words(b['static-grid']), 'independent static grid')
    native=words(w[0xa000:0xb000])
    chain=[]; slot=u(w,0xdfc)
    while slot:
        require(0x1000<=slot<0x2000 and slot%64==0 and slot not in chain, 'linked entity chain')
        chain.append(slot); slot=u(w,slot+44)
    geometry_witnesses=[]
    for source in FIXED[map_id]+RESIDENTS[map_id]:
        if label=='D-again' and source in RESIDENTS[map_id]: continue
        at=source&0x3fffff; position=[rom[at+1]*16+8,rom[at+2]*16]
        matches=[p for p in chain if [u(w,p),u(w,p+2)]==position]
        require(len(matches)==1, 'unique linked source-origin footprint witness')
        p=matches[0]
        geometry=[int.from_bytes(w[0x10000+p+off:0x10002+p+off],'little',signed=True) for off in (0x28,0x2a,0x2c,0x2e)]
        require(geometry==[-8,32 if source==0x838da6 else 16,-16,16], 'native footprint geometry')
        geometry_witnesses.append({'source':source,'slot':p,'position':position,'geometry':geometry,'resume':w[p+12]*65536+u(w,p+10)})
    differences=compare_grid(expected,native,set())
    # No blanket bit-15 or low-word exclusion: retain every full-grid mismatch.
    # Post-door low-word deltas belong to navigation; later D deltas to wandering AI.
    changed_tiles=sorted({i//32 for i,(a,z) in enumerate(zip(b['graphics'],v)) if a!=z})
    reconstructed=bytearray(b['graphics']); animations=[]
    for selector in (0xf,0x10,0x11):
        frames=animation_frames(rom,selector)
        _,destination,size=frames[0]
        if v[destination:destination+size]==reconstructed[destination:destination+size]: continue
        matches=[source for source,dst,n in frames if rom[source:source+n]==v[dst:dst+n]]
        require(matches, 'native animation frame must match a source-table payload')
        source=matches[0]
        reconstructed[destination:destination+size]=rom[source:source+size]
        animations.append({'selector':selector,'source':source,'destination_byte':destination,'size':size})
    require(reconstructed==v[:len(reconstructed)], 'all 768 tiles including source-selected animation')
    high=equal_pixels=changed_pixels=mutated_words=0
    for y in range(camera[1]//8+2,camera[1]//8+26):
        for x in range(camera[0]//8+2,camera[0]//8+30):
            cell=(y//2)*32+x//2; quadrant=(y%2)*2+x%2
            word=u(b['definitions'],(native[cell]&511)*8+quadrant*2)
            require(u(v,2*(0x3800+(y%32)*32+x%32))==word, 'hardware BG2 full tile word')
            source_word=u(b['definitions'],cells[cell]*8+quadrant*2)
            mutated_words+=source_word!=word
            high+=bool(word&0x2000)
            for py in range(8):
                for px in range(8):
                    a=sample(b['graphics'],source_word,px,py); z=sample(v,word,px,py)
                    if a==z: equal_pixels+=1
                    else: changed_pixels+=1
    require(0<high<672, 'both BG priorities sampled')
    return {'map':map_id,'camera':camera,'files':{k:sha(b) for k,b in blobs.items()},'geometry_witnesses':geometry_witnesses,'attribute_differences':attribute_differences,'grid_differences':differences,'animation_frames':animations,'changed_graphics_tiles':changed_tiles,'tile_words':672,'high_words':high,'mutated_sector_words':mutated_words,'equal_index_priority_pixels':equal_pixels,'different_base_index_priority_pixels':changed_pixels}

def report(rom_path, native_path, run_path, returned_path):
    rom=Path(rom_path).read_bytes(); require(sha(rom)==ROM_SHA,'ROM identity')
    native=Path(native_path); run=Path(run_path)
    exports=json.loads((run/'export/export.json').read_text())
    require([e['map'] for e in exports]==list(MAPS), 'export admission set')
    for e in exports:
        require(e['files']==exports[0]['files'] and e['sources']==exports[0]['sources'], 'full natural sheet variant')
        for name,digest in e['files'].items(): require(sha((run/f"export/{e['map']:x}"/name).read_bytes())==digest, 'export plane pin')
        for s in e['sources']: require(sha(rom[s['start']:s['end']])==s['sha256'],'export source pin')
    contract=json.loads((HERE/'profiles.json').read_text())
    require(contract['rom_sha256']==ROM_SHA and contract['width_cells']==32 and contract['height_cells']==64, 'profile identity/dimensions')
    require(contract['global_event_bits']==[0x20,0xfb] and contract['hardware_background']==2 and contract['bgmode']==9 and contract['passive_action_mask_clear']==0x50, 'profile conditions')
    require([(p['map'],p['history']) for p in contract['profiles']]==[(i,h) for i in MAPS for h in (['post-intro-first-load','ordinary-load'] if i==15 else ['ordinary-load'])], 'profile coverage/history')
    raw_cells=words((run/'export/f/cells').read_bytes()); attrs=(run/'export/f/attributes').read_bytes()
    base=attributed_grid(raw_cells,attrs)
    require(contract['base_grid_sha256']==sha((run/'export/f/static-grid').read_bytes()), 'base grid contract')
    for p in contract['profiles']:
        grid,bounds=source_profile(rom,p['map'],raw_cells,attrs,p['history']=='post-intro-first-load')
        require(p['camera']==bounds[:2] and p['fixed_sources']==FIXED[p['map']] and p['resident_sources']==RESIDENTS[p['map']], 'source profile contract')
        require(p['added_bit15_cells']==[i for i,(a,b) in enumerate(zip(base,grid)) if a!=b] and p['grid_sha256']==sha(b''.join(v.to_bytes(2,'little') for v in grid)), 'source-compiled collision contract')
    traces={}
    for segment,map_id in SEGMENTS.items():
        directory=run/segment
        meta=json.loads((directory/'stops.json').read_text())
        require(meta['segment']==segment and [s['pc'] for s in meta['stops']]==[0x868d6b,0x868d6f,0x868d72], 'writer stops')
        for s in meta['stops']:
            require(s['map']==map_id and s['x']==0xb9 and s['db']==0x81 and s['p']&0x20 and s['dp']==0, 'writer context')
            if s['pc']!=0x868d6b: require(s['a']&255==9,'actual BGMODE write')
            for ext,key in [('wram','wram_sha256'),('pcs','pcs_sha256')]:
                require(sha((directory/f"{s['pc']:06x}.{ext}").read_bytes())==s[key], 'native writer pin')
        traces[segment]=meta
    captures={}
    for label,map_id in CHECKPOINTS.items():
        a=inspect(rom,native/'a',run/f'export/{map_id:x}',label,map_id)
        require(a==inspect(rom,native/'b',run/f'export/{map_id:x}',label,map_id), 'independent native runs')
        captures[label]=a
    returned=Path(returned_path)
    a=inspect(rom,returned/'a',run/'export/f','f7250',0xf)
    require(a==inspect(rom,returned/'b',run/'export/f','f7250',0xf), 'independent returned F runs')
    captures['returned-F']=a
    return {'exports':exports,'traces':traces,'captures':captures}

if __name__=='__main__':
    require(len(sys.argv)==5, 'check.py ROM CENSUS_REPLAY BACKGROUND_RUN RETURNED_F_REPLAY')
    reference=json.loads((HERE/'reference.json').read_text())
    for s in reference['sources']:
        data=Path(sys.argv[1]).read_bytes() if 'start' in s else (ROOT/s['path']).read_bytes()
        require(sha(data[s['start']:s['end']] if 'start' in s else data)==s['sha256'],'source consumer pin')
    actual=report(*sys.argv[1:])
    # JSON canonicalizes tuples to arrays.
    require(json.loads(json.dumps(actual))==reference['report'], 'qualification report changed')
    print('Six identical source backgrounds; hardware BG2/$09 confirmed. Full-grid deltas retained explicitly; see docs/house-backgrounds.md.')
