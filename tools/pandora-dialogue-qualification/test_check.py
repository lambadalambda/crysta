"""ROM-free controls plus optional local ROM/export mutation checks (also under -O)."""
from pathlib import Path
from hashlib import sha256
import copy
import json
import sys
import tempfile
import unittest
from unittest.mock import patch
from contextlib import contextmanager
import native_check
from check import check, glyph, reconstruct
from native_check import compare, compare_capture

FIXTURE = None
if len(sys.argv) == 3:
    FIXTURE = (Path(sys.argv.pop(1)), Path(sys.argv.pop(1)))


def fake_rom(stream):
    rom = bytearray(0x400000)
    for index in range(6):
        at = 0x78C99 + index * 5
        rom[at:at+5] = bytes([0xA9, 0xD4 if index == 5 else 1, 0x8D]) + (0x610+index).to_bytes(2, 'little')
    rom[0x88000:0x88000+len(stream)] = bytes(stream)
    return rom


class ReconstructionTests(unittest.TestCase):
    def test_planes_and_quadrants(self):
        data = bytearray(64)
        data[0] = data[17] = data[32] = data[33] = 128
        data[62] = 1
        self.assertEqual([glyph(data)[i] for i in (0, 8, 128, 255)], [1, 2, 3, 1])
        with self.assertRaises(ValueError):
            glyph(data[:63])

    def test_order_ids_acknowledgements(self):
        pages = reconstruct(fake_rom([0xC0, 1, 0xD5, 2, 0xD3]), 0x888000)
        self.assertEqual([p['page_id'] for p in pages], [0x8880000, 0x8880001])
        self.assertEqual([p['acknowledgement'] for p in pages], ['next', 'end'])
        self.assertEqual([p['boundary_source'] for p in pages], [0x888002, 0x888004])

    def test_transformed_font_and_long_call(self):
        rom = fake_rom([0xC4, 0, 0xC1, 0xCC, 0x95, 0xA1, 0x88, 0xD3])
        rom[0x8A195:0x8A197] = bytes([1, 0xD4])
        rom[0x348040:0x348080] = bytes([255])*64
        page = reconstruct(rom, 0x888000)[0]
        self.assertEqual(page['glyphs'][0]['text_source'], 0x88A195)
        self.assertEqual(set(page['pixels']), {0})
        self.assertEqual(page['boundary_source'], 0x888007)

    def test_top_window_is_page_relative(self):
        page = reconstruct(fake_rom([0xC4, 1, 0xDA, 1, 0xD3]), 0x888000)[0]
        self.assertEqual(page['glyphs'][0]['position'], [0, 0])

    def test_button_icon_comes_from_default_writes_and_call_table(self):
        rom = fake_rom([0xC1, 0xE3, 0x40, 0, 0xD3])
        for i, address in enumerate((0x634,0x636,0x638,0x63A,0x63E,0x63C)):
            value = (0x80,0x8000,0x40,0x4000,0x10,0x20)[i]
            at = 0x5BEF7+i*6
            rom[at:at+6] = bytes([0xA9]) + value.to_bytes(2,'little') + bytes([0x8D]) + address.to_bytes(2,'little')
        rom[0x59711:0x59713] = (0x971D).to_bytes(2,'little')
        rom[0x5971D:0x5971F] = bytes([0x38,0xD4])
        page = reconstruct(rom, 0x888000)[0]
        self.assertEqual(page['glyphs'], [dict(text_source=0x85971D, font_source=0xB48E00, position=[0,0])])
        rom[0x5BEF7] = 0
        with self.assertRaisesRegex(ValueError, 'default controller'):
            reconstruct(rom, 0x888000)

    def test_custom_geometry_and_mode_persist_across_pages(self):
        pages = reconstruct(fake_rom([0xC4,0,0xC2,6,6,24,6,1,0xD5,1,0xD3]), 0x888000)
        self.assertEqual([(p['width'],p['height'],p['background_index'],len(p['pixels'])) for p in pages],
                         [(192,48,0,9216)]*2)
        with self.assertRaisesRegex(ValueError, 'page boundary'):
            reconstruct(fake_rom([0xC1]+[1,0xD5]*15+[1,0xD3]), 0x888000)

    def test_blank_unknown_and_unacknowledged_clear_rejected(self):
        for stream in ([0xC0, 0xD3], [0xC0, 1, 0xC0, 0xD3], [0xFF]):
            with self.subTest(stream=stream), self.assertRaises(ValueError):
                reconstruct(fake_rom(stream), 0x888000)


class NativeTests(unittest.TestCase):
    def fixture(self):
        wram = bytearray(0x20000)
        wram[0xDB4:0xDB8] = (0x6A80).to_bytes(2, 'little') + (0x504).to_bytes(2, 'little')
        # Unique tiles in each cell; one foreground pixel is deliberately compared.
        for y in range(6):
            for x in range(28):
                at = 0x1D504+y*64+x*2
                wram[at:at+2] = (y*28+x).to_bytes(2, 'little')
        vram = bytearray([255]*0x10000)
        vram[0xE000] ^= 128
        expected = bytearray([3]*(224*48))
        expected[0] = 2
        page = dict(width=224, height=48, background_index=3, glyphs=[dict(position=[0, 0])])
        return page, expected, wram, vram

    def test_nonvacuous_cells_and_pixel_tamper(self):
        page, expected, wram, vram = self.fixture()
        self.assertEqual(compare(page, expected, wram, vram), (192, 1))
        vram[0xE000] ^= 128
        with self.assertRaisesRegex(ValueError, 'font cell'):
            compare(page, expected, wram, vram)

    def test_blank_or_empty_comparison_rejected(self):
        page, expected, wram, vram = self.fixture()
        with self.assertRaises(ValueError):
            compare(dict(page, glyphs=[]), expected, wram, vram)
        expected[0] = 3
        vram[0xE000] = 255
        with self.assertRaisesRegex(ValueError, 'nonvacuous'):
            compare(page, expected, wram, vram)

    def test_ack_pointer_tamper(self):
        page, expected, wram, vram = self.fixture()
        page.update(acknowledgement='end', boundary_source=0x888010)
        wram[0xDC0:0xDC3] = (0x888010).to_bytes(3, 'little')
        self.assertEqual(compare_capture(page, expected, wram, vram, None), (192, 1))
        wram[0xDC0] ^= 1
        with self.assertRaisesRegex(ValueError, 'ack pointer'):
            compare_capture(page, expected, wram, vram, None)

    def test_cursor_cannot_hide_ink(self):
        with self.assertRaisesRegex(ValueError, 'reserved space'):
            compare(*self.fixture(), cursor=[0, 0])


class NativeOrchestrationTests(unittest.TestCase):
    @contextmanager
    def fixture(self):
        # Isolate native orchestration: source authentication/reconstruction has its
        # own ROM-backed tests above. Captures here are entirely synthetic.
        page, expected, wram, vram = NativeTests().fixture()
        page.update(page_id=1, file='synthetic.indexed', acknowledgement='end', boundary_source=0x888010)
        wram[0xDC0:0xDC3] = (0x888010).to_bytes(3, 'little')
        wram[0x634:0x636] = (0x80).to_bytes(2, 'little')
        sample = dict(label='selected', page_id=1, compared_pixels=192, foreground_pixels=1,
                      wram_sha256=sha256(wram).hexdigest(), vram_sha256=sha256(vram).hexdigest())
        reference = dict(capture_sets={'original': dict(samples=[sample], source_only_page_ids=[])},
                         source_only_page_ids=[])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/page['file']).write_bytes(expected)
            (root/'export.json').write_text(json.dumps(dict(pages=[page])))
            (root/'selected.wram').write_bytes(wram)
            (root/'selected.vram').write_bytes(vram)
            read_text = Path.read_text
            def read(path, *args, **kwargs):
                return json.dumps(reference) if path.name == 'native-reference.json' else read_text(path, *args, **kwargs)
            with patch('native_check.check'), patch('native_check.choices', return_value=[{}]), \
                 patch('native_check.default_assignments', return_value={0x634:0x80}), \
                 patch.object(Path, 'read_text', read):
                yield root, reference

    def test_valid_orchestration(self):
        with self.fixture() as (root, _):
            result = native_check.inspect(b'', root, root, 'original')
            self.assertEqual((result['total_compared_pixels'],result['total_foreground_pixels']), (192,1))
            self.assertEqual(result['source_only_page_ids'], [])

    def test_uncompared_capture_byte_is_still_hash_authenticated(self):
        for extension in ('wram','vram'):
            with self.subTest(extension=extension), self.fixture() as (root, _):
                path = root/f'selected.{extension}'
                data = bytearray(path.read_bytes())
                data[0x100] ^= 1  # outside all compared text cells and runtime fields
                path.write_bytes(data)
                with self.assertRaisesRegex(ValueError, 'selected native reference'):
                    native_check.inspect(b'', root, root, 'original')

    def test_unknown_set_missing_capture_and_changed_defaults_never_fall_back(self):
        with self.fixture() as (root, _):
            with self.assertRaisesRegex(ValueError, 'explicit native capture set'):
                native_check.inspect(b'', root, root, 'unrecognized')
            data = bytearray((root/'selected.wram').read_bytes())
            data[0x634] ^= 1
            (root/'selected.wram').write_bytes(data)
            with self.assertRaisesRegex(ValueError, 'default controller'):
                native_check.inspect(b'', root, root, 'original')
            (root/'selected.vram').unlink()
            with self.assertRaises(FileNotFoundError):
                native_check.inspect(b'', root, root, 'original')

    def test_source_only_inventories_and_empty_selection_rejected(self):
        for mutation in ('aggregate','per set','empty'):
            with self.subTest(mutation=mutation), self.fixture() as (root, reference):
                if mutation == 'aggregate':
                    reference['source_only_page_ids'] = [1]
                elif mutation == 'per set':
                    reference['capture_sets']['original']['source_only_page_ids'] = [1]
                else:
                    reference['capture_sets']['original']['samples'] = []
                    reference['source_only_page_ids'] = [1]
                    reference['capture_sets']['original']['source_only_page_ids'] = [1]
                error = 'selected native reference' if mutation == 'empty' else 'source-only set'
                with self.assertRaisesRegex(ValueError, error):
                    native_check.inspect(b'', root, root, 'original')


@unittest.skipIf(FIXTURE is None, 'optional local ROM/export paths absent')
class ExportTests(unittest.TestCase):
    def run_check(self, rom, root):
        return check(rom, root)

    def test_valid_export(self):
        report = self.run_check(FIXTURE[0].read_bytes(), FIXTURE[1])
        self.assertGreater(len(report['pages']), 0)
        self.assertGreater(len(report['requests']), 0)

    def test_map13_refusal_appends_without_changing_existing_pages_or_direct_route(self):
        report = self.run_check(FIXTURE[0].read_bytes(), FIXTURE[1])
        self.assertEqual(len(report['requests']), 33)
        self.assertEqual(len(report['pages']), 76)
        self.assertEqual(len(report['invocations']), 34)
        self.assertFalse(any(r['source'] == 0x88B7E3 for r in report['invocations']))
        self.assertEqual(sha256(json.dumps(report['pages'][:74], sort_keys=True).encode()).hexdigest(),
                         '82215b329e8e0fec1c6962724351f8254d0d5fcbe65ea513118d69dff3671fbc')
        self.assertEqual(report['requests'][-1], dict(source=0x88B7E3, choice_catalog=None,
                                                    page_ids=[0x88B7E30,0x88B7E31]))
        self.assertEqual([(p['boundary_source'],p['acknowledgement']) for p in report['pages'][-2:]],
                         [(8960007, 'next'), (8960045, 'end')])
        rom = FIXTURE[0].read_bytes()
        self.assertEqual([int.from_bytes(rom[p:p+2], 'little') for p in (0x8B68B,0x8B68D,0x8B68F)],
                         [0xB691,0xB69C,0xB691])  # cancel/result2 enter the same refusal site

    def test_mid_reveal_cursor_is_not_a_page_boundary(self):
        rom = FIXTURE[0].read_bytes()
        pages = reconstruct(rom, 0x88ADF2)
        self.assertEqual(rom[0x8AE50], 0x56)
        self.assertEqual([(p['boundary_source'],p['acknowledgement']) for p in pages],
                         [(0x88AE29,'next'),(0x88AE5E,'end')])
        self.assertTrue(any(g['text_source'] == 0x88AE50 for p in pages for g in p['glyphs']))

    def test_wrong_rom(self):
        with self.assertRaisesRegex(ValueError, 'identity'):
            self.run_check(bytes(0x400000), FIXTURE[1])

    def test_mutations(self):
        rom = FIXTURE[0].read_bytes()
        original = json.loads((FIXTURE[1]/'export.json').read_text())
        changes = {
            'blank': lambda m: m.update(pages=[]),
            'page order': lambda m: m['pages'].reverse(),
            'page ID': lambda m: m['pages'][0].update(page_id=0),
            'background': lambda m: m['pages'][0].update(background_index=0),
            'invocations order': lambda m: m['invocations'].reverse(),
            'invocations blank': lambda m: m.update(invocations=[]),
            'invocation ID': lambda m: m['invocations'][0].update(source=0),
            'invocation site': lambda m: m['invocations'][0].update(site=0),
            'ack': lambda m: m['pages'][0].update(acknowledgement='wrong'),
            'boundary': lambda m: m['pages'][0].update(boundary_source=0),
            'glyph blank': lambda m: m['pages'][0].update(glyphs=[]),
            'font': lambda m: m['pages'][0]['glyphs'][0].update(font_source=0),
            'path': lambda m: m['pages'][0].update(file='../outside'),
            'dimensions': lambda m: m['pages'][0].update(width=225),
            'hash': lambda m: m['pages'][0].update(sha256='0'*64),
            'requests blank': lambda m: m.update(requests=[]),
            'requests order': lambda m: m['requests'].reverse(),
            'request ID': lambda m: m['requests'][0].update(source=0),
            'request target': lambda m: m['requests'][0].update(page_ids=[]),
            'choice': lambda m: m['choices'][0]['options'][0].update(neighbors=[None]*4),
        }
        for label in (*changes, 'pixel', 'blank raster'):
            with self.subTest(mutation=label), tempfile.TemporaryDirectory(dir='local') as directory:
                root = Path(directory)
                for page in original['pages']:
                    (root/page['file']).write_bytes((FIXTURE[1]/page['file']).read_bytes())
                meta = copy.deepcopy(original)
                if label in changes:
                    changes[label](meta)
                else:
                    page = meta['pages'][0]
                    pixels = bytearray((root/page['file']).read_bytes())
                    if label == 'pixel':
                        pixels[0] ^= 1
                    else:
                        pixels[:] = bytes([3])*len(pixels)
                    (root/page['file']).write_bytes(pixels)
                    page['sha256'] = sha256(pixels).hexdigest()
                (root/'export.json').write_text(json.dumps(meta))
                with self.assertRaises(ValueError):
                    self.run_check(rom, root)


if __name__ == '__main__':
    unittest.main()
