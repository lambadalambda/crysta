"""Synthetic regression checks; no ROM or native capture needed."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("cadence_verify", Path(__file__).with_name("verify.py"))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


def fixture():
    common = bytearray(0x1a0c)
    def word(data, at, value):
        data[at:at + 2] = value.to_bytes(2, "little")
    for selector, axis, pointer, value in [(0x60, 0, 0x6d18, 1), (0x68, 1, 0x6fc8, 1), (0x69, 1, 0x6fd4, 0xffff)]:
        word(common, selector * 4 + axis * 2, pointer)
        for i, v in enumerate([0, value, 0, 0, 0xffff, pointer + 2]):
            word(common, pointer - 0x6000 + 2 + i * 2, v)
    body = bytearray(0x28b)
    for selector in range(6):
        at = 0x20 + selector * 20
        word(body, selector * 2, at)
        durations = [0] if selector < 3 else [7] * 4
        for duration in durations:
            word(body, at, duration)
            at += 4
        word(body, at, 0xffff)
    wram = bytearray(0x20000)
    wram[0x16000:0x17a0c] = common
    wram[0x44b:0x44e] = bytes.fromhex("37 f0 ab")
    rows = [{"sample": i, "frame": 8175 + i, "map": 13, "class": 0,
             "e": [0] * 32, "aux": [0] * 32} for i in range(401)]
    frames = {r["frame"]: r for r in rows}
    for first, selector, start, delta in [(8195, 3, (72, 672), (0, 1)), (8371, 4, (72, 688), (0, -1)), (8403, 5, (72, 672), (1, 0))]:
        frames[first - 1]["e"][:2] = start
        for i in range(32):
            e, aux = frames[first + i]["e"], frames[first + i]["aux"]
            e[:2] = [start[a] + delta[a] * (i // 2 + 1) for a in range(2)]
            e[7], e[16], e[17] = 7 - i % 8, i // 8 + 1, 1
            aux[4] = selector
    for i in range(16):
        e = frames[8227 + i]["e"]
        e[:2], e[16], e[17] = [72, 688], 1, 16 - i
    frames[8243]["e"][17] = 16
    return common, body, wram, rows


class VerificationTests(unittest.TestCase):
    def test_complete_bounded_observations(self):
        verify.verify(*fixture())

    def test_stream_signed_values_loop_and_null(self):
        common, _, _, _ = fixture()
        self.assertEqual(verify.stream(common, 0x6fd4, 32), [-1, 0] * 16)
        self.assertEqual(verify.stream(common, 0, 32), [0] * 32)

    def test_rejects_changed_evidence(self):
        common, body, wram, rows = fixture()
        bad_wram = bytearray(wram)
        bad_wram[0x179ff] ^= 1  # outside the three observed streams: compare ALL bytes
        cases = [(common, body, bad_wram, rows), (common, body, wram[:-1], rows),
                 (common, body, wram, rows[:-1])]
        for frame, field, index, value in [(8195, "e", 1, 672), (8227, "e", 17, 8),
                                           (8371, "aux", 4, 3), (8403, "e", 7, 6)]:
            bad = copy.deepcopy(rows)
            bad[frame - 8175][field][index] = value
            cases.append((common, body, wram, bad))
        for args in cases:
            with self.subTest(case=cases.index(args)):
                with self.assertRaises(AssertionError):
                    verify.verify(*args)


if __name__ == "__main__":
    unittest.main()
