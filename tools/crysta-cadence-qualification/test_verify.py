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


def town_fixture():
    common, _, wram, _ = fixture()
    def word(data, at, value):
        data[at:at + 2] = value.to_bytes(2, "little")
    body = bytearray(0x71d)
    for selector in range(6):
        at = 0x20 + selector * 20
        word(body, selector * 2, at)
        for duration in [7] * (2 if selector < 3 else 4):
            word(body, at, duration)
            at += 4
        word(body, at, 0xffff)
    rows = [{"sample": i, "frame": 11394 + i, "map": 10, "class": 2,
             "e": [0] * 32, "aux": [0] * 32} for i in range(201)]
    frames = {r["frame"]: r for r in rows}
    position = [552, 416]
    def idle(first, selector, begin=0):
        for i in range(begin, 16):
            e, aux = frames[first + i - begin]["e"], frames[first + i - begin]["aux"]
            e[:2], e[7], e[16], e[17], aux[4] = position, 7 - i % 8, i // 8 + 1, 1, selector
    def step(first, selector, delta, count=32):
        for i in range(count):
            e, aux = frames[first + i]["e"], frames[first + i]["aux"]
            if i % 2 == 0:
                position[:] = [position[a] + delta[a] for a in range(2)]
            e[:2], e[7], e[16], e[17], aux[4] = list(position), 7 - i % 8, i // 8 + 1, 1, selector
    idle(11394, 2, begin=2)
    frames[11408]["e"][:2] = position
    for first, kind, selector, _ in verify.TOWN_ACTIONS:
        if kind == "idle":
            idle(first, selector)
            frames[first + 16]["e"][:2] = list(position)
        else:
            unit = {3: (0, 1), 4: (0, -1), 5: (1, 0)}[selector]
            step(first, selector, unit)
            frames[first + 32]["e"][:2] = list(position)
    step(11575, 5, (1, 0), count=20)
    return common, bytes(body), wram, rows


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
        for number, args in enumerate(cases):
            with self.subTest(case=number):
                with self.assertRaises(AssertionError):
                    verify.verify(*args)


class TownTests(unittest.TestCase):
    def test_every_town_frame_is_bound(self):
        verify.verify_town(*town_fixture())

    def test_rejects_changed_town_evidence(self):
        common, body, wram, rows = town_fixture()
        cases = [(common, body, wram, rows[:-1]), (common, body[:-1], wram, rows)]
        bad_wram = bytearray(wram)
        bad_wram[0x179ff] ^= 1
        cases.append((common, body, bytes(bad_wram), rows))
        for at in (0x20 + 20, 0x20 + 3 * 20):  # an idle and a walking record
            bad_body = bytearray(body)
            bad_body[at] = 6
            cases.append((common, bytes(bad_body), wram, rows))
        for frame, field, index, value in [
                (11394, "class", None, 0), (11400, "map", None, 13), (11409, "e", 0, 552),
                (11441, "e", 17, 1), (11441, "e", 16, 4), (11450, "aux", 4, 5),
                (11457, "e", 0, 569), (11470, "e", 16, 1), (11500, "aux", 12, 1),
                (11520, "e", 1, 400), (11560, "aux", 4, 3), (11574, "aux", 8, 1),
                (11590, "e", 0, 700), (11400, "e", 7, 3)]:
            bad = copy.deepcopy(rows)
            if index is None:
                bad[frame - 11394][field] = value
            else:
                bad[frame - 11394][field][index] = value
            cases.append((common, body, wram, bad))
        for number, args in enumerate(cases):
            with self.subTest(case=number):
                with self.assertRaises(AssertionError):
                    verify.verify_town(*args)


if __name__ == "__main__":
    unittest.main()
