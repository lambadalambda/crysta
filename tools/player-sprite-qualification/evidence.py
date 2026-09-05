"""Experimental reproducible ROM/WRAM/VRAM comparison; no capture atlas."""
if not __debug__:
    raise RuntimeError('Python optimization disables qualification; run without -O/PYTHONOPTIMIZE')

from pathlib import Path
import hashlib, json, sys

def u(b, i): return int.from_bytes(b[i:i+2], 'little')
def digest(b): return hashlib.sha256(b).hexdigest()
def tile(b):
    return [sum(((b[(p//2)*16+y*2+p%2] >> (7-x)) & 1) << p for p in range(4)) for y in range(8) for x in range(8)]
GAMMA = [0,1,3,6,10,15,21,28,36,45,55,66,78,91,105,120,136,144,152,160,168,176,184,192,200,208,216,224,232,240,248,255]
def rgb(v): return tuple(GAMMA[v>>s & 31] for s in (0,5,10))
def inspect(r, directory, f):
    root = Path(directory)/f'f{f}'
    w,v,c,pixels = (root.with_suffix('.'+s).read_bytes() for s in ('wram','vram','cgram','pixels'))
    ptr = ((w[0x1012]&63)<<16)+u(w,0x1100a)
    resource = u(w,0x13016)
    source = int.from_bytes(r[0xa252+6*resource:0xa255+6*resource], 'little')&0x3fffff
    mirror = bool(u(w,0x1008)&0x4000)
    anchor = [r[ptr-4+mirror],r[ptr-2]]
    assert anchor == [u(w,0x1018),u(w,0x101a)]
    n = r[ptr+12]
    components=[]; ids=[]; expected=[]; image={}
    camera = (256,256 if u(w,0x47e)==16 else 0)
    for i in range(n):
        a=ptr+13+7*i; b=r[a:a+7]; word=u(b,5); tid=word&511
        if tid not in ids: ids.append(tid)
        j=ids.index(tid); slot=j*2+(16 if j>=8 else 0)
        for delta in (0,16):
            assert r[source+(tid+delta)*32:source+(tid+delta)*32+64] == v[0x8000+(slot+delta)*32:0x8000+(slot+delta)*32+64], (f,i,tid,delta,'VRAM')
        x,y=b[1+mirror]-anchor[0],b[3]-anchor[1]
        attr=(word&0xfe00)^(0x4000 if mirror else 0)
        expected.append(bytes([(u(w,0x1000)+x-camera[0])&255,(u(w,0x1002)+y-camera[1]-1)&255,slot,attr>>8]))
        size=16 if b[0]&1 else 8
        components.append({'offset':[x,y],'size':size,'tile':tid,'word':word,'slot':slot})
        for dy in range(size):
            for dx in range(size):
                tx=size-1-dx if attr&0x4000 else dx
                ty=size-1-dy if attr&0x8000 else dy
                t=tid+(tx//8)+(ty//8)*16
                color=tile(r[source+t*32:source+t*32+32])[(ty%8)*8+tx%8]
                if color: image.setdefault((x+dx,y+dy),128+((word>>9)&7)*16+color)
    pattern=b''.join(expected)
    oam_start=w[0xa00:0xc00].find(pattern)
    assert oam_start>=0 and oam_start%4==0,(f,'OAM composition')
    for i,comp in enumerate(components):
        slot=oam_start//4+i; high=(w[0xc00+slot//4]>>(2*(slot%4)))&3
        assert high>>1 == (comp['size']==16)
    assert c[256:288]==r[0x31d831:0x31d851]
    hw=root.with_suffix('.oam').read_bytes()
    assert root.with_suffix('.obj').read_bytes()==bytes([2,0])
    assert hw[oam_start:oam_start+len(pattern)]==pattern
    for i in range(n):
        slot=oam_start//4+i
        assert ((hw[512+slot//4]>>(2*(slot%4)))&3)==((w[0xc00+slot//4]>>(2*(slot%4)))&3)
    # Separate output surface: report a bounded translation comparison, not a latch claim.
    matches=[]
    for shift in range(4,13):
      for shift_x in range(-4,5):
        equal=0
        for (x,y),index in image.items():
            sx=u(w,0x1000)+x-camera[0]+shift_x; sy=u(w,0x1002)+y-camera[1]+shift
            pos=(sy*512+sx*2)*4
            equal += tuple(pixels[pos:pos+3][::-1])==rgb(u(c,index*2))
        matches.append((equal,shift_x,shift))
    _,shift_x,shift=max(matches)
    observed=bytes(channel for (x,y) in sorted(image) for channel in pixels[((u(w,0x1002)+y-camera[1]+shift)*512+(u(w,0x1000)+x-camera[0]+shift_x)*2)*4:((u(w,0x1002)+y-camera[1]+shift)*512+(u(w,0x1000)+x-camera[0]+shift_x)*2)*4+3])
    return {'observed_opaque_rgb_sha256':digest(observed),'frame':f,'map':u(w,0x47e),'pointer':hex(ptr|0x800000),'resource':resource,'mirror':mirror,'anchor':anchor,'components':components,'oam_start':oam_start//4,'opaque_pixels':len(image),'best_image_translation':list(max(matches)),'indexed_sha256':digest(bytes(image.get((x,y),0) for y in range(-48,16) for x in range(-32,32))),'hashes':{s:digest(root.with_suffix('.'+s).read_bytes()) for s in ('wram','vram','cgram','oam','obj')}}

if __name__=='__main__':
    r=Path(sys.argv[1]).read_bytes()
    assert digest(r)=='f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
    report=[inspect(r,sys.argv[2],f) for f in [6800,6805,6815,6825,6890,6905,6915,6925,7050,7105,7114,7250,7315,7325,7360]]
    print(json.dumps(report,indent=2))
