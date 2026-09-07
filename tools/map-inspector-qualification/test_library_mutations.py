#!/usr/bin/env python3
"""Prove library bridge controls remain active under normal and optimized Python."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
MUTATIONS = (
    ('descriptor-fields',
     "require(set(descriptor) == DESCRIPTOR_FIELDS, 'library descriptor fields mismatch')",
     "require(True, 'library descriptor fields mismatch')"),
    ('replacement-inventory',
     "require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),\n            'replaced source inventory mismatch')",
     "require(True, 'replaced source inventory mismatch')"),
    ('group-source',
     "require(descriptor[field] == hashes(repo, files), f'{field} source mismatch')",
     "require(True, f'{field} source mismatch')"),
    ('producer-fields',
     "require(set(producer) == PRODUCER_FIELDS, 'library producer fields mismatch')",
     "require(True, 'library producer fields mismatch')"),
    ('producer-bounded-source',
     "require(producer['source_hashes'] == current_sources(descriptor, predecessor),\n            'recorded bounded source inventory mismatch')",
     "require(True, 'recorded bounded source inventory mismatch')"),
)


def main():
    source = (HERE / 'library_bridge.py').read_text()
    results = []
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for dependency in (
            'bridge.py', 'check.py', 'test_library_bridge.py', 'library-producer.json',
            'current-producer.json', 'producer-bridge.json', 'observer.json', 'migration.json'):
            (root / dependency).write_bytes((HERE / dependency).read_bytes())
        # Tests authenticate the real repository, while importing each mutated
        # bridge from this private directory.
        check_source = (root / 'check.py').read_text().replace(
            'REPO = HERE.parent.parent', f'REPO = Path({str(HERE.parent.parent)!r})')
        (root / 'check.py').write_text(check_source)
        for name, old, new in MUTATIONS:
            if source.count(old) != 1:
                raise ValueError(f'mutation anchor drift: {name}')
            (root / 'library_bridge.py').write_text(source.replace(old, new))
            for options in (['-B'], ['-O', '-B']):
                result = subprocess.run(
                    [sys.executable, *options, str(root / 'test_library_bridge.py')],
                    capture_output=True, text=True)
                if result.returncode == 0 or 'FAIL:' not in result.stderr:
                    raise ValueError(f'mutation escaped or test broke: {name}: {result.stderr}')
                results.append({'mutation': name, 'optimized': '-O' in options, 'detected': True})
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
