#!/usr/bin/env python3
"""One explicit same-policy preview producer bridge; never renews output pins."""
import json
from pathlib import Path
import sys

import check
from check import require, sha, load

EPOCH = 'headless-sync-video-v1'
MAIN = 'crates/map-inspector/src/main.rs'
OLD_MAIN_SHA = '7736b543c442e6e4c2789fb13f6f177d1335e78f11810d5023a49c313b27a4d3'
OBSERVER_SHA = '7fabf5688943eca89c43ad5aee02d348187fc3491296b1c401535553fbe3a718'
MIGRATION_SHA = 'db249179718cb6bcf9c1755093d441d0094dd39fc836d079220defd3b289ab3c'
ANCHOR = b'mod opening_qualification;\n'
# Final parent registration: deliberately not an extensible allowlist.
REGISTRATIONS = b'pub mod pandora_navigation;\npub mod pandora_progression;\n'
# Reviewed preview modules/hooks, NOT a retroactive historical inventory or a
# claim of a complete transitive build-input inventory.
ADDITIONAL_FILES = tuple('crates/map-inspector/' + name for name in (
    'src/pandora_navigation.rs', 'src/pandora_progression.rs', 'src/room_art.rs',
    'src/room_art/backgrounds.rs', 'src/room_art/carry.rs', 'src/room_art/door.rs',
    'src/room_art/pandora.rs', 'src/room_art/world_patches.rs',
    'src/room_camera.rs', 'src/room_preview.rs', 'src/room_server.rs', 'web/room-slice.html',
))


def additional_hashes(repo):
    return {name: sha((repo / name).read_bytes()) for name in ADDITIONAL_FILES}


def frozen_json(path, digest):
    data = path.read_bytes()
    require(sha(data) == digest, 'frozen evidence identity mismatch')
    return json.loads(data)


def verify_main_delta(old, current):
    require(sha(old) == OLD_MAIN_SHA, 'unauthenticated old main blob')
    require(old.count(ANCHOR) == 1, 'ambiguous registration anchor')
    require(current == old.replace(ANCHOR, ANCHOR + REGISTRATIONS, 1),
            'main differs from exact registration-only byte insertion')


def verify_descriptor(repo, original, current):
    require(current.get('schema_version') == 1 and current.get('epoch') == original['epoch'] == EPOCH,
            'current producer epoch/identity mismatch')
    require(current['policy'] == original['policy'] == check.POLICY, 'producer policy mismatch')
    require(current['original_descriptor_sha256'] == OBSERVER_SHA and
            current['migration_sha256'] == MIGRATION_SHA, 'historical identity substitution')
    check.verify_sources(repo, current['source_hashes'])
    require({k: v for k, v in current['source_hashes'].items() if k != MAIN} ==
            {k: v for k, v in original['source_hashes'].items() if k != MAIN},
            'non-main historical source changed')
    require(set(current['additional_source_hashes']) == set(ADDITIONAL_FILES),
            'incomplete additional source inventory')
    require(additional_hashes(repo) == current['additional_source_hashes'], 'additional source mismatch')


def verify_current(repo, fixed_repo, descriptor_path):
    original = frozen_json(check.HERE / 'observer.json', OBSERVER_SHA)
    frozen_json(check.HERE / 'migration.json', MIGRATION_SHA)
    check.verify_sources(fixed_repo, original['source_hashes'])
    descriptor = load(descriptor_path)
    verify_descriptor(repo, original, descriptor)
    verify_main_delta((fixed_repo / MAIN).read_bytes(), (repo / MAIN).read_bytes())
    return descriptor


def verify_producer(root, descriptor, descriptor_sha, repo, rom, save):
    producer = load(root / 'producer.json')
    require(producer['schema_version'] == 2 and producer['epoch'] == EPOCH and
            producer['policy'] == check.POLICY, 'current producer identity mismatch')
    require(producer['descriptor_sha256'] == descriptor_sha and
            producer['source_hashes'] == descriptor['source_hashes'] and
            producer['additional_source_hashes'] == descriptor['additional_source_hashes'],
            'recorded current producer source/descriptor mismatch')
    require(producer['rom_sha256'] == check.ROM and producer['sram_sha256'] == check.SRAM,
            'recorded current producer input mismatch')
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
            'recorded current source/input path substitution')
    require(producer['invocation'] == [str(root.resolve() / 'map-inspector'), 'capture',
                                     producer['rom_path'], producer['sram_path']],
            'unexpected capture invocation')
    require(producer['process']['pid'] > 0 and producer['process']['exit_code'] == 0 and
            producer['process']['started_ns'] < producer['process']['finished_ns'],
            'invalid process provenance')
    return producer


def compare_unchanged(accepted_a, accepted_b, current_a, current_b, fixed_repo):
    roots = (accepted_a, accepted_b, current_a, current_b)
    require(len({(p / 'capture').resolve() for p in roots}) == 4, 'four distinct capture roots required')
    report = frozen_json(check.HERE / 'migration.json', MIGRATION_SHA)
    manifests = []
    for root in roots:
        check.authenticate(root / 'capture', report['fixed'])
        manifests.append(check.validate_capture(root / 'capture', fixed_repo))
    for root, manifest in zip(roots[1:], manifests[1:]):
        require(manifest == manifests[0], 'complete manifest changed')
        require(check.nonpixels(manifest) == check.nonpixels(manifests[0]), 'nonpixel manifest changed')
        for name in report['fixed']:
            require((root / 'capture' / name).read_bytes() == (accepted_a / 'capture' / name).read_bytes(),
                    f'capture bytes changed: {name}')
    digest = sha(json.dumps(check.nonpixels(manifests[0]), sort_keys=True, separators=(',', ':')).encode())
    check.verify_nonpixel_digest({'nonpixel_manifest_sha256': digest}, report['nonpixel_manifest_sha256'])
    return {'inventory': check.inventory(current_a / 'capture'),
            'manifest_sha256': sha((current_a / 'capture/capture.json').read_bytes()),
            'nonpixel_manifest_sha256': digest}


def audit(rom, save, old, accepted_a, accepted_b, current_a, current_b, old_repo, fixed_repo, repo, descriptor_path):
    # Explicit historical authentication still reproduces the original evidence;
    # no descriptor selection based on which source tree happens to pass.
    historical = check.audit(rom, save, old, accepted_a, accepted_b, old_repo, fixed_repo)
    require(historical == frozen_json(check.HERE / 'migration.json', MIGRATION_SHA),
            'accepted historical producer envelopes changed')
    require(len(save.read_bytes()) == 8192, 'owned SRAM extent mismatch')
    descriptor = verify_current(repo, fixed_repo, descriptor_path)
    descriptor_sha = sha(descriptor_path.read_bytes())
    producers = [verify_producer(root, descriptor, descriptor_sha, repo, rom, save) for root in (current_a, current_b)]
    require(producers[0]['target_dir'] != producers[1]['target_dir'], 'isolated builds required')
    require(producers[0]['process']['run_id'] != producers[1]['process']['run_id'], 'distinct process identities required')
    equality = compare_unchanged(accepted_a, accepted_b, current_a, current_b, fixed_repo)
    return {'schema_version': 1, 'epoch': EPOCH, 'policy': check.POLICY,
            'original_descriptor_sha256': OBSERVER_SHA, 'migration_sha256': MIGRATION_SHA,
            'current_descriptor_sha256': descriptor_sha,
            'main_delta': {'old_sha256': OLD_MAIN_SHA, 'current_sha256': descriptor['source_hashes'][MAIN],
                           'anchor_utf8': ANCHOR.decode(), 'insert_utf8': REGISTRATIONS.decode()},
            'additional_source_hashes': descriptor['additional_source_hashes'],
            'rom_sha256': check.ROM, 'sram_sha256': check.SRAM,
            'producers': producers, **equality}


if __name__ == '__main__':
    if len(sys.argv) != 12:
        sys.exit('usage: bridge.py ROM SRAM OLD ACCEPTED-A ACCEPTED-B CURRENT-A CURRENT-B OLD-REPO FIXED-REPO CURRENT-REPO CURRENT-DESCRIPTOR')
    print(json.dumps(audit(*map(Path, sys.argv[1:])), indent=2, sort_keys=True))
