"""Read-only native spear acquisition/frozen-return qualification; never runs an oracle."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
PANDORA = ROOT.parent / 'pandora-qualification'
sys.path.insert(0, str(PANDORA))
import check as accepted
from check import checkpoint, require_equal, timeline
from source import ROM_SHA, project as pandora_project, require, sha
from epoch import EPOCH, POLICY, SURFACES, observer_sources

FINAL = accepted.FINAL
EMPTY = [0] * 24
SPEAR = [0x81, 1] + [0] * 22  # 7F8048..7F805F: item 81, count 1; NOT an equip claim.


def commands(path):
    return [json.loads(line) for line in path.read_bytes().splitlines()]


def extension():
    return commands(ROOT / 'route.jsonl')[len(commands(PANDORA / 'route.jsonl')) - 1:-1]


def validate_recipe(route, retained, prefix):
    require_equal(route, retained, 'exact retained input/observation recipe')
    require(route.startswith(b''.join(prefix.splitlines(keepends=True)[:-1])),
            'accepted fresh prefix changed')


def validate(result):
    """Hash-independent controls: prompt != consent != event242 != actual inventory."""
    p = result['points']
    labels = [c['label'] for c in extension()]
    require(set(p) == set(labels), 'missing/extra departure checkpoints')
    effects = list(FINAL)
    inventory = EMPTY
    transitions = {
        'spear-talk': ([0x240], []), 'spear-accept-1': ([0x241], []),
        'spear-take-A': ([0x242], []), 'spear-presentation-wait': ([1], []),
        'return-sequence-2': ([3], [1]), 'return-sequence-10': ([2], []),
        'return-sequence-19': ([0xa], []), 'return-sequence-20': ([0xfe, 0x23], []),
    }
    for label in labels:
        if label in transitions:
            add, remove = transitions[label]
            effects = sorted((set(effects) - set(remove)) | set(add))
        if label == 'spear-take-request':
            inventory = SPEAR
        require(p[label]['events'] == effects, f'{label}: exact full event progression')
        require(p[label]['weapon_region'] == inventory, f'{label}: actual weapon item/count/region')

    def fields(label, **expected):
        require(all(p[label][k] == v for k, v in expected.items()), f'{label}: semantic boundary {expected}')

    def ordinary(label, m, pos, facing):
        fields(label, map=m, position=pos, facing=facing, script=0x84a258,
               control=0, input_disable=0, phase='idle', held_slot=0)

    for label, pos, facing in [
        ('departure-L', [120, 192], 1), ('departure-L-rest', [120, 192], 1),
        ('weapon-lane-left-rest', [110, 192], 2), ('weapon-lane-up1-rest', [104, 155], 1),
        ('weapon-lane-up2-rest', [104, 109], 1), ('weapon-lane-left2-rest', [94, 109], 2),
        ('weapon-lane-up3-rest', [72, 80], 1),
    ]:
        ordinary(label, 0x41, pos, facing)
    for label, pos, facing in [
        ('weapon-door-wait', [136, 464], 1), ('spear-left-rest', [72, 464], 2),
        ('spear-up-rest', [72, 448], 1), ('spear-A', [72, 448], 1),
        ('spear-request', [72, 448], 1), ('spear-around-left-rest', [40, 448], 2),
        ('spear-around-up-rest', [40, 402], 1), ('spear-around-right-rest', [65, 400], 3),
        ('spear-near-up-rest', [72, 384], 1), ('spear-refusal-1', [72, 384], 1),
        ('spear-accept-1', [72, 384], 1), ('spear-reapproach-rest', [72, 384], 1),
    ]:
        ordinary(label, 0x42, pos, facing)
    for label in ['spear-accepted-neutral', 'spear-collect-A', 'spear-collect-request'] + [
            f'spear-grant-{i}' for i in range(1, 10)]:
        fields(label, map=0x42, position=[72, 384], facing=0, phase='idle', control=0,
               input_disable=0, script=0x84a2e5 if label == 'spear-collect-request' else 0x84a2a3)
    fields('spear-talk', phase='text', map=0x42, position=[72, 384], facing=1)
    for label in ('spear-page-5', 'spear-repeat-2'):
        fields(label, phase='choice', text_bank=0xffff, cursor=0xdbad, selection=0,
               map=0x42, position=[72, 384], input_disable=0)
    # The label "retry-choice" is an ACK page, not the actual retry choice.
    for label, cursor in [('spear-talk-wait', 0xdadf), ('spear-page-1', 0xdb0c),
                          ('spear-page-2', 0xdb2e), ('spear-page-3', 0xdb57),
                          ('spear-page-4', 0xdb93), ('spear-refusal', 0xdbe9),
                          ('spear-retry', 0xdb2e), ('spear-retry-choice', 0xdb57),
                          ('spear-repeat-1', 0xdb93), ('spear-accept-request', 0xdc0d)]:
        fields(label, phase='ack', text_bank=0xf589, cursor=cursor, input_disable=0,
               map=0x42, position=[72, 384], facing=1)
    fields('spear-take-A', phase='text', facing=1, map=0x42, position=[72, 384], input_disable=0)
    for label in ('spear-take-request', 'spear-take-1'):
        fields(label, map=0x42, position=[72, 384], facing=1, input_disable=0xff50,
               phase='idle', script=0x84bf00, control=0)
    for label, cursor in [('spear-presentation-wait', 0xdc4d), ('return-sequence-1', 0xdc77)]:
        fields(label, map=0x42, phase='ack', text_bank=0x5289, cursor=cursor, input_disable=0)
    fields('return-sequence-2', map=0x21, position=[136, 368], facing=0, phase='idle', input_disable=0xff50)
    cursors = [0xb408, 0xb41f, 0xb444, 0xb47e, 0xb4b5, 0xb4e4, 0xb506,
               0xb1a3, 0xb1bb, 0xb1e0, 0xb20c, 0xb21e, 0xb246, 0xb279, 0xb2a4, 0xb2cd]
    for i, cursor in enumerate(cursors, 3):
        fields(f'return-sequence-{i}', map=0x21, position=[136, 368], cursor=cursor,
               text_bank=0x5c88, phase='text' if i == 10 else 'ack',
               facing=0 if i < 6 or i >= 12 else 2,
               input_disable=0xff50 if i == 6 or i >= 12 else 0)
    for label in ('return-sequence-19', 'return-sequence-20-A'):
        fields(label, map=0x21, position=[136, 464], facing=1, phase='ack',
               text_bank=0x5288, cursor=0xb2f9, input_disable=0xff50, script=0x88af96)
    for label, pos, facing in [('return-sequence-20', [136, 464], 0),
                               ('frozen-return-control', [136, 464], 0),
                               ('frozen-return-left-rest', [120, 464], 2),
                               ('frozen-return-up-rest', [120, 448], 1),
                               ('frozen-return-stable', [120, 448], 1)]:
        ordinary(label, 0x21, pos, facing)


def check_prefix(rom, root, lines, rows, sources, provenance):
    """Reconstruct the accepted report from prefix rows, without temporary/restored state."""
    retained = json.loads((PANDORA / 'reference.json').read_text())
    house = json.loads((PANDORA / 'prefix-reference.json').read_text())
    end = retained['final_frame']
    house_end = max(point['frame'] for point in house['checkpoints'].values())
    prefix_rows = [row for row in rows if row['frame'] <= end]
    raw = b''.join(line for line, row in zip(lines, rows) if row['frame'] <= end)
    require(sha(raw) == retained['frame_log_sha256'], 'accepted prefix log changed')
    require(sha(b''.join(line for line, row in zip(lines, rows) if row['frame'] <= house_end))
            == house['frame_log_sha256'], 'accepted house prefix log changed')
    require_equal(sources, house['observer_source_hashes'], 'prefix observer source changed')
    require(house['observer_epoch'] == EPOCH, 'prefix observer epoch changed')
    for label, point in house['checkpoints'].items():
        require_equal({ext: sha((root / f'{label}.{ext}').read_bytes()) for ext in SURFACES},
                      point['hashes'], f'accepted prefix capture {label} changed')
    recipe = (PANDORA / 'route.jsonl').read_bytes()
    cmds = commands(PANDORA / 'route.jsonl')
    selected = {c['label'] for c in cmds[:-1] if c['frames'] > 1}
    points = {r['label']: checkpoint(root, r, rom) for r in prefix_rows
              if r['kind'] == 'checkpoint' and r['frame'] > house_end and r['label'] in selected}
    tutorial = []
    for row in prefix_rows:
        m = row['map']
        if m in (0x41, 0x42, 0x43, 0x44) and (not tutorial or tutorial[-1] != m):
            tutorial.append(m)
    observed = dict(rom_sha256=ROM_SHA, route_sha256=sha(recipe), frame_log_sha256=sha(raw),
                    final_frame=timeline(cmds, prefix_rows), tutorial_map_path=tutorial, points=points,
                    observer_source_hashes=sources, provenance=provenance,
                    observation_policy=POLICY, observer_epoch=EPOCH)
    accepted.validate(observed)
    require_equal(observed, retained, 'accepted Pandora prefix changed')


def evidence(rom, root):
    """Capture projection shared by report and initial pinning; no oracle or raw state writes."""
    root = Path(root)
    route = (root / 'route.jsonl').read_bytes()
    validate_recipe(route, (ROOT / 'route.jsonl').read_bytes(), (PANDORA / 'route.jsonl').read_bytes())
    raw = root.with_suffix('.jsonl').read_bytes()
    lines = raw.splitlines(keepends=True)
    rows = [json.loads(line) for line in lines]
    last = timeline([json.loads(line) for line in route.splitlines()], rows)
    # Keep every frame/checkpoint; reject missing AND extra artifacts, including one-frame edges.
    labels = ['boot'] + [c['label'] for c in commands(ROOT / 'route.jsonl')[:-1]]
    require({str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()} ==
            {'route.jsonl'} | {f'{label}.{ext}' for label in labels for ext in SURFACES},
            'missing/extra capture artifacts')
    sources = observer_sources()
    provenance = {str(p.relative_to(ROOT.parent)): sha(p.read_bytes()) for p in
                  [accepted.HOUSE / 'probe.rs', accepted.HOUSE / 'build.sh',
                   ROOT.parent / 'new-game-qualification/bootstrap.rs']}
    check_prefix(rom, root, lines, rows, sources, provenance)
    selected = {c['label'] for c in extension()}
    points = {}
    for row in rows:
        if row['kind'] == 'checkpoint' and row['label'] in selected:
            point = checkpoint(root, row, rom)
            point['weapon_region'] = list((root / f"{row['label']}.wram").read_bytes()[0x18048:0x18060])
            points[row['label']] = point
    result = dict(rom_sha256=ROM_SHA, route_sha256=sha(route), frame_log_sha256=sha(raw),
                  final_frame=last, points=points, observer_source_hashes=sources,
                  provenance=provenance, observation_policy=POLICY, observer_epoch=EPOCH,
                  prefix_reference_sha256=sha((PANDORA / 'reference.json').read_bytes()))
    validate(result)
    return result


def report(rom_path, root):
    rom = Path(rom_path).read_bytes()
    require(sha(rom) == ROM_SHA, 'owned Japanese ROM authentication')
    require_equal(pandora_project(rom), json.loads((PANDORA / 'source.json').read_text()),
                  'accepted source metadata mismatch')
    # Source agent owns this helper; ROM-free controls can run before it arrives.
    from source_contract import project
    metadata = project(rom)
    require_equal(metadata, json.loads((ROOT / 'source.json').read_text()), 'source metadata mismatch')
    result = evidence(rom, root)
    result['source_metadata'] = metadata
    files = [ROOT / name for name in ('check_departure.py', 'test_departure.py',
             'source_contract.py', 'test_source_contract.py', 'source.json', 'replay.sh')]
    files += [PANDORA / name for name in ('check.py', 'epoch.py', 'source.py', 'source.json', 'prefix-reference.json')]
    # reference.json is deliberately NOT in its own provenance.
    result['provenance'].update({str(p.relative_to(ROOT.parent)): sha(p.read_bytes()) for p in files})
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'usage: check_departure.py JP_ROM CAPTURE_ROOT')
    require_equal(report(sys.argv[1], sys.argv[2]), json.loads((ROOT / 'reference.json').read_text()),
                  'retained evidence changed (semantic controls passed)')
    print(f'Fresh New Game → Pandora spear acquisition → frozen map21 two-axis control verified ({EPOCH})')
