"""ROM-free planar tests plus optional local-only tamper checks of an export."""
from pathlib import Path
import copy
import json
import sys
import tempfile
import unittest
from check import check, glyph
from native_check import compare

FIXTURE = None
if len(sys.argv) == 3:
    FIXTURE = (Path(sys.argv.pop(1)), Path(sys.argv.pop(1)))

class PlanarTests(unittest.TestCase):
    def test_planes_and_quadrants(self):
        data = bytearray(64)
        data[0] = data[17] = data[32] = data[33] = 128
        data[62] = 1
        pixels = glyph(data)
        self.assertEqual([pixels[i] for i in (0,8,128,255)], [1,2,3,1])
        with self.assertRaises(ValueError):
            glyph(data[:63])

    def test_native_pixel_mismatch_and_cursor_exemption(self):
        wram = bytearray(0x20000)
        wram[0xDB4:0xDB8] = (0x6A80).to_bytes(2,'little') + (0x504).to_bytes(2,'little')
        vram = bytearray(0x10000)
        page = dict(glyphs=[dict(position=[0,0])])
        expected = bytearray(224*48)
        self.assertEqual(compare(page,expected,wram,vram),192)
        vram[0xE000] = 128
        with self.assertRaisesRegex(ValueError,'font cell'):
            compare(page,expected,wram,vram)
        with self.assertRaisesRegex(ValueError,'reserved space'):
            compare(page,expected,wram,vram,[0,0])
        # Keep the synthetic cursor tile confined to the first eight pixels.
        wram[0x1D506] = wram[0x1D546] = 1
        for y in range(16):
            expected[y*224:y*224+8] = bytes([3])*8
        self.assertEqual(compare(page,expected,wram,vram,[0,0]),64)

@unittest.skipIf(FIXTURE is None, 'optional ROM/export paths absent')
class TamperTests(unittest.TestCase):
    def test_wrong_rom_rejected(self):
        with self.assertRaisesRegex(ValueError, 'identity'):
            check(bytes(0x400000), FIXTURE[1])

    def test_export_mutations_rejected(self):
        rom = FIXTURE[0].read_bytes()
        original = json.loads((FIXTURE[1]/'export.json').read_text())
        changes = [
            lambda m: m['pages'].pop(),
            lambda m: m['pages'][0].update(acknowledgement='next'),
            lambda m: m['pages'][0].update(boundary_source=0),
            lambda m: m['pages'][0]['glyphs'][0].update(font_source=0xB48000),
            lambda m: m['pages'][0].update(file='../outside'),
            lambda m: m['pages'][0].update(width=225),
            lambda m: m['pages'][0].update(sha256='0'*64),
            lambda m: m['choices'][0]['options'][0].update(position=[8,16]),
            lambda m: m['choices'][1]['options'][0].update(neighbors=[None]*4),
        ]
        for change in changes:
            with self.subTest(change=change), tempfile.TemporaryDirectory(dir='local') as directory:
                root = Path(directory)
                for page in original['pages']:
                    (root/page['file']).write_bytes((FIXTURE[1]/page['file']).read_bytes())
                meta = copy.deepcopy(original)
                change(meta)
                (root/'export.json').write_text(json.dumps(meta))
                with self.assertRaises(ValueError):
                    check(rom, root)
        with tempfile.TemporaryDirectory(dir='local') as directory:
            root = Path(directory)
            (root/'export.json').write_text(json.dumps(original))
            page = original['pages'][0]
            pixels = bytearray((FIXTURE[1]/page['file']).read_bytes())
            pixels[0] ^= 1
            (root/page['file']).write_bytes(pixels)
            with self.assertRaisesRegex(ValueError, 'raster'):
                check(rom, root)

if __name__ == '__main__':
    unittest.main()
