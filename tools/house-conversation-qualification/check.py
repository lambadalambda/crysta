"""Check one fresh input-only journey; no native state loads or capture seeding."""
import json
import sys
from pathlib import Path
from source import ROM_SHA, project, require, sha, u

ROOT = Path(__file__).resolve().parent
# Expected observations, not production timeline/data. Hex values are request cursors.
# label: map, position, phase, cursor, selection, event26
EXPECT = {
    'boot': (15, [304,112], 'idle', None, 0, False),
    'north-door-settled': (11, [120,191], 'page', 0x8fef, 0, False),
    'entry-no-ack': (11, [120,128], 'idle', 0x8fef, 0, False),
    'entry-after-A': (11, [120,128], 'page', 0x9027, 0, False),
    'first-no-ack': (11, [120,128], 'page', 0x9027, 0, False),
    'first-A1': (11, [120,128], 'page', 0x9027, 0, False),
    'first-page2': (11, [120,128], 'choice', 0x9047, 0, True),
    'choice-no-confirm': (11, [120,128], 'choice', 0x9047, 0, True),
    'first-followup': (11, [120,128], 'page', 0x90f8, 0, True),
    'followup-no-ack': (11, [120,128], 'page', 0x90f8, 0, True),
    'first-complete': (11, [120,128], 'page', 0x90f8, 0, True),
    'repeat-wrong-wait': (11, [120,128], 'page', 0x9126, 0, True),
    'repeat-start': (11, [120,128], 'page', 0x9155, 0, True),
    'followup-closed': (11, [120,128], 'page', 0x9155, 0, True),
    'wrong-facing-settled': (11, [120,128], 'idle', 0x9155, 0, True),
    'repeat-page': (11, [120,128], 'choice', 0x917e, 0, True),
    'repeat-choice-second': (11, [120,128], 'choice', 0x917e, 1, True),
    'repeat-cancel-page': (11, [120,128], 'page', 0x91eb, 1, True),
    'repeat-after-L': (11, [120,128], 'page', 0x9217, 1, True),
    'repeat-after-A': (11, [120,128], 'idle', 0x9217, 0, True),
    'away-negative': (11, [120,128], 'idle', 0x9217, 0, True),
    'repeat2-choice': (11, [120,128], 'choice', 0x917e, 0, True),
    'repeat2-followup': (11, [120,128], 'page', 0x91aa, 0, True),
    'repeat2-page2': (11, [120,128], 'page', 0x91d5, 0, True),
    'repeat2-closed': (11, [120,128], 'idle', 0x91d5, 0, True),
    'open-D': (13, [120,625], 'idle', None, 0, True),
    'exit-trigger': (13, [120,721], 'idle', None, 0, True),
    'exit-loading': (10, [120,738], 'idle', None, 0, True),
    'landed-A': (10, [504,769], 'idle', None, 0, True),
    'exterior-walk-down-settled': (10, [504,815], 'idle', None, 0, True),
    'exterior-walk-settled': (10, [538,815], 'idle', None, 0, True),
}

def checkpoint(root, row):
    label = row['label']
    w = (root / f'{label}.wram').read_bytes()
    require(len(w) == 131072, 'WRAM extent')
    bank = u(w, 0xdc2)
    linked = []
    p = u(w, 0xdfc)
    while p:
        require(0x1000 <= p < 0x2000 and p % 64 == 0 and p not in linked, 'native list cycle/extent')
        linked.append(p)
        p = u(w, p + 44)
    actors = [{'slot': p, 'position': [u(w,p),u(w,p+2)],
               'script': u(w,p+10) | w[p+12] << 16} for p in linked]
    result = {key: row[key] for key in ('label','frame','map','position','camera','script','facing','events')}
    require(result['map'] == u(w,0x47e) and result['position'] == [u(w,0x1000),u(w,0x1002)], 'checkpoint/log mismatch')
    require(result['camera'] == [u(w,0x81e),u(w,0x822)] and result['facing'] == u(w,0x1014)
            and result['script'] == (u(w,0x100a) | w[0x100c] << 16), 'checkpoint player mismatch')
    require(result['events'] == [i for i in range(512) if w[0x6c0+i//8] >> (i%8) & 1], 'checkpoint event mismatch')
    result.update(phase='choice' if bank == 0xffff else 'page' if bank & 255 else 'idle',
                  cursor=u(w,0xdc0), selection=u(w,0xdce), gate_cell=u(w,0xab0e),
                  gate_continuation_present=any(a['script'] == 0x88a9bc for a in actors),
                  linked_sha256=sha(json.dumps(actors,sort_keys=True).encode()))
    if label == 'entry-no-ack':
        candidates = [p for p in linked if u(w,0x10020+p) == 0x8ede and w[p+12] == 0x88]
        require(len(candidates) == 1, 'B callback identity')
        p = candidates[0]
        result['target'] = {'position':[u(w,p),u(w,p+2)], 'policy':u(w,p+6),
                            'rectangle':[u(w,0x10000+p+k) for k in (0x28,0x2a,0x2c,0x2e)]}
    result['hashes'] = {ext: sha((root/f'{label}.{ext}').read_bytes()) for ext in ('state','wram','vram','cgram','pixels','oam','obj')}
    return result

def validate(result):
    points = result['checkpoints']
    require(set(points) == set(EXPECT), 'missing/extra semantic checkpoints')
    for label, (map_id, position, phase, cursor, selection, granted) in EXPECT.items():
        p = points[label]
        require(p['map'] == map_id and p['position'] == position, f'{label}: endpoint')
        require(p['phase'] == phase and (cursor is None or p['cursor'] == cursor), f'{label}: dialogue wait')
        require(p['selection'] == selection, f'{label}: choice selection')
        require(p['events'] == ([32,38,251] if granted else [32,251]), f'{label}: event timing/effects')
    require(points['away-negative']['facing'] == 0, 'wrong-facing negative control')
    d = points['open-D']
    require(d['gate_cell'] == 0x592, 'D gate occupancy survived post-grant load')
    require(not d['gate_continuation_present'], 'D hidden gate still linked')
    require(points['entry-no-ack']['target'] == {'position':[120,112], 'policy':0x200,
            'rectangle':[0xfff8,16,0xfff0,16]}, 'B target rectangle/policy')
    require(points['landed-A']['camera'] == [376,657], 'landing camera')
    require(points['exterior-walk-settled']['camera'] == [410,703], 'walking camera')
    require(result['first_grant'] == {'frame':8409,'label':'first-page2'}, 'grant before required acknowledgement or after choice')
    require(result['transition'] == [
        {'frame':11674,'map':13,'position':[120,722],'script':0x84b975},
        {'frame':11690,'map':10,'position':[120,738],'script':0x84b975},
        {'frame':11723,'map':10,'position':[0,0],'script':0},
        {'frame':11724,'map':10,'position':[504,752],'script':0x84a12e},
        {'frame':11760,'map':10,'position':[504,769],'script':0x84a258},
    ], 'departure/load/spawn/settled distinction')

def report(rom_path, root):
    rom = Path(rom_path).read_bytes()
    require(sha(rom) == ROM_SHA, 'owned Japanese ROM authentication')
    require(project(rom) == json.loads((ROOT/'source.json').read_text()), 'source metadata mismatch')
    root = Path(root)
    route = (ROOT/'route.jsonl').read_bytes()
    require(route == (root/'route.jsonl').read_bytes(), 'retained route mismatch')
    commands = [json.loads(line) for line in route.splitlines()]
    require(commands[-1] == {'finish': True}, 'missing finish')
    rows = [json.loads(line) for line in root.with_suffix('.jsonl').read_text().splitlines()]
    checkpoints = [r for r in rows if r['kind'] == 'checkpoint']
    frames = [r for r in rows if r['kind'] == 'frame']
    require([p['label'] for p in checkpoints] == ['boot'] + [c['label'] for c in commands[:-1]], 'checkpoint sequence')
    require([r['frame'] for r in frames] == list(range(6801,12060)), 'missing/reordered/extra frame evidence')
    frame = 6800
    for command, point in zip(commands[:-1], checkpoints[1:]):
        frame += command['frames']
        require(point['frame'] == frame, 'route frame mismatch')
    grant = next(r for r in frames if 38 in r['events'])
    require(all(r['events'] == ([32,251] if r['frame'] < grant['frame'] else [32,38,251]) for r in frames), 'nonmonotonic/unexpected event changes')
    selected = {r['label']: checkpoint(root,r) for r in checkpoints if r['label'] in EXPECT}
    transition = [{k:r[k] for k in ('frame','map','position','script')} for r in frames if r['frame'] in (11674,11690,11723,11724,11760)]
    result = {'rom_sha256':ROM_SHA, 'route_sha256':sha(route),
              'bootstrap_sha256':sha((ROOT.parent/'new-game-qualification/bootstrap.rs').read_bytes()),
              'frame_log_sha256':sha(root.with_suffix('.jsonl').read_bytes()),
              'first_grant':{k:grant[k] for k in ('frame','label')},
              'transition':transition, 'checkpoints':selected}
    validate(result)
    return result

if __name__ == '__main__':
    result = report(sys.argv[1], sys.argv[2])
    ref = ROOT/'reference.json'
    if len(sys.argv) == 4 and sys.argv[3] == '--record':
        ref.write_text(json.dumps(result,indent=2)+'\n')
    else:
        require(result == json.loads(ref.read_text()), 'committed native metadata mismatch')
        print('One fresh journey: acknowledgement-gated event26, choices/repeat, D load gate, A spawn/settled/walk verified')
