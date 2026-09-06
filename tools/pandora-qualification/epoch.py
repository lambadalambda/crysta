"""Synchronous observer epoch and read-only, exhaustive migration audit helpers."""
from copy import deepcopy
import json
from pathlib import Path
import sys
from source import ROOT, require, sha

sys.path.insert(0, str(ROOT.parent / 'oracle-video-qualification'))
from compare import compare

EPOCH = 'headless-sync-video-v1'
POLICY = ('one-empty-SRAM-Session; bootstrap6800; save_state-sync-at-boot-and-every-command; '
          'no-restore; finish-flush-exit; headless-sync-video-v1; '
          'pixels=completed-last-explicit-run_frame-before-save_state-sync')
OBSERVER_FILES = (
    'crates/oracle/build.rs', 'crates/oracle/src/lib.rs',
    'vendor/ares/ares-unity.cpp', 'vendor/ares/shims.cpp',
    'vendor/ares/ares/ares/ares.hpp', 'vendor/ares/ares/ares/node/video/screen.cpp',
    'vendor/ares/ares/sfc/ppu/main.cpp', 'vendor/ares/ares/sfc/ppu/color.cpp',
    'vendor/ares/ares/sfc/system/serialization.cpp',
)
SURFACES = ('state', 'wram', 'vram', 'cgram', 'pixels', 'oam', 'obj')


def observer_sources(repo=ROOT.parent.parent):
    return {name: sha((repo / name).read_bytes()) for name in OBSERVER_FILES}


def audit_captures(old, fresh, twin, sibling, route):
    # Imported lazily: check uses this module's observer policy, not its audit API.
    from check import timeline
    require(len({root.resolve() for root in (old, fresh, twin, sibling)}) == 4,
            'migration requires four distinct capture roots')
    commands = [json.loads(line) for line in route.splitlines()]
    labels = ['boot'] + [c['label'] for c in commands[:-1]]
    expected = {'route.jsonl'} | {f'{label}.{ext}' for label in labels for ext in SURFACES}
    for root in (old, fresh, twin, sibling):
        require(root.is_dir(), 'missing capture directory')
        names = {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}
        require(names == expected, 'missing/extra capture artifacts')
        require((root / 'route.jsonl').read_bytes() == route, 'frozen recipe changed')
        timeline(commands, [json.loads(line) for line in root.with_suffix('.jsonl').read_bytes().splitlines()])
    pairs = {'old_to_fixed': compare(old, fresh), 'twins': compare(fresh, twin),
             'sibling': compare(sibling, fresh)}
    require(pairs['twins']['byte_equal'] and pairs['sibling']['byte_equal'],
            'fixed epoch not independently byte-reproduced')
    prior = pairs['old_to_fixed']
    require(prior['frame_log']['byte_equal'], 'full frame log changed across epochs')
    require(all(Path(name).suffix == '.pixels' and value['left']['bytes'] == value['right']['bytes']
                for name, value in prior['changed'].items()),
            'non-pixel data or pixel extent changed across epochs')
    return pairs


def migrate_pixels(reference, fresh, points_key):
    """Pure metadata projection; caller must first audit the entire native roots."""
    result = deepcopy(reference)
    for label, point in result[points_key].items():
        point['hashes']['pixels'] = sha((fresh / f'{label}.pixels').read_bytes())
    return result
