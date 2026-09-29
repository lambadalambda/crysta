#!/usr/bin/env python3
"""One explicit oracle audio-capture repin bridge; renews no output pin."""
from functools import lru_cache
import json
from pathlib import Path
import subprocess
import sys

import bridge
import check
import library_bridge
import repin_bridge
from check import load, require, sha
from library_bridge import hashes

EPOCH = bridge.EPOCH
KIND = 'map-inspector-oracle-repin-producer'
REPIN_DESCRIPTOR_SHA = 'e58a0f232a8ce9cc86186e515a9156ca32c4fd992c4a7f38cd859c117a247c33'
REPIN_BRIDGE_SHA = '5fed82d65b682ca001dfa55b3ce6f9c60f9911d7b9cd0313e461e688b167173f'
# Bounded native frame audio capture and CPU audio-port tracing changed both
# sides of the oracle FFI boundary. Neither is on the capture path's output,
# but that is exactly the claim this stage has to evidence, not assume.
REPLACED_FILES = ('crates/oracle/src/lib.rs', 'vendor/ares/shims.cpp')
DESCRIPTOR_FIELDS = {
    'schema_version', 'kind', 'epoch', 'policy', 'original_descriptor_sha256',
    'migration_sha256', 'predecessor_descriptor_sha256', 'predecessor_bridge_sha256',
    'replaced_source_hashes',
}
PRODUCER_FIELDS = repin_bridge.PRODUCER_FIELDS


def frozen_repin():
    descriptor = bridge.frozen_json(check.HERE / 'repin-producer.json', REPIN_DESCRIPTOR_SHA)
    report = bridge.frozen_json(check.HERE / 'repin-producer-bridge.json', REPIN_BRIDGE_SHA)
    return descriptor, report


def repin_sources():
    """The exact sources the frozen repin producer was built from."""
    return repin_bridge.current_sources(frozen_repin()[0])


def current_sources(descriptor):
    current = repin_sources()
    for name, identities in descriptor['replaced_source_hashes'].items():
        current[name] = identities['current_sha256']
    return current


@lru_cache(maxsize=None)
def historical_bytes(name, digest):
    """Committed bytes of `name` with SHA-256 `digest`, for ROM-free controls.

    Earlier stages' trees are reconstructed from Git history, authenticated by
    the pinned digest itself; nothing here is trusted by the bridge.
    """
    revisions = subprocess.check_output(
        ['git', 'rev-list', 'HEAD', '--', name], cwd=check.REPO, text=True).split()
    for revision in revisions:
        for candidate in (revision, f'{revision}^'):
            blob = subprocess.run(['git', 'show', f'{candidate}:{name}'], cwd=check.REPO,
                                  capture_output=True).stdout
            if sha(blob) == digest:
                return blob
    raise ValueError(f'no committed {name} with SHA-256 {digest}')


def verify_oracle_repin_descriptor(repo, fixed_repo, descriptor_path):
    repin, _ = frozen_repin()
    descriptor = load(descriptor_path)
    require(set(descriptor) == DESCRIPTOR_FIELDS, 'oracle repin descriptor fields mismatch')
    require(descriptor['schema_version'] == 1 and descriptor['kind'] == KIND and
            descriptor['epoch'] == repin['epoch'] == EPOCH and
            descriptor['policy'] == repin['policy'] == check.POLICY,
            'oracle repin producer identity mismatch')
    require(descriptor['original_descriptor_sha256'] == bridge.OBSERVER_SHA and
            descriptor['migration_sha256'] == bridge.MIGRATION_SHA and
            descriptor['predecessor_descriptor_sha256'] == REPIN_DESCRIPTOR_SHA and
            descriptor['predecessor_bridge_sha256'] == REPIN_BRIDGE_SHA,
            'oracle repin predecessor identity substitution')
    require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),
            'replaced source inventory mismatch')
    old_sources = repin_sources()
    for name, identities in descriptor['replaced_source_hashes'].items():
        require(set(identities) == {'predecessor_sha256', 'current_sha256'},
                'replacement identity fields mismatch')
        require(identities['predecessor_sha256'] == old_sources[name],
                'replacement predecessor mismatch')
        require(identities['current_sha256'] != identities['predecessor_sha256'],
                'replacement must authenticate a real delta')
    expected_current = current_sources(descriptor)
    require(hashes(repo, tuple(expected_current)) == expected_current,
            'unchanged predecessor source mismatch')
    # main.rs is inherited unchanged, so the repin stage's exact proof still holds.
    repin_bridge.verify_repin_main_delta((fixed_repo / bridge.MAIN).read_bytes(),
                                         (repo / bridge.MAIN).read_bytes())
    return descriptor


def verify_oracle_repin_producer(root, descriptor, descriptor_sha, repo, rom, save):
    producer = load(root / 'producer.json')
    require(set(producer) == PRODUCER_FIELDS, 'oracle repin producer fields mismatch')
    require(producer['schema_version'] == 5 and producer['kind'] == KIND and
            producer['epoch'] == EPOCH and producer['policy'] == check.POLICY,
            'oracle repin producer identity mismatch')
    require(producer['descriptor_sha256'] == descriptor_sha,
            'recorded oracle repin descriptor mismatch')
    require(producer['replaced_source_hashes'] == descriptor['replaced_source_hashes'],
            'recorded oracle repin source mismatch')
    require(producer['source_hashes'] == current_sources(descriptor),
            'recorded bounded source inventory mismatch')
    library_bridge.verify_producer_envelope(root, producer, repo, rom, save)
    return producer


def audit(rom, save, oracle_a, oracle_b, fixed_repo, repo, descriptor_path):
    _, repin_report = frozen_repin()
    require(len(save.read_bytes()) == 8192, 'owned SRAM extent mismatch')
    descriptor = verify_oracle_repin_descriptor(repo, fixed_repo, descriptor_path)
    descriptor_sha = sha(descriptor_path.read_bytes())
    producers = [verify_oracle_repin_producer(root, descriptor, descriptor_sha, repo, rom, save)
                 for root in (oracle_a, oracle_b)]
    require(producers[0]['target_dir'] != producers[1]['target_dir'], 'isolated builds required')
    require(producers[0]['process']['run_id'] != producers[1]['process']['run_id'],
            'distinct process identities required')
    # The frozen repin report carries the same byte inventory as the library
    # report it was itself proved against.
    equality = repin_bridge.compare_to_frozen(repin_report, oracle_a, oracle_b, fixed_repo)
    return {
        'schema_version': 1, 'kind': KIND, 'epoch': EPOCH, 'policy': check.POLICY,
        'original_descriptor_sha256': bridge.OBSERVER_SHA,
        'migration_sha256': bridge.MIGRATION_SHA,
        'predecessor_descriptor_sha256': REPIN_DESCRIPTOR_SHA,
        'predecessor_bridge_sha256': REPIN_BRIDGE_SHA,
        'oracle_repin_descriptor_sha256': descriptor_sha,
        'main_delta': repin_report['main_delta'],
        'replaced_source_hashes': descriptor['replaced_source_hashes'],
        'rom_sha256': check.ROM, 'sram_sha256': check.SRAM,
        'producers': producers, **equality,
    }


if __name__ == '__main__':
    if len(sys.argv) != 8:
        sys.exit('usage: oracle_repin_bridge.py ROM SRAM ORACLE-A ORACLE-B FIXED-REPO '
                 'CURRENT-REPO ORACLE-REPIN-DESCRIPTOR')
    print(json.dumps(audit(*map(Path, sys.argv[1:])), indent=2, sort_keys=True))
