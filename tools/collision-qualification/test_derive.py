#!/usr/bin/env python3
"""ROM-free controls for the collision derivation.

Synthetic layers with a known answer, so a derivation bug is caught without a
sweep. These prove the solver reports contradiction rather than a best fit.
"""
import json
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import derive  # noqa: E402

WALL, FLOOR = 14, 0


def layer(width, height, solid_cells, attr_floor=FLOOR, attr_wall=WALL):
    cells = []
    for y in range(height):
        for x in range(width):
            a = attr_wall if (x, y) in solid_cells else attr_floor
            cells.append((a & 0x7F) << 9 | 0x40)
    return {'map': 0xF, 'width': width, 'height': height, 'cells': cells}


def frames(samples, control=160, map_id=0xF):
    """samples: (label, held, [(x, y), ...])"""
    out = []
    for label, held, positions in samples:
        for x, y in positions:
            out.append({'kind': 'frame', 'label': label, 'held': held,
                        'position': [x, y], 'frame': 0,
                        'control': control, 'map': map_id})
    return out


class DeriveGates(unittest.TestCase):
    def test_attribute_extraction_recovers_the_unambiguous_six_bits(self):
        # Bits 9..14 invert the loader transform exactly. Bit 6 of the source
        # attribute lands in word bit 15, which gameplay also writes, so it is
        # deliberately dropped rather than reported as attribute data.
        for attribute in range(0x80):
            for index in (0, 1, 0x1FF):
                word = index | ((attribute & 0x7F) << 9)
                lay = {'width': 1, 'height': 1, 'cells': [word]}
                self.assertEqual(derive.attribute(lay, 0, 0), attribute & 0x3F)

    def test_out_of_bounds_is_none_not_wrapped(self):
        lay = layer(4, 4, set())
        self.assertIsNone(derive.attribute(lay, -1, 0))
        self.assertIsNone(derive.attribute(lay, 4, 0))
        self.assertIsNone(derive.attribute(lay, 0, 4))

    def test_stall_needs_a_sustained_press(self):
        # A short hold against a wall is not yet evidence.
        short = frames([('a', ['Right'], [(32, 32)] * (derive.STALL_FRAMES - 1))])
        self.assertEqual(derive.stalls(short), [])
        long = frames([('a', ['Right'], [(32, 32)] * derive.STALL_FRAMES)])
        self.assertEqual(derive.stalls(long), [(32, 32, 1, 0)])

    def test_moving_run_is_not_a_stall(self):
        moving = frames([('a', ['Right'],
                          [(32 + i, 32) for i in range(derive.STALL_FRAMES * 2)])])
        self.assertEqual(derive.stalls(moving), [])

    def test_mirror_offsets_outside_the_sprite_are_not_admitted(self):
        # An offset far off the player can be internally consistent while
        # reporting the partition inverted; the sprite window excludes it.
        lay = layer(8, 8, {(5, y) for y in range(8)})
        samples = frames([('walk', ['Right'], [(x, 32) for x in range(64, 80)]),
                          ('press', ['Right'], [(79, 32)] * derive.STALL_FRAMES)])
        wide, *_ = derive.solve(samples, lay, range(-24, 25), range(-24, 25))
        self.assertTrue(any(r['walkable'] == [WALL] for r in wide),
                        'expected a mirror solution outside the sprite window')
        bounded, *_ = derive.solve(samples, lay)
        self.assertTrue(bounded)
        self.assertFalse(any(r['walkable'] == [WALL] for r in bounded))

    def test_solver_recovers_a_planted_partition(self):
        # Floor row at cy=2, wall at cx=5. Player word sits one cell right of
        # its collision point, so only dx<0 offsets can be consistent.
        lay = layer(8, 8, {(5, y) for y in range(8)})
        positions = [(x, 32) for x in range(64, 80)]
        samples = frames([('walk', ['Right'], positions),
                          ('press', ['Right'], [(79, 32)] * derive.STALL_FRAMES)])
        results, points, contacts, _cells, _distinct = derive.solve(samples, lay)
        self.assertEqual(contacts, 1)
        self.assertTrue(results)
        for r in results:
            self.assertEqual(r['walkable'], [FLOOR])
            self.assertEqual(r['solid'], [WALL])

    def test_contradictory_samples_yield_no_offset(self):
        # Every cell is wall, yet the player both stands and is blocked: no
        # single-point model can satisfy that, and none must be invented.
        lay = layer(8, 8, {(x, y) for x in range(8) for y in range(8)})
        samples = frames([('walk', ['Right'], [(x, 32) for x in range(16, 48)]),
                          ('press', ['Right'], [(48, 32)] * derive.STALL_FRAMES)])
        results, *_ = derive.solve(samples, lay)
        self.assertEqual(results, [])

    def test_diagonal_and_empty_holds_are_ignored(self):
        mixed = frames([('a', ['Right', 'Up'], [(32, 32)] * derive.STALL_FRAMES),
                        ('b', [], [(32, 32)] * derive.STALL_FRAMES)])
        self.assertEqual(derive.stalls(mixed), [])

    def test_stall_spanning_a_control_change_is_not_wall_contact(self):
        # Standing still because input is not admitted is not a wall.
        held = [(32, 32)] * derive.STALL_FRAMES
        mixed = frames([('a', ['Right'], held[:10])], control=160) + \
            frames([('a', ['Right'], held[10:])], control=0)
        self.assertEqual(derive.stalls(mixed), [])
        steady = frames([('a', ['Right'], held)], control=160)
        self.assertEqual(derive.stalls(steady), [(32, 32, 1, 0)])

    def test_samples_from_another_map_are_refused(self):
        lay = layer(8, 8, set())
        lay['map'] = 0xF
        with self.assertRaises(SystemExit):
            derive.check_one_map(frames([('a', ['Right'], [(1, 1)])], map_id=0x41), lay)
        mixed = frames([('a', ['Right'], [(1, 1)])], map_id=0xF) + \
            frames([('b', ['Right'], [(2, 2)])], map_id=0x41)
        with self.assertRaises(SystemExit):
            derive.check_one_map(mixed, lay)
        derive.check_one_map(frames([('a', ['Right'], [(1, 1)])], map_id=0xF), lay)

    def test_dynamic_bit_does_not_split_an_attribute(self):
        # Word bit 15 is set during play; including it would make one attribute
        # read as two depending on whether a cell had been touched.
        lay = {'width': 2, 'height': 1, 'cells': [0x1ce8, 0x9ce8]}
        self.assertEqual(derive.attribute(lay, 0, 0), derive.attribute(lay, 1, 0))
        self.assertEqual(derive.attribute(lay, 0, 0), 14)

    def test_round_trips_through_the_documented_file_shapes(self):
        lay = layer(8, 8, {(5, y) for y in range(8)})
        with tempfile.TemporaryDirectory() as d:
            run = Path(d) / 'run.jsonl'
            rows = frames([('walk', ['Right'], [(x, 32) for x in range(64, 80)])])
            run.write_text('\n'.join(json.dumps(r) for r in rows) + '\n')
            loaded = derive.load(run)
        self.assertEqual(len(loaded), 16)
        self.assertEqual(loaded[0]['position'], [64, 32])


if __name__ == '__main__':
    unittest.main()
