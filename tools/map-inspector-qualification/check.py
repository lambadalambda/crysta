#!/usr/bin/env python3
"""Read-only, exhaustive gate for this slot3/no-save fixture (not Pandora)."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import struct
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
LABELS = (1600, 1840)
SIZES = {'wram': 131072, 'vram': 65536, 'cgram': 512}
ROM = 'f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548'
SRAM = '709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055'
SCENARIO = 'qualified-slot-3-right-movement'
POLICY = ('one-authenticated-slot3-SRAM-Session; labels0..=1840; '
          'Start[400,408),A[1100,1112),Right[1800,1840); '
          'observe-after-run_frame1601/1841; no-save/no-restore; process-exit; '
          'RGB=256x240-even-columns-of-512x480-ABI-BGR-to-RGB')
SOURCE_FILES = (
    'Cargo.lock', 'crates/map-inspector/Cargo.toml',
    'crates/map-inspector/src/main.rs', 'crates/map-inspector/web/viewer.html',
    'crates/oracle/build.rs', 'crates/oracle/src/lib.rs',
    'vendor/ares/ares-unity.cpp', 'vendor/ares/shims.cpp',
    'vendor/ares/ares/ares/ares.hpp', 'vendor/ares/ares/ares/node/video/screen.cpp',
    'vendor/ares/ares/sfc/ppu/main.cpp', 'vendor/ares/ares/sfc/ppu/color.cpp',
    'vendor/ares/ares/sfc/system/serialization.cpp',
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def source_hashes(repo):
    return {name: sha((repo / name).read_bytes()) for name in SOURCE_FILES}


def verify_sources(repo, expected):
    require(set(expected) == set(SOURCE_FILES), 'incomplete source inventory')
    require(source_hashes(repo) == expected, 'producer source mismatch')


def inventory(root):
    return {str(p.relative_to(root)): {'bytes': p.stat().st_size, 'sha256': sha(p.read_bytes())}
            for p in sorted(root.rglob('*')) if p.is_file()}


def authenticate(root, expected):
    require(inventory(root) == expected, 'capture differs from archived inventory')


def bmp_header():
    header = bytearray(54)
    header[:2] = b'BM'
    for offset, value in ((2, 184374), (10, 54), (14, 40), (18, 256), (22, -240), (34, 184320)):
        struct.pack_into('<i', header, offset, value)
    struct.pack_into('<HH', header, 26, 1, 24)
    return bytes(header)


def rgb(bitmap):
    require(len(bitmap) == 184374 and bitmap[:54] == bmp_header(), 'RGB bitmap extent/header mismatch')
    bgr = bitmap[54:]
    result = bytearray(len(bgr))
    result[0::3], result[1::3], result[2::3] = bgr[2::3], bgr[1::3], bgr[0::3]
    return bytes(result)


def html(manifest):
    return (REPO / 'crates/map-inspector/web/viewer.html').read_text().replace(
        '__CAPTURE_JSON__', json.dumps(manifest, sort_keys=True, separators=(',', ':')).replace('<', '\\u003c'))


def validate_capture(root):
    expected = {'capture.json', 'index.html'} | {
        name for label in LABELS for name in
        [f'reference-{label}.bmp', *[f'{s}-{label}.bin' for s in SIZES]]}
    require(set(inventory(root)) == expected, 'capture inventory missing/extra files')
    manifest = load(root / 'capture.json')
    require(manifest['schema_version'] == 1 and manifest['scenario'] == SCENARIO, 'capture recipe mismatch')
    require(manifest['rom_sha256'] == ROM and manifest['sram_sha256'] == SRAM, 'owned input mismatch')
    points = manifest['checkpoints']
    require([(p['label'], p['frame']) for p in points] == [(l, l + 1) for l in LABELS], 'observation schedule mismatch')
    for point in points:
        label = point['label']
        for surface, size in SIZES.items():
            data = (root / f'{surface}-{label}.bin').read_bytes()
            require(len(data) == size, f'{surface} extent mismatch')
            require(sha(data) == point[f'{surface}_sha256'], f'{surface} manifest hash mismatch')
        require(sha(rgb((root / f'reference-{label}.bmp').read_bytes())) == point['rgb_sha256'], 'RGB manifest hash mismatch')
    require((root / 'index.html').read_text() == html(manifest), 'derived HTML differs from complete manifest')
    return manifest


def nonpixels(manifest):
    result = deepcopy(manifest)
    for point in result['checkpoints']:
        del point['rgb_sha256']
    return result


def compare_captures(old, fixed, twin):
    require(len({p.resolve() for p in (old, fixed, twin)}) == 3, 'three distinct capture roots required')
    manifests = [validate_capture(p) for p in (old, fixed, twin)]
    require(nonpixels(manifests[0]) == nonpixels(manifests[1]), 'complete non-pixel manifest mismatch')
    inventories = [inventory(p) for p in (old, fixed, twin)]
    # Byte comparison, not selected player/map assertions or merely reported hashes.
    for name in inventories[0]:
        left, right = (old / name).read_bytes(), (fixed / name).read_bytes()
        if name.endswith('.bin'):
            require(left == right, f'complete non-pixel surface mismatch: {name}')
        require((fixed / name).read_bytes() == (twin / name).read_bytes(), f'fixed twins differ: {name}')
    changed = {name: {'old': value, 'fixed': inventories[1][name]}
               for name, value in inventories[0].items()
               if value != inventories[1][name] and name.endswith('.bmp')}
    return {'old': inventories[0], 'fixed': inventories[1], 'twin': inventories[2],
            'nonpixel_manifest_sha256': sha(json.dumps(nonpixels(manifests[0]), sort_keys=True, separators=(',', ':')).encode()),
            'changed': changed, 'checkpoints': [
                {k: v for k, v in p.items() if k.endswith('_sha256') or k in ('label', 'frame')}
                for p in manifests[1]['checkpoints']]}


def verify_nonpixel_digest(report, expected):
    require(report['nonpixel_manifest_sha256'] == expected, 'archived non-pixel digest mismatch')


def audit(rom, save, old, fixed, twin, old_repo):
    require(sha(rom.read_bytes()) == ROM and sha(save.read_bytes()) == SRAM, 'owned ROM/SRAM mismatch')
    archived = load(HERE / 'epochs/threaded-video-v0/reference.json')
    observer = load(HERE / 'observer.json')
    require(archived['policy'] == observer['policy'] == POLICY, 'observation policy mismatch')
    verify_sources(old_repo, archived['source_hashes'])
    verify_sources(REPO, observer['source_hashes'])
    authenticate(old / 'capture', archived['inventory'])
    archive_test = HERE / 'epochs/threaded-video-v0/local_capture.rs'
    require(sha(archive_test.read_bytes()) == archived['test_sha256'], 'archived test mismatch')
    manifests = [validate_capture(root / 'capture') for root in (old, fixed, twin)]
    require([p['rgb_sha256'] for p in manifests[0]['checkpoints']] == archived['rgb_sha256'], 'archived RGB pins not reproduced')
    producers = []
    for root, sources in ((old, archived['source_hashes']), (fixed, observer['source_hashes']), (twin, observer['source_hashes'])):
        producer = load(root / 'producer.json')
        require(producer['source_hashes'] == sources, 'recorded producer source mismatch')
        require(producer['rom_sha256'] == ROM and producer['sram_sha256'] == SRAM, 'recorded producer input mismatch')
        require(sha((root / 'map-inspector').read_bytes()) == producer['binary_sha256'], 'retained binary mismatch')
        require(sha((root / 'build.log').read_bytes()) == producer['build_log_sha256'], 'retained build log mismatch')
        producers.append(producer)
    report = compare_captures(*[r / 'capture' for r in (old, fixed, twin)])
    verify_nonpixel_digest(report, archived['nonpixel_manifest_sha256'])
    return {'schema_version': 1, 'policy': POLICY, 'epoch': observer['epoch'],
            'producers': producers, **report}


if __name__ == '__main__':
    if len(sys.argv) != 7:
        sys.exit('usage: check.py ROM SRAM OLD FIXED TWIN OLD-SOURCE-REPO')
    print(json.dumps(audit(*map(Path, sys.argv[1:])), indent=2, sort_keys=True))
