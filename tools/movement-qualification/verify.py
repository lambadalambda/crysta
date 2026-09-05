"""Local falsifiable qualification model, NOT production support. All inputs/outputs ignored."""
import csv
import os
import json
from hashlib import sha256
from pathlib import Path
ROOT=Path('local/movement')
O={0,2,22}; S={12,14}
class Unqualified(Exception): pass
def word(w,i): return int.from_bytes(w[i:i+2],'little')
def material(w,x,y,house_materials=False,old_edge=False):
    # Only interior samples; map-edge neighbor behavior intentionally excluded.
    width=w[0x827]*16; height=(word(w,0x862)+1)//width
    col=x//16; row=y//16
    if not (0<=col<width and 0<=row<height): raise Unqualified('map boundary')
    raw=word(w,0xa000+2*(row*width+col)); t=(raw>>9)&31
    if house_materials:
        if old_edge and t in (6,7): raise Unqualified(f'old slope={t},raw={raw:04x}')
        if raw&0x8000: return 'S'  # Passive class-3 override, not an action hook.
        if t==16: return 'P'
    if raw&0x8000 or t not in O|S: raise Unqualified(f'cell={raw:04x},type={t}')
    return 'O' if t in O else 'S'
def admit_cardinal(inputs):
    """Conservative experiment guard, not a reconstruction of the dash cooldown."""
    seen=set(); previous=''
    for direction in inputs:
        if direction not in ('', 'Left', 'Right', 'Up', 'Down'):
            raise Unqualified(f'non-cardinal input: {direction}')
        if direction and direction!=previous:
            if direction in seen:
                raise Unqualified(f'reactivated direction: {direction}')
            seen.add(direction)
        previous=direction

def collide(w,x,y,dx,dy,flat_only=False,house_materials=False):
    if house_materials and word(w,0x980)&0x50:
        raise Unqualified('passive collision action-hook contract violated')
    def sample(u,v,old=False):
        return material(w,u,v,house_materials=house_materials,old_edge=old)
    for axis,d in [(0,dx),(1,dy)]:
        if not d: continue
        old=x if axis==0 else y
        # Conservatively reject unqualified old-edge cells too (6/7 divert before new dispatch).
        if axis==0:
            old_u=x-8 if d<0 else x+7; old_v=y-16
            sample(old_u,old_v,old=True)
            if old_v&15: sample(old_u,(old_v//16+1)*16,old=True)
        else:
            old_u=x-8; old_v=y-16 if d<0 else y-1
            sample(old_u,old_v,old=True)
            if old_u&15: sample((old_u//16+1)*16,old_v,old=True)
        if axis==0:
            x+=d; edge=x-8 if d<0 else x+8; u=edge-(d>0);v=y-16;q=v&15
            a=sample(u,v);b=sample(u,(v//16+1)*16) if q else a
        else:
            y+=d; edge=y-16 if d<0 else y;u=x-8;v=edge-(d>0);q=u&15
            a=sample(u,v);b=sample((u//16+1)*16,v) if q else a
        if flat_only and a!=b:
            raise Unqualified('mixed open/solid pair: corner response excluded')
        if a==b=='O':continue
        nudge=1 if (a,b) in {('S','O'),('P','O'),('S','P')} and q>=8 else -1 if (a,b) in {('O','S'),('O','P'),('P','S')} and q<8 else 0
        if axis==0:y+=nudge
        else:x+=nudge
        snap=bool(edge&8) if d<0 else not bool(edge&8)
        result=((edge&~15)+(24 if axis==0 else 32)) if d<0 else ((edge&~15)-(8 if axis==0 else 0))
        if not snap:result=old
        if axis==0:x=result
        else:y=result
    return x,y

def verify(name,start=1601,grid_frame=None,end=None,flat_only=False,house_materials=False):
    rows=[r for r in csv.DictReader(open(ROOT/name/'frames.csv'))
          if int(r['frame'])>=start and (end is None or int(r['frame'])<=end)]
    if flat_only or house_materials:
        admit_cardinal(r['input'] for r in rows[1:])
    w=(ROOT/name/f'f{grid_frame or rows[-1]["frame"]}.wram').read_bytes()
    delayed='';active='';age=0;qualified=[];excluded=[]
    for prev,r in zip(rows,rows[1:]):
        if (flat_only or house_materials) and int(r['flags'],16)&0x0406 != 0x0404:
            raise Unqualified(f"non-walking player flags at completed {r['frame']}")
        if delayed!=active:active=delayed;age=0
        else:age+=1
        step=0 if not active or age==0 or (active in ('Left','Right') and age%54==0 and not os.getenv('NAIVE_CADENCE')) else (1 if (age%54 if active in ('Left','Right') else age)%2 else 2)
        dx=step*({'Left':-1,'Right':1}.get(active,0));dy=step*({'Up':-1,'Down':1}.get(active,0))
        assert (dx,dy)==(int(r['outx']),int(r['outy'])),(name,r['frame'],'cadence',dx,dy,r['outx'],r['outy'])
        delayed=r['input']
        if prev['map']!=r['map']:raise Unqualified('transition')
        try:
            pred=collide(w,int(prev['x']),int(prev['y']),dx,dy,flat_only=flat_only,house_materials=house_materials)
            assert pred==(int(r['x']),int(r['y'])),(name,r['frame'],pred,r['x'],r['y'])
            qualified.append(int(r['frame']))
        except Unqualified as e:
            if flat_only or house_materials: raise  # Minimal profile fails closed; diagnostics may enumerate exclusions.
            excluded.append((r['frame'],str(e)))
    print(name,'matched',len(qualified),'excluded',len(excluded),excluded[:2])
    if not os.getenv('NAIVE_CADENCE'):
        artifact='house-qualification.json' if house_materials else 'flat-qualification.json' if flat_only else 'qualification.json'
        (ROOT/name/artifact).write_text(json.dumps(dict(start_frame=start, end_frame=int(rows[-1]['frame']), flat_only=flat_only, csv_sha256=sha256((ROOT/name/'frames.csv').read_bytes()).hexdigest(), grid_sha256=sha256(w).hexdigest(), matched_position_frames=qualified, rejected_position_frames=excluded),indent=2)+'\n')
    return len(qualified),len(excluded)

HOUSE_CASES = {
    'wall-Up': (1790, ['Up:1601:1780'], ''),
    'cadence': (1670, ['Right:1601:1611','Left:1611:1621','Up:1621:1631','Down:1631:1641','Right:1651:1652'], ''),
    'trace-flag': (1603, ['Up:1601:1780'], '80e1df,80e71b,80e2d0,80e2d6,80e32c,80d401'),
    'trace-partial': (1623, ['Right:1601:1611','Left:1611:1621','Up:1621:1631'], '80d3a4,80d3b6,80d3f1,80d3f7,80d401'),
}
HOUSE_PINS = {
    'wall-Up': ('12cdc28567f70e47fe5f76036a0658bd5700898a3f3ee5341a142a516e8bbb9b',
                '8290e49fcf5e7d2dbd7d5121df6e02ea4d2005dffdb8f9141358e33eab7ce47f'),
    'cadence': ('f892cb3dd8c29412ad2d21f43707583121fbfc87c07c9cd16798ccbedb6d39d5',
                '19f30fa29e888570bcacb199716c7f21287c5ce403366ba40ba3d99d13f912c4'),
}

HOUSE_HOOK_PINS = {
    'wall-Up': 'c2f176bfaeaffe91a562af0131e7618072f07882bdd857442819416e9f347a13',
    'cadence': 'f74236f8602a5102a3e692de0476da51a262689b44c3fea1bb5ed3859c618dfa',
    'trace-flag': '19dc7282d4d2e552a5e64cbec3befceb060b7c8e3eff4a29848116996cfed2bc',
    'trace-partial': '6a38641d2f67fbf1c8e3f34d486d2d083c98bf7c9bc9fa0cd5ce4c275a801fe9',
}
HOUSE_TRACE_CSV_PINS = {
    'trace-flag': '0bb435c12f98aab24f4c03a4bdc56922c371a0e150c0adfbe0460c03d62a0671',
    'trace-partial': 'e7602da9c94b18c6bca0c25329e5ab5783cbd562ceb279fd90a1b0e2830b91ca',
}

def material_source_checks(rom):
    # First-sample tables followed by O, P, S second-sample tables (NOT O,S,P).
    # Targets: pass, negative nudge test, positive nudge test, unconditional block.
    for base, entries, targets in [(0xd542,(0xd3ac,0xd3c0,0xd3b6),(0xd3ca,0xd3f1,0xd3e6,0xd3fd)),
                                   (0xd8e8,(0xd7a0,0xd7b4,0xd7aa),(0xd7be,0xd7d3,0xd7c8,0xd7de)),
                                   (0xdc60,(0xdb48,0xdb5c,0xdb52),(0xdb66,0xdb7b,0xdb70,0xdb86)),
                                   (0xdfdc,(0xdebe,0xded2,0xdec8),(0xdedc,0xdef1,0xdee6,0xdefc))]:
        clear, negative, positive, block = targets
        assert tuple(word(rom,base+2*t) for t in (0,12,16))==entries
        for offset in (0,64,128,192):
            assert word(rom,base+offset)==word(rom,base+offset+4)==word(rom,base+offset+44)
            assert word(rom,base+offset+6)==word(rom,base+offset+24)==word(rom,base+offset+28)
        for offset, expected in [(64,(clear,negative,negative)),
                                 (128,(positive,negative,block)),
                                 (192,(positive,block,positive))]:
            assert tuple(word(rom,base+offset+2*t) for t in (0,12,16))==expected
    for entry in (0xe2d0,0xe41c,0xe56e,0xe6bd):
        assert rom[entry:entry+7]==bytes.fromhex('ad 80 09 89 50 00 f0')


def capture_house_materials():
    """Own ignored helper derived from the existing probe; one fresh boot/process."""
    import subprocess
    root=Path('local/house-materials')
    assets=[('local/Tenchi Souzou (Japan).sfc','f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'),
            ('local/saves/Terranigma.srm','709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055')]
    for path,digest in assets: assert sha256(Path(path).read_bytes()).hexdigest()==digest,path
    material_source_checks(Path(assets[0][0]).read_bytes())
    source=Path('tools/movement-qualification/probe.rs').read_text()
    for anchor in ('    for f in 0..end {', '            let i = |j| u(j) as i16;',
                   '            let r = s.cpu_registers();'):
        assert source.count(anchor)==1, f'probe source drift: {anchor}'
    source=source.replace('    for f in 0..end {', '''    let mut hooks = std::fs::File::create(format!("{out}/hooks.csv")).unwrap();
    writeln!(hooks, "frame,status97c,action980").unwrap();
    for f in 0..end {''')
    source=source.replace('            let i = |j| u(j) as i16;', '''            let i = |j| u(j) as i16;
            writeln!(hooks, "{},{:04x},{:04x}", f+1, u(0x97c), u(0x980)).unwrap();''')
    source=source.replace('            let r = s.cpu_registers();', '''            assert_eq!(tr.stop, oracle::CpuTraceStop::TargetReached);
            let r = s.cpu_registers();''')
    project=root/'probe'; (project/'src').mkdir(parents=True,exist_ok=True)
    (project/'src/main.rs').write_text(source)
    (project/'Cargo.toml').write_text('''[package]
name = "house-material-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oracle = { path = "../../../crates/oracle" }
rom = { path = "../../../crates/rom" }
''')
    subprocess.run(['cargo','build','--release','--manifest-path',str(project/'Cargo.toml')],check=True)
    for run in ('first','second'):
        for name,(end,inputs,pcs) in HOUSE_CASES.items():
            out=root/run/name; out.mkdir(parents=True,exist_ok=True)
            env=dict(os.environ); env.pop('PCS',None)
            if pcs: env['PCS']=pcs
            with (out/'stops.txt').open('w') as log:
                subprocess.run([str(project/'target/release/house-material-probe'),str(out),str(end),*inputs],env=env,stdout=log,check=True)
    for path,digest in assets: assert sha256(Path(path).read_bytes()).hexdigest()==digest,path


def verify_house_materials():
    global ROOT
    ROOT=Path('local/house-materials/first')
    for name,(end,_,pcs) in HOUSE_CASES.items():
        first=ROOT/name; second=ROOT.parent/'second'/name
        expected={'frames.csv','hooks.csv','stops.txt','f1601.wram',f'f{end}.wram'}
        for index,pc in enumerate(pcs.split(',') if pcs else []):
            expected.update(f'step{index}-{pc}.{ext}' for ext in ('trace','wram'))
        for directory in (first,second):
            # Qualification summaries are derived, not raw capture artifacts.
            assert {p.name for p in directory.iterdir() if p.suffix!='.json'}==expected
        for artifact in expected: assert (first/artifact).read_bytes()==(second/artifact).read_bytes(),(name,artifact)
        if name in HOUSE_PINS:
            for artifact,pin in zip(('frames.csv',f'f{end}.wram'),HOUSE_PINS[name]):
                assert sha256((first/artifact).read_bytes()).hexdigest()==pin,(name,artifact)
        assert sha256((first/'hooks.csv').read_bytes()).hexdigest()==HOUSE_HOOK_PINS[name]
        if pcs:
            assert sha256((first/'frames.csv').read_bytes()).hexdigest()==HOUSE_TRACE_CSV_PINS[name]
            stops=(first/'stops.txt').read_text().splitlines()
            assert len(stops)==len(pcs.split(','))
            assert all(f' frame={end} ' in stop for stop in stops)
        hooks=list(csv.DictReader((first/'hooks.csv').open()))
        assert len(hooks)==end-1601+1
        for frame,hook in enumerate(hooks,1601):
            assert int(hook['frame'])==frame
            assert int(hook['status97c'],16)==0 and int(hook['action980'],16)&0x50==0,(name,frame,hook)
    flag=ROOT/'trace-flag'
    w=(flag/'step2-80e2d0.wram').read_bytes()
    assert word(w,0x980)&0x50==0 and word(w,0x97c)==0
    # The observed branch returns directly without action dispatch or script writes.
    path=(flag/'step4-80e32c.trace').read_text().splitlines()
    assert [line.split()[0] for line in path]==['80e32c']  # Resuming a stop executes that instruction before recording.
    partial=ROOT/'trace-partial'
    before=(partial/'step3-80d3f7.wram').read_bytes()
    after=(partial/'step4-80d401.wram').read_bytes()
    assert (word(before,0x1000),word(after,0x1000))==(459,458)
    assert word(before,0x1c)&15==3
    assert verify('wall-Up',house_materials=True)==(189,0)
    # Unique four-direction interval only: deliberately exclude 1651+.
    assert verify('cadence',end=1650,grid_frame=1670,house_materials=True)==(49,0)

if __name__=='__main__':
    import sys
    if '--capture-house-materials' in sys.argv: capture_house_materials()
    if '--house-materials' in sys.argv or '--capture-house-materials' in sys.argv:
        verify_house_materials()
        sys.exit(0)
    cases={'wall-Left':(189,0),'wall-Right':(189,0),'wall-Down':(189,0),
           'wall-Up':(11,178),'up-central':(179,0),'up-type12':(179,0),'cadence':(60,9),
           'map10-Left':(199,0),'map10-Right':(199,0),'map10-Down':(199,0),'map10-up-wall':(249,0),'corner-positive':(60,0),'corner-negative':(60,0)}
    for name,expected in cases.items():
        assert verify(name,1801 if name.startswith(('map10-','corner-')) else 1601)==expected
    # The quick-retap fixture is evidence of an unsupported action, not walking.
    try:
        verify('pulses')
    except AssertionError as e:
        assert e.args[0][:3]==('pulses','1608','cadence'),e
        print('pulses: correctly rejects accelerated/dash stream at completed frame 1608')
    else:
        raise AssertionError('dash must not be accepted as simple walking')

    # Smaller production candidate: no mixed-pair/corner handling at all.
    for name in ['wall-Left','wall-Right','wall-Down','up-central','up-type12','map10-Down']:
        assert verify(name,1801 if name.startswith(('map10-','corner-')) else 1601,flat_only=True)==cases[name]
    assert verify('doorway-approach',grid_frame=1601,flat_only=True)==(80,0)
    # The next frame is already transition-controlled, despite its one-pixel delta.
    try:
        verify('map10-Down',grid_frame=1601,end=1682,flat_only=True)
    except Unqualified as e:
        assert str(e)=='non-walking player flags at completed 1682',e
        print('doorway: walking ownership ends at completed 1681; rejects 1682')
    else:
        raise AssertionError('transition-controlled frame must not qualify as walking')
    for name in ['map10-Left','map10-Right','map10-up-wall']:
        try:
            verify(name,1801,flat_only=True)
        except Unqualified as e:
            assert str(e)=='mixed open/solid pair: corner response excluded',e
            print(name+': strict profile correctly stops at mixed wall pair')
        else:
            raise AssertionError(name+': mixed-pair trajectory must stay outside strict profile')
