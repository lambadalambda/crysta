"""Source/linked-entity/OAM census. Experimental observations, not a game VM."""
from pathlib import Path
import hashlib
import json
import sys

MAPS = (0xB,0xC,0xD,0xE,0xF,0x10,0x11,0x20,0x21)
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'

def require(ok, message):
    if not ok: raise ValueError(message)

def u(b,p): return int.from_bytes(b[p:p+2],'little')
def sha(b): return hashlib.sha256(b).hexdigest()

def exit_records(rom,map_id):
    table=0x18000+map_id*2
    require(len(rom)>=table+2,'truncated exit table')
    offset=u(rom,table); require(offset>=0x8000,'invalid exit pointer')
    start=at=0x10000+offset; records=[]
    for _ in range(32):
        require(at<min(len(rom),0x20000),'missing exit terminator')
        if rom[at]==255: return records,[start,at+1]
        require(at+12<=min(len(rom),0x20000),'bank-crossing/truncated exit')
        b=rom[at:at+12]
        require(b[2]>0 and b[3]>0 and b[6]==0 and u(b,4)<0x8000,'unsupported house exit shape')
        records.append({'source':at|0x800000,'rectangle':list(b[:4]),'destination':u(b,4),'mode':b[6],'selector':b[7],'position':[u(b,8),u(b,10)]})
        at+=12
    raise ValueError('unbounded exit list')

def native_entities(w):
    require(len(w)==131072,'WRAM extent')
    def entity(p):
        return {'slot':p,'position':[u(w,p),u(w,p+2)],'flags':[u(w,p+i) for i in (4,6,8)],'resume':w[p+12]*65536+u(w,p+10),'animation_base':w[p+18]*65536+u(w,p+16),'facing':u(w,p+20),'composition':w[p+18]*65536+u(w,p+0x1000a),'selector':u(w,p+0x10008),'cursor':u(w,p+32),'resource':u(w,p+0x12016),'draw_override':u(w,p+0x11020)}
    chain=[]; p=u(w,0xdfc)
    while p:
        require(0x1000<=p<0x2000 and p%64==0 and p not in chain,'invalid/cyclic native list')
        chain.append(p);p=u(w,p+44)
    # Unlinked nonzero records are diagnostic remnants, NOT present actors.
    unlinked=[entity(p) for p in range(0x1000,0x2000,64) if p not in chain and any(w[p:p+64])]
    return [entity(p) for p in chain],unlinked

def oam_ownership(rom,w,oam,entities):
    require(len(oam)==544,'OAM extent')
    owners=[]; occupied=set()
    for e in entities:
        p=e['slot'];pointer=e['composition'];bank=pointer>>16; offset=pointer&65535;flags=e['flags'][2]
        if offset==0 or e['flags'][0]&0x8000: continue
        require(bank==0x7e or bank>=0x80,'unsupported composition bank')
        memory=w if bank==0x7e else rom
        at=offset if bank==0x7e else pointer&0x3fffff
        require(4<=at and at+13<=len(memory),'composition header extent')
        count=memory[at+12]; require(0<count<=128,'composition count')
        b=memory[at-4:at+13+count*7];require(len(b)==17+count*7,'composition extent')
        h=bool(flags&0x4000);v=bool(flags&0x8000);ax=b[h];ax=ax-256 if ax>127 else ax;ay=b[2+v]
        require([u(w,p+24),u(w,p+26)]==[ax&65535,ay],'native anchors')
        pattern=bytearray();highs=[];ids=[];priorities=set();palettes=set()
        for i in range(count):
            c=b[17+i*7:24+i*7];require(c[0] in (0,1),'component size')
            word=u(c,5);tid=word&511
            if p==0x1000:
                if tid not in ids:ids.append(tid)
                j=ids.index(tid);tid=j*2+(16 if j>=8 else 0)
            else:tid=(tid+(256 if flags&0x100 else 0))&511
            x=(e['position'][0]+c[1+h]-ax-u(w,0x81e))&511
            y=(e['position'][1]+c[3+v]-ay-u(w,0x822)-1)&255
            attr=(word&0xfe00)^(flags&0xc000)
            pattern.extend([x&255,y,tid&255,(attr|tid)>>8]);highs.append((x>>8)|(c[0]<<1))
            priorities.add(word>>12&3);palettes.add(word>>9&7)
        matches=[i for i in range(0,512-len(pattern)+1,4) if oam[i:i+len(pattern)]==pattern]
        require(len(matches)==1,f'unmatched/ambiguous entity OAM {p:04x}')
        first=matches[0]//4;slots=set(range(first,first+count));require(not slots&occupied,'overlapping ownership')
        for i,high in enumerate(highs):
            slot=first+i;require((oam[512+slot//4]>>(2*(slot%4)))&3==high,'OAM ninth-X/size')
        require(w[0xa00+first*4:0xa00+first*4+len(pattern)]==pattern,'WRAM draw staging')
        occupied.update(slots)
        owners.append({'entity':p,'first_oam':first,'components':count,'priorities':sorted(priorities),'palettes':sorted(palettes),'composition_sha256':sha(b)})
    active={i for i in range(128) if oam[i*4:i*4+4]!=bytes([224]*4)}
    require(occupied<=active,'inactive OAM claimed')
    auxiliary=[{'slot':i,'x':oam[i*4]+((oam[512+i//4]>>(i%4*2))&1)*256,'y':oam[i*4+1],'tile':oam[i*4+2]+(oam[i*4+3]&1)*256,'palette':oam[i*4+3]>>1&7,'priority':oam[i*4+3]>>4&3,'size':16 if (oam[512+i//4]>>(i%4*2))&2 else 8} for i in sorted(active-occupied)]
    verify_ordinary_order(entities,owners,u(w,0x822))
    return owners,auxiliary

def verify_ordinary_order(entities,owners,camera_y):
    # Ordinary inverse-Y buckets prepend ties; neither shadow nor text uses this path.
    by_slot={e['slot']:(ordinal,e) for ordinal,e in enumerate(entities)}
    ordinary=[o for o in owners if by_slot[o['entity']][1]['flags'][1]&0x7800==0 and by_slot[o['entity']][1]['draw_override']==0]
    require(all(0<=by_slot[o['entity']][1]['position'][1]-camera_y<256 for o in ordinary),'ordinary depth domain')
    expected=sorted(ordinary,key=lambda o:(by_slot[o['entity']][1]['position'][1],by_slot[o['entity']][0]),reverse=True)
    require([o['entity'] for o in expected]==[o['entity'] for o in sorted(ordinary,key=lambda o:o['first_oam'])],'ordinary depth/tie OAM order')

def verify_auxiliary(rom,w,oam,auxiliary):
    require(u(w,0x1080e)==0,'unqualified auxiliary world-OBJ queue')
    extent=u(w,0x10810)
    if extent==0:
        require(not auxiliary,'unexplained OAM without a text producer')
        return
    require(u(w,0x47e)==13 and extent==24,'unqualified text overlay extent/map')
    pointer=int.from_bytes(w[0x10806:0x10809],'little')
    require(pointer==0x838cda,'unqualified scene-label source')
    require(rom[pointer&0x3fffff:(pointer&0x3fffff)+8]==bytes([0x80,0xa2,0x82,0x56,0x53,0x80,0x47,0xd4]),'changed scene-label encoding')
    expected=[]
    for i in range(4):
        x=128-4*12//2+i*12;tile=176+i*2
        require(w[0x10992+i*6:0x10998+i*6]==bytes([x,0,48,0,tile,52]),'scene-label native staging')
        require(oam[i*4:i*4+4]==bytes([x,48,tile,52]),'scene-label OAM pass')
        require((oam[512+i//4]>>(2*(i%4)))&3==2,'scene-label ninth-X/size')
        expected.append({'slot':i,'x':x,'y':48,'tile':tile,'palette':2,'priority':3,'size':16})
    require(auxiliary==expected,'unexplained OAM beyond qualified scene label')

def checkpoint(rom,root,label):
    blobs={s:(root/f'{label}.{s}').read_bytes() for s in ('wram','vram','cgram','oam','obj','pixels')}
    w=blobs['wram'];require(blobs['obj']==bytes([2,0]),'OBJSEL/first sprite')
    require(len(blobs['vram'])==65536 and len(blobs['cgram'])==512 and len(blobs['pixels'])==512*480*4,'surface extents')
    entities,unlinked=native_entities(w)
    owners,aux=oam_ownership(rom,w,blobs['oam'],entities)
    verify_auxiliary(rom,w,blobs['oam'],aux)
    return {'label':label,'map':u(w,0x47e),'camera':[u(w,0x81e),u(w,0x822)],'event_bits':[i*8+j for i,b in enumerate(w[0x6c0:0x700]) for j in range(8) if b>>j&1],'entities':entities,'unlinked_nonzero_not_members':unlinked,'oam_owners':owners,'auxiliary_oam':aux,'hashes':{s:sha(b) for s,b in blobs.items()}}

def source_graph(rom):
    result=[]
    for map_id in MAPS:
        records,extent=exit_records(rom,map_id)
        result.append({'map':map_id,'table':0x818000+map_id*2,'extent':extent,'sha256':sha(rom[extent[0]:extent[1]]),'exits':records})
    require({e['destination'] for s in result for e in s['exits'] if e['destination'] not in MAPS}=={0xA,0x122},'house boundary changed')
    return result

if __name__=='__main__':
    rom=Path(sys.argv[1]).read_bytes();require(sha(rom)==ROM_SHA,'Japanese ROM authentication')
    root=Path(sys.argv[2]);labels=sys.argv[3:]
    print(json.dumps({'rom_sha256':ROM_SHA,'graph':source_graph(rom),'checkpoints':[checkpoint(rom,root,label) for label in labels]},indent=2))
