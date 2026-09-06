"""ROM-free observer epoch migration controls; no emulator or state restoration."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

from epoch import EPOCH, POLICY, OBSERVER_FILES, audit_captures, migrate_pixels, observer_sources
from source import ROOT, sha


class EpochTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.roots = [self.base / name / 'journey' for name in ('old', 'a', 'b', 'sibling')]
        self.route = b'{"label":"wait","buttons":[],"frames":1}\n{"finish":true}\n'
        rows = [dict(kind='checkpoint', label='boot', frame=6800),
                dict(kind='frame', label='wait', frame=6801),
                dict(kind='checkpoint', label='wait', frame=6801)]
        self.log = ''.join(json.dumps(row) + '\n' for row in rows)
        for root in self.roots:
            root.mkdir(parents=True)
            (root / 'route.jsonl').write_bytes(self.route)
            root.with_suffix('.jsonl').write_text(self.log)
            for label in ('boot', 'wait'):
                for ext in ('state', 'wram', 'vram', 'cgram', 'pixels', 'oam', 'obj'):
                    (root / f'{label}.{ext}').write_bytes(b'old!' if root == self.roots[0] else b'new!' if ext == 'pixels' else b'old!')

    def audit(self):
        return audit_captures(*self.roots, self.route)

    def test_accepts_only_reproduced_pixel_epoch(self):
        report = self.audit()
        self.assertEqual(set(report['old_to_fixed']['changed']), {'boot.pixels', 'wait.pixels'})
        self.assertTrue(report['twins']['byte_equal'])
        self.assertTrue(report['sibling']['byte_equal'])

    def test_rejects_each_nonpixel_or_route_mutation(self):
        for ext in ('state', 'wram', 'vram', 'cgram', 'oam', 'obj', 'jsonl'):
            path = self.roots[1] / ('route.jsonl' if ext == 'jsonl' else f'wait.{ext}')
            original = path.read_bytes()
            with self.subTest(ext=ext), self.assertRaises(ValueError):
                path.write_bytes(b'bad!')
                self.audit()
            path.write_bytes(original)

    def test_rejects_even_matching_new_nonpixel_changes(self):
        for root in self.roots[1:]:
            (root / 'wait.wram').write_bytes(b'bad!')
        with self.assertRaises(ValueError):
            self.audit()

    def test_rejects_each_unreproduced_pixel_root(self):
        for root in self.roots[1:]:
            path = root / 'wait.pixels'
            with self.subTest(root=root.name), self.assertRaises(ValueError):
                path.write_bytes(b'bad!')
                self.audit()
            path.write_bytes(b'new!')

    def test_rejects_missing_extra_resized_and_log_changes(self):
        for action in ('missing', 'extra', 'resized', 'log'):
            with self.subTest(action=action), self.assertRaises(ValueError):
                root = self.roots[1]
                if action == 'missing': (root / 'wait.state').unlink()
                if action == 'extra': (root / 'extra.pixels').write_bytes(b'new!')
                if action == 'resized': (root / 'wait.pixels').write_bytes(b'longer')
                if action == 'log': root.with_suffix('.jsonl').write_text(self.log + '\n')
                self.audit()
            (root / 'wait.state').write_bytes(b'old!')
            (root / 'extra.pixels').unlink(missing_ok=True)
            (root / 'wait.pixels').write_bytes(b'new!')
            root.with_suffix('.jsonl').write_text(self.log)

    def test_rejects_empty_or_unqualified_schedule(self):
        for root in self.roots:
            root.with_suffix('.jsonl').write_text(self.log.replace('6801', '6802'))
        with self.assertRaises(ValueError):
            self.audit()
        with self.assertRaises(ValueError):
            audit_captures(*(self.base / 'absent' for _ in range(4)), self.route)

    def test_rejects_aliased_replica_roots(self):
        with self.assertRaisesRegex(ValueError, 'distinct capture roots'):
            audit_captures(self.roots[0], self.roots[1], self.roots[1], self.roots[3], self.route)

    def test_identical_fixed_changes_reach_cross_epoch_extent_and_log_guards(self):
        for mutation, message in [('extent', 'pixel extent'), ('log', 'full frame log')]:
            for root in self.roots[1:]:
                if mutation == 'extent':
                    (root / 'wait.pixels').write_bytes(b'longer')
                else:
                    # Extra metadata leaves the command/checkpoint timeline valid.
                    rows = [dict(json.loads(line), map=12) for line in self.log.splitlines()]
                    root.with_suffix('.jsonl').write_text(''.join(json.dumps(row) + '\n' for row in rows))
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, message):
                self.audit()
            for root in self.roots[1:]:
                (root / 'wait.pixels').write_bytes(b'new!')
                root.with_suffix('.jsonl').write_text(self.log)

    def test_pixel_projection_preserves_all_other_fields_and_old_object(self):
        old = {'points': {'boot': {'position': [1, 2], 'hashes': {'pixels': 'old', 'wram': 'fixed'}}},
               'observation_policy': 'old-policy', 'provenance': {'build': 'old'}}
        before = deepcopy(old)
        new = migrate_pixels(old, self.roots[1], 'points')
        self.assertEqual(old, before)
        self.assertNotEqual(new['points']['boot']['hashes']['pixels'], 'old')
        new['points']['boot']['hashes']['pixels'] = 'old'
        self.assertEqual(new, old)

    def test_observer_provenance_covers_required_build_and_video_sources(self):
        required = {'crates/oracle/build.rs', 'vendor/ares/ares-unity.cpp',
                    'vendor/ares/ares/ares/ares.hpp', 'vendor/ares/shims.cpp',
                    'vendor/ares/ares/sfc/system/serialization.cpp'}
        self.assertTrue(required <= set(OBSERVER_FILES))
        for name in OBSERVER_FILES:
            path = self.base / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'source')
        before = observer_sources(self.base)
        for name in OBSERVER_FILES:
            path = self.base / name
            path.write_bytes(b'changed')
            self.assertNotEqual(observer_sources(self.base)[name], before[name])
            path.write_bytes(b'source')
        self.assertEqual(EPOCH, 'headless-sync-video-v1')


class RetainedEpochTests(unittest.TestCase):
    def test_migrated_references_preserve_semantics_and_archive_identity(self):
        ledger = json.loads((ROOT / 'migration.json').read_text())
        self.assertEqual(ledger['to_epoch'], EPOCH)
        self.assertEqual(ledger['recipe_sha256'], sha((ROOT / 'route.jsonl').read_bytes()))
        self.assertEqual(ledger['source_contract_sha256'], sha((ROOT / 'source.json').read_bytes()))
        for name, digest in ledger['legacy_reference_sha256'].items():
            self.assertEqual(sha((ROOT / 'epochs/threaded-video-v0' / name).read_bytes()), digest)
        for name, key, hash_key in [('reference.json', 'points', 'new_reference_sha256'),
                                     ('prefix-reference.json', 'checkpoints', 'new_prefix_sha256')]:
            with self.subTest(reference=name):
                current = json.loads((ROOT / name).read_text())
                old = json.loads((ROOT / 'epochs/threaded-video-v0' / name).read_text())
                self.assertEqual(sha((ROOT / name).read_bytes()), ledger[hash_key])
                self.assertEqual(current.pop('observer_epoch'), EPOCH)
                self.assertEqual(current.pop('observer_source_hashes'), observer_sources())
                if key == 'points':
                    self.assertEqual(current['observation_policy'], POLICY)
                    current['observation_policy'] = old['observation_policy']
                    current['observer_source_hashes'] = old['observer_source_hashes']
                for label in current[key]:
                    current[key][label]['hashes']['pixels'] = old[key][label]['hashes']['pixels']
                self.assertEqual(current, old)


if __name__ == '__main__':
    unittest.main()
