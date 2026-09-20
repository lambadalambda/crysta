#!/usr/bin/env python3
"""One explicit rustfmt-only repin bridge; renews no output pin."""
import json
from pathlib import Path
import sys

import bridge
import check
import library_bridge
from check import load, require, sha
from library_bridge import hashes

EPOCH = bridge.EPOCH
KIND = 'map-inspector-repin-producer'
LIBRARY_DESCRIPTOR_SHA = '00298d9350a143abeb83bb95ae093feba81d6c9850ab4722bf015834d88f6143'
LIBRARY_BRIDGE_SHA = '18cfd3ec329e70159d3ad7613dd73f826d03b55c573274661337f9277060b75d'
# The whole point of this stage: one inert rustfmt declaration reorder in a
# pinned producer source, applied with evidence instead of by argument.
REPLACED_FILES = ('crates/map-inspector/src/main.rs',)
PINNED_MODS = b'mod house_progression;\nmod house_profiles;\n'
FORMATTED_MODS = b'mod house_profiles;\nmod house_progression;\n'
DESCRIPTOR_FIELDS = {
    'schema_version', 'kind', 'epoch', 'policy', 'original_descriptor_sha256',
    'migration_sha256', 'predecessor_descriptor_sha256', 'predecessor_bridge_sha256',
    'replaced_source_hashes',
}
# Unchanged library/qualification/adapter inventories stay derived from the
# frozen library descriptor; this stage deliberately has no field to restate them.
PRODUCER_FIELDS = library_bridge.PRODUCER_FIELDS - set(library_bridge.GROUPS)


def frozen_library():
    descriptor = bridge.frozen_json(check.HERE / 'library-producer.json', LIBRARY_DESCRIPTOR_SHA)
    report = bridge.frozen_json(check.HERE / 'library-producer-bridge.json', LIBRARY_BRIDGE_SHA)
    return descriptor, report


def library_sources():
    """The exact sources the frozen library producer was built from."""
    predecessor = library_bridge.frozen_predecessor()[1]
    return library_bridge.current_sources(frozen_library()[0], predecessor)


def current_sources(descriptor):
    current = library_sources()
    for name, identities in descriptor['replaced_source_hashes'].items():
        current[name] = identities['current_sha256']
    return current


def verify_repin_main_delta(old, current):
    """Undo only the exact reorder, then require the registration-only proof.

    Both anchors must occur exactly once, which is what makes the accepted set a
    singleton: the reconstruction is injective, so no other main.rs passes.
    """
    require(old.count(PINNED_MODS) == 1, 'ambiguous module order in authenticated old main')
    require(current.count(FORMATTED_MODS) == 1, 'ambiguous module-order anchor')
    bridge.verify_main_delta(old, current.replace(FORMATTED_MODS, PINNED_MODS, 1))


def verify_repin_descriptor(repo, fixed_repo, descriptor_path):
    library, _ = frozen_library()
    descriptor = load(descriptor_path)
    require(set(descriptor) == DESCRIPTOR_FIELDS, 'repin descriptor fields mismatch')
    require(descriptor['schema_version'] == 1 and descriptor['kind'] == KIND and
            descriptor['epoch'] == library['epoch'] == EPOCH and
            descriptor['policy'] == library['policy'] == check.POLICY,
            'repin producer identity mismatch')
    require(descriptor['original_descriptor_sha256'] == bridge.OBSERVER_SHA and
            descriptor['migration_sha256'] == bridge.MIGRATION_SHA and
            descriptor['predecessor_descriptor_sha256'] == LIBRARY_DESCRIPTOR_SHA and
            descriptor['predecessor_bridge_sha256'] == LIBRARY_BRIDGE_SHA,
            'repin predecessor identity substitution')
    require(set(descriptor['replaced_source_hashes']) == set(REPLACED_FILES),
            'replaced source inventory mismatch')
    old_sources = library_sources()
    for name, identities in descriptor['replaced_source_hashes'].items():
        require(set(identities) == {'predecessor_sha256', 'current_sha256'},
                'replacement identity fields mismatch')
        require(identities['predecessor_sha256'] == old_sources[name],
                'replacement predecessor mismatch')
        require(identities['current_sha256'] == sha((repo / name).read_bytes()),
                'replacement current source mismatch')
        require(identities['current_sha256'] != identities['predecessor_sha256'],
                'replacement must authenticate a real delta')
    expected_current = current_sources(descriptor)
    require(hashes(repo, tuple(expected_current)) == expected_current,
            'unchanged predecessor source mismatch')
    verify_repin_main_delta((fixed_repo / bridge.MAIN).read_bytes(),
                            (repo / bridge.MAIN).read_bytes())
    return descriptor


def verify_repin_producer(root, descriptor, descriptor_sha, repo, rom, save):
    producer = load(root / 'producer.json')
    require(set(producer) == PRODUCER_FIELDS, 'repin producer fields mismatch')
    require(producer['schema_version'] == 4 and producer['kind'] == KIND and
            producer['epoch'] == EPOCH and producer['policy'] == check.POLICY,
            'repin producer identity mismatch')
    require(producer['descriptor_sha256'] == descriptor_sha, 'recorded repin descriptor mismatch')
    require(producer['replaced_source_hashes'] == descriptor['replaced_source_hashes'],
            'recorded repin source mismatch')
    require(producer['source_hashes'] == current_sources(descriptor),
            'recorded bounded source inventory mismatch')
    library_bridge.verify_producer_envelope(root, producer, repo, rom, save)
    return producer


def compare_to_frozen(library_report, repin_a, repin_b, fixed_repo):
    """Same-output proof against the frozen library report.

    The five accepted capture roots the full library audit consumes were
    retained under ignored local/ in worktrees that no longer exist, and
    migration.json pins their non-reproducible binary/build-log identities, so
    that chain can never be re-derived. The frozen report's own inventory is
    the strongest still-available expectation: it carries per-file sizes and
    SHA-256 values, so matching it is byte equality with the accepted output.
    """
    roots = (repin_a, repin_b)
    require(len({(p / 'capture').resolve() for p in roots}) == 2,
            'two distinct capture roots required')
    manifests = [check.validate_capture(root / 'capture', fixed_repo) for root in roots]
    require(manifests[1] == manifests[0], 'complete manifest changed')
    frozen_inventory = library_report['inventory']
    for root in roots:
        require(check.inventory(root / 'capture') == frozen_inventory,
                'capture inventory differs from the frozen library producer')
    for name in frozen_inventory:
        require((repin_a / 'capture' / name).read_bytes() ==
                (repin_b / 'capture' / name).read_bytes(),
                f'capture bytes differ between repin roots: {name}')
    manifest_sha = sha((repin_a / 'capture/capture.json').read_bytes())
    require(manifest_sha == library_report['manifest_sha256'], 'complete manifest changed')
    digest = sha(json.dumps(check.nonpixels(manifests[0]), sort_keys=True,
                            separators=(',', ':')).encode())
    require(digest == library_report['nonpixel_manifest_sha256'], 'nonpixel manifest changed')
    return {'inventory': check.inventory(repin_a / 'capture'),
            'manifest_sha256': manifest_sha, 'nonpixel_manifest_sha256': digest}


def audit(rom, save, repin_a, repin_b, fixed_repo, repo, descriptor_path):
    library, library_report = frozen_library()
    require(len(save.read_bytes()) == 8192, 'owned SRAM extent mismatch')
    descriptor = verify_repin_descriptor(repo, fixed_repo, descriptor_path)
    descriptor_sha = sha(descriptor_path.read_bytes())
    producers = [verify_repin_producer(root, descriptor, descriptor_sha, repo, rom, save)
                 for root in (repin_a, repin_b)]
    require(producers[0]['target_dir'] != producers[1]['target_dir'], 'isolated builds required')
    require(producers[0]['process']['run_id'] != producers[1]['process']['run_id'],
            'distinct process identities required')
    equality = compare_to_frozen(library_report, repin_a, repin_b, fixed_repo)
    return {
        'schema_version': 1, 'kind': KIND, 'epoch': EPOCH, 'policy': check.POLICY,
        'original_descriptor_sha256': bridge.OBSERVER_SHA,
        'migration_sha256': bridge.MIGRATION_SHA,
        'predecessor_descriptor_sha256': LIBRARY_DESCRIPTOR_SHA,
        'predecessor_bridge_sha256': LIBRARY_BRIDGE_SHA,
        'repin_descriptor_sha256': descriptor_sha,
        'main_delta': {
            'predecessor_sha256': library_sources()[bridge.MAIN],
            'current_sha256': descriptor['replaced_source_hashes'][bridge.MAIN]['current_sha256'],
            'old_sha256': bridge.OLD_MAIN_SHA,
            'anchor_utf8': bridge.ANCHOR.decode(),
            'insert_utf8': bridge.REGISTRATIONS.decode(),
            'reorder_from_utf8': PINNED_MODS.decode(),
            'reorder_to_utf8': FORMATTED_MODS.decode(),
        },
        'replaced_source_hashes': descriptor['replaced_source_hashes'],
        'rom_sha256': check.ROM, 'sram_sha256': check.SRAM,
        'producers': producers, **equality,
    }


if __name__ == '__main__':
    if len(sys.argv) != 8:
        sys.exit('usage: repin_bridge.py ROM SRAM REPIN-A REPIN-B FIXED-REPO CURRENT-REPO REPIN-DESCRIPTOR')
    print(json.dumps(audit(*map(Path, sys.argv[1:])), indent=2, sort_keys=True))
