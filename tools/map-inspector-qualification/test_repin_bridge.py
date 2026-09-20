#!/usr/bin/env python3
"""ROM-free controls for the explicit rustfmt repin bridge."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

import bridge
import check
import library_bridge as library
import repin_bridge as repin

MAIN = bridge.MAIN


class RepinDescriptorGates(unittest.TestCase):
    def setUp(self):
        self.repo = check.REPO
        self.descriptor = check.load(check.HERE / 'repin-producer.json')
        self.library = check.load(check.HERE / 'library-producer.json')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'descriptor.json'
        self.fixed = Path(self.temp.name) / 'fixed'
        fixed_main = self.fixed / MAIN
        fixed_main.parent.mkdir(parents=True)
        fixed_main.write_bytes(self.old_main())

    def old_main(self):
        """Undo both pinned deltas: the reorder, then the registration insert."""
        ordered = (self.repo / MAIN).read_bytes().replace(
            repin.FORMATTED_MODS, repin.PINNED_MODS, 1)
        registered = bridge.ANCHOR + bridge.REGISTRATIONS
        index = ordered.index(registered)
        return ordered[:index + len(bridge.ANCHOR)] + ordered[index + len(registered):]

    def materialize(self, name, main=None):
        """A complete bounded tree, optionally with substituted main.rs bytes.

        Complete rather than main-only so a disabled gate falls through to the
        next real check instead of a missing-file error.
        """
        root = Path(self.temp.name) / name
        for source in repin.current_sources(self.descriptor):
            target = root / source
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((self.repo / source).read_bytes())
        if main is not None:
            (root / MAIN).write_bytes(main)
        return root.resolve()

    def unformatted_repo(self):
        """The tree as the frozen library stage saw it: reorder reverted."""
        return self.materialize('unformatted', (self.repo / MAIN).read_bytes().replace(
            repin.FORMATTED_MODS, repin.PINNED_MODS, 1))

    def verify(self, descriptor=None):
        self.path.write_text(json.dumps(descriptor or self.descriptor))
        return repin.verify_repin_descriptor(self.repo, self.fixed, self.path)

    def test_settled_sources_and_exact_two_delta_proof(self):
        self.assertEqual(self.verify(), self.descriptor)
        self.assertEqual(set(self.descriptor['replaced_source_hashes']), set(repin.REPLACED_FILES))
        old, main = self.old_main(), (self.repo / MAIN).read_bytes()
        self.assertEqual(check.sha(old), bridge.OLD_MAIN_SHA)
        repin.verify_repin_main_delta(old, main)
        # The registration-only model no longer accepts the reformatted file,
        # and the reorder must be present exactly once.
        with self.assertRaises(ValueError):
            bridge.verify_main_delta(old, main)
        with self.assertRaisesRegex(ValueError, 'anchor'):
            repin.verify_repin_main_delta(old, main + repin.FORMATTED_MODS)
        with self.assertRaisesRegex(ValueError, 'anchor'):
            repin.verify_repin_main_delta(old, main.replace(
                repin.FORMATTED_MODS, repin.PINNED_MODS, 1))
        # Anchored on 'ambiguous module order', not 'authenticated old main':
        # the latter is also a substring of bridge.py's fallback message, so it
        # would be satisfied even with this gate disabled.
        with self.assertRaisesRegex(ValueError, 'ambiguous module order'):
            repin.verify_repin_main_delta(old + repin.PINNED_MODS, main)

    def test_identity_and_descriptor_shape_mutations_are_rejected(self):
        for field in repin.DESCRIPTOR_FIELDS:
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
            self.verify(self.library)

    def test_replacement_identity_and_delta_mutations_are_rejected(self):
        for identity in ('predecessor_sha256', 'current_sha256'):
            changed = deepcopy(self.descriptor)
            changed['replaced_source_hashes'][MAIN][identity] = 'substitution'
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                self.verify(changed)
            changed = deepcopy(self.descriptor)
            changed['replaced_source_hashes'][MAIN].pop(identity)
            with self.subTest(missing=identity), self.assertRaisesRegex(ValueError, 'fields'):
                self.verify(changed)
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes'][MAIN]['extra'] = 'reseal'
        with self.assertRaisesRegex(ValueError, 'fields'):
            self.verify(changed)
        # A replacement must authenticate a real delta, not reseal an identity.
        # Reaching this gate needs the tree reverted too, otherwise the disk-hash
        # comparison above fires first and the control proves nothing.
        unformatted = self.unformatted_repo()
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes'][MAIN]['current_sha256'] = \
            changed['replaced_source_hashes'][MAIN]['predecessor_sha256']
        self.path.write_text(json.dumps(changed))
        with self.assertRaisesRegex(ValueError, 'real delta'):
            repin.verify_repin_descriptor(unformatted, self.fixed, self.path)
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes'].pop(MAIN)
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify(changed)
        # An unchanged library source has no field in which to be resealed.
        changed = deepcopy(self.descriptor)
        unchanged = 'crates/oracle/src/lib.rs'
        changed['replaced_source_hashes'][unchanged] = {
            'predecessor_sha256': repin.library_sources()[unchanged],
            'current_sha256': check.sha((self.repo / unchanged).read_bytes()),
        }
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify(changed)

    def test_unchanged_sources_are_authenticated_against_disk(self):
        # Inheriting a source from the frozen stage is a claim about bytes on
        # disk, not a free pass for anything the descriptor does not mention.
        for name in ('crates/oracle/src/lib.rs', 'crates/map-inspector/src/room_preview.rs'):
            root = self.materialize(f'drift-{Path(name).stem}')
            (root / name).write_bytes((root / name).read_bytes() + b'\n// drift\n')
            self.path.write_text(json.dumps(self.descriptor))
            with self.subTest(source=name), self.assertRaisesRegex(
                    ValueError, 'unchanged predecessor source mismatch'):
                repin.verify_repin_descriptor(root, self.fixed, self.path)

    def test_frozen_library_stage_cannot_be_substituted(self):
        library_descriptor, library_report = repin.frozen_library()
        self.assertEqual(check.sha((check.HERE / 'library-producer.json').read_bytes()),
                         repin.LIBRARY_DESCRIPTOR_SHA)
        self.assertEqual(check.sha((check.HERE / 'library-producer-bridge.json').read_bytes()),
                         repin.LIBRARY_BRIDGE_SHA)
        self.assertEqual(library_descriptor['kind'], library.KIND)
        self.assertEqual(library_report['library_descriptor_sha256'],
                         repin.LIBRARY_DESCRIPTOR_SHA)
        # The repin's declared predecessor is that exact frozen main identity.
        self.assertEqual(self.descriptor['replaced_source_hashes'][MAIN]['predecessor_sha256'],
                         repin.library_sources()[MAIN])

    def producer(self, root):
        root.mkdir()
        rom, save = self.repo / 'rom', self.repo / 'save'
        value = {
            'schema_version': 4, 'kind': repin.KIND, 'epoch': repin.EPOCH,
            'policy': check.POLICY, 'commit': 'settled', 'descriptor_sha256': 'descriptor',
            'source_hashes': repin.current_sources(self.descriptor),
            'replaced_source_hashes': self.descriptor['replaced_source_hashes'],
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
        for name, field in (('map-inspector', 'binary_sha256'), ('build.log', 'build_log_sha256'),
                            ('stdout.txt', 'stdout_sha256'), ('stderr.txt', 'stderr_sha256')):
            (root / name).write_bytes(name.encode())
            value[field] = check.sha(name.encode())
        return value

    def test_repin_producer_provenance_substitutions_are_rejected(self):
        root = Path(self.temp.name) / 'run'
        producer = self.producer(root)

        def verify(value):
            (root / 'producer.json').write_text(json.dumps(value))
            return repin.verify_repin_producer(
                root, self.descriptor, 'descriptor', self.repo,
                self.repo / 'rom', self.repo / 'save')

        self.assertEqual(verify(producer), producer)
        for field, value in [
            ('schema_version', 3), ('kind', library.KIND),
            ('descriptor_sha256', repin.LIBRARY_DESCRIPTOR_SHA),
            ('source_hashes', repin.library_sources()),
            ('replaced_source_hashes', {}),
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
