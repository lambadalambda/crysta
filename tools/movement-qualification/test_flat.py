"""ROM-free tests for the deliberately smaller flat-wall experiment."""
import unittest
import sys
sys.dont_write_bytecode = True
from verify import Unqualified, admit_cardinal, collide


def grid():
    w = bytearray(0x20000)
    w[0x827] = 2  # 32 cells per row
    w[0x862:0x864] = (2047).to_bytes(2, 'little')
    return w


def cell(w, x, y, kind, flagged=False):
    raw = kind << 9 | (0x8000 if flagged else 0)
    offset = 0xa000 + 2 * (y * 32 + x)
    w[offset:offset + 2] = raw.to_bytes(2, 'little')


class FlatDomain(unittest.TestCase):
    def test_open_cardinal_steps(self):
        for dx, dy in [(1, 0), (-2, 0), (0, 2), (0, -1)]:
            self.assertEqual(collide(grid(), 104, 112, dx, dy, flat_only=True),
                             (104 + dx, 112 + dy))

    def test_four_full_walls(self):
        for dx, dy, column, row in [(1, 0, 7, 6), (-1, 0, 5, 6),
                                    (0, 1, 6, 7), (0, -1, 6, 5)]:
            w = grid()
            cell(w, column, row, 14)
            self.assertEqual(collide(w, 104, 112, dx, dy, flat_only=True), (104, 112))

    def test_mixed_pair_is_outside_minimal_domain(self):
        w = grid()
        cell(w, 7, 7, 14)
        with self.assertRaises(Unqualified):
            collide(w, 104, 113, 1, 0, flat_only=True)

    def test_unknown_and_flagged_cells_stop(self):
        for kind, flagged in [(16, False), (6, False), (12, True)]:
            w = grid()
            cell(w, 7, 6, kind, flagged)
            with self.assertRaises(Unqualified):
                collide(w, 104, 112, 1, 0, flat_only=True)

    def test_cardinal_history_admission(self):
        admit_cardinal(['Left'] * 56 + ['Down'] * 24)
        admit_cardinal(['Left', 'Right', 'Up', 'Down', ''])
        for inputs in [['Left', '', 'Left'], ['Left', 'Right', 'Left'],
                       ['Left+Down'], ['A']]:
            with self.assertRaises(Unqualified):
                admit_cardinal(inputs)


if __name__ == '__main__':
    unittest.main()
