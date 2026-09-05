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
def material(w,x,y):
    # Only interior samples; map-edge neighbor behavior intentionally excluded.
    width=w[0x827]*16; height=(word(w,0x862)+1)//width
    col=x//16; row=y//16
    if not (0<=col<width and 0<=row<height): raise Unqualified('map boundary')
    raw=word(w,0xa000+2*(row*width+col)); t=(raw>>9)&31
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

def collide(w,x,y,dx,dy,flat_only=False):
    for axis,d in [(0,dx),(1,dy)]:
        if not d: continue
        old=x if axis==0 else y
        # Conservatively reject unqualified old-edge cells too (6/7 divert before new dispatch).
        if axis==0:
            old_u=x-8 if d<0 else x+7; old_v=y-16
            material(w,old_u,old_v)
            if old_v&15: material(w,old_u,(old_v//16+1)*16)
        else:
            old_u=x-8; old_v=y-16 if d<0 else y-1
            material(w,old_u,old_v)
            if old_u&15: material(w,(old_u//16+1)*16,old_v)
        if axis==0:
            x+=d; edge=x-8 if d<0 else x+8; u=edge-(d>0);v=y-16;q=v&15
            a=material(w,u,v);b=material(w,u,(v//16+1)*16) if q else a
        else:
            y+=d; edge=y-16 if d<0 else y;u=x-8;v=edge-(d>0);q=u&15
            a=material(w,u,v);b=material(w,(u//16+1)*16,v) if q else a
        if flat_only and a!=b:
            raise Unqualified('mixed open/solid pair: corner response excluded')
        if a==b=='O':continue
        nudge=1 if a=='S' and b=='O' and q>=8 else -1 if a=='O' and b=='S' and q<8 else 0
        if axis==0:y+=nudge
        else:x+=nudge
        snap=bool(edge&8) if d<0 else not bool(edge&8)
        result=((edge&~15)+(24 if axis==0 else 32)) if d<0 else ((edge&~15)-(8 if axis==0 else 0))
        if not snap:result=old
        if axis==0:x=result
        else:y=result
    return x,y

def verify(name,start=1601,grid_frame=None,end=None,flat_only=False):
    rows=[r for r in csv.DictReader(open(ROOT/name/'frames.csv'))
          if int(r['frame'])>=start and (end is None or int(r['frame'])<=end)]
    if flat_only:
        admit_cardinal(r['input'] for r in rows[1:])
    w=(ROOT/name/f'f{grid_frame or rows[-1]["frame"]}.wram').read_bytes()
    delayed='';active='';age=0;qualified=[];excluded=[]
    for prev,r in zip(rows,rows[1:]):
        if flat_only and int(r['flags'],16)&0x0406 != 0x0404:
            raise Unqualified(f"non-walking player flags at completed {r['frame']}")
        if delayed!=active:active=delayed;age=0
        else:age+=1
        step=0 if not active or age==0 or (active in ('Left','Right') and age%54==0 and not os.getenv('NAIVE_CADENCE')) else (1 if (age%54 if active in ('Left','Right') else age)%2 else 2)
        dx=step*({'Left':-1,'Right':1}.get(active,0));dy=step*({'Up':-1,'Down':1}.get(active,0))
        assert (dx,dy)==(int(r['outx']),int(r['outy'])),(name,r['frame'],'cadence',dx,dy,r['outx'],r['outy'])
        delayed=r['input']
        if prev['map']!=r['map']:raise Unqualified('transition')
        try:
            pred=collide(w,int(prev['x']),int(prev['y']),dx,dy,flat_only=flat_only)
            assert pred==(int(r['x']),int(r['y'])),(name,r['frame'],pred,r['x'],r['y'])
            qualified.append(int(r['frame']))
        except Unqualified as e:
            if flat_only: raise  # Minimal profile fails closed; diagnostics may enumerate exclusions.
            excluded.append((r['frame'],str(e)))
    print(name,'matched',len(qualified),'excluded',len(excluded),excluded[:2])
    if not os.getenv('NAIVE_CADENCE'):
        artifact='flat-qualification.json' if flat_only else 'qualification.json'
        (ROOT/name/artifact).write_text(json.dumps(dict(start_frame=start, end_frame=int(rows[-1]['frame']), flat_only=flat_only, csv_sha256=sha256((ROOT/name/'frames.csv').read_bytes()).hexdigest(), grid_sha256=sha256(w).hexdigest(), matched_position_frames=qualified, rejected_position_frames=excluded),indent=2)+'\n')
    return len(qualified),len(excluded)
if __name__=='__main__':
    cases={'wall-Left':(189,0),'wall-Right':(189,0),'wall-Down':(189,0),
           'wall-Up':(11,178),'up-central':(179,0),'up-type12':(179,0),'cadence':(60,9),
           'map10-Left':(199,0),'map10-Right':(199,0),'map10-Down':(199,0),'map10-up-wall':(249,0)}
    for name,expected in cases.items():
        assert verify(name,1801 if name.startswith('map10-') else 1601)==expected
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
        assert verify(name,1801 if name.startswith('map10-') else 1601,flat_only=True)==cases[name]
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
