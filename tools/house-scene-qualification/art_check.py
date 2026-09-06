"""Independent ROM-export / fresh native art checks; metadata only is committed."""
from pathlib import Path
import json
import sys
from census import ROM_SHA, checkpoint, require, sha, u
from check import RESIDENTS, canonical, hardware_census

GAMMA = [0,1,3,6,10,15,21,28,36,45,55,66,78,91,105,120,136,144,152,160,168,176,184,192,200,208,216,224,232,240,248,255]

def tile(b):
    require(len(b)==32,'planar tile extent')
    return bytes(sum(((b[(p//2)*16+y*2+p%2]>>(7-x))&1)<<p for p in range(4)) for y in range(8) for x in range(8))

def parts(c):
    require(len(c)>=17 and 0<c[16]<=128 and len(c)==17+7*c[16],'composition extent')
    return [c[i:i+7] for i in range(17,len(c),7)]

def relocate(c,source_palette,target_palette):
    out=bytearray(c)
    for i,_ in enumerate(parts(c)):
        at=22+i*7
        out[at:at+2]=((u(c,at)-source_palette*512+target_palette*512)&65535).to_bytes(2,'little')
    return bytes(out)

def raster(c,tiles,hflip):
    require(len(tiles)==256*64,'decoded tile extent')
    h=int(hflip);ax=c[h] if c[h]<128 else c[h]-256;ay=c[2]
    image={};rects=[]
    for b in parts(c):
        require(b[0] in (0,1),'component size')
        size=8<<b[0];word=u(b,5);tid=word&511
        require(tid<256 and (size==8 or (tid%16<15 and tid<240)),'unsupported tile boundary')
        x=b[1+h]-ax;y=b[3]-ay;rects.append((x,y,x+size,y+size))
        for dy in range(size):
            for dx in range(size):
                tx=size-1-dx if bool(word&0x4000)^hflip else dx
                ty=size-1-dy if word&0x8000 else dy
                color=tiles[(tid+tx//8+ty//8*16)*64+(ty%8)*8+tx%8]
                if color:image.setdefault((x+dx,y+dy),(128+(word>>9&7)*16+color,word>>12&3))
    l=min(r[0] for r in rects);t=min(r[1] for r in rects);r=max(r[2] for r in rects);b=max(r[3] for r in rects)
    values=[image.get((x,y),(0,255)) for y in range(t,b) for x in range(l,r)]
    return [l,t,r,b],bytes(v[0] for v in values),bytes(v[1] for v in values)

def rgba(indexed,palette,base):
    def color(index):
        require(base<index<base+16,'palette index')
        word=u(palette,(index-base)*2)
        return bytes(((word>>s&31)<<3)|(word>>s&31)>>2 for s in (0,5,10))+b'\xff'
    return b''.join(color(i) if i else bytes(4) for i in indexed)

def exported(rom,root):
    meta=json.loads((root/'export.json').read_text());actors=meta['actors']
    require(len(actors)==10 and len({a['id'] for a in actors})==10,'complete unique roster')
    expected={v for m in RESIDENTS.values() for v in m.values()}|{0x88d618}
    require({a['id'] for a in actors}==expected,'admitted source IDs')
    for a in actors:
        d=root/f"{a['id']:06x}";tiles=(d/'tiles').read_bytes();palette=(d/'palette').read_bytes()
        require(len(palette)==32,'palette extent')
        for name,data in [('tiles',tiles),('palette',palette)]:require(sha(data)==a[name+'_sha256'],'resource hash')
        for source in a['sources']:
            require(0<=source['start']<source['end']<=len(rom),'source extent')
            require(sha(rom[source['start']:source['end']])==source['sha256'],'ROM source hash')
        for f in a['frames']:
            blobs={name:(d/f"{f['index']}-{name}").read_bytes() for name in ('composition','source-composition','indexed','rgba','priorities')}
            for name,data in blobs.items():require(sha(data)==f[name.replace('-','_')+'_sha256'],'frame hash')
            c=blobs['composition'];raw=blobs['source-composition']
            source_palette=u(raw,22)>>9&7
            require(relocate(raw,source_palette,(a['palette_base']-128)//16)==c,'palette-only relocation')
            bounds,indexed,priorities=raster(c,tiles,a['hflip'])
            require(bounds==f['bounds'] and indexed==blobs['indexed'] and priorities==blobs['priorities'],'independent raster')
            require(sum(i!=0 for i in indexed)==f['opaque'],'opaque count')
            require(rgba(indexed,palette,a['palette_base'])==blobs['rgba'],'independent natural RGBA')
    return meta

LABELS=('boot','room10','north-door-settled','settled11','wait11',
        'settledC-sample00','settledC-sample03','settledC-sample19','wait11-sample19')

def inspect(rom,root,export,meta):
    reports=[];seen=set()
    by_id={a['id']:a for a in meta['actors']}
    for label in LABELS:
        c=hardware_census(checkpoint(rom,root,label))
        w=(root/f'{label}.wram').read_bytes();v=(root/f'{label}.vram').read_bytes()
        cg=(root/f'{label}.cgram').read_bytes();pixels=(root/f'{label}.pixels').read_bytes()
        owners={o['entity']:o for o in c['oam_owners']};entities=c['entities']
        admitted=[];witnesses=[]
        for e in entities:
            identity=RESIDENTS[c['map']].get(e['slot'])
            if c['map']==15 and e['composition']==0xa2f0ac:identity=0x88d618
            if identity is None:continue
            a=by_id[identity];d=export/f'{identity:06x}';admitted.append((e,a))
            require(e['position']==a['position'] and e['selector']==a['selector'],'native source setup/selector')
            require(bool(e['flags'][2]&0x4000)==a['hflip'] and e['flags'][2]&0x8000==0,'native actor flip')
            palette=(d/'palette').read_bytes();base=a['palette_base']
            # The native scene updates unused resident color 14 to white.
            # Preserve the ROM palette in the API; never hide a used-color mismatch.
            for color in range(16):
                expected=0x7fff if base==192 and color==14 else u(palette,color*2)
                require(u(cg,(base+color)*2)==expected,'native CGRAM palette / unused white override')
            tiles=(d/'tiles').read_bytes()
            # F shares scene graphics with Ark's dynamic low tile uploads.
            tids=range(0xad,0xb0) if identity==0x88d618 else range(256)
            origin=0x8000 if identity==0x88d618 else 0xa000
            for tid in tids:require(tile(v[origin+tid*32:origin+(tid+1)*32])==tiles[tid*64:(tid+1)*64],'native source-indexed VRAM tile')
            for f in a['frames']:
                frame=(d/f"{f['index']}-composition").read_bytes()
                if 'packet' in f['key']:
                    at=(e['animation_base']&65535)+f['key']['offset']-4
                    require(e['animation_base']>>16==0x7e and w[at:at+len(frame)]==frame,'native relocated bounded list composition')
                else:
                    at=(f['key']['direct']&0x3fffff)-4
                    require(rom[at:at+len(frame)]==frame,'native direct composition')
                if owners[e['slot']]['composition_sha256']!=f['composition_sha256']:continue
                require(e['facing']==f['facing'],'native list facing')
                seen.add((identity,f['index']))
                bounds,indexed,_=raster(frame,tiles,a['hflip']);l,t,right,bottom=bounds
                require(base!=192 or 206 not in indexed,'native palette override must be unused')
                equal=0;total=0;isolated=0
                for y in range(t,bottom):
                    for x in range(l,right):
                        index=indexed[(y-t)*(right-l)+x-l]
                        if not index:continue
                        sx=e['position'][0]+x-c['camera'][0];sy=e['position'][1]+y-c['camera'][1]
                        require(0<=sx<256 and 0<=sy<224,'isolated witness on screen')
                        at=((sy+8)*512+sx*2)*4;word=u(cg,index*2)
                        same=tuple(pixels[at:at+3][::-1])==tuple(GAMMA[word>>s&31] for s in (0,5,10))
                        equal+=same;total+=1
                        # Fixed masks, not masks inferred from pixel equality:
                        # seated lower bodies are behind the table; F's right
                        # side is not qualified against final BG/effect blending.
                        exposed=(y < -8 if identity in (0x838c1e,0x838c28) else x < 3 if identity==0x88d618 else True)
                        if exposed:
                            require(same,'isolated opaque framebuffer pixel');isolated+=1
                require(isolated>0,'nonempty framebuffer witness')
                witnesses.append({'id':identity,'record':f['index'],'isolated_pixels_equal':isolated,'opaque_pixels_equal':equal,'opaque_pixels':total})
        ranked=sorted(admitted,key=lambda pair:pair[1]['tie_rank'])
        native=[e['slot'] for e in entities if e['slot']==0x1000 or any(e['slot']==p[0]['slot'] for p in admitted)]
        require(native==[e['slot'] for e,_ in ranked]+[0x1000],'per-map source tie ranks including Ark')
        require(all(a['ark_tie_rank']==len(admitted) for _,a in admitted),'Ark tie rank')
        reports.append({'label':label,'hardware_sha256':sha(canonical(c)),'witnesses':witnesses})
    expected={(a['id'],f['index']) for a in meta['actors'] if a['id']!=0x838cb4 for f in a['frames']}
    require(seen==expected,'every displayed bounded list record witnessed')
    return reports

def setup(root,export,meta):
    report=json.loads((root/'setup.json').read_text());entity=report['entity']
    require(entity==0x1040,'D creation entity')
    require([s['pc'] for s in report['stops']]==[0x80f5d3,0x80ed99,0x80ee11,0x80f5e4,0x88a83c],'creation trace stages')
    for stop in report['stops']:
        for name in ('wram','pcs'):require(sha((root/f"{stop['pc']:06x}.{name}").read_bytes())==stop[name+'_sha256'],'setup capture hash')
        require(stop['x']==entity,'creation stays on the selected source entity')
    a=next(a for a in meta['actors'] if a['id']==0x838cb4)
    c=next(a for a in meta['actors'] if a['id']==0x838c28)
    require(a['position']==[72,672] and a['selector']==3 and not a['hflip'],'D creation export contract')
    require(a['frames']==c['frames'],'D art uses the independently witnessed C list')
    w=(root/'80f5e4.wram').read_bytes()
    require([u(w,entity),u(w,entity+2)]==a['position'],'D completed creation origin')
    require([u(w,entity+i) for i in (4,6,8)]==[0x5100,0,0x100],'D initial flags')
    require(u(w,entity+20)==0 and u(w,entity+0x10008)==3 and u(w,entity+30)==0,'D initial facing/selector/timer')
    require(w[entity+10:entity+13]==bytes([0x3c,0xa8,0x88]),'D has not executed its first AI instruction')
    require(u(w,entity+16)==0x7000 and w[entity+18]==0x7e,'D relocated packet base')
    require(u(w,entity+0x1000a)==0x7163,'D initialized first composition')
    for f in a['frames']:
        frame=(export/'838cb4'/f"{f['index']}-composition").read_bytes();at=0x7000+f['key']['offset']-4
        require(w[at:at+len(frame)]==frame,'D created list relocation')
    frame=(export/'838cb4'/'0-composition').read_bytes()
    require([u(w,entity+24),u(w,entity+26)]==[frame[0],frame[2]],'D initial anchors')
    return report

def report(rom_path,directory):
    rom=Path(rom_path).read_bytes();require(sha(rom)==ROM_SHA,'Japanese ROM authentication')
    root=Path(directory);meta=exported(rom,root/'export')
    runs=[inspect(rom,root/run,root/'export',meta) for run in ('a','b')]
    require(runs[0]==runs[1],'two independent fresh art runs differ')
    creation=[setup(root/run,root/'export',meta) for run in ('setup-a','setup-b')]
    require(creation[0]==creation[1],'two independent fresh D creation runs differ')
    return {'rom_sha256':ROM_SHA,'export':meta,'captures':runs[0],'creation':creation[0],
            'itinerary_sha256':sha(Path('tools/house-scene-qualification/art-route.jsonl').read_bytes()),
            'palette_override_sources':[{'start':a,'end':b,'sha256':sha(rom[a:b])} for a,b in [(0xdac09,0xdac37),(0x5f98f,0x5f9cb)]]}

if __name__=='__main__':
    result=report(sys.argv[1],sys.argv[2]);reference=Path('tools/house-scene-qualification/art-reference.json')
    if len(sys.argv)==4 and sys.argv[3]=='--record':reference.write_text(json.dumps(result,indent=2)+'\n')
    else:
        require(result==json.loads(reference.read_text()),'committed art reference mismatch')
        print('House art: ten source actors; all bounded list records / OAM / tiles / used colors / isolated pixels; D creation-only pose')
