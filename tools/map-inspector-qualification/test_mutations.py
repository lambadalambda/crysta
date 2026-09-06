#!/usr/bin/env python3
"""Prove targeted controls fail when their guard is disabled; never edit sources."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
MUTATIONS = (
    ('all-require', 'if not condition:', 'if False:', None),
    ('source', "require(source_hashes(repo) == expected, 'producer source mismatch')", "require(True, 'producer source mismatch')", 'test_every_source_pin'),
    ('archive', "require(inventory(root) == expected, 'capture differs from archived inventory')", "require(True, 'capture differs from archived inventory')", 'test_archive_authentication'),
    ('nonpixel-digest', "require(report['nonpixel_manifest_sha256'] == expected,", 'require(True,', 'test_archived_nonpixel_digest'),
    ('aliases', "require(len({p.resolve() for p in (old, fixed, twin)}) == 3,", 'require(True,', 'test_alias'),
    ('full-manifest', 'require(nonpixels(manifests[0]) == nonpixels(manifests[1]),', 'require(True,', 'test_full_manifest'),
    ('fixed-pixels', "require((fixed / name).read_bytes() == (twin / name).read_bytes(),", 'require(True,', 'test_fixed_pixel_mutation'),
    ('schedule', "require([(p['label'], p['frame']) for p in points] == [(l, l + 1) for l in LABELS],", 'require(True,', 'test_recipe_and_input'),
    ('rgb-hash', "require(sha(rgb((root / f'reference-{label}.bmp').read_bytes())) == point['rgb_sha256'],", 'require(True,', 'test_surface_hash_and_extent'),
    ('bgr-conversion', 'bgr[2::3], bgr[1::3], bgr[0::3]', 'bgr[0::3], bgr[1::3], bgr[2::3]', 'test_conversion'),
)


def main():
    source = (HERE / 'check.py').read_text().replace('REPO = HERE.parent.parent', f'REPO = Path({str(HERE.parent.parent)!r})')
    results = []
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / 'test_check.py').write_bytes((HERE / 'test_check.py').read_bytes())
        for name, old, new, test in MUTATIONS:
            if source.count(old) != 1:
                raise ValueError(f'mutation anchor drift: {name}')
            (root / 'check.py').write_text(source.replace(old, new))
            for options in (['-B'], ['-O', '-B']):
                command = [sys.executable, *options, str(root / 'test_check.py')]
                if test:
                    command.append(f'Gates.{test}')
                result = subprocess.run(command, capture_output=True, text=True)
                if result.returncode == 0 or 'FAIL:' not in result.stderr:
                    raise ValueError(f'mutation escaped or test broke: {name}: {result.stderr}')
                results.append({'mutation': name, 'optimized': '-O' in options, 'detected': True})
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
