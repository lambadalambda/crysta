"""Bounded source/export/hardware comparisons. All checks survive Python -O."""
from pathlib import Path
import hashlib
import json
import sys

ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
GAMMA = [0,1,3,6,10,15,21,28,36,45,55,66,78,91,105,120,136,144,152,160,168,176,184,192,200,208,216,224,232,240,248,255]

def require(ok, message):
    if not ok: raise ValueError(message)

def u(b, i): return int.from_bytes(b[i:i+2], 'little')
def sha(b): return hashlib.sha256(b).hexdigest()
def ptr(b): return int.from_bytes(b, 'little') & 0x3fffff

def tile(b):
    return bytes(sum(((b[(p//2)*16+y*2+p%2] >> (7-x)) & 1) << p for p in range(4)) for y in range(8) for x in range(8))

def verify_oam(oam, pattern, sizes):
    require(len(oam) == 544 and len(pattern) == len(sizes)*4, 'OAM extent')
    matches = [i for i in range(0,512-len(pattern)+1,4) if oam[i:i+len(pattern)] == pattern]
    require(len(matches) == 1, 'unique OAM component sequence')
    start = matches[0]//4
    for i,size in enumerate(sizes):
        slot=start+i
        high=(oam[512+slot//4] >> (2*(slot%4))) & 3
        require(high == (2 if size == 16 else 0), 'OAM size / ninth X bit')
    return start

def painter_order(ark_y, npc_y, native_list):
    require(native_list.count(0x1000)==1 and native_list.count(0x1040)==1, 'native pair missing/duplicated')
    return sorted([0x1000,0x1040], key=lambda e: (ark_y if e==0x1000 else npc_y,native_list.index(e)))

def linked_list(w):
    out=[]; p=u(w,0xdfc)
    while p:
        require(0x1000 <= p < 0x2000 and p%64==0 and p not in out, 'invalid entity chain')
        out.append(p); p=u(w,p+0x2c)
    return out

def inspect(r, root, export, f):
    blobs={s:(root/f'f{f}.{s}').read_bytes() for s in ('wram','vram','cgram','oam','obj','pixels')}
    w,v,c,oam,pixels=(blobs[s] for s in ('wram','vram','cgram','oam','pixels'))
    require([len(w),len(v),len(c),len(pixels)]==[131072,65536,512,512*480*4], 'capture extents')
    require(blobs['obj']==bytes([2,0]), 'OBJSEL / first sprite')
    require(u(w,0x47e)==16 and [u(w,0x81e),u(w,0x822)]==[256,256], 'room/camera')
    meta=json.loads((export/'export.json').read_text())
    pos=meta['position']; frame=(export/'composition').read_bytes(); raw=(export/'source-composition').read_bytes()
    require(pos==[u(w,0x1040),u(w,0x1042)] and u(w,0x1054)==3 and u(w,0x1048)&0x4000==0,'resident pose/position')
    require(w[0x104a:0x104d]==bytes([0xdc,0x98,0x88]) and u(w,0x11048)==2,'ordinary script/selector')
    require(u(w,0x1050)==0x7000 and w[0x1052]==0x7e and u(w,0x1104a)==0x7168,'relocated native pose')
    require(frame==w[0x7164:0x7164+len(frame)], 'ROM-derived relocated composition')
    require(raw[:17]==frame[:17] and len(raw)==len(frame), 'composition prefix extent')
    for a in range(17,len(frame),7):
        require(raw[a:a+5]==frame[a:a+5] and u(frame,a+5)==((u(raw,a+5)&~0xe00)|0xa00),'palette-only relocation')
    require(c[416:448]==r[0xc2b2c:0xc2b4c], 'natural palette source')
    tiles=(export/'tiles').read_bytes()
    require(len(tiles)==256*64 and tiles==b''.join(tile(v[a:a+32]) for a in range(0xa000,0xc000,32)), 'all 256 source tiles / VRAM')
    require([u(w,0x1058),u(w,0x105a)]==[frame[0],frame[2]], 'native anchors')
    image={}; expected=bytearray(); sizes=[]
    for i in range(frame[16]):
        b=frame[17+7*i:24+7*i]; word=u(b,5); tid=word&511; size=16 if b[0] else 8
        x=b[1]-frame[0]; y=b[3]-frame[2]
        require((word>>9)&7==5 and (word>>12)&3==2, 'palette5 / OBJ2')
        expected.extend([(pos[0]+x-256)&255,(pos[1]+y-256-1)&255,(tid+256)&255,((word&0xfe00)|(tid+256))>>8]); sizes.append(size)
        for dy in range(size):
            for dx in range(size):
                tx=size-1-dx if word&0x4000 else dx; ty=size-1-dy if word&0x8000 else dy
                color=tiles[(tid+tx//8+ty//8*16)*64+(ty%8)*8+tx%8]
                if color: image.setdefault((x+dx,y+dy),208+color)
    npc_slot=verify_oam(oam,expected,sizes)
    require(w[0xa00+4*npc_slot:0xa00+4*npc_slot+len(expected)]==expected,'WRAM draw staging')
    l,t,right,bottom=meta['bounds']
    indexed=bytes(image.get((x,y),0) for y in range(t,bottom) for x in range(l,right))
    require(indexed==(export/'indexed').read_bytes(),'independent shared raster composition')
    priorities=bytes(2 if i else 255 for i in indexed)
    require(priorities==(export/'priorities').read_bytes(),'export priority')
    def natural(index):
        color=u(c,index*2)
        return bytes(((color>>s&31)<<3)|((color>>s&31)>>2) for s in (0,5,10))
    rgba=b''.join(natural(i)+b'\xff' if i else bytes(4) for i in indexed)
    require(rgba==(export/'rgba').read_bytes(),'natural RGBA')
    for (x,y),index in image.items():
        # Fixed +8 ares output rows; stationary pose needs no guessed translation.
        at=((pos[1]+y-256+8)*512+(pos[0]+x-256)*2)*4
        color=u(c,index*2)
        require(tuple(pixels[at:at+3][::-1])==tuple(GAMMA[color>>s&31] for s in (0,5,10)),'opaque framebuffer pixel')
    native=linked_list(w)
    for entity in (0x1000,0x1040):
        require(u(w,entity+6)&0x7800==0 and u(w,0x11020+entity)==0 and u(w,entity+4)&0x8000==0,'ordinary depth eligibility')
    require(native.index(0x1040)<native.index(0x1000),'native tie order')
    # Ark ordinary frame's first component: source tile 0 is remapped to slot 0.
    ark_ptr=((w[0x1012]&63)<<16)+u(w,0x1100a)
    require(ark_ptr==0x24a54e and u(w,0x1008)&0x4000==0,'ordinary Ark witness')
    ark=r[ark_ptr-4:ark_ptr+13+r[ark_ptr+12]*7]; pattern=bytearray(); ids=[]; ark_sizes=[]
    for i in range(ark[16]):
        b=ark[17+i*7:24+i*7]; word=u(b,5); tid=word&511
        if tid not in ids: ids.append(tid)
        j=ids.index(tid); slot=j*2+(16 if j>=8 else 0)
        pattern.extend([(u(w,0x1000)+b[1]-ark[0]-256)&255,(u(w,0x1002)+b[3]-ark[2]-256-1)&255,slot,word>>8]); ark_sizes.append(16 if b[0] else 8)
    ark_slot=verify_oam(oam,pattern,ark_sizes)
    order=painter_order(u(w,0x1002),pos[1],native)
    require(order==[0x1000,0x1040] and npc_slot<ark_slot,'source depth / hardware order')
    return {'frame':f,'native_list':native,'npc_oam':npc_slot,'ark_oam':ark_slot,'opaque_equal':len(image),'painter_order':order,'hashes':{s:sha(b) for s,b in blobs.items()}}

def report(rom_path, directory):
    r=Path(rom_path).read_bytes(); require(sha(r)==ROM_SHA,'Japanese ROM authentication')
    root=Path(directory); export=root/'export'; meta=json.loads((export/'export.json').read_text())
    require(meta['map']==16 and meta['position']==[r[0x38d7d]*16+8,r[0x38d7e]*16] and meta['pose_key']==[0xd64b77,0x168] and meta['palette_base']==208,'source identity')
    for s in meta['sources']: require(sha(r[s['start']:s['end']])==s['sha256'],'source range hash')
    for name in ('indexed','rgba','priorities'): require(sha((export/name).read_bytes())==meta[name+'_sha256'],'export hash')
    runs=[]
    for run in ('a','b'):
        w=(root/run/'f6800.wram').read_bytes(); require(len(w)==131072 and u(w,0x47e)==15 and [u(w,0x1000),u(w,0x1002)]==[304,112],'fresh bootstrap checkpoint')
        runs.append([inspect(r,root/run,export,f) for f in (7050,7100,7200,7300)])
    require(runs[0]==runs[1],'two fresh processes differ')
    # Metadata/hashes only: never record palette, tile, composition or output bytes.
    meta.pop('colors')
    consumers=[(0xeae6,0xec1c),(0xec5d,0xec79),(0xf3f1,0xf5d0),(0xfa97,0xfb5a),(0xfc36,0xfc78),(0xfd85,0xfdb0),(0xfe8f,0xff0d)]
    return {'rom_sha256':ROM_SHA,'export':meta,'captures':runs[0],'consumer_ranges':[{'start':a,'end':b,'sha256':sha(r[a:b])} for a,b in consumers],'object_cpp_sha256':sha(Path('vendor/ares/ares/sfc/ppu/object.cpp').read_bytes())}

if __name__=='__main__':
    result=report(sys.argv[1],sys.argv[2])
    if len(sys.argv)==4 and sys.argv[3]=='--record':
        Path('tools/house-npc-qualification/reference.json').write_text(json.dumps(result,indent=2)+'\n')
    else:
        require(result==json.loads(Path('tools/house-npc-qualification/reference.json').read_text()),'reference metadata mismatch')
        print('House NPC: two fresh boots, four poses, source/OAM/VRAM/CGRAM/raster/order exact')
