#!/usr/bin/env python3
"""One explicit same-output library/Wasm producer bridge; never renews old pins."""
import json
from pathlib import Path
import sys

import bridge
import projection
import check
from check import load, require, sha

EPOCH = bridge.EPOCH
PREDECESSOR_DESCRIPTOR_SHA = '85de8d72d6f0a433345645f5dd86f5c80f8e1ffd18549fb357a59b2d97590714'
PREDECESSOR_BRIDGE_SHA = '46a2b7fda7525c8c7da664b83ec182160b39f4a67d8d0e958c573cea606795c0'
KIND = 'map-inspector-library-producer'
REPLACED_FILES = (
    'Cargo.lock',
    'crates/map-inspector/Cargo.toml',
    'crates/map-inspector/src/room_preview.rs',
    'crates/map-inspector/web/room-slice.html',
)
LIBRARY_FILES = (
    'crates/map-inspector/src/lib.rs',
    'crates/map-inspector/src/static_background.rs',
    'crates/map-inspector/src/visual_export.rs',
)
QUALIFICATION_FILES = ('crates/map-inspector/tests/public_preview.rs',)
ADAPTER_FILES = (
    'Cargo.toml',
    'crates/pandora-web/Cargo.toml',
    'crates/pandora-web/examples/parity.rs',
    'crates/pandora-web/src/lib.rs',
    'crates/pandora-web/tests/session.rs',
    'tools/pandora-preview/README.md',
    'tools/pandora-preview/bootstrap.mjs',
    'tools/pandora-preview/bootstrap.test.mjs',
    'tools/pandora-preview/build.sh',
    'tools/pandora-preview/main.mjs',
    'tools/pandora-preview/parity-actions.json',
    'tools/pandora-preview/parity.mjs',
    'tools/pandora-preview/runtime-loader.js',
    'tools/pandora-preview/worker.mjs',
)
GROUPS = {
    'library_source_hashes': LIBRARY_FILES,
    'qualification_source_hashes': QUALIFICATION_FILES,
    'adapter_source_hashes': ADAPTER_FILES,
}
DESCRIPTOR_FIELDS = {
    'schema_version', 'kind', 'epoch', 'policy', 'original_descriptor_sha256',
    'migration_sha256', 'predecessor_descriptor_sha256', 'predecessor_bridge_sha256',
    'replaced_source_hashes', *GROUPS,
}
PRODUCER_FIELDS = {
    'schema_version', 'kind', 'epoch', 'policy', 'commit', 'descriptor_sha256',
    'source_hashes', 'replaced_source_hashes', *GROUPS,
    'binary_sha256', 'rom_sha256', 'sram_sha256', 'build_command', 'target_dir',
    'invocation', 'rustc', 'cargo', 'cxx', 'build_env', 'build_log_sha256',
    'fresh_target', 'source_repo', 'rom_path', 'sram_path', 'stdout_sha256',
    'stderr_sha256', 'process',
}


def hashes(repo, files):
    return {name: projection.effective_sha(repo, name) for name in files}


def frozen_predecessor():
    descriptor_path = check.HERE / 'current-producer.json'
    report_path = check.HERE / 'producer-bridge.json'
    predecessor = bridge.frozen_json(descriptor_path, PREDECESSOR_DESCRIPTOR_SHA)
    report = bridge.frozen_json(report_path, PREDECESSOR_BRIDGE_SHA)
    return descriptor_path, predecessor, report


def predecessor_sources(predecessor):
    return predecessor['source_hashes'] | predecessor['additional_source_hashes']


def current_sources(descriptor, predecessor):
    current = predecessor_sources(predecessor).copy()
    for name, identities in descriptor['replaced_source_hashes'].items():
        current[name] = identities['current_sha256']
    return current


def verify_library_descriptor(repo, predecessor_repo, fixed_repo, descriptor_path):
    predecessor_path, predecessor, _ = frozen_predecessor()
    bridge.verify_current(predecessor_repo, fixed_repo, predecessor_path)
    descriptor = load(descriptor_path)
    require(set(descriptor) == DESCRIPTOR_FIELDS, 'library descriptor fields mismatch')
    require(descriptor['schema_version'] == 1 and descriptor['kind'] == KIND and
            descriptor['epoch'] == predecessor['epoch'] == EPOCH and
            descriptor['policy'] == predecessor['policy'] == check.POLICY,
            'library producer identity mismatch')
    require(descriptor['original_descriptor_sha256'] == bridge.OBSERVER_SHA and
            descriptor['migration_sha256'] == bridge.MIGRATION_SHA and
            descriptor['predecessor_descriptor_sha256'] == PREDECESSOR_DESCRIPTOR_SHA and
            descriptor['predecessor_bridge_sha256'] == PREDECESSOR_BRIDGE_SHA,
            'library predecessor identity substitution')
    require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),
            'replaced source inventory mismatch')
    old_sources = predecessor_sources(predecessor)
    for name, identities in descriptor['replaced_source_hashes'].items():
        require(set(identities) == {'predecessor_sha256', 'current_sha256'},
                'replacement identity fields mismatch')
        require(identities['predecessor_sha256'] == old_sources[name],
                'replacement predecessor mismatch')
        require(identities['current_sha256'] == projection.effective_sha(repo, name),
                'replacement current source mismatch')
        require(identities['current_sha256'] != identities['predecessor_sha256'],
                'replacement must authenticate a real delta')
    for field, files in GROUPS.items():
        require(set(descriptor[field]) == set(files), f'{field} inventory mismatch')
        require(descriptor[field] == hashes(repo, files), f'{field} source mismatch')
    expected_current = current_sources(descriptor, predecessor)
    require(hashes(repo, tuple(expected_current)) == expected_current,
            'unchanged predecessor source mismatch')
    bridge.verify_main_delta((fixed_repo / bridge.MAIN).read_bytes(), (repo / bridge.MAIN).read_bytes())
    return descriptor


def verify_library_producer(root, descriptor, descriptor_sha, repo, rom, save):
    producer = load(root / 'producer.json')
    require(set(producer) == PRODUCER_FIELDS, 'library producer fields mismatch')
    require(producer['schema_version'] == 3 and producer['kind'] == KIND and
            producer['epoch'] == EPOCH and producer['policy'] == check.POLICY,
            'library producer identity mismatch')
    require(producer['descriptor_sha256'] == descriptor_sha,
            'recorded library descriptor mismatch')
    for field in ('replaced_source_hashes', *GROUPS):
        require(producer[field] == descriptor[field], 'recorded library source mismatch')
    predecessor = frozen_predecessor()[1]
    require(producer['source_hashes'] == current_sources(descriptor, predecessor),
            'recorded bounded source inventory mismatch')
    verify_producer_envelope(root, producer, repo, rom, save)
    return producer


def verify_producer_envelope(root, producer, repo, rom, save):
    """Build/input/process provenance shared by every descriptor-mode stage."""
    require(producer['rom_sha256'] == check.ROM and producer['sram_sha256'] == check.SRAM,
            'recorded producer input mismatch')
    for name, field in (('map-inspector', 'binary_sha256'), ('build.log', 'build_log_sha256'),
                        ('stdout.txt', 'stdout_sha256'), ('stderr.txt', 'stderr_sha256')):
        require(sha((root / name).read_bytes()) == producer[field], f'retained {name} mismatch')
    require(producer['build_command'] == ['cargo', 'build', '--locked', '-p', 'map-inspector'],
            'unexpected build command')
    require(producer['fresh_target'] is True and Path(producer['target_dir']) == root.resolve() / 'target',
            'build target was not isolated')
    require(producer['build_env']['CARGO_TARGET_DIR'] == producer['target_dir'], 'build target env mismatch')
    require(producer['source_repo'] == str(repo.resolve()) and
            producer['rom_path'] == str(rom.resolve()) and producer['sram_path'] == str(save.resolve()),
            'recorded producer source/input path substitution')
    require(producer['invocation'] == [str(root.resolve() / 'map-inspector'), 'capture',
                                     producer['rom_path'], producer['sram_path']],
            'unexpected capture invocation')
    process = producer['process']
    require(set(process) == {'run_id', 'pid', 'exit_code', 'started_ns', 'finished_ns'} and
            isinstance(process['run_id'], str) and process['run_id'] and
            process['pid'] > 0 and process['exit_code'] == 0 and
            process['started_ns'] < process['finished_ns'], 'invalid process provenance')


def audit(rom, save, old, accepted_a, accepted_b, prior_a, prior_b, library_a, library_b,
          old_repo, fixed_repo, predecessor_repo, repo, descriptor_path):
    predecessor_path, _, frozen_report = frozen_predecessor()
    prior_report = bridge.audit(rom, save, old, accepted_a, accepted_b, prior_a, prior_b,
                                old_repo, fixed_repo, predecessor_repo, predecessor_path)
    require(prior_report == frozen_report, 'predecessor bridge changed')
    require(len(save.read_bytes()) == 8192, 'owned SRAM extent mismatch')
    descriptor = verify_library_descriptor(repo, predecessor_repo, fixed_repo, descriptor_path)
    descriptor_sha = sha(descriptor_path.read_bytes())
    producers = [verify_library_producer(root, descriptor, descriptor_sha, repo, rom, save)
                 for root in (library_a, library_b)]
    require(producers[0]['target_dir'] != producers[1]['target_dir'], 'isolated builds required')
    require(producers[0]['process']['run_id'] != producers[1]['process']['run_id'],
            'distinct process identities required')
    equality = bridge.compare_unchanged(accepted_a, accepted_b, library_a, library_b, fixed_repo)
    return {
        'schema_version': 1, 'kind': KIND, 'epoch': EPOCH, 'policy': check.POLICY,
        'original_descriptor_sha256': bridge.OBSERVER_SHA,
        'migration_sha256': bridge.MIGRATION_SHA,
        'predecessor_descriptor_sha256': PREDECESSOR_DESCRIPTOR_SHA,
        'predecessor_bridge_sha256': PREDECESSOR_BRIDGE_SHA,
        'library_descriptor_sha256': descriptor_sha,
        'main_delta': frozen_report['main_delta'],
        'replaced_source_hashes': descriptor['replaced_source_hashes'],
        **{field: descriptor[field] for field in GROUPS},
        'rom_sha256': check.ROM, 'sram_sha256': check.SRAM,
        'producers': producers, **equality,
    }


if __name__ == '__main__':
    if len(sys.argv) != 15:
        sys.exit('usage: library_bridge.py ROM SRAM OLD ACCEPTED-A ACCEPTED-B PRIOR-A PRIOR-B LIBRARY-A LIBRARY-B OLD-REPO FIXED-REPO PREDECESSOR-REPO CURRENT-REPO LIBRARY-DESCRIPTOR')
    print(json.dumps(audit(*map(Path, sys.argv[1:])), indent=2, sort_keys=True))
