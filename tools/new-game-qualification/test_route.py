"""ROM-free tests for honest static/runtime grid comparison."""
import unittest
from route_check import compare_grids


class GridTest(unittest.TestCase):
    def test_identical(self):
        self.assertEqual(compare_grids(bytes(4096), bytes(4096)), [])

    def test_reports_high_bit_instead_of_hiding_it(self):
        runtime = bytearray(4096)
        runtime[635] = 128
        self.assertEqual(compare_grids(bytes(4096), runtime),
                         [dict(index=317, x=29, y=9, static=0, runtime=32768)])

    def test_rejects_low_bits_and_bad_extent(self):
        runtime = bytearray(4096)
        runtime[634] = 1
        for a, b in [(bytes(4096), runtime), (bytes(2), bytes(2))]:
            with self.assertRaises(AssertionError):
                compare_grids(a, b)


if __name__ == '__main__':
    unittest.main()
