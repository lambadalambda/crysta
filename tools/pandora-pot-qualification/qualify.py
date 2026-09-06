"""Read-only, exact-original pot projection. No emulator/restore/imported state runtime.

Outputs private test inputs, never production initializers. The parent fresh replay
is a separate gate: its hash is deliberately not accepted by this projector.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
REFERENCE = ROOT.parent / 'pandora-qualification'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def contiguous(rows, first, last):
    require(all(r['kind'] == 'frame' for r in rows), 'not passive frame evidence')
    require([r['frame'] for r in rows] == list(range(first, last + 1)),
            'missing, duplicate, restored or reordered frames')
    return rows


# Start at idle before the sampled A; all native frame calls through recovered
# ordinary player control. Includes movement commands and neutral pauses, not
# isolated hand-picked successful frames. Checkpoints are authenticated separately.
SEGMENTS = [('miss', 'pot-first-ready', 'pot-first-A', 'door-hit1'),
            ('fa-hit', 'left-pot-face-rest', 'left-pot-A', 'door-real-hit1'),
            ('fb-hit', 'middle-route-ready', 'middle-grab', 'door-hit2')]


def project(log, captures, output):
    reference = json.loads((REFERENCE / 'reference.json').read_text())
    require(sha(log.read_bytes()) == reference['frame_log_sha256'],
            'not exact retained original; parent fresh replay is a separate gate')
    route_bytes = (REFERENCE / 'route.jsonl').read_bytes()
    require(sha(route_bytes) == reference['route_sha256'], 'route changed')
    commands = [json.loads(l) for l in route_bytes.splitlines()]
    require(commands[-1] == {'finish': True}, 'no fresh finish')
    rows = [json.loads(l) for l in log.read_text().splitlines()]
    # Reuse, do not relax, source owner's complete observation schedule checker.
    sys.path.insert(0, str(REFERENCE))
    from check import timeline
    timeline(commands, rows)
    command = {c['label']: c for c in commands[:-1]}
    checkpoints = {r['label']: r for r in rows if r['kind'] == 'checkpoint'}
    output.mkdir(parents=True, exist_ok=True)
    manifest = {'original_log_sha256': sha(log.read_bytes()), 'segments': {}}
    for name, before, start, end in SEGMENTS:
        labels = list(command)
        admitted = labels[labels.index(start):labels.index(end) + 1]
        selected = [r for r in rows if r['label'] in admitted and r['kind'] == 'frame']
        contiguous(selected, checkpoints[before]['frame'] + 1, checkpoints[end]['frame'])
        # Authenticate every full checkpoint used by this bounded segment against
        # the existing reference (including unchanged resident occupancy).
        hashes = {}
        for label in [before] + admitted:
            w = (captures / f'{label}.wram').read_bytes()
            require(len(w) == 131072, 'full WRAM extent')
            digest = sha(w)
            if label in reference['points']:
                require(digest == reference['points'][label]['hashes']['wram'], 'checkpoint hash')
            # Unselected checkpoints get new pot-owned pins, not fabricated parent pins.
            u = lambda p: int.from_bytes(w[p:p + 2], 'little')
            cp = checkpoints[label]
            require([u(0x1000), u(0x1002)] == cp['position'] and u(0x980) == cp['control']
                    and u(0x47e) == cp['map'], 'checkpoint/log disagreement')
            hashes[label] = digest
        w = (captures / f'{before}.wram').read_bytes()
        grid = w[0xa000:0xb000]
        (output / f'{name}.grid').write_bytes(grid)
        cp = checkpoints[before]
        lines = [','.join(map(str, [*cp['position'], cp['facing']]))]
        for r in selected:
            require(r['map'] == 12 and not {0x2f, 0x3f, 0x42}.intersection(r['events']),
                    'not direct C branch')
            for slot, position in [(0x1040, [152, 368]), (0x1080, [56, 384]),
                                   (0x10c0, [184, 416]), (0x1100, [216, 368])]:
                # After a true hit parent reactions may move residents. Never use
                # that later geometry as the earlier carrying collision grid.
                if r['frame'] <= checkpoints[admitted[-2]]['frame']:
                    actor = next(a for a in r['actors'] if a['slot'] == slot)
                    require(actor['position'] == position, 'direct resident occupancy')
            buttons = command[r['label']]['buttons']
            require(len(buttons) <= 1, 'diagonal/combined action not admitted')
            button = {'Down': 0, 'Up': 1, 'Left': 2, 'Right': 3, 'A': 4}.get(
                buttons[0] if buttons else None, 5)
            pot = next(a for a in r['actors'] if a['slot'] == 0x12c0)
            door = next(a for a in r['actors'] if a['slot'] == 0x1140)
            lines.append(','.join(map(str, [r['frame'], button, *r['position'], r['facing'],
                                           r['control'], r['script'], *pot['position'],
                                           pot['script'], pot['flags'], door['script']])))
        data = ('\n'.join(lines) + '\n').encode()
        (output / f'{name}.csv').write_bytes(data)
        manifest['segments'][name] = {'csv_sha256': sha(data), 'grid_sha256': sha(grid),
                                      'first': selected[0]['frame'], 'last': selected[-1]['frame'],
                                      'checkpoints': hashes}
    return manifest


if __name__ == '__main__':
    require(len(sys.argv) == 4, 'usage: qualify.py ORIGINAL.jsonl ORIGINAL-captures LOCAL-output')
    result = project(*map(Path, sys.argv[1:]))
    require(result == json.loads((ROOT / 'reference.json').read_text()), 'pot exact-reference mismatch')
    print(json.dumps(result, indent=2))
