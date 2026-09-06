"""Synthetic checker regressions; no ROM or captured art required."""
import unittest
from check import require, verify_oam, painter_order

class Checks(unittest.TestCase):
    def test_oam_components_and_size_bits(self):
        oam = bytearray(544)
        pattern = bytes([160,126,162,43,168,151,191,43])
        oam[:8] = pattern
        oam[512] = 2
        self.assertEqual(verify_oam(oam,pattern,[16,8]),0)
        oam[512] = 0
        with self.assertRaises(ValueError): verify_oam(oam,pattern,[16,8])
        oam[512] = 2; oam[3] ^= 16
        with self.assertRaises(ValueError): verify_oam(oam,pattern,[16,8])
    def test_tie_is_native_list_not_entity_id(self):
        self.assertEqual(painter_order(353,416,[0x1040,0x1000]),[0x1000,0x1040])
        self.assertEqual(painter_order(416,416,[0x1040,0x1000]),[0x1040,0x1000])
        self.assertEqual(painter_order(416,416,[0x1000,0x1040]),[0x1000,0x1040])
        with self.assertRaises(ValueError): painter_order(416,416,[0x1040])
    def test_fail_closed(self):
        with self.assertRaises(ValueError): require(False,'tampered')

if __name__ == '__main__': unittest.main()
