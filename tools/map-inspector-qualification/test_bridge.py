#!/usr/bin/env python3
"""ROM-free controls for the one explicit preview producer bridge."""
from copy import deepcopy
from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import patch
import shutil

import check
import bridge
import test_check


class BridgeGates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name).resolve()
        self.old = b'// fixture\nmod opening_qualification;\nfn main() {}\n'
        self.current = (b'// fixture\nmod opening_qualification;\n'
                        b'pub mod pandora_navigation;\npub mod pandora_progression;\nfn main() {}\n')
        self.original = check.load(check.HERE / 'observer.json')
        for name in check.SOURCE_FILES:
            path = self.repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(self.current if name == bridge.MAIN else b'source')
        for name in bridge.ADDITIONAL_FILES:
            path = self.repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'additional')
        self.original['source_hashes'] = check.source_hashes(self.repo)
        self.original['source_hashes'][bridge.MAIN] = check.sha(self.old)
        self.descriptor = dict(schema_version=1, epoch=bridge.EPOCH, policy=check.POLICY,
                               original_descriptor_sha256=bridge.OBSERVER_SHA,
                               migration_sha256=bridge.MIGRATION_SHA,
                               source_hashes=check.source_hashes(self.repo),
                               additional_source_hashes=bridge.additional_hashes(self.repo))
        p = patch.object(bridge, 'OLD_MAIN_SHA', check.sha(self.old))
        p.start(); self.addCleanup(p.stop)

    def verify(self, descriptor=None):
        bridge.verify_descriptor(self.repo, self.original, descriptor or self.descriptor)
        bridge.verify_main_delta(self.old, (self.repo / bridge.MAIN).read_bytes())

    def test_exact_registration_bytes(self):
        self.verify()
        mutations = [self.current + b'\n', self.current.replace(b'\n', b'\r\n'),
                     self.current.replace(b'fn main()', b'fn changed()'),
                     self.current.replace(bridge.REGISTRATIONS, b''),
                     self.current.replace(b'pub mod pandora_progression;\n', b''),
                     self.current.replace(b'pub mod pandora_navigation;\n', b''),
                     self.current.replace(bridge.REGISTRATIONS, bridge.REGISTRATIONS * 2),
                     self.current.replace(b'pub mod pandora_navigation;', b'mod pandora_navigation;'),
                     self.current.replace(b'mod opening_qualification;', b'mod opening_qualification; ')]
        for changed in mutations:
            with self.subTest(changed=changed):
                with self.assertRaisesRegex(ValueError, 'registration'):
                    bridge.verify_main_delta(self.old, changed)

    def test_old_blob_must_be_authenticated(self):
        with self.assertRaisesRegex(ValueError, 'old main'):
            bridge.verify_main_delta(self.old + b'\n', self.current + b'\n')

    def test_descriptor_identity_substitution(self):
        for field in ('epoch', 'policy', 'original_descriptor_sha256', 'migration_sha256'):
            changed = deepcopy(self.descriptor); changed[field] = 'substitution'
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify(changed)
        with self.assertRaises(ValueError):
            self.verify(self.original)

    def test_cannot_repin_other_historical_sources(self):
        for name in check.SOURCE_FILES:
            if name == bridge.MAIN:
                continue
            path = self.repo / name; path.write_bytes(b'changed')
            changed = deepcopy(self.descriptor)
            changed['source_hashes'][name] = check.sha(b'changed')
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'historical'):
                self.verify(changed)
            path.write_bytes(b'source')

    def test_whole_main_gate_and_additional_sources(self):
        for name in (bridge.MAIN, *bridge.ADDITIONAL_FILES):
            path = self.repo / name; original = path.read_bytes(); path.write_bytes(b'changed')
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'source'):
                self.verify()
            path.write_bytes(original)
        for field in ('source_hashes', 'additional_source_hashes'):
            for name in self.descriptor[field]:
                changed = deepcopy(self.descriptor); del changed[field][name]
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'inventory'):
                    self.verify(changed)

    def test_nonregistration_change_even_when_resealed(self):
        changed = self.current.replace(b'fn main()', b'fn changed()')
        (self.repo / bridge.MAIN).write_bytes(changed)
        self.descriptor['source_hashes'][bridge.MAIN] = check.sha(changed)
        with self.assertRaisesRegex(ValueError, 'registration'):
            self.verify()

    def producer(self, root, descriptor_sha='descriptor'):
        root.mkdir()
        rom, save = self.repo / 'rom', self.repo / 'save'
        producer = dict(schema_version=2, epoch=bridge.EPOCH, policy=check.POLICY,
                        descriptor_sha256=descriptor_sha, source_hashes=self.descriptor['source_hashes'],
                        additional_source_hashes=self.descriptor['additional_source_hashes'],
                        rom_sha256=check.ROM, sram_sha256=check.SRAM,
                        build_command=['cargo', 'build', '--locked', '-p', 'map-inspector'],
                        fresh_target=True, target_dir=str(root / 'target'),
                        build_env={'CARGO_TARGET_DIR': str(root / 'target')},
                        source_repo=str(self.repo), rom_path=str(rom), sram_path=str(save),
                        invocation=[str(root / 'map-inspector'), 'capture', str(rom), str(save)],
                        process=dict(pid=123, exit_code=0, started_ns=1, finished_ns=2, run_id='one'))
        for name, field in (('map-inspector', 'binary_sha256'), ('build.log', 'build_log_sha256'),
                            ('stdout.txt', 'stdout_sha256'), ('stderr.txt', 'stderr_sha256')):
            (root / name).write_bytes(name.encode()); producer[field] = check.sha(name.encode())
        return producer

    def test_current_process_provenance_and_substitution(self):
        root = self.repo / 'run'
        producer = self.producer(root)
        rom, save = self.repo / 'rom', self.repo / 'save'
        def verify(value):
            (root / 'producer.json').write_text(json.dumps(value))
            return bridge.verify_producer(root, self.descriptor, 'descriptor', self.repo, rom, save)
        self.assertEqual(verify(producer), producer)
        for field, value in [('schema_version', 1), ('descriptor_sha256', 'original'),
                             ('source_hashes', self.original['source_hashes']),
                             ('additional_source_hashes', {}), ('rom_sha256', 'wrong'),
                             ('fresh_target', False), ('target_dir', '/shared'),
                             ('build_command', ['cargo', 'build']),
                             ('build_env', {'CARGO_TARGET_DIR': '/shared'}),
                             ('source_repo', '/different-repo'), ('rom_path', '/different-rom'),
                             ('sram_path', '/different-save'), ('stdout_sha256', 'wrong'),
                             ('binary_sha256', 'wrong'), ('build_log_sha256', 'wrong'),
                             ('stderr_sha256', 'wrong'), ('invocation', ['verify']),
                             ('process', dict(pid=123, exit_code=1, started_ns=1, finished_ns=2)),
                             ('process', dict(pid=0, exit_code=0, started_ns=1, finished_ns=2)),
                             ('process', dict(pid=123, exit_code=0, started_ns=2, finished_ns=2)),
                             ('process', dict(pid=123, exit_code=0, started_ns=3, finished_ns=2))]:
            changed = deepcopy(producer); changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(changed)

    def test_audit_requires_two_process_identities(self):
        rom, save = self.repo / 'rom', self.repo / 'save'
        save.write_bytes(bytes(8192))
        descriptor = self.repo / 'descriptor.json'
        descriptor.write_text(json.dumps(self.descriptor))
        roots = [self.repo / name for name in ('a', 'b')]
        producers = [self.producer(root, check.sha(descriptor.read_bytes())) for root in roots]
        for index, (root, producer) in enumerate(zip(roots, producers)):
            producer['process']['run_id'] = str(index)
            (root / 'producer.json').write_text(json.dumps(producer))
        migration = check.load(check.HERE / 'migration.json')
        args = (rom, save, self.repo / 'old', self.repo / 'accepted-a', self.repo / 'accepted-b',
                *roots, self.repo / 'old-repo', self.repo / 'fixed-repo', self.repo, descriptor)
        with patch.object(check, 'audit', return_value=migration) as historical, \
                patch.object(bridge, 'verify_current', return_value=self.descriptor), \
                patch.object(bridge, 'compare_unchanged', return_value={'inventory': {}}) as compare:
            report = bridge.audit(*args)
            self.assertEqual(report['producers'], producers)
            self.assertEqual(historical.call_args.args[-2:], (args[7], args[8]))
            compare.assert_called_once_with(args[3], args[4], *roots, args[8])
            producers[1]['process']['run_id'] = producers[0]['process']['run_id']
            (roots[1] / 'producer.json').write_text(json.dumps(producers[1]))
            with self.assertRaisesRegex(ValueError, 'distinct process'):
                bridge.audit(*args)
            compare.assert_called_once()

    def test_current_capture_requires_explicit_descriptor_pair(self):
        import capture
        with self.assertRaisesRegex(ValueError, 'together'):
            capture.run(self.repo, self.repo / 'out', self.repo / 'rom', self.repo / 'save',
                        current_descriptor=self.repo / 'descriptor')
        with self.assertRaisesRegex(ValueError, 'together'):
            capture.run(self.repo, self.repo / 'out', self.repo / 'rom', self.repo / 'save',
                        fixed_source_repo=self.repo / 'fixed')
        self.assertFalse((self.repo / 'out').exists())

    def test_frozen_identity_substitution(self):
        with self.assertRaisesRegex(ValueError, 'frozen'):
            bridge.frozen_json(check.HERE / 'observer.json', bridge.MIGRATION_SHA)
        self.assertEqual(bridge.frozen_json(check.HERE / 'observer.json', bridge.OBSERVER_SHA)['epoch'], bridge.EPOCH)


class CaptureGates(unittest.TestCase):
    def setUp(self):
        fixture = test_check.Gates()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        self.roots = [fixture.base / name for name in ('accepted-a', 'accepted-b', 'current-a', 'current-b')]
        for root in self.roots:
            shutil.copytree(fixture.roots[0], root / 'capture')
        report = dict(fixed=check.inventory(self.roots[0] / 'capture'),
                      nonpixel_manifest_sha256=fixture.audit()['nonpixel_manifest_sha256'])
        p = patch.object(bridge, 'frozen_json', return_value=report)
        p.start(); self.addCleanup(p.stop)

    def compare(self):
        return bridge.compare_unchanged(*self.roots, check.REPO)

    def test_all_ten_files_and_full_manifest(self):
        self.assertEqual(len(self.compare()['inventory']), 10)
        for name in check.inventory(self.roots[0] / 'capture'):
            with self.subTest(name=name):
                for root in self.roots[2:]:
                    path = root / 'capture' / name
                    path.write_bytes(path.read_bytes() + b' ')
                with self.assertRaises(ValueError):
                    self.compare()
                for root in self.roots[2:]:
                    shutil.copy2(self.roots[0] / 'capture' / name, root / 'capture' / name)

    def test_capture_alias(self):
        shutil.rmtree(self.roots[2] / 'capture')
        (self.roots[2] / 'capture').symlink_to(self.roots[0] / 'capture', target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'distinct'):
            self.compare()


if __name__ == '__main__':
    unittest.main()
