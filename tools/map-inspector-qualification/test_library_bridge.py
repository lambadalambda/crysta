#!/usr/bin/env python3
"""ROM-free controls for the explicit library producer bridge."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import bridge
import check
import library_bridge as library
import repin_bridge as repin


class LibraryDescriptorGates(unittest.TestCase):
    def library_repo(self):
        """Materialize the tree as the frozen library producer saw it.

        The working tree has since moved on by exactly one rustfmt reorder (see
        repin-producer.json), so these ROM-free controls reconstruct the library
        stage instead of reading whatever HEAD happens to be.
        """
        root = Path(self.temp.name) / 'library-repo'
        names = set(library.current_sources(self.descriptor, self.predecessor))
        names.update(*library.GROUPS.values())
        names.add(bridge.MAIN)
        for name in names:
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((check.REPO / name).read_bytes())
        main = (check.REPO / bridge.MAIN).read_bytes()
        (root / bridge.MAIN).write_bytes(
            main.replace(repin.FORMATTED_MODS, repin.PINNED_MODS, 1))
        # Resolved: the envelope gate compares recorded paths against repo.resolve().
        return root.resolve()

    def setUp(self):
        self.descriptor = check.load(check.HERE / 'library-producer.json')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'descriptor.json'
        self.predecessor = check.load(check.HERE / 'current-producer.json')
        self.repo = self.library_repo()
        self.fixed = Path(self.temp.name) / 'fixed'
        main = (self.repo / bridge.MAIN).read_bytes()
        registered = bridge.ANCHOR + bridge.REGISTRATIONS
        index = main.index(registered)
        old = main[:index + len(bridge.ANCHOR)] + main[index + len(registered):]
        fixed_main = self.fixed / bridge.MAIN
        fixed_main.parent.mkdir(parents=True)
        fixed_main.write_bytes(old)
        verify = patch.object(bridge, 'verify_current', return_value=self.predecessor)
        verify.start(); self.addCleanup(verify.stop)

    def verify(self, descriptor=None):
        self.path.write_text(json.dumps(descriptor or self.descriptor))
        return library.verify_library_descriptor(self.repo, self.repo, self.fixed, self.path)

    def test_settled_sources_and_exact_main_proof(self):
        self.assertEqual(self.verify(), self.descriptor)
        self.assertEqual(set(self.descriptor['replaced_source_hashes']), set(library.REPLACED_FILES))
        self.assertEqual(sum(map(len, library.GROUPS.values())), 18)
        main = (self.repo / bridge.MAIN).read_bytes()
        registered = bridge.ANCHOR + bridge.REGISTRATIONS
        index = main.index(registered)
        old = main[:index + len(bridge.ANCHOR)] + main[index + len(registered):]
        bridge.verify_main_delta(old, main)

    def test_identity_and_descriptor_shape_mutations_are_rejected(self):
        for field in library.DESCRIPTOR_FIELDS:
            changed = deepcopy(self.descriptor)
            changed.pop(field)
            with self.subTest(missing=field), self.assertRaises(ValueError):
                self.verify(changed)
        changed = deepcopy(self.descriptor); changed['extra'] = 'fallback'
        with self.assertRaisesRegex(ValueError, 'fields'):
            self.verify(changed)
        for field in ('kind', 'epoch', 'policy', 'original_descriptor_sha256',
                      'migration_sha256', 'predecessor_descriptor_sha256',
                      'predecessor_bridge_sha256'):
            changed = deepcopy(self.descriptor); changed[field] = 'substitution'
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify(changed)
        with self.assertRaises(ValueError):
            self.verify(self.predecessor)

    def test_replacement_identity_and_delta_mutations_are_rejected(self):
        for name in library.REPLACED_FILES:
            changed = deepcopy(self.descriptor); changed['replaced_source_hashes'].pop(name)
            with self.subTest(missing=name), self.assertRaisesRegex(ValueError, 'inventory'):
                self.verify(changed)
            for identity in ('predecessor_sha256', 'current_sha256'):
                changed = deepcopy(self.descriptor)
                changed['replaced_source_hashes'][name][identity] = 'substitution'
                with self.subTest(name=name, identity=identity), self.assertRaises(ValueError):
                    self.verify(changed)
            changed = deepcopy(self.descriptor)
            changed['replaced_source_hashes'][name]['extra'] = 'reseal'
            with self.assertRaisesRegex(ValueError, 'fields'):
                self.verify(changed)
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes']['crates/map-inspector/src/main.rs'] = {
            'predecessor_sha256': self.predecessor['source_hashes'][bridge.MAIN],
            'current_sha256': self.predecessor['source_hashes'][bridge.MAIN],
        }
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify(changed)

    def test_explicit_group_inventory_and_source_mutations_are_rejected(self):
        for field, files in library.GROUPS.items():
            for name in files:
                changed = deepcopy(self.descriptor); changed[field].pop(name)
                with self.subTest(field=field, missing=name), self.assertRaisesRegex(ValueError, 'inventory'):
                    self.verify(changed)
                changed = deepcopy(self.descriptor); changed[field][name] = 'substitution'
                with self.subTest(field=field, source=name), self.assertRaisesRegex(ValueError, 'source'):
                    self.verify(changed)
            changed = deepcopy(self.descriptor); changed[field]['unexpected/file'] = 'fallback'
            with self.assertRaisesRegex(ValueError, 'inventory'):
                self.verify(changed)

    def test_unchanged_predecessor_source_cannot_be_resealed(self):
        changed = deepcopy(self.descriptor)
        # Unchanged predecessor sources are derived from the frozen predecessor;
        # there is deliberately no field in which to reseal one.
        changed['replaced_source_hashes'][bridge.MAIN] = {
            'predecessor_sha256': self.predecessor['source_hashes'][bridge.MAIN],
            'current_sha256': check.sha((self.repo / bridge.MAIN).read_bytes()),
        }
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify(changed)

    def producer(self, root):
        root.mkdir()
        rom, save = self.repo / 'rom', self.repo / 'save'
        value = {
            'schema_version': 3, 'kind': library.KIND, 'epoch': library.EPOCH,
            'policy': check.POLICY, 'commit': 'settled', 'descriptor_sha256': 'descriptor',
            'source_hashes': library.current_sources(self.descriptor, self.predecessor),
            'rom_sha256': check.ROM, 'sram_sha256': check.SRAM,
            'build_command': ['cargo', 'build', '--locked', '-p', 'map-inspector'],
            'fresh_target': True, 'target_dir': str(root.resolve() / 'target'),
            'build_env': {'CARGO_TARGET_DIR': str(root.resolve() / 'target')},
            'rustc': 'rustc', 'cargo': 'cargo', 'cxx': 'cxx',
            'source_repo': str(self.repo), 'rom_path': str(rom), 'sram_path': str(save),
            'invocation': [str(root.resolve() / 'map-inspector'), 'capture', str(rom), str(save)],
            'process': {'pid': 123, 'exit_code': 0, 'started_ns': 1,
                        'finished_ns': 2, 'run_id': 'one'},
        }
        for field in ('replaced_source_hashes', *library.GROUPS):
            value[field] = self.descriptor[field]
        for name, field in (('map-inspector', 'binary_sha256'), ('build.log', 'build_log_sha256'),
                            ('stdout.txt', 'stdout_sha256'), ('stderr.txt', 'stderr_sha256')):
            (root / name).write_bytes(name.encode())
            value[field] = check.sha(name.encode())
        return value

    def test_library_producer_provenance_substitutions_are_rejected(self):
        root = Path(self.temp.name) / 'run'
        producer = self.producer(root)

        def verify(value):
            (root / 'producer.json').write_text(json.dumps(value))
            with patch.object(library, 'frozen_predecessor', return_value=(None, self.predecessor, None)):
                return library.verify_library_producer(
                    root, self.descriptor, 'descriptor', self.repo,
                    self.repo / 'rom', self.repo / 'save')

        self.assertEqual(verify(producer), producer)
        for field, value in [
            ('schema_version', 2), ('kind', 'preview-producer'),
            ('descriptor_sha256', library.PREDECESSOR_DESCRIPTOR_SHA),
            ('source_hashes', self.predecessor['source_hashes']),
            ('replaced_source_hashes', {}), ('library_source_hashes', {}),
            ('qualification_source_hashes', {}), ('adapter_source_hashes', {}),
            ('rom_sha256', 'wrong'), ('sram_sha256', 'wrong'),
            ('build_command', ['cargo', 'build']), ('fresh_target', False),
            ('target_dir', '/shared'), ('build_env', {'CARGO_TARGET_DIR': '/shared'}),
            ('source_repo', '/different'), ('rom_path', '/different'),
            ('sram_path', '/different'), ('invocation', ['verify']),
            ('binary_sha256', 'wrong'), ('build_log_sha256', 'wrong'),
            ('stdout_sha256', 'wrong'), ('stderr_sha256', 'wrong'), ('fallback', True),
            ('process', {'pid': 0, 'exit_code': 0, 'started_ns': 1,
                         'finished_ns': 2, 'run_id': 'one'}),
            ('process', {'pid': 123, 'exit_code': 0, 'started_ns': 1,
                         'finished_ns': 2, 'run_id': ''}),
        ]:
            changed = deepcopy(producer); changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify(changed)


if __name__ == '__main__':
    unittest.main()
