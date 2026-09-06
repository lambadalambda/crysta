"""The diagnostic comparator must retain pixel and non-pixel mismatches alike."""
from pathlib import Path
import tempfile
import unittest
from compare import compare


class ComparisonTests(unittest.TestCase):
    def test_exact_bytes_names_and_logs(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = (Path(tmp) / name for name in ('a', 'b'))
            a.mkdir()
            b.mkdir()
            for root in (a, b):
                (root / 'frame.pixels').write_bytes(b'\x01\x02\x03\x00')
                (root / 'frame.wram').write_bytes(b'ram')
                root.with_suffix('.jsonl').write_bytes(b'log')
            self.assertTrue(compare(a, b)['byte_equal'])
            for name in ('frame.pixels', 'frame.wram'):
                (b / name).write_bytes(b'changed')
                result = compare(a, b)
                self.assertFalse(result['byte_equal'])
                self.assertIn(name, result['changed'])
                (b / name).write_bytes((a / name).read_bytes())
            (b / 'extra.obj').write_bytes(b'')
            (b / 'frame.wram').unlink()
            b.with_suffix('.jsonl').write_bytes(b'different log')
            result = compare(a, b)
            self.assertFalse(result['byte_equal'])
            self.assertEqual(result['only_left'], ['frame.wram'])
            self.assertEqual(result['only_right'], ['extra.obj'])
            self.assertFalse(result['frame_log']['byte_equal'])


if __name__ == '__main__':
    unittest.main()
