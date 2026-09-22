"""Read-only qualification from accepted frozen return through first-tower INTERIOR."""
import json
from pathlib import Path
import sys

import check_departure as departure
from check_departure import (ROOT, ROM_SHA, EPOCH, POLICY, SURFACES, SPEAR, commands,
                             require, require_equal, sha, timeline, validate_recipe)

PREFIX_END = 53586
PREFIX_FILES = [departure.accepted.HOUSE / 'probe.rs', departure.accepted.HOUSE / 'build.sh',
                ROOT.parent / 'new-game-qualification/bootstrap.rs']
PREFIX_FILES += [ROOT / name for name in ('check_departure.py', 'test_departure.py',
                 'source_contract.py', 'test_source_contract.py', 'source.json', 'replay.sh')]
PREFIX_FILES += [departure.PANDORA / name for name in
                 ('check.py', 'epoch.py', 'source.py', 'source.json', 'prefix-reference.json')]


def extension():
    return commands(ROOT / 'tower-route.jsonl')[len(commands(ROOT / 'route.jsonl')) - 1:-1]


def hashes(paths):
    return {str(p.relative_to(ROOT.parent)): sha(p.read_bytes()) for p in paths}


def checkpoint(root, row, rom):
    point = departure.checkpoint(root, row, rom)
    point['weapon_region'] = list((root / f"{row['label']}.wram").read_bytes()[0x18048:0x18060])
    return point


def current_checkpoint(root, row, rom):
    point = checkpoint(root, row, rom)
    wram = (root / f"{row['label']}.wram").read_bytes()
    # Source-decoded gear selection words, distinct from acquired inventory slots.
    point.update(equipped_weapon=int.from_bytes(wram[0x64a:0x64c], 'little'),
                 equipped_armor=int.from_bytes(wram[0x64c:0x64e], 'little'))
    return point


def validate(result):
    """Finite captured boundaries, independent of exact hashes; not a gameplay VM."""
    p = result['points']
    labels = [c['label'] for c in extension()]
    require(set(p) == set(labels), 'missing/extra tower checkpoints')
    require(result['map_path'] == [0x21, 0x20, 0xe, 0xc, 0xd, 0xa, 3, 0x100, 0x101],
            'return/world/first-tower-interior itinerary')
    effects = sorted(departure.FINAL + [2, 3, 0xa, 0x23, 0xfe, 0x240, 0x241, 0x242])
    transitions = {
        'return-stairs-21-rest': ([], [2, 3, 0xa]), 'return-C-exit-rest': ([0x14], []),
        'elder-near-A': ([0x21], []), 'elder-accept-4': ([0x296], []),
        'frozen-house-exit': ([], [0x14]), 'frozen-town-arrival': ([0x14], []),
        'frozen-town-page-4': ([0x3c], []), 'underworld-arrival': ([], [0x14]),
        'tower-approach-arrival': ([0x100], []), 'guardian-page-1': ([1], []),
        'guardian-response-2': ([2], []), 'guardian-response-6': ([3, 0x115], []),
        'first-tower-transition-control': ([], [1, 2, 3]),
    }
    for label in labels:
        if label in transitions:
            add, remove = transitions[label]
            effects = sorted((set(effects) - set(remove)) | set(add))
        require(p[label]['events'] == effects, f'{label}: exact full event progression')
        require(p[label]['weapon_region'] == SPEAR, f'{label}: retained spear item/count/region')
        require(p[label]['equipped_weapon'] == p[label]['equipped_armor'] == 0,
                f'{label}: acquisition is not gear selection')

    def fields(label, **expected):
        require(all(p[label][k] == v for k, v in expected.items()), f'{label}: semantic boundary {expected}')

    def ordinary(label, m, pos, facing, script=0x84a258):
        fields(label, map=m, position=pos, facing=facing, script=script, control=0,
               input_disable=0, phase='idle', held_slot=0)

    for label, m, pos, facing in [
        ('return-stair-up-rest', 0x21, [120, 384], 1),
        ('return-around-left-rest', 0x21, [110, 384], 2),
        ('return-around-right-rest', 0x21, [138, 384], 3),
        ('return-20-right-rest', 0x20, [391, 880], 3),
        ('return-20-up-rest', 0x20, [391, 880], 1),
        ('return-20-align-rest', 0x20, [410, 880], 3),
        ('return-E-arrival', 0xe, [104, 880], 0), ('return-E-right-rest', 0xe, [153, 880], 3),
        ('return-E-stair-rest', 0xc, [184, 368], 0), ('return-C-down-rest', 0xc, [184, 417], 0),
        ('return-C-left-rest', 0xc, [120, 417], 2), ('return-C-exit-rest', 0xd, [120, 625], 0),
        ('elder-approach-rest', 0xd, [120, 701], 0), ('elder-talk-A', 0xd, [120, 701], 0),
        ('elder-request', 0xd, [120, 701], 0), ('elder-close-rest', 0xd, [120, 704], 0),
        ('elder-accept-4', 0xd, [120, 704], 0),
        ('guardian-complete-stable', 0x100, [256, 959], 1),
        ('first-tower-transition-control', 0x101, [128, 623], 1),
        ('first-tower-left-rest', 0x101, [112, 623], 2),
        ('first-tower-up-rest', 0x101, [112, 607], 1),
        ('first-tower-neutral-stable', 0x101, [112, 607], 1),
    ]:
        ordinary(label, m, pos, facing)
    for label, m, pos in [('return-stairs-21-rest', 0x20, [360, 872]),
                           ('return-20-stair-rest', 0xe, [104, 868])]:
        ordinary(label, m, pos, 0, script=0x84bd3e)
    fields('elder-near-A', map=0xd, position=[120, 704], facing=0, phase='text')
    for label, cursor in [('elder-near-request', 0x8c7d), ('elder-page-1', 0x8ca4),
                          ('elder-page-2', 0x8cd0), ('elder-page-3', 0x8cf4),
                          ('elder-accept-request', 0x8d48), ('elder-accept-1', 0x8d7c),
                          ('elder-accept-2', 0x8da5), ('elder-accept-3', 0x8dd3)]:
        fields(label, map=0xd, position=[120, 704], facing=0, phase='ack',
               text_bank=0xf588, cursor=cursor, input_disable=0)
    fields('elder-page-4', phase='choice', text_bank=0xffff, cursor=0x8d12, selection=0)
    fields('frozen-town-arrival', map=0xa, position=[504, 769], phase='text', cursor=0x8578)
    for i, cursor in enumerate((0x857a, 0x85a0, 0x85cb), 1):
        fields(f'frozen-town-page-{i}', map=0xa, position=[504, 769 if i == 1 else 868],
               facing=0, phase='ack', text_bank=0x3488, cursor=cursor, input_disable=0 if i == 1 else 0xff50)
    for label in ('frozen-town-page-4', 'frozen-town-control'):
        ordinary(label, 0xa, [504, 868], 0, script=0x84a2a3)
    # World mode has its own player scripts; it is NOT ordinary interior control.
    for label, pos, facing, script in [
        ('underworld-arrival', [536, 544], 0, 0x84df19),
        ('underworld-south-rest', [536, 752], 0, 0x84df19),
        ('underworld-west-rest', [392, 704], 2, 0x84df33),
        ('underworld-south2-rest', [408, 896], 0, 0x84df19),
        ('underworld-west2-rest', [184, 880], 2, 0x84df33),
        ('tower-align-east-rest', [216, 880], 3, 0x84df33),
    ]:
        fields(label, map=3, position=pos, facing=facing, script=script,
               control=0, input_disable=0, phase='idle')
    fields('tower-approach-arrival', map=0x100, position=[256, 1007], facing=1,
           script=0x84a258, phase='idle', input_disable=0xfff0)
    for label, cursor in [('tower-arrival-settle', 0x8f69), ('tower-door-up', 0x8f69),
                          ('tower-door-wait', 0x8f69), ('tower-intro-1', 0x8f7d)]:
        fields(label, map=0x100, position=[256, 1007], facing=1, script=0x84a258,
               phase='ack', text_bank=0x5290, cursor=cursor, input_disable=0)
    ordinary('tower-intro-2', 0x100, [256, 1007], 0, script=0x84a2a3)
    for label in ('tower-guardian-approach', 'tower-guardian-request', 'guardian-response-6'):
        ordinary(label, 0x100, [256, 959], 1, script=0x90fb23)
    for label, cursor in [('guardian-page-1', 0x8d56), ('guardian-page-2', 0x8d73),
                          ('guardian-page-3', 0x8da4), ('guardian-response', 0x8dec),
                          ('guardian-response-1', 0x8e0f), ('guardian-response-2', 0x8e28),
                          ('guardian-response-3', 0x8ec7), ('guardian-response-4', 0x8ef7),
                          ('guardian-response-5', 0x8f08)]:
        fields(label, map=0x100, position=[256, 959], facing=1, script=0x90fb23,
               phase='ack', text_bank=0x5c90, cursor=cursor, input_disable=0)
    fields('guardian-page-4', map=0x100, position=[256, 959], script=0x90fb23,
           phase='choice', text_bank=0xffff, cursor=0x8dd1, selection=0)


def verify_prefix(rom, root, lines, rows):
    """Rebuild the accepted departure projection; its original Pandora gates still run."""
    retained = json.loads((ROOT / 'reference.json').read_text())
    prefix_rows = [r for r in rows if r['frame'] <= PREFIX_END]
    raw = b''.join(line for line, row in zip(lines, rows) if row['frame'] <= PREFIX_END)
    require(sha(raw) == retained['frame_log_sha256'], 'accepted departure prefix log changed')
    require(retained['final_frame'] == PREFIX_END, 'accepted departure endpoint changed')
    require_equal(departure.pandora_project(rom), json.loads((departure.PANDORA / 'source.json').read_text()),
                  'accepted Pandora source metadata mismatch')
    from source_contract import project
    metadata = project(rom)
    require_equal(metadata, json.loads((ROOT / 'source.json').read_text()), 'accepted departure source metadata mismatch')
    sources = departure.observer_sources()
    provenance = hashes(PREFIX_FILES)
    departure.check_prefix(rom, root, lines, rows, sources,
                           {str(p.relative_to(ROOT.parent)): provenance[str(p.relative_to(ROOT.parent))]
                            for p in PREFIX_FILES[:3]})
    selected = {c['label'] for c in departure.extension()}
    points = {r['label']: checkpoint(root, r, rom) for r in prefix_rows
              if r['kind'] == 'checkpoint' and r['label'] in selected}
    observed = dict(rom_sha256=ROM_SHA, route_sha256=sha((ROOT / 'route.jsonl').read_bytes()),
                    frame_log_sha256=sha(raw), final_frame=timeline(commands(ROOT / 'route.jsonl'), prefix_rows),
                    points=points, observer_source_hashes=sources, provenance=provenance,
                    observation_policy=POLICY, observer_epoch=EPOCH, source_metadata=metadata,
                    prefix_reference_sha256=sha((departure.PANDORA / 'reference.json').read_bytes()))
    departure.validate(observed)
    require_equal(observed, retained, 'accepted departure prefix changed')
    return observed


def evidence(rom, root):
    root = Path(root)
    route = (root / 'route.jsonl').read_bytes()
    validate_recipe(route, (ROOT / 'tower-route.jsonl').read_bytes(), (ROOT / 'route.jsonl').read_bytes())
    cmds = [json.loads(line) for line in route.splitlines()]
    raw = root.with_suffix('.jsonl').read_bytes()
    lines = raw.splitlines(keepends=True)
    rows = [json.loads(line) for line in lines]
    last = timeline(cmds, rows)
    labels = ['boot'] + [c['label'] for c in cmds[:-1]]
    require({str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()} ==
            {'route.jsonl'} | {f'{label}.{ext}' for label in labels for ext in SURFACES},
            'missing/extra capture artifacts')
    prefix = verify_prefix(rom, root, lines, rows)
    later = [r for r in rows if r['frame'] > PREFIX_END]
    points = {r['label']: current_checkpoint(root, r, rom) for r in later if r['kind'] == 'checkpoint'}
    maps = []
    for row in later:
        if not maps or maps[-1] != row['map']:
            maps.append(row['map'])
    result = dict(rom_sha256=ROM_SHA, route_sha256=sha(route), frame_log_sha256=sha(raw),
                  final_frame=last, points=points, map_path=maps,
                  observer_source_hashes=prefix['observer_source_hashes'], provenance=prefix['provenance'],
                  observation_policy=POLICY, observer_epoch=EPOCH,
                  prefix_reference_sha256=sha((ROOT / 'reference.json').read_bytes()))
    validate(result)
    return result


def report(rom_path, root):
    rom = Path(rom_path).read_bytes()
    require(sha(rom) == ROM_SHA, 'owned Japanese ROM authentication')
    from tower_source import project  # Source task may arrive after these ROM-free controls.
    metadata = project(rom)
    require_equal(metadata, json.loads((ROOT / 'tower-source.json').read_text()), 'tower source metadata mismatch')
    result = evidence(rom, root)
    result['source_metadata'] = metadata
    result['provenance'].update(hashes(ROOT / name for name in
        ('check_tower.py', 'test_tower.py', 'tower_source.py', 'test_tower_source.py', 'tower-source.json', 'replay-tower.sh')))
    # The new reference is not hashed into itself; the immutable accepted prefix is.
    return result


if __name__ == '__main__':
    require(len(sys.argv) == 3, 'usage: check_tower.py JP_ROM CAPTURE_ROOT')
    require_equal(report(sys.argv[1], sys.argv[2]), json.loads((ROOT / 'tower-reference.json').read_text()),
                  'retained tower evidence changed (semantic controls passed)')
    print(f'Fresh New Game → accepted spear/frozen prefix → first tower map101 two-axis control verified ({EPOCH})')
