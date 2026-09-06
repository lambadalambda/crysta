#!/usr/bin/env python3
"""ROM-free negative controls for the exhaustive renewal gate."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import check


class Gates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.roots = [self.base / name for name in ('old', 'a', 'b')]
        for root in self.roots:
            root.mkdir()
            for label in check.LABELS:
                for surface, size in check.SIZES.items():
                    (root / f'{surface}-{label}.bin').write_bytes(bytes(size))
                (root / f'reference-{label}.bmp').write_bytes(check.bmp_header() + bytes(256 * 240 * 3))
            cp = [dict(label=l, frame=l + 1, rgb_sha256=check.sha(bytes(256 * 240 * 3)),
                       **{f'{s}_sha256': check.sha(bytes(n)) for s, n in check.SIZES.items()})
                  for l in check.LABELS]
            self.manifest(root, dict(schema_version=1, scenario=check.SCENARIO,
                                    rom_sha256=check.ROM, sram_sha256=check.SRAM, checkpoints=cp))

    def manifest(self, root, value):
        (root / 'capture.json').write_text(json.dumps(value))
        (root / 'index.html').write_text(check.html(value))

    def mutate_manifest(self, root, mutate):
        value = json.loads((root / 'capture.json').read_text())
        mutate(value)
        self.manifest(root, value)

    def audit(self):
        return check.compare_captures(*self.roots)

    def test_equal(self):
        self.assertEqual(self.audit()['changed'], {})

    def test_missing_extra(self):
        for name in ('wram-1600.bin', 'unexpected'):
            with self.subTest(name=name):
                path = self.roots[1] / name
                if path.exists():
                    data = path.read_bytes(); path.unlink()
                    with self.assertRaisesRegex(ValueError, 'inventory'): self.audit()
                    path.write_bytes(data)
                else:
                    path.write_bytes(b'')
                    with self.assertRaisesRegex(ValueError, 'inventory'): self.audit()
                    path.unlink()

    def test_alias(self):
        with self.assertRaisesRegex(ValueError, 'distinct'):
            check.compare_captures(self.roots[0], self.roots[1], self.roots[1])

    def test_nonpixel_even_with_updated_manifest(self):
        for surface, size in check.SIZES.items():
            with self.subTest(surface=surface):
                data = b'\x01' + bytes(size - 1)
                for root in self.roots[1:]:
                    (root / f'{surface}-1600.bin').write_bytes(data)
                    self.mutate_manifest(root, lambda m: m['checkpoints'][0].update({f'{surface}_sha256': check.sha(data)}))
                with self.assertRaisesRegex(ValueError, 'non-pixel'): self.audit()
                for root in self.roots[1:]:
                    (root / f'{surface}-1600.bin').write_bytes(bytes(size))
                    self.mutate_manifest(root, lambda m: m['checkpoints'][0].update({f'{surface}_sha256': check.sha(bytes(size))}))

    def test_full_manifest(self):
        for root in self.roots[1:]:
            self.mutate_manifest(root, lambda m: m.update(unselected_field=1))
        with self.assertRaisesRegex(ValueError, 'non-pixel'): self.audit()

    def test_fixed_pixel_mutation(self):
        root = self.roots[2]
        path = root / 'reference-1840.bmp'
        data = bytearray(path.read_bytes()); data[-1] = 42; path.write_bytes(data)
        rgb = check.rgb(path.read_bytes())
        self.mutate_manifest(root, lambda m: m['checkpoints'][1].update(rgb_sha256=check.sha(rgb)))
        with self.assertRaisesRegex(ValueError, 'fixed'): self.audit()

    def test_old_pixel_change_only(self):
        root = self.roots[0]
        path = root / 'reference-1840.bmp'
        data = bytearray(path.read_bytes()); data[-1] = 42; path.write_bytes(data)
        self.mutate_manifest(root, lambda m: m['checkpoints'][1].update(rgb_sha256=check.sha(check.rgb(data))))
        self.assertEqual(set(self.audit()['changed']), {'reference-1840.bmp'})

    def test_recipe_and_input(self):
        for field, value in [('sram_sha256', check.sha(b'')), ('rom_sha256', 'wrong'), ('scenario', 'Pandora')]:
            with self.subTest(field=field):
                original = json.loads((self.roots[0] / 'capture.json').read_text())
                self.mutate_manifest(self.roots[0], lambda m: m.update({field: value}))
                with self.assertRaises(ValueError): self.audit()
                self.manifest(self.roots[0], original)
        self.mutate_manifest(self.roots[0], lambda m: m['checkpoints'][0].update(frame=1602))
        with self.assertRaisesRegex(ValueError, 'schedule'): self.audit()

    def test_surface_hash_and_extent(self):
        for name in ('wram-1600.bin', 'reference-1600.bmp'):
            path = self.roots[0] / name
            data = path.read_bytes()
            for bad in (data[:-1], data[:-1] + b'\x01'):
                path.write_bytes(bad)
                with self.assertRaises(ValueError): self.audit()
            path.write_bytes(data)

    def test_conversion(self):
        data = check.bmp_header() + bytes([1, 2, 3]) * (256 * 240)
        self.assertEqual(check.rgb(data), bytes([3, 2, 1]) * (256 * 240))
        bad = bytearray(data); bad[22] ^= 1
        with self.assertRaises(ValueError): check.rgb(bad)

    def test_every_source_pin(self):
        expected = {name: check.sha(b'original') for name in check.SOURCE_FILES}
        for name in expected:
            path = self.base / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(b'original')
        check.verify_sources(self.base, expected)
        for name in expected:
            with self.subTest(name=name):
                path = self.base / name; path.write_bytes(b'mutation')
                with self.assertRaisesRegex(ValueError, 'source'): check.verify_sources(self.base, expected)
                path.write_bytes(b'original')
        incomplete = deepcopy(expected); incomplete.pop('crates/oracle/build.rs')
        with self.assertRaisesRegex(ValueError, 'source'): check.verify_sources(self.base, incomplete)

    def test_archived_nonpixel_digest(self):
        report = self.audit()
        check.verify_nonpixel_digest(report, report['nonpixel_manifest_sha256'])
        with self.assertRaisesRegex(ValueError, 'archived non-pixel digest'):
            check.verify_nonpixel_digest(report, 'wrong')

    def test_historical_audit_requires_explicit_fixed_source(self):
        # Stop immediately after source authentication: no private fixtures needed.
        old_repo, fixed_repo = self.base / 'old-source', self.base / 'fixed-source'
        rom, save = self.base / 'rom', self.base / 'save'
        rom.write_bytes(b'rom'); save.write_bytes(b'save')
        with patch.object(check, 'ROM', check.sha(b'rom')), patch.object(check, 'SRAM', check.sha(b'save')):
            with patch.object(check, 'verify_sources', side_effect=[None, RuntimeError('source checked')]) as verify:
                with self.assertRaisesRegex(RuntimeError, 'source checked'):
                    check.audit(rom, save, *self.roots, old_repo, fixed_repo)
                self.assertEqual([call.args[0] for call in verify.call_args_list], [old_repo, fixed_repo])
            with self.assertRaises(TypeError):
                check.audit(rom, save, *self.roots, old_repo)

    def test_archive_authentication(self):
        inventory = check.inventory(self.roots[0])
        check.authenticate(self.roots[0], inventory)
        (self.roots[0] / 'wram-1600.bin').write_bytes(b'wrong')
        with self.assertRaisesRegex(ValueError, 'archived'): check.authenticate(self.roots[0], inventory)


if __name__ == '__main__':
    unittest.main()
