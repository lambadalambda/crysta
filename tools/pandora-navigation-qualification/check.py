"""ROM-export/native edge-sample evidence. No CPU, pixels, capture initializers or writes."""
import hashlib
import json
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
ROM_SHA = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
def require(ok, message):
    if not ok:
        raise ValueError(message)
def sha(data):
    return hashlib.sha256(data).hexdigest()

# Every command between these boundaries belongs to its immutable source phase.
# Actions/presentation remain retained in the authenticated full log but are not
# claimed as ordinary motion. The checker reports its entire eligible sample count.
PHASES = [
    ('town-west', 'town-door-open', 'a'),
    ('enter-13', 'enter-13', 'a-north-open'),
    ('13-north', 'leave13-out', '13'),
    ('return-town-down', 'return-home-open-rest', 'a'),
    ('return-home-enter', 'return-home-enter', 'a-home-open'),
    ('return-home-C28', 'return-home-C28', 'd-return'),
    ('pot-corridor-right', 'pot-first-A', 'c-direct'),
    ('carry-first-down', 'left-pot-A', 'c-after-miss'),
    ('left-carry-down', 'door-throw-hit1', 'c-held-fa'),
    ('middle-route-down', 'middle-grab', 'c-first-hit'),
    ('middle-carry-right', 'door-throw-hit2', 'c-held-fb'),
    ('stairs-E', 'stairs-E', 'c-departed'),
    ('E-left', 'stairs-20', 'e'),
    ('20-left', 'stairs-21', '20'),
    ('21-toward-box', '21-near-box', '21'),
    ('21-box-contact', '21-open-contact', '21-contact'),
    ('pandora-left', 'pandora-neutral-stable', '41-control'),
]
# Use a settled capture of the SAME source phase, never the destination sheet at
# the end of a transition command. Native grids are evidence, not source outputs.
WITNESSES = {'a':'town-door-ready','a-north-open':'town-door-open','a-home-open':'return-home-open-rest',
    '13':'13-target-ready','d-return':'return-home-D28','c-direct':'C-direct-ready',
    'c-after-miss':'door-hit1','c-held-fa':'left-pot-held','c-first-hit':'door-real-hit1-closed',
    'c-held-fb':'middle-held','c-departed':'door-passable','e':'landed-E','20':'landed-20',
    '21':'21-entry-closed','21-contact':'21-box-contact-rest','41-control':'pandora-tour-control'}
SCRIPTS = {0x84a351,0x84a367,0x84a385,0x84b50d,0x84b51e,0x84b533}
DIRECTIONS = {'Down':(0,1),'Up':(0,-1),'Left':(-1,0),'Right':(1,0)}

def samples(position, direction):
    x,y = position
    require(x >= 8 and y >= 16, 'sample anchor bounds')
    horizontal = direction in ('Left','Right')
    u,v = {'Up':(x-8,y-16),'Down':(x-8,y-1),'Left':(x-8,y-16),'Right':(x+7,y-16)}[direction]
    result = [(u//16,v//16)]
    if (v if horizontal else u) % 16:
        result.append((u//16,v//16+1) if horizontal else (u//16+1,v//16))
    return result

def classify(m,cell,raw,d,old,control):
    require(control in (0,0x20,0xa0),'unqualified sample mode')
    kind=(raw>>9)&31
    require(not old or kind not in (6,7),'old edge slope')
    if raw&0x8000:return 'solid'
    if kind in (0,2,22):return 'open'
    if kind in (12,14) or (kind==25 and m==10):return 'solid'
    if kind==16:return 'partial'
    lane=d=='Up' and {12:(11,21),14:(6,53),32:(22,53)}.get(m)==cell
    if lane and raw==0x3acb:return 'open'
    if lane and m==12 and raw==0x0b81:return 'partial'
    raise ValueError('unqualified sample material')

def compare_sample(profile, grid, native, cell):
    x,y = cell
    left,top,right,bottom = profile['halo']
    require(left <= x < right and top <= y < bottom, 'sample outside admission')
    i = y*profile['width']+x
    expected, actual = grid[i], native[i]
    if expected == actual:
        return False
    # Deliberate source-frozen bird collision, never all bit15 or all actors.
    frozen = {(a['position'][0]//16,(a['position'][1]-16)//16)
              for a in profile['actors'] if a['source'] in (0x838a19,0x838a23,0x838a2d)}
    require(profile['map'] == 10 and cell in frozen and expected == (actual | 0x8000)
            and not actual & 0x8000, f'non-source sample difference {profile["name"]} {cell}: {expected:04x}/{actual:04x}')
    return True

def check_transition(row, spec):
    require(row['map'] == spec['destination'] and row['position'] == spec['settled'], 'settled transition endpoint')
    require(row['script'] == 0x84a258 and row['control'] == 0, 'settled ordinary control')

def report(export, capture):
    metadata_bytes = (export/'export.json').read_bytes()
    meta = json.loads(metadata_bytes)
    require(meta['rom_sha256'] == ROM_SHA and meta['schema'] == 1, 'source export identity')
    profiles = {p['name']:p for p in meta['profiles']}
    require(set(profiles) == set(WITNESSES), 'source profile membership')
    route_bytes = (HERE.parent/'pandora-qualification/route.jsonl').read_bytes()
    commands = [json.loads(l) for l in route_bytes.splitlines()]
    log_bytes = capture.with_suffix('.jsonl').read_bytes()
    rows = [json.loads(l) for l in log_bytes.splitlines()]
    # Existing source-owned schedule validation only; no observer metadata import.
    sys.path.insert(0,str(HERE.parent/'pandora-qualification'))
    import importlib.util
    spec=importlib.util.spec_from_file_location('pandora_source_schedule',HERE.parent/'pandora-qualification/check.py')
    checker=importlib.util.module_from_spec(spec);spec.loader.exec_module(checker)
    checker.timeline(commands,rows)
    cps = {r['label']:r for r in rows if r['kind']=='checkpoint'}
    buttons = {c['label']:c['buttons'] for c in commands if 'label' in c}
    labels = list(buttons)
    phase = {}; phase_ids={}; coverage={}
    for first,last,name in PHASES:
        coverage[first]={'sampled_frames':0,'excluded_script':0,'excluded_command':0,'excluded_transfer':0}
        for label in labels[labels.index(first):labels.index(last)+1]:
            require(label not in phase,'overlapping phase admission')
            phase[label] = name; phase_ids[label]=first
    grids, natives, witnesses = {},{},{}
    for name,p in profiles.items():
        data = (export/f'{name}.grid').read_bytes()
        require(sha(data) == p['grid_sha256'] and len(data) == p['width']*p['height']*2,'compiled grid extent/hash')
        grids[name] = struct.unpack(f'<{len(data)//2}H',data)
        w = (capture/f'{WITNESSES[name]}.wram').read_bytes()
        require(len(w)==131072 and int.from_bytes(w[0x47e:0x480],'little')==p['map'],'native witness map/extent')
        cp=cps[WITNESSES[name]]
        u=lambda a:int.from_bytes(w[a:a+2],'little')
        require([u(0x1000),u(0x1002)]==cp['position'] and u(0x980)==cp['control'] and u(0x47e)==cp['map'],'witness log agreement')
        events={i for i in range(1024) if w[0x6c0+i//8]>>(i%8)&1}
        require(sorted(i for i in events if i<512)==cp['events'],'witness event projection agreement')
        require((u(0x100a)|(w[0x100c]<<16))==cp['script'],'witness script agreement')
        if p['map']==12:
            require({0x28,0x2e}.issubset(events) and not {0x2f,0x3f,0x42}.intersection(events),'direct C witness branch')
        if name in ('c-departed','e','20'):require(0x292 in events,'opened shared phase')
        if name=='21-contact':require(1 in events and 0x22 not in events,'closed contact phase')
        if name=='41-control':require({0x243,0x244}.issubset(events),'completed tour witness')
        natives[name] = struct.unpack(f'<{len(data)//2}H',w[0xa000:0xa000+len(data)])
        witnesses[name] = sha(w)
    totals = {name:{'frames':0,'samples':0,'frozen_source_only':0,'kinds':set(),'cells':set(),'frozen_differences':{}} for name in profiles}
    for r in rows:
        if r['kind']!='frame' or r['label'] not in phase:continue
        audit=coverage[phase_ids[r['label']]]
        if r['script'] not in SCRIPTS:
            audit['excluded_script']+=1;continue
        name = phase[r['label']]
        if name=='21-contact' and 1 not in r['events']:name='21'
        p=profiles[name]
        if r['map']!=p['map']:
            require({'enter-13':19,'leave13-out':10,'return-home-enter':13,'return-home-C28':12,'stairs-E':14,'stairs-20':32,'stairs-21':33}.get(r['label'])==r['map'],'unexpected map mismatch')
            audit['excluded_transfer']+=1;continue
        bs = buttons[r['label']]
        if len(bs)!=1 or bs[0] not in DIRECTIONS:
            audit['excluded_command']+=1;continue # not a cadence claim
        require(r['control'] in (0,0x20,0xa0), 'unqualified movement control')
        d=bs[0]; dx,dy=DIRECTIONS[d]; t=totals[name];t['frames']+=1;audit['sampled_frames']+=1
        # Observed edge plus both possible one/two-pixel forward candidates. This
        # is conservative sample coverage, NOT reconstruction of native velocity.
        for step in (0,1,2):
            position=(r['position'][0]+dx*step,r['position'][1]+dy*step)
            for cell in samples(position,d):
                frozen=compare_sample(p,grids[name],natives[name],cell)
                raw=grids[name][cell[1]*p['width']+cell[0]]
                classify(p['map'],cell,raw,d,step==0,r['control'])
                if frozen:
                    actor=next(a for a in p['actors'] if (a['position'][0]//16,(a['position'][1]-16)//16)==cell)
                    key=(cell[0],cell[1],raw,natives[name][cell[1]*p['width']+cell[0]],actor['source'])
                    t['frozen_differences'][key]=t['frozen_differences'].get(key,0)+1
                t['samples']+=1;t['frozen_source_only']+=frozen;t['kinds'].add((raw>>9)&31);t['cells'].add(cell)
    for t in totals.values():
        require(t['samples']>0,'empty source profile sample coverage')
        t['kinds']=sorted(t['kinds']);t['cells']=[list(c) for c in sorted(t['cells'])]
        t['frozen_differences']=[{'cell':[k[0],k[1]],'source_word':k[2],'native_word':k[3],'actor':k[4],'occurrences':v} for k,v in sorted(t['frozen_differences'].items())]
    require(all(c['sampled_frames']>0 for c in coverage.values()),'empty declared phase')
    transition_labels={0x818d6b:'return-home-D28',0x818d8f:'landed-13',0x818eac:'returned-A-from13',
        0x818dfe:'landed-A',0x818e0a:'return-home-C28-rest',0x818df1:'landed-E',0x818e2f:'landed-20',0x818fc1:'landed-21'}
    require(len(meta['transfers'])==8 and {s['source'] for s in meta['transfers']}==set(transition_labels),'exact required exit identities')
    transitions=[]
    for spec in meta['transfers']:
        label=transition_labels[spec['source']]
        check_transition(cps[label],spec)
        transitions.append({'source':spec['source'],'label':label,'frame':cps[label]['frame'],'position':cps[label]['position']})
    contact_rows=[r for r in rows if r['kind']=='frame' and r['label']=='21-box-contact']
    bounds=meta['first_contact']['bounds']
    inside=lambda p,b: b[0]<=p[0]<=b[2] and b[1]<=p[1]<=b[3]
    first=next(i for i,r in enumerate(contact_rows) if inside(r['position'],bounds))
    require(contact_rows[first]['position']==[136,370] and 1 not in contact_rows[first]['events'],'first actual contact boundary')
    require(all(1 not in r['events'] and r['map']==33 for r in contact_rows[:first+1]),'premature contact callback')
    callback=contact_rows[first+1]
    require(callback['position']==[136,369] and 1 in callback['events'] and callback['script']==0x8480c4,'queued callback then recoil')
    require(cps['21-box-contact-rest']['position']==[136,359],'settled recoil endpoint')
    require({1,2}.issubset(cps['21-warning-complete-neutral']['events']) and 0x22 not in cps['21-warning-complete-neutral']['events'] and not inside(cps['21-warning-complete-neutral']['position'],meta['box_opening']['bounds']),'neutral outside opening gate')
    require(cps['21-opening']['map']==33 and cps['21-opening']['position']==[136,368] and 0x22 in cps['21-opening']['events'],'forced box reload')
    approach=[r for r in rows if r['kind']=='frame' and r['label']=='21-open-contact']
    eligible=next((r for r in approach if inside(r['position'],meta['box_opening']['bounds'])),None)
    require(eligible is not None and eligible['position']==[136,368] and {1,2}.issubset(eligible['events']) and 0x22 not in eligible['events'],'pre-reload positive opening witness')
    contact={'opening_first_eligible':eligible['frame'],'first':contact_rows[first]['frame'],'callback_recoil':callback['frame'],'rest':cps['21-box-contact-rest']['position']}
    return {'schema':1,'contact':contact,'export_sha256':sha(metadata_bytes),'route_sha256':sha(route_bytes),'log_sha256':sha(log_bytes),
            'wram':witnesses,'witness_labels':WITNESSES,'comparison_policy':'fixed settled native phase grids versus conservative observed-position edge samples; not per-frame grids or native cadence','phase_coverage':coverage,'samples':totals,'transitions':transitions}

if __name__ == '__main__':
    require(len(sys.argv)==4,'usage: check.py EXPORT CAPTURE_A CAPTURE_B')
    result=report(Path(sys.argv[1]),Path(sys.argv[2]))
    require(result==report(Path(sys.argv[1]),Path(sys.argv[3])),'independent native roots differ')
    require(result==json.loads((HERE/'reference.json').read_text()),'strict navigation metadata reference')
    print('Navigation source/native samples and transitions match both roots; frozen differences/pacing limits retained.')
