"""Prefix-aware evidence checker. Reads captures; never boots, restores or writes game state."""
import json
from pathlib import Path
import sys
from source import ROOT, ROM_SHA, Source, project, require, sha

HOUSE = ROOT.parent / 'house-conversation-qualification'
STORY = [0x20, 0x26, 0x27, 0x28, 0x2e, 0xfb]
FINAL = sorted(STORY + [0x22, 0x243, 0x244, 0x292])
DISCOVERY_LABELS = {'cellar-without28-blocked', 'resident13-choice-no-confirm',
                    'resident13-refused', 'resident13-granted28', 'door-dash2-rest',
                    'throw-pot-impact', 'second-impact', 'door-second-hit',
                    '21-warning-complete-neutral', '21-opening-wait',
                    'inside-box-neutral', 'inside-box-left-rest', 'inside-box-up-rest'}
SURFACES = ('state', 'wram', 'vram', 'cgram', 'pixels', 'oam', 'obj')


def timeline(commands, rows):
    require(commands and commands[-1] == {'finish': True}, 'missing final finish')
    commands = commands[:-1]
    require(all(set(c) == {'label', 'buttons', 'frames'} and 0 < c['frames'] <= 2000
                for c in commands), 'command shape/budget')
    labels = [c['label'] for c in commands]
    require(len(set(labels)) == len(labels) and 'boot' not in labels, 'duplicate label')
    require(all(r['kind'] in ('frame', 'checkpoint') for r in rows), 'unknown observation')
    at = 0
    require(rows and rows[0]['kind'] == 'checkpoint' and rows[0]['label'] == 'boot'
            and rows[0]['frame'] == 6800, 'fresh bootstrap checkpoint')
    at += 1
    frame = 6800
    for c in commands:
        for _ in range(c['frames']):
            frame += 1
            require(at < len(rows) and rows[at]['kind'] == 'frame'
                    and rows[at]['label'] == c['label'] and rows[at]['frame'] == frame,
                    'missing/reordered/extra frame evidence')
            at += 1
        require(at < len(rows) and rows[at]['kind'] == 'checkpoint'
                and rows[at]['label'] == c['label'] and rows[at]['frame'] == frame,
                'synchronizing checkpoint schedule')
        at += 1
    require(at == len(rows), 'extra observations')
    return frame


def checkpoint(root, row, rom):
    label = row['label']
    w = (root / f'{label}.wram').read_bytes()
    require(len(w) == 131072, 'WRAM extent')
    u = lambda p: int.from_bytes(w[p:p + 2], 'little')
    fields = {'map': u(0x47e), 'position': [u(0x1000), u(0x1002)],
              'camera': [u(0x81e), u(0x822)], 'script': u(0x100a) | w[0x100c] << 16,
              'control': u(0x980), 'facing': u(0x1014), 'flags': u(0x1004)}
    require(all(row[k] == v for k, v in fields.items()), 'checkpoint/log disagreement')
    events = [i for i in range(0x400) if w[0x6c0 + i // 8] >> (i % 8) & 1]
    require(row['events'] == [i for i in events if i < 512], 'event/log disagreement')
    bank, cursor = u(0xdc2), u(0xdc0)
    phase = 'choice' if bank == 65535 else 'text' if bank & 255 else 'idle'
    boundary = None
    if phase == 'text':
        address = ((bank & 255) << 16) | cursor
        value = Source(rom).u(address, 1)
        if value in (0xd3, 0xd5):
            phase, boundary = 'ack', {'source': address, 'opcode': value}
    fields.update(frame=row['frame'], events=events, input_disable=u(0x45e),
                  text_bank=bank, cursor=cursor, phase=phase, boundary=boundary,
                  selection=u(0xdce), held_slot=u(0x988), held_kind=u(0x9c7),
                  hit_counter=u(0x640), door_cells=[u(0xa516), u(0xa556)] if fields['map'] == 12 else None,
                  hashes={ext: sha((root / f'{label}.{ext}').read_bytes()) for ext in SURFACES})
    return fields


def validate(result):
    """Semantic controls, separate from exact-reference equality and byte hashes."""
    p = result['points']
    require(p['13-choice']['phase'] == 'choice' and 0x28 not in p['13-choice']['events'],
            'resident choice must precede grant28')
    require(p['13-followup-4']['phase'] == 'ack' and 0x28 not in p['13-followup-4']['events'],
            'last resident request has not returned')
    require(p['13-grant28']['events'] == [0x20, 0x26, 0x28, 0xfb], 'grant28 effect')
    require(p['C-direct-ready']['events'] == STORY, 'direct C choice/effect; no refusal branch')
    for label, slot, kind in [('left-pot-held', 0x98a, 0xfa), ('middle-held', 0x98f, 0xfb)]:
        require(p[label]['held_slot'] == slot and p[label]['held_kind'] == kind, 'source pot lift identity')
    require(p['door-hit1']['hit_counter'] == 0 and 0x292 not in p['door-hit1']['events'],
            'off-target pot is not a door hit')
    require(p['door-real-hit1']['hit_counter'] == 1
            and p['door-real-hit1']['door_cells'] == [0x1da7, 0x0b81]
            and 0x292 not in p['door-real-hit1']['events'], 'first hit is not an open door')
    require(p['door-hit2']['hit_counter'] == 2 and 0x292 in p['door-hit2']['events'], 'second hit opens event292')
    require(p['door-passable']['door_cells'] == [0x1cf6, 0x3acb], 'door patch/occupancy release')
    for label, m, pos in [('landed-E', 14, [152, 880]), ('landed-20', 32, [408, 880]),
                           ('landed-21', 33, [136, 128])]:
        require(p[label]['map'] == m and p[label]['position'] == pos, 'stairs settled endpoint')
    require(0x22 not in p['21-warning-complete-neutral']['events']
            and {1, 2} <= set(p['21-warning-complete-neutral']['events']), 'warning is not opening')
    require(0x22 in p['21-opening-wait']['events'] and p['21-opening-wait']['input_disable'] & 0xff50,
            'opening is not regained control')
    endpoints = [('pandora-tour-control', [136, 208]), ('pandora-left-rest', [120, 208]),
                 ('pandora-up-rest', [120, 192]), ('pandora-neutral-stable', [120, 192])]
    for label, pos in endpoints:
        v = p[label]
        require(v['map'] == 0x41 and v['position'] == pos and v['events'] == FINAL,
                'post-Pandora endpoint/effects')
        require(v['script'] == 0x84a258 and v['control'] == 0 and v['input_disable'] == 0
                and v['phase'] == 'idle' and v['held_slot'] == 0, 'regained ordinary control')
    require(result['tutorial_map_path'] == [0x41, 0x44, 0x42, 0x43, 0x41], 'mandatory first-time tutorial')
    require(not any(set(v['events']) & {0x21, 0x23, 0x2f, 0x3f, 0x42, 0xfe}
                    for v in p.values()), 'later/refusal story state outside endpoint')


def validate_discovery(result):
    p = result['points']
    v = p['cellar-without28-blocked']
    require(v['map'] == 12 and v['position'] == [184, 368]
            and v['events'] == [1, 0x20, 0x26, 0xfb], 'missing28 closed-cellar control')
    require(p['resident13-choice-no-confirm']['phase'] == 'choice'
            and 0x28 not in p['resident13-choice-no-confirm']['events'], 'unconfirmed choice control')
    require(p['resident13-refused']['events'] == [1, 0x20, 0x26, 0xfb], 'cancel does not grant28')
    require(p['resident13-granted28']['events'] == [1, 0x20, 0x26, 0x28, 0xfb], 'retry grant28')
    require(p['door-dash2-rest']['hit_counter'] == 0, 'dash is not a thrown-object hit')
    require(p['throw-pot-impact']['hit_counter'] == 1 and p['second-impact']['hit_counter'] == 1
            and 0x292 not in p['second-impact']['events'], 'off-target throw must not count')
    require(p['door-second-hit']['hit_counter'] == 2 and 0x292 in p['door-second-hit']['events'],
            'second actual hit, not second consumed pot')
    require(0x22 not in p['21-warning-complete-neutral']['events']
            and 0x22 in p['21-opening-wait']['events'], 'warning/open distinction')
    for label, position in [('inside-box-neutral', [136, 208]),
                            ('inside-box-left-rest', [120, 208]), ('inside-box-up-rest', [120, 192])]:
        v = p[label]
        require(v['position'] == position and v['map'] == 0x41 and v['phase'] == 'idle'
                and v['input_disable'] == 0 and v['script'] == 0x84a258
                and {0x22, 0x243, 0x244, 0x292} <= set(v['events']), 'discovery control endpoint')


def report(rom_path, root, discovery=False):
    rom = Path(rom_path).read_bytes()
    require(sha(rom) == ROM_SHA, 'owned Japanese ROM authentication')
    require(project(rom) == json.loads((ROOT / 'source.json').read_text()), 'source metadata mismatch')
    root = Path(root)
    route = (root / 'route.jsonl').read_bytes()
    require(route == (ROOT / ('discovery-route.jsonl' if discovery else 'route.jsonl')).read_bytes(), 'exact retained input/observation recipe')
    prefix = (HOUSE / 'route.jsonl').read_bytes().splitlines(keepends=True)[:-1]
    require(route.startswith(b''.join(prefix)), 'accepted fresh prefix changed')
    commands = [json.loads(l) for l in route.splitlines()]
    raw = root.with_suffix('.jsonl').read_bytes()
    lines = raw.splitlines(keepends=True)
    rows = [json.loads(l) for l in lines]
    last = timeline(commands, rows)
    house = json.loads((HOUSE / 'reference.json').read_text())
    require(sha(b''.join(l for l, row in zip(lines, rows) if row['frame'] <= 12059))
            == house['frame_log_sha256'], 'accepted prefix log changed')
    for label, point in house['checkpoints'].items():
        require({ext: sha((root / f'{label}.{ext}').read_bytes()) for ext in SURFACES} == point['hashes'],
                'accepted prefix capture changed')
    # One-frame input edges remain in the hash-bound log and schedule, not duplicated in the semantic table.
    selected = DISCOVERY_LABELS if discovery else {c['label'] for c in commands[:-1] if c['frames'] > 1}
    points = {r['label']: checkpoint(root, r, rom) for r in rows
              if r['kind'] == 'checkpoint' and r['frame'] > 12059 and r['label'] in selected}
    tutorial = []
    for row in rows:
        m = row['map']
        if m in (0x41, 0x42, 0x43, 0x44) and (not tutorial or tutorial[-1] != m):
            tutorial.append(m)
    repo = ROOT.parent.parent
    observer_files = ['crates/oracle/src/lib.rs', 'vendor/ares/shims.cpp',
                      'vendor/ares/ares/sfc/system/serialization.cpp']
    result = {'rom_sha256': ROM_SHA, 'route_sha256': sha(route), 'frame_log_sha256': sha(raw),
              'final_frame': last, 'tutorial_map_path': tutorial, 'points': points,
              'observer_source_hashes': {name: sha((repo / name).read_bytes()) for name in observer_files},
              'provenance': {str(p.relative_to(ROOT.parent)): sha(p.read_bytes()) for p in
                             [HOUSE / 'probe.rs', HOUSE / 'build.sh',
                              ROOT.parent / 'new-game-qualification/bootstrap.rs']},
              'observation_policy': 'one-empty-SRAM-Session; bootstrap6800; save_state-sync-at-boot-and-every-command; no-restore; finish-flush-exit'}
    (validate_discovery if discovery else validate)(result)
    return result


if __name__ == '__main__':
    options = sys.argv[3:]
    require(set(options) <= {'--record', '--discovery'}, 'unknown option')
    discovery = '--discovery' in options
    result = report(sys.argv[1], sys.argv[2], discovery)
    ref = ROOT / ('discovery-reference.json' if discovery else 'reference.json')
    if '--record' in options:
        ref.write_text(json.dumps(result, indent=2) + '\n')
    else:
        require(result == json.loads(ref.read_text()), 'retained semantic evidence changed')
        print('Fresh discovery controls verified' if discovery else
              'Fresh input-only New Game → Pandora tour → two-axis control verified')
