"""Synthetic contracts: no cartridge or captures needed."""
import unittest
import check

class Checks(unittest.TestCase):
    def test_camera_is_not_sheet_extent(self):
        self.assertEqual(check.camera_bounds(bytes([0x40, 0x40])), [0, 0, 1024, 1024])
        self.assertEqual(check.camera([504, 769], [0, 0, 1024, 1024], 256), [376, 657])
        self.assertEqual(check.camera([1000, 1200], [0, 0, 1024, 1024], 256), [768, 768])
        for record in (b'', b'\xff\x40', b'\x00\x40'):
            with self.assertRaises(ValueError): check.camera_bounds(record)

    def test_ring_wrap_uses_hardware_not_source_stride(self):
        self.assertEqual(check.ring_byte(0x3800, 33, 34), 2*(0x3800+65))
        self.assertEqual(check.definition_word([7]*5120, bytes(range(256))*16, 64, 3, 5), 0x3f3e)
        with self.assertRaises(ValueError): check.definition_word([0]*5120, bytes(4096), 64, 128, 0)
        with self.assertRaises(ValueError): check.ring_byte(0x3800, -1, 0)

    def test_grid_keeps_every_full_word_and_bounds_window(self):
        base = [0]*5120
        native = base.copy(); native[0] = 0x8000
        report = check.grid_comparison(base, native, 64, [29,47,36,53])
        self.assertEqual(report, [[0,0,0x8000]])
        native[47*64+29] = 0x8000
        with self.assertRaises(ValueError): check.grid_comparison(base, native, 64, [29,47,36,53])
        native = base.copy(); native[0] = 1
        with self.assertRaises(ValueError): check.grid_comparison(base, native, 64, [29,47,36,53])
        with self.assertRaises(ValueError): check.grid_comparison(base, base[:-1], 64, [29,47,36,53])
        with self.assertRaises(ValueError): check.grid_comparison(base, base, 64, [29,47,65,53])

    def test_visible_oam_inventory_rejects_other_size_modes(self):
        oam = bytearray(544)
        for i in range(128): oam[i*4+1] = 240
        oam[:4] = bytes([80,48,176,0x34]); oam[512] = 2
        self.assertEqual(check.visible_oam(oam, bytes([2,0])), [[0,80,48,16,3]])
        with self.assertRaises(ValueError): check.visible_oam(oam, bytes([0x22,0]))
        with self.assertRaises(ValueError): check.visible_oam(oam[:-1], bytes([2,0]))

    def test_source_attributes_rebuild_without_native_words(self):
        attributes=bytearray(512); attributes[7]=0x8e
        self.assertEqual(check.attributed_grid([7], attributes), [0x1c07])
        with self.assertRaises(ValueError): check.attributed_grid([512],attributes)

    def test_sample_preserves_transparency_priority_and_flip(self):
        tile = bytearray(32); tile[0] = 0x80
        self.assertEqual(check.sample(tile, 0x2800, 0, 0), [33, True])
        self.assertEqual(check.sample(tile, 0x2800, 1, 0), [0, False])
        self.assertEqual(check.sample(tile, 0x6800, 7, 0), [33, True])
        with self.assertRaises(ValueError): check.sample(tile, 1, 0, 0)

if __name__ == '__main__': unittest.main()
