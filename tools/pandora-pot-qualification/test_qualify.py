"""Tests for the read-only pot evidence projection (no native restores)."""
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from qualify import contiguous, project, require
from source_contract import project as source_project


class EvidenceTests(unittest.TestCase):
    def test_contiguous_includes_each_frame(self):
        rows = [dict(kind='frame', frame=n, label='carry') for n in range(10, 14)]
        self.assertEqual(contiguous(rows, 10, 13), rows)
        for bad in (rows[1:], rows[:-1], rows[:2] + rows[3:], rows[::-1], rows + rows[-1:]):
            with self.assertRaises(ValueError):
                contiguous(bad, 10, 13)

    def test_checkpoint_is_not_an_extra_frame(self):
        rows = [dict(kind='frame', frame=10), dict(kind='checkpoint', frame=10)]
        with self.assertRaises(ValueError):
            contiguous(rows, 10, 10)

    def test_restore_or_unknown_observer_is_not_accepted(self):
        for kind in ('restore', 'load_state', 'teleport', 'unknown'):
            with self.assertRaises(ValueError):
                contiguous([dict(kind=kind, frame=10)], 10, 10)

    def test_wrong_rom_is_not_source_evidence(self):
        with self.assertRaisesRegex(ValueError, 'ROM authentication'):
            source_project(b'not the owned ROM')

    def test_modified_or_restore_log_cannot_create_fixtures(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            log = root / 'restore.jsonl'
            log.write_text('{"kind":"restore","frame":18833}\n')
            with self.assertRaisesRegex(ValueError, 'not exact retained original'):
                project(log, root / 'captures', root / 'output')
            self.assertFalse((root / 'output').exists())

    def test_checks_survive_optimized_python(self):
        with self.assertRaises(ValueError):
            require(False, 'deliberate failure')


if __name__ == '__main__':
    unittest.main()
