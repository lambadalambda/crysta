"""Audited pixel/observer migration. No emulation, restore or default writes.

Usage: migrate.py ROM OLD FIXED TWIN SIBLING [--write]
The four roots must be complete frozen-route captures. --write publishes only
current reference/prefix/migration metadata, never the archived old evidence.
"""
import json
from pathlib import Path
import sys
from check import checkpoint, require_equal, validate
from epoch import EPOCH, POLICY, SURFACES, audit_captures, migrate_pixels, observer_sources
from source import ROOT, ROM_SHA, project, require, sha

LEGACY = ROOT / 'epochs/threaded-video-v0'


def encoded(value):
    return (json.dumps(value, indent=2) + '\n').encode()


def migration(rom_path, old, fresh, twin, sibling):
    rom = Path(rom_path).read_bytes()
    require(sha(rom) == ROM_SHA, 'owned Japanese ROM authentication')
    require_equal(project(rom), json.loads((ROOT / 'source.json').read_text()), 'source contract changed')
    route = (ROOT / 'route.jsonl').read_bytes()
    audit = audit_captures(old, fresh, twin, sibling, route)
    prior = json.loads((LEGACY / 'reference.json').read_text())
    prefix = json.loads((LEGACY / 'prefix-reference.json').read_text())
    require(sha(route) == prior['route_sha256'], 'legacy frozen recipe changed')
    raw = old.with_suffix('.jsonl').read_bytes()
    require(sha(raw) == prior['frame_log_sha256'], 'old full log not authenticated by legacy reference')
    rows = [json.loads(line) for line in raw.splitlines()]
    by_label = {r['label']: r for r in rows if r['kind'] == 'checkpoint'}
    for label, point in prior['points'].items():
        require_equal(checkpoint(old, by_label[label], rom), point, f'legacy checkpoint {label}')
    validate(prior)
    for label, point in prefix['checkpoints'].items():
        require_equal({ext: sha((old / f'{label}.{ext}').read_bytes()) for ext in SURFACES},
                      point['hashes'], f'legacy prefix {label}')
    prefix_lines = [line for line, row in zip(raw.splitlines(keepends=True), rows) if row['frame'] <= 12059]
    require(sha(b''.join(prefix_lines)) == prefix['frame_log_sha256'], 'legacy prefix log changed')
    house_route = (ROOT.parent / 'house-conversation-qualification/route.jsonl').read_bytes()
    require(sha(house_route) == prefix['route_sha256']
            and route.startswith(b''.join(house_route.splitlines(keepends=True)[:-1])), 'legacy input prefix changed')
    # Authenticate current policy against the independently reviewed video fix;
    # old observer fields remain historical evidence, not current source claims.
    video = json.loads((ROOT.parent / 'oracle-video-qualification/evidence.json').read_text())
    sources = observer_sources()
    require_equal(sources, {name: video['source_sha256'][name]['after'] for name in sources},
                  'reviewed synchronous observer source changed')
    require_equal(prior['observer_source_hashes'],
                  {name: video['source_sha256'][name]['before'] for name in prior['observer_source_hashes']},
                  'legacy observer source changed')
    for name, digest in prior['provenance'].items():
        require(sha((ROOT.parent / name).read_bytes()) == digest, 'probe/bootstrap/build recipe changed')
    current = migrate_pixels(prior, fresh, 'points')
    current.update(observer_source_hashes=sources, observation_policy=POLICY, observer_epoch=EPOCH)
    current_prefix = migrate_pixels(prefix, fresh, 'checkpoints')
    current_prefix.update(observer_source_hashes=sources, observer_epoch=EPOCH)
    validate(current)
    ledger = {'from_epoch': 'threaded-video-v0', 'to_epoch': EPOCH,
              'scope': 'Pandora frozen main route and embedded conversation prefix only; discovery and other wrappers not renewed',
              'recipe_sha256': sha(route), 'source_contract_sha256': sha((ROOT / 'source.json').read_bytes()),
              'legacy_reference_sha256': {p.name: sha(p.read_bytes()) for p in sorted(LEGACY.glob('*.json'))},
              'new_reference_sha256': sha(encoded(current)), 'new_prefix_sha256': sha(encoded(current_prefix)),
              'observer_source_hashes': sources, 'comparisons': audit}
    return {'reference.json': current, 'prefix-reference.json': current_prefix, 'migration.json': ledger}


if __name__ == '__main__':
    require(len(sys.argv) in (6, 7) and (len(sys.argv) == 6 or sys.argv[6] == '--write'), __doc__)
    results = migration(sys.argv[1], *(Path(p) for p in sys.argv[2:6]))
    for name, result in results.items():
        path = ROOT / name
        if len(sys.argv) == 7:
            path.write_bytes(encoded(result))
        else:
            require_equal(json.loads(path.read_text()), result, f'audited migration {name} changed')
    print('Audited pixel-only observer migration verified; full logs/nonpixels/recipe unchanged, fixed twins and sibling exact')
