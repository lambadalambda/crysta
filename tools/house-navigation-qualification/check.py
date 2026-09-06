"""Bounded source/native navigation evidence, never a production initializer."""
from pathlib import Path
import hashlib
import json
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'house-scene-qualification'))
from census import exit_records, ROM_SHA

MAPS = (11, 12, 13, 15, 16, 17)
DEPARTURES = {0x84b975: 5, 0x84b9ae: 6, 0x84b9da: 7, 0x84ba06: 8}
VECTORS = {5:(0,1), 6:(0,-1), 7:(-1,0), 8:(1,0)}
SOURCE_RANGES = [(0x906f,0x90ac),(0x949b,0x9505),(0xbc2f,0xbc60),
    (0x489d1,0x48a40),(0x79180,0x79480),(0x79724,0x79845),
    (0x7c783,0x7c980),(0xd8c63,0xd8ce1),(0xd8df8,0xd8ed6),
    (0xd8797,0xd89b9),(0xf7f3,0xf815),(0x487fe,0x48820),
    (0x4b941,0x4bc2d)]


def require(ok, message):
    if not ok: raise ValueError(message)


def u(b, p): return int.from_bytes(b[p:p+2], 'little')
def sha(b): return hashlib.sha256(b).hexdigest()
def canonical(x): return json.dumps(x, sort_keys=True, separators=(',',':')).encode()
def events(w): return [8*i+j for i,b in enumerate(w[0x6c0:0x700]) for j in range(8) if b>>j&1]


def door_delta(before, after):
    require(len(before)==len(after)==131072, 'WRAM extent')
    for w in (before,after):
        require((u(w,0x47e),u(w,0x1000),u(w,0x1002))==(12,136,352), 'door target')
        require(events(w)==[32,251], 'door action must not grant progression')
    changes=[]
    for at in range(0xa000,0xb000,2):
        if before[at:at+2]!=after[at:at+2]:
            index=(at-0xa000)//2
            changes.append([index%32,index//32,u(before,at),u(after,at)])
    require(changes==[[8,19,0x1cf2,0x1cf6],[8,20,0x1cf3,0xf7]], 'exact two-cell door mutation')
    return changes


def source(rom):
    require(sha(rom)==ROM_SHA, 'Japanese ROM authentication')
    graph=[]
    for map_id in MAPS:
        exits,extent=exit_records(rom,map_id)
        graph.append({'map':map_id,'extent':extent,'sha256':sha(rom[slice(*extent)]),'exits':exits})
    return {'rom_sha256':ROM_SHA,'graph':graph,'ranges':[{'extent':[a,b],'sha256':sha(rom[a:b])} for a,b in SOURCE_RANGES]}


def selected(exits, position):
    x,y=position[0]-8,position[1]-16
    for e in exits:
        a,b,w,h=e['rectangle']
        if (((x>>4)-a)&255) < w and (((y>>4)-b)&255) < h:
            return e if 0<=x-a*16<w*16-15 and 0<=y-b*16<h*16-15 else None
    return None


def transitions(rom, rows):
    results=[]
    for i,row in enumerate(rows[1:],1):
        if row['script'] not in DEPARTURES or rows[i-1]['script'] in DEPARTURES: continue
        prev=rows[i-1]
        e=selected(exit_records(rom,prev['map'])[0],prev['position'])
        require(e is not None and e['selector']==DEPARTURES[row['script']], 'source ordered handoff')
        candidates=rows[i:i+200]
        init=next((r for r in candidates if r['map']==e['destination'] and r['script']==0x84a12e),None)
        require(init is not None, 'bounded destination initialization')
        at=0xd8985+e['selector']*4
        adjustment=[int.from_bytes(rom[at+j:at+j+2],'little',signed=True) for j in (0,2)]
        anchor=[e['position'][j]+adjustment[j]+(8,16)[j] for j in range(2)]
        require(init['position']==anchor, 'source queue-derived destination anchor')
        settled=next((r for r in candidates if r['frame']>init['frame'] and r['map']==e['destination'] and r['flags']==0x414),None)
        require(settled is not None,'bounded settled arrival')
        endpoint=[anchor[j]+17*VECTORS[e['selector']][j] for j in range(2)]
        require(settled['position']==endpoint, 'qualified 17-step arrival endpoint')
        results.append({'source':e['source'],'source_map':prev['map'],'handoff_frame':prev['frame'],
            'handoff':prev['position'],'destination':e['destination'],'anchor_frame':init['frame'],
            'anchor':anchor,'settled_frame':settled['frame'],'endpoint':endpoint})
    return results


# Ordinary movement ownership only: native entry/controller scheduling is not
# relabelled as walking latency. Each segment starts at its native setup (zero)
# phase. The core semantic doorway/action policy explicitly starts fresh epochs.
SEGMENTS = [
    ('a','settledC','westC',2),('a','westC','northC',1),
    ('a','north-door-wait','north-door-push',1),('a','north-door-settled','leaveB',0),
    ('a','settledD','southD',0),('a','southD','eastD',3),('a','wait11','returnD',2),
    ('extra-a','settledC','eastC',3),('extra-a','returned10','align11',3),
    ('extra-a','aligned11','south-to11',0),('extra-a','north11','back-to10',1),
]


def walking_rows(root, traces):
    csv=[]; pins=[]
    for run,checkpoint,label,direction in SEGMENTS:
        script=(0x84a351,0x84a367,0x84a385,0x84a385)[direction]
        rows=traces[run]
        start=next(i for i,r in enumerate(rows) if r['label']==label and r['script']==script and r['facing']==direction)
        chosen=[]
        for row in rows[start:]:
            if row['label']!=label or row['script']!=script or row['facing']!=direction: break
            require(row['control']&0x50==0 and row['flags']==0x415,'ordinary passive-collision admission')
            chosen.append(row)
        require(chosen and chosen[0]['position']==rows[start-1]['position'],'zero/setup movement phase')
        x,y=rows[start-1]['position']
        csv.append(f'S,{run},{checkpoint},{x},{y},{direction}')
        csv.extend(f'F,{r["frame"]},{r["position"][0]},{r["position"][1]}' for r in chosen)
        w=(root/run/(checkpoint+'.wram')).read_bytes()
        pins.append({'run':run,'checkpoint':checkpoint,'wram_sha256':sha(w),'label':label,
                     'first':chosen[0]['frame'],'last':chosen[-1]['frame'],'count':len(chosen)})
    return '\n'.join(csv)+'\n',pins


def report(rom, root):
    result=source(rom); traces={}; runs={}
    for run in ('a','b','extra-a','extra-b'):
        raw=(root/(run+'.jsonl')).read_bytes()
        entries=[json.loads(line) for line in raw.splitlines()]
        source_rows=[r for r in entries if r.get('kind')=='source-door']
        require(len(source_rows)==1,'source resource compilation')
        if run=='a': result['source_door']=source_rows[0]
        require(source_rows[0]==result['source_door'],'same source resource recipes')
        frames=[r for r in entries if r.get('kind')=='frame']
        require(frames[0]['frame']==6800 and frames[0]['map']==15 and frames[0]['position']==[304,112], 'fresh shared bootstrap')
        require(all(b['frame']==a['frame']+1 for a,b in zip(frames,frames[1:])), 'complete per-frame trace')
        require(set(r['map'] for r in frames)<=set(MAPS), 'closed house boundary')
        traces[run]=frames
        runs[run]={'trace_sha256':sha(raw),'transitions':transitions(rom,frames)}
    require(runs['a']==runs['b'] and runs['extra-a']==runs['extra-b'],'independent fresh replay equality')
    for run in ('a','b'):
        directory=root/run
        before=(directory/'settledB.wram').read_bytes()
        after=(directory/'north-door-wait.wram').read_bytes()
        door_delta(before,after)
        for w in (before,after):
            require(sha(w[0x2000:0x3000])==result['source_door']['resources'][0]['decoded_sha256'],'source door definitions')
            require(sha(w[0x10000:0x10200])==result['source_door']['resources'][1]['decoded_sha256'],'source door attributes')
        for tile,attribute in [(0xf6,14),(0xf7,0)]:
            require(after[0x10000+tile]&127==attribute,'replacement attribute reconstruction')
        old=(directory/'settledB.vram').read_bytes();new=(directory/'north-door-wait.vram').read_bytes()
        expected=[]
        for row,tile in [(19,0xf6),(20,0xf7)]:
            base=u(after,0x836)+((row*16&248)<<2)+16
            for j,delta in enumerate((0,1,32,33)):
                at=base+delta; expected.append(at)
                require(u(new,at*2)==u(after,0x2000+tile*8+j*2),'complete replacement visual descriptor')
        require([i//2 for i in range(0,len(new),2) if old[i:i+2]!=new[i:i+2]]==expected,'exact eight-word door VRAM mutation')
        for label in ('north-door-settled','returnedC','settledD','settled11','D-again','C-after-gate'):
            w=(directory/(label+'.wram')).read_bytes()
            require((u(w,0xa4d0),u(w,0xa510))==(0x1cf6,0xf7) and events(w)==[32,251], 'shared-sheet persistence, no0026')
        result['door']={'collision':door_delta(before,after),'before_wram_sha256':sha(before),
                       'after_wram_sha256':sha(after),'before_vram_sha256':sha(old),'after_vram_sha256':sha(new)}
    result['runs']={k:runs[k] for k in ('a','extra-a')}
    csv,result['walking']=walking_rows(root,traces)
    return result,csv


if __name__=='__main__':
    require(len(sys.argv)==3,'check.py ROM local/REPLAY')
    root=Path(sys.argv[2]); result,csv=report(Path(sys.argv[1]).read_bytes(),root)
    expected=json.loads(Path(__file__).with_name('reference.json').read_text())
    require(result==expected,'changed source/native qualification')
    (root/'walking.csv').write_text(csv)
    print(f'Qualified source exits, door mutation, visual words and {sum(p["count"] for p in result["walking"])} ordinary frames')
