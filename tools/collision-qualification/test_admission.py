#!/usr/bin/env python3
"""ROM-free controls: static table projection is not final movement admission."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_admission as admission
from test_derive import frames, layer


class AdmissionGates(unittest.TestCase):
    def setUp(self):
        self.table = [0] * 32
        self.table[3] = self.table[14] = 0x80
        self.rows = frames([('press', ['Right'], [(40, 32)] * 20)])
        for row in self.rows:
            row['actors'] = []
        self.layer = layer(4, 4, {(3, 1)})

    def fixture(self, root):
        run = root / 'run.jsonl'
        run.write_text('\n'.join(json.dumps(r) for r in self.rows))
        run.with_suffix('').mkdir(exist_ok=True)
        (run.with_suffix('') / 'layer-000f.json').write_text(json.dumps(self.layer))
        return run

    def test_five_bit_mask_and_dynamic_branch_before_shift(self):
        for attr in range(64):
            for index in (0, 0x100, 0x1ff):
                word = attr << 9 | index
                self.assertEqual(admission.probe_attribute(word), attr & 31)
                self.assertEqual(admission.probe_attribute(word | 0x8000), 3)
        # Distinguish the erroneous post-shift substitution of entry 6.
        self.table[6] = 0
        self.layer['cells'][6] = 0x8000
        report = admission.compare(self.table, self.layer, self.rows)
        self.assertEqual(report['stood_on_refused'], {'dyn': 20})

    def test_table_zero_admits_nonzero_refuses_and_reports_disagreements(self):
        report = admission.compare(self.table, self.layer, self.rows)
        self.assertEqual(report['disagreements'], 0)
        self.table[14] = 0
        report = admission.compare(self.table, self.layer, self.rows)
        self.assertEqual(report['stalled_against_admitted'], {(14,): 1})
        self.assertEqual(report['disagreements'], 1)
        self.table[0] = 1
        self.assertEqual(admission.compare(self.table, self.layer, self.rows)
                         ['stood_on_refused'], {0: 20})

    def test_invalid_tables_and_layers_fail_closed(self):
        for table in ([], [0] * 31, [0] * 33, [False] * 32, [256] * 32):
            with self.subTest(table=table), self.assertRaises(ValueError):
                admission.compare(table, self.layer, self.rows)
        for patch_layer in ({'map': 16}, {'map': None}, {'width': 0},
                            {'height': True}, {'cells': []}, {'cells': [0] * 16},
                            {'cells': [-1] * 16}, {'cells': [65536] * 16},
                            {'cells': [True] * 16}):
            with self.subTest(layer=patch_layer), self.assertRaises(ValueError):
                admission.compare(self.table, {**self.layer, **patch_layer}, self.rows)

    def test_empty_samples_and_out_of_bounds_are_not_coverage(self):
        with self.assertRaises(ValueError):
            admission.compare(self.table, self.layer, [])
        for position in ([0, 0], [72, 32]):
            rows = [{**r, 'position': position} for r in self.rows]
            with self.subTest(position=position), self.assertRaises(ValueError):
                admission.compare(self.table, self.layer, rows)
        # Occupancy is inside; the contacted edge is outside.
        rows = [{**r, 'position': [56, 32]} for r in self.rows]
        with self.assertRaises(ValueError):
            admission.compare(self.table, self.layer, rows)

    def test_run_requires_every_map_layer_and_at_least_one_contact(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            run = self.fixture(root)
            self.assertEqual(admission.check_run(self.table, run)[0]['map'], 15)
            extra = {**self.rows[-1], 'frame': 20, 'map': 16}
            with run.open('a') as f:
                f.write('\n' + json.dumps(extra))
            with self.assertRaises(FileNotFoundError):
                admission.check_run(self.table, run)
            run.write_text(json.dumps(self.rows[0]))
            with self.assertRaisesRegex(ValueError, 'contact'):
                admission.check_run(self.table, run)

    def test_strict_evidence_requires_map_control_actors_and_frame(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            for field in ('map', 'control', 'actors', 'frame'):
                run = self.fixture(root)
                rows = [{k: v for k, v in r.items() if k != field} for r in self.rows]
                run.write_text('\n'.join(json.dumps(r) for r in rows))
                with self.subTest(field=field), self.assertRaises(ValueError):
                    admission.check_run(self.table, run)

    def test_rom_identity_and_copier_header_normalization(self):
        rom = bytearray(0x10000)
        rom[admission.TABLE:admission.TABLE + 32] = bytes(self.table)
        digest = hashlib.sha256(rom).hexdigest()
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'rom.sfc'
            with patch.object(admission, 'ROM_SHA', digest):
                for header in (b'', bytes(512)):
                    path.write_bytes(header + rom)
                    self.assertEqual(admission.read_rom(path), bytes(rom))
                    self.assertEqual(admission.read_table(path), self.table)
                for data in (b'', b'short', rom[:-1], rom + b'x'):
                    path.write_bytes(data)
                    with self.assertRaises(ValueError):
                        admission.read_table(path)
                rom[admission.TABLE] ^= 1
                path.write_bytes(rom)
                with self.assertRaisesRegex(ValueError, 'ROM'):
                    admission.read_table(path)

    def test_cli_status_and_honest_scope(self):
        with tempfile.TemporaryDirectory() as d:
            run = self.fixture(Path(d))
            with patch.object(admission, 'read_table', return_value=self.table):
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    self.assertEqual(admission.main(['rom', str(run)]), 0)
                self.assertIn('right-edge table-precheck', out.getvalue())
                self.assertIn('final movement NOT qualified', out.getvalue())
                self.table[14] = 0
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(admission.main(['rom', str(run)]), 1)
                run.unlink()
                with contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(admission.main(['rom', str(run)]), 2)

    def test_cli_rejects_wrong_rom_under_normal_and_optimized_python(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'wrong.sfc'
            path.write_bytes(bytes(0x10000))
            for flags in ([], ['-O']):
                result = subprocess.run([sys.executable, *flags, admission.__file__,
                                         str(path), str(Path(d) / 'missing.jsonl')],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('ROM', result.stderr)


if __name__ == '__main__':
    unittest.main()
