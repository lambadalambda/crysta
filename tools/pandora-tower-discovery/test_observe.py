#!/usr/bin/env python3
"""Unit tests for the discovery observation reader. No ROM or capture needed."""

from __future__ import annotations

import unittest

import observe


def wram(flags=(), map_id=observe.MAP_HALL, position=(120, 192), facing=1, control=0):
    w = bytearray(0x20000)
    for f in flags:
        w[observe.EVENT_BLOCK + f // 8] |= 1 << (f % 8)
    w[observe.MAP : observe.MAP + 2] = map_id.to_bytes(2, "little")
    w[observe.POSITION : observe.POSITION + 2] = position[0].to_bytes(2, "little")
    w[observe.POSITION + 2 : observe.POSITION + 4] = position[1].to_bytes(2, "little")
    w[observe.FACING : observe.FACING + 2] = facing.to_bytes(2, "little")
    w[observe.CONTROL : observe.CONTROL + 2] = control.to_bytes(2, "little")
    return bytes(w)


class TestLiteralLayout(unittest.TestCase):
    """Byte-literal checks, so a wrong base, bit order or endianness is caught.

    The `wram()` builder below encodes with the inverse of the expression under
    test and therefore cannot detect those; these can.
    """

    def test_flag_bit_order_within_a_byte(self):
        w = bytearray(0x20000)
        w[observe.EVENT_BLOCK] = 0b0000_0101  # flags $000 and $002
        w[observe.EVENT_BLOCK + 1] = 0b1000_0000  # flag $00F
        self.assertEqual(observe.flags(bytes(w)), {0x000, 0x002, 0x00F})

    def test_flag_base_address(self):
        w = bytearray(0x20000)
        w[observe.EVENT_BLOCK - 1] = 0xFF  # just below the block
        self.assertEqual(observe.flags(bytes(w)), set())

    def test_tour_flag_lands_where_the_documented_flag_lands(self):
        # $244 -> byte $6C0 + $48, bit 4.
        w = bytearray(0x20000)
        w[0x6C0 + 0x48] = 1 << 4
        self.assertEqual(observe.flags(bytes(w)), {0x244})

    def test_words_are_little_endian(self):
        w = bytearray(0x20000)
        w[observe.POSITION] = 0x34
        w[observe.POSITION + 1] = 0x12
        self.assertEqual(observe.word(bytes(w), observe.POSITION), 0x1234)


class TestObserve(unittest.TestCase):
    def test_reads_flags_above_the_probe_window(self):
        # The probe's own JSONL stops at $200; these must still be visible.
        self.assertEqual(observe.flags(wram([0x243, 0x244, 0x292])), {0x243, 0x244, 0x292})

    def test_reads_scalar_state(self):
        state = observe.observe(wram([0x20], map_id=0x44, position=(40, 112), facing=2, control=160))
        self.assertEqual(state["map"], 0x44)
        self.assertEqual(state["position"], [40, 112])
        self.assertEqual(state["facing"], 2)
        self.assertEqual(state["control"], 160)
        self.assertEqual(state["flags"], [0x20])

    def test_endpoint_capture_has_not_progressed(self):
        self.assertEqual(
            observe.progressed(wram(observe.ENDPOINT_FLAGS)),
            {"new_flags": [], "lost_flags": [], "left_hall": False},
        )

    def test_continuation_flags_are_reported(self):
        # $FE/$23 are what $88AF3F/AF43 write in the later continuation.
        change = observe.progressed(wram(set(observe.ENDPOINT_FLAGS) | {0x23, 0xFE}))
        self.assertEqual(change["new_flags"], [0x23, 0xFE])
        self.assertFalse(change["left_hall"])

    def test_leaving_the_hall_is_reported(self):
        self.assertTrue(observe.progressed(wram(observe.ENDPOINT_FLAGS, map_id=0x21))["left_hall"])

    def test_missing_endpoint_flags_are_reported(self):
        change = observe.progressed(wram(set(observe.ENDPOINT_FLAGS) - {0x244}))
        self.assertEqual(change["lost_flags"], [0x244])


class TestSweepLine(unittest.TestCase):
    def test_quiet_row_is_quiet(self):
        line = observe.sweep_line(
            "x/fb-1-Left.wram", wram(observe.ENDPOINT_FLAGS, control=observe.CONTROL_HELD)
        )
        self.assertEqual(line, "fb-1-Left: map=65 pos=[120, 192]")

    def test_dead_input_is_called_out(self):
        # The failure mode that silently invalidated a whole sweep.
        line = observe.sweep_line("x/a.wram", wram(observe.ENDPOINT_FLAGS, control=0))
        self.assertIn("result void", line)

    def test_progression_outranks_dead_input(self):
        line = observe.sweep_line(
            "x/a.wram", wram(set(observe.ENDPOINT_FLAGS) | {0x23}, control=0)
        )
        self.assertIn("PROGRESSED", line)
        self.assertNotIn("result void", line)


if __name__ == "__main__":
    unittest.main()
