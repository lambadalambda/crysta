#!/usr/bin/env python3
"""Prove repin bridge controls remain active under normal and optimized Python."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
MUTATIONS = (
    ('descriptor-fields',
     "require(set(descriptor) == DESCRIPTOR_FIELDS, 'repin descriptor fields mismatch')",
     "require(True, 'repin descriptor fields mismatch')"),
    ('predecessor-identity',
     "require(descriptor['original_descriptor_sha256'] == bridge.OBSERVER_SHA and\n"
     "            descriptor['migration_sha256'] == bridge.MIGRATION_SHA and\n"
     "            descriptor['predecessor_descriptor_sha256'] == LIBRARY_DESCRIPTOR_SHA and\n"
     "            descriptor['predecessor_bridge_sha256'] == LIBRARY_BRIDGE_SHA,\n"
     "            'repin predecessor identity substitution')",
     "require(True, 'repin predecessor identity substitution')"),
    ('replacement-inventory',
     "require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),\n"
     "            'replaced source inventory mismatch')",
     "require(True, 'replaced source inventory mismatch')"),
    ('replacement-real-delta',
     "require(identities['current_sha256'] != identities['predecessor_sha256'],\n"
     "                'replacement must authenticate a real delta')",
     "require(True, 'replacement must authenticate a real delta')"),
    ('unchanged-source',
     "require(hashes(repo, tuple(expected_current)) == expected_current,\n"
     "            'unchanged predecessor source mismatch')",
     "require(True, 'unchanged predecessor source mismatch')"),
    ('reorder-anchor',
     "require(current.count(FORMATTED_MODS) == 1, 'ambiguous module-order anchor')",
     "require(True, 'ambiguous module-order anchor')"),
    ('old-main-injectivity',
     "require(old.count(PINNED_MODS) == 1, 'ambiguous module order in authenticated old main')",
     "require(True, 'ambiguous module order in authenticated old main')"),
    ('producer-fields',
     "require(set(producer) == PRODUCER_FIELDS, 'repin producer fields mismatch')",
     "require(True, 'repin producer fields mismatch')"),
    ('producer-bounded-source',
     "require(producer['source_hashes'] == current_sources(descriptor),\n"
     "            'recorded bounded source inventory mismatch')",
     "require(True, 'recorded bounded source inventory mismatch')"),
)


def main():
    source = (HERE / 'repin_bridge.py').read_text()
    results = []
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for dependency in (
            'bridge.py', 'check.py', 'library_bridge.py', 'test_repin_bridge.py',
            'repin-producer.json', 'repin-producer-bridge.json', 'library-producer.json',
            'library-producer-bridge.json', 'current-producer.json', 'producer-bridge.json',
            'observer.json', 'migration.json'):
            (root / dependency).write_bytes((HERE / dependency).read_bytes())
        # Tests authenticate the real repository, while importing each mutated
        # bridge from this private directory.
        check_source = (root / 'check.py').read_text().replace(
            'REPO = HERE.parent.parent', f'REPO = Path({str(HERE.parent.parent)!r})')
        (root / 'check.py').write_text(check_source)
        for name, old, new in MUTATIONS:
            if source.count(old) != 1:
                raise ValueError(f'mutation anchor drift: {name}')
            (root / 'repin_bridge.py').write_text(source.replace(old, new))
            for options in (['-B'], ['-O', '-B']):
                result = subprocess.run(
                    [sys.executable, *options, str(root / 'test_repin_bridge.py')],
                    capture_output=True, text=True)
                if result.returncode == 0 or 'FAIL:' not in result.stderr:
                    raise ValueError(f'mutation escaped or test broke: {name}: {result.stderr}')
                results.append({'mutation': name, 'optimized': '-O' in options, 'detected': True})
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
