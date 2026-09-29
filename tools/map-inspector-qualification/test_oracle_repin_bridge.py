#!/usr/bin/env python3
"""ROM-free controls for the explicit oracle audio-capture repin bridge."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

import bridge
import check
import oracle_repin_bridge as oracle
import repin_bridge as repin

MAIN = bridge.MAIN
LIB = 'crates/oracle/src/lib.rs'


class OracleRepinDescriptorGates(unittest.TestCase):
    def setUp(self):
        self.repo = check.REPO
        self.descriptor = check.load(check.HERE / 'oracle-repin-producer.json')
        self.repin = check.load(check.HERE / 'repin-producer.json')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'descriptor.json'
        self.fixed = self.fixed_repo('fixed', self.old_main())

    def old_main(self):
        """Undo both pinned repin deltas: the reorder, then the registration insert."""
        ordered = (self.repo / MAIN).read_bytes().replace(
            repin.FORMATTED_MODS, repin.PINNED_MODS, 1)
        registered = bridge.ANCHOR + bridge.REGISTRATIONS
        index = ordered.index(registered)
        return ordered[:index + len(bridge.ANCHOR)] + ordered[index + len(registered):]

    def fixed_repo(self, name, main):
        fixed_main = Path(self.temp.name) / name / MAIN
        fixed_main.parent.mkdir(parents=True)
        fixed_main.write_bytes(main)
        return fixed_main.parents[3]

    def materialize(self, name, **replacements):
        """A complete bounded tree, optionally with substituted source bytes.

        Complete rather than partial so a disabled gate falls through to the
        next real check instead of a missing-file error.
        """
        root = Path(self.temp.name) / name
        for source in oracle.current_sources(self.descriptor):
            target = root / source
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(replacements.get(source, (self.repo / source).read_bytes()))
        return root.resolve()

    def verify(self, descriptor=None, repo=None, fixed=None):
        self.path.write_text(json.dumps(descriptor or self.descriptor))
        return oracle.verify_oracle_repin_descriptor(
            repo or self.repo, fixed or self.fixed, self.path)

    def test_settled_sources_and_authenticated_pre_state(self):
        self.assertEqual(self.verify(), self.descriptor)
        replaced = self.descriptor['replaced_source_hashes']
        self.assertEqual(set(replaced), set(oracle.REPLACED_FILES))
        for name, identities in replaced.items():
            # The declared pre-state is the frozen repin stage's identity, and
            # those exact bytes exist in history: a real, inspectable delta.
            self.assertEqual(identities['predecessor_sha256'], oracle.repin_sources()[name])
            self.assertEqual(check.sha(oracle.historical_bytes(name, identities['predecessor_sha256'])),
                             identities['predecessor_sha256'])
            self.assertEqual(identities['current_sha256'],
                             check.sha((self.repo / name).read_bytes()))
        # main.rs is inherited, not replaced: the repin stage's identity holds.
        self.assertEqual(oracle.current_sources(self.descriptor)[MAIN],
                         self.repin['replaced_source_hashes'][MAIN]['current_sha256'])

    def test_identity_and_descriptor_shape_mutations_are_rejected(self):
        for field in oracle.DESCRIPTOR_FIELDS:
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
        # The frozen repin descriptor cannot stand in for this stage.
        with self.assertRaises(ValueError):
            self.verify(self.repin)

    def test_replacement_identity_and_delta_mutations_are_rejected(self):
        for name in oracle.REPLACED_FILES:
            for identity in ('predecessor_sha256', 'current_sha256'):
                changed = deepcopy(self.descriptor)
                changed['replaced_source_hashes'][name][identity] = 'substitution'
                with self.subTest(name=name, identity=identity), self.assertRaises(ValueError):
                    self.verify(changed)
                changed = deepcopy(self.descriptor)
                changed['replaced_source_hashes'][name].pop(identity)
                with self.subTest(name=name, missing=identity), \
                        self.assertRaisesRegex(ValueError, 'fields'):
                    self.verify(changed)
            changed = deepcopy(self.descriptor)
            changed['replaced_source_hashes'][name]['extra'] = 'reseal'
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'fields'):
                self.verify(changed)
            # A replacement must authenticate a real delta, not reseal an
            # identity. Checked on the tree the resealed descriptor describes,
            # so only the real-delta gate stands in the way.
            changed = deepcopy(self.descriptor)
            identities = changed['replaced_source_hashes'][name]
            identities['current_sha256'] = identities['predecessor_sha256']
            reverted = self.materialize(f'reverted-{Path(name).stem}', **{
                name: oracle.historical_bytes(name, identities['predecessor_sha256'])})
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'real delta'):
                self.verify(changed, reverted)
            changed = deepcopy(self.descriptor)
            changed['replaced_source_hashes'].pop(name)
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'inventory'):
                self.verify(changed)
        # An inherited source has no field in which to be resealed.
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes'][MAIN] = deepcopy(
            self.repin['replaced_source_hashes'][MAIN])
        with self.assertRaisesRegex(ValueError, 'inventory'):
            self.verify(changed)

    def test_predecessor_must_be_the_frozen_repin_identity(self):
        # A fabricated pre-state that still differs from current reaches no
        # other gate: only the predecessor comparison rejects it.
        changed = deepcopy(self.descriptor)
        changed['replaced_source_hashes'][LIB]['predecessor_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'replacement predecessor mismatch'):
            self.verify(changed)

    def test_sources_are_authenticated_against_disk(self):
        # Both the replaced and the inherited identities are claims about bytes
        # on disk, including a tree still at the frozen repin stage.
        predecessor = oracle.historical_bytes(
            LIB, self.descriptor['replaced_source_hashes'][LIB]['predecessor_sha256'])
        cases = {name: (self.repo / name).read_bytes() + b'\n// drift\n' for name in (
            LIB, 'vendor/ares/shims.cpp', 'vendor/ares/ares-unity.cpp',
            'crates/map-inspector/src/room_preview.rs')}
        cases['stale-oracle'] = predecessor
        for label, data in cases.items():
            name = LIB if label == 'stale-oracle' else label
            root = self.materialize(f'drift-{Path(label).stem}', **{name: data})
            with self.subTest(source=label), self.assertRaisesRegex(
                    ValueError, 'unchanged predecessor source mismatch'):
                self.verify(repo=root)

    def test_inherited_main_proof_needs_the_authenticated_old_main(self):
        tampered = self.fixed_repo('tampered', self.old_main() + b'\n')
        with self.assertRaisesRegex(ValueError, 'unauthenticated old main blob'):
            self.verify(fixed=tampered)

    def test_frozen_repin_stage_cannot_be_substituted(self):
        descriptor, report = oracle.frozen_repin()
        self.assertEqual(check.sha((check.HERE / 'repin-producer.json').read_bytes()),
                         oracle.REPIN_DESCRIPTOR_SHA)
        self.assertEqual(check.sha((check.HERE / 'repin-producer-bridge.json').read_bytes()),
                         oracle.REPIN_BRIDGE_SHA)
        self.assertEqual(descriptor['kind'], repin.KIND)
        self.assertEqual(report['repin_descriptor_sha256'], oracle.REPIN_DESCRIPTOR_SHA)

    def producer(self, root):
        root.mkdir()
        rom, save = self.repo / 'rom', self.repo / 'save'
        value = {
            'schema_version': 5, 'kind': oracle.KIND, 'epoch': oracle.EPOCH,
            'policy': check.POLICY, 'commit': 'settled', 'descriptor_sha256': 'descriptor',
            'source_hashes': oracle.current_sources(self.descriptor),
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

    def test_oracle_repin_producer_provenance_substitutions_are_rejected(self):
        root = Path(self.temp.name) / 'run'
        producer = self.producer(root)

        def verify(value):
            (root / 'producer.json').write_text(json.dumps(value))
            return oracle.verify_oracle_repin_producer(
                root, self.descriptor, 'descriptor', self.repo,
                self.repo / 'rom', self.repo / 'save')

        self.assertEqual(verify(producer), producer)
        for field, value in [
            ('schema_version', 4), ('kind', repin.KIND),
            ('descriptor_sha256', oracle.REPIN_DESCRIPTOR_SHA),
            ('source_hashes', oracle.repin_sources()),
            ('replaced_source_hashes', self.repin['replaced_source_hashes']),
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
