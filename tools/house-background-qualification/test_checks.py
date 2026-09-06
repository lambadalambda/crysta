"""ROM-free tests for bounded background/collision qualification helpers."""
import unittest
from check import attributed_grid, stamp, compare_grid, sample, camera_bounds, animation_frames

class Checks(unittest.TestCase):
    def test_attributes_replace_high_bits(self):
        attrs = bytearray(512); attrs[1] = 0xff
        self.assertEqual(attributed_grid([0xfe01, 0], attrs), [0xfe01, 0])
        attrs[1] = 12
        self.assertEqual(attributed_grid([0xfe01], attrs), [0x1801])

    def test_stamp_uses_native_small_and_rectangle_rules(self):
        grid = [2] * 2048
        stamp(grid, (120,720), (-8,16,-16,16))
        stamp(grid, (440,368), (-8,32,-16,16))
        self.assertEqual([i for i,v in enumerate(grid) if v&0x8000], [731,732,1415])
        self.assertEqual(grid[1415], 0x8002)
        with self.assertRaises(ValueError): stamp(grid, (0,0), (-8,16,-16,16))

    def test_actor_mask_never_hides_low_words_or_fixed_gate(self):
        self.assertEqual(compare_grid([0x8001,0x8002], [1,0x8002], {0}), [])
        self.assertEqual(compare_grid([0x8001,0x8002], [2,2], {0}), [(0,0x8001,2),(1,0x8002,2)])
        with self.assertRaises(ValueError): compare_grid([0], [], set())

    def test_full_word_pixel_flips_palette_priority_and_zero(self):
        graphics = bytearray(32); graphics[0] = 128
        self.assertEqual(sample(graphics, 0x2c00, 0, 0), (49,True))
        self.assertEqual(sample(graphics, 0x2c00, 1, 0), (0,False))
        self.assertEqual(sample(graphics, 0xec00, 7, 7), (49,True))
        with self.assertRaises(ValueError): sample(graphics, 1, 0, 0)

    def test_animation_repetition_advances_source_not_destination(self):
        rom=bytearray(0x1c0000)
        rom[0x1b8020:0x1b8022]=bytes([0,1])
        rom[0x1b8100:0x1b8108]=bytes([2,0,2,0x90,2,0x80,0,12])
        self.assertEqual(animation_frames(rom,0x10), [(0x1b8200,0x520,128),(0x1b8280,0x520,128)])
        rom[0x1b8100]=255
        with self.assertRaises(ValueError): animation_frames(rom,0x10)

    def test_camera_nibbles(self):
        self.assertEqual(camera_bounds(bytes([0x11,0x12])), [256,512,512,768])
        with self.assertRaises(ValueError): camera_bounds(bytes([0xff,0]))

if __name__ == '__main__': unittest.main()
