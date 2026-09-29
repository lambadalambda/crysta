#!/usr/bin/env python3
"""Prove oracle repin bridge controls remain active under normal and optimized Python."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
MUTATIONS = (
    ('descriptor-fields',
     "require(set(descriptor) == DESCRIPTOR_FIELDS, 'oracle repin descriptor fields mismatch')",
     "require(True, 'oracle repin descriptor fields mismatch')"),
    ('descriptor-identity',
     "require(descriptor['schema_version'] == 1 and descriptor['kind'] == KIND and\n"
     "            descriptor['epoch'] == repin['epoch'] == EPOCH and\n"
     "            descriptor['policy'] == repin['policy'] == check.POLICY,\n"
     "            'oracle repin producer identity mismatch')",
     "require(True, 'oracle repin producer identity mismatch')"),
    ('predecessor-identity',
     "require(descriptor['original_descriptor_sha256'] == bridge.OBSERVER_SHA and\n"
     "            descriptor['migration_sha256'] == bridge.MIGRATION_SHA and\n"
     "            descriptor['predecessor_descriptor_sha256'] == REPIN_DESCRIPTOR_SHA and\n"
     "            descriptor['predecessor_bridge_sha256'] == REPIN_BRIDGE_SHA,\n"
     "            'oracle repin predecessor identity substitution')",
     "require(True, 'oracle repin predecessor identity substitution')"),
    ('replacement-inventory',
     "require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),\n"
     "            'replaced source inventory mismatch')",
     "require(True, 'replaced source inventory mismatch')"),
    ('replacement-fields',
     "require(set(identities) == {'predecessor_sha256', 'current_sha256'},\n"
     "                'replacement identity fields mismatch')",
     "require(True, 'replacement identity fields mismatch')"),
    ('replacement-predecessor',
     "require(identities['predecessor_sha256'] == old_sources[name],\n"
     "                'replacement predecessor mismatch')",
     "require(True, 'replacement predecessor mismatch')"),
    ('replacement-real-delta',
     "require(identities['current_sha256'] != identities['predecessor_sha256'],\n"
     "                'replacement must authenticate a real delta')",
     "require(True, 'replacement must authenticate a real delta')"),
    ('unchanged-source',
     "require(hashes(repo, tuple(expected_current)) == expected_current,\n"
     "            'unchanged predecessor source mismatch')",
     "require(True, 'unchanged predecessor source mismatch')"),
    ('inherited-main-proof',
     "    repin_bridge.verify_repin_main_delta((fixed_repo / bridge.MAIN).read_bytes(),\n"
     "                                         (repo / bridge.MAIN).read_bytes())\n",
     ""),
    ('producer-fields',
     "require(set(producer) == PRODUCER_FIELDS, 'oracle repin producer fields mismatch')",
     "require(True, 'oracle repin producer fields mismatch')"),
    ('producer-identity',
     "require(producer['schema_version'] == 5 and producer['kind'] == KIND and\n"
     "            producer['epoch'] == EPOCH and producer['policy'] == check.POLICY,\n"
     "            'oracle repin producer identity mismatch')",
     "require(True, 'oracle repin producer identity mismatch')"),
    ('producer-descriptor',
     "require(producer['descriptor_sha256'] == descriptor_sha,",
     "require(True,"),
    ('producer-replacement',
     "require(producer['replaced_source_hashes'] == descriptor['replaced_source_hashes'],",
     "require(True,"),
    ('producer-bounded-source',
     "require(producer['source_hashes'] == current_sources(descriptor),\n"
     "            'recorded bounded source inventory mismatch')",
     "require(True, 'recorded bounded source inventory mismatch')"),
)


def main():
    source = (HERE / 'oracle_repin_bridge.py').read_text()
    results = []
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for dependency in (
            'bridge.py', 'check.py', 'library_bridge.py', 'repin_bridge.py',
            'projection.py', 'test_oracle_repin_bridge.py', 'oracle-repin-producer.json',
            'repin-producer.json', 'repin-producer-bridge.json', 'library-producer.json',
            'library-producer-bridge.json', 'current-producer.json', 'producer-bridge.json',
            'observer.json', 'migration.json'):
            (root / dependency).write_bytes((HERE / dependency).read_bytes())
        shutil.copytree(HERE / 'pinned', root / 'pinned')
        # Tests authenticate the real repository, while importing each mutated
        # bridge from this private directory.
        check_source = (root / 'check.py').read_text().replace(
            'REPO = HERE.parent.parent', f'REPO = Path({str(HERE.parent.parent)!r})')
        (root / 'check.py').write_text(check_source)
        for name, old, new in MUTATIONS:
            if source.count(old) != 1:
                raise ValueError(f'mutation anchor drift: {name}')
            (root / 'oracle_repin_bridge.py').write_text(source.replace(old, new))
            for options in (['-B'], ['-O', '-B']):
                result = subprocess.run(
                    [sys.executable, *options, str(root / 'test_oracle_repin_bridge.py')],
                    capture_output=True, text=True)
                if result.returncode == 0 or 'FAIL:' not in result.stderr:
                    raise ValueError(f'mutation escaped or test broke: {name}: {result.stderr}')
                results.append({'mutation': name, 'optimized': '-O' in options, 'detected': True})
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
