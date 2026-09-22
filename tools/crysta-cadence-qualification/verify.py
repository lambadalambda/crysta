"""Bounded ROM-decoded common resource versus native cadence witness."""
import argparse
import json
from pathlib import Path
import subprocess

# Qualification assertions must never silently disappear under python -O.
if not __debug__:
    raise RuntimeError("qualification requires assertions; do not use python -O")


def u(data, at):
    assert 0 <= at and at + 2 <= len(data), "word outside decoded resource"
    return int.from_bytes(data[at:at + 2], "little")


# Exact native common-stream algorithm: initial pointer is two bytes BEFORE
# the duration, not the first duration. A negative duration jumps to a loop
# target; a negative VALUE is a signed velocity, not an end marker.
def stream(common, pointer, ticks):
    if pointer == 0:
        return [0] * ticks
    at = pointer - 0x6000
    counter = 0
    values = []
    for _ in range(ticks):
        counter -= 1
        if counter < 0:
            at += 2
            duration = u(common, at)
            if duration & 0x8000:
                at = u(common, at + 2) - 0x6000
                duration = u(common, at)
            assert duration < 0x8000
            counter = duration
            at += 2
        value = u(common, at)
        values.append(value if value < 0x8000 else value - 0x10000)
    return values


def verify(common, body, wram, rows):
    assert len(wram) == 0x20000, "expected full native WRAM image"
    assert len(common) == 0x1a0c and common == wram[0x16000:0x17a0c], "complete common resource differs"
    assert len(body) == 0x28b, "unexpected descriptor resource length"
    assert wram[0x44b:0x44e] == bytes.fromhex("37 f0 ab"), "cached common source differs"
    for selector, pointers, cycle in [
        (0x60, (0x6d18, 0), (1, 0)),
        (0x68, (0, 0x6fc8), (1, 0)),
        (0x69, (0, 0x6fd4), (-1, 0)),
    ]:
        assert tuple(u(common, selector * 4 + axis * 2) for axis in range(2)) == pointers
        axis = 0 if selector == 0x60 else 1
        assert stream(common, pointers[axis], 32) == list(cycle) * 16

    # Descriptor's ordinary display lists, independent of the velocity stream.
    for selector in range(6):
        at = u(body, selector * 2)
        durations = []
        while u(body, at) < 0x8000:
            durations.append(body[at])
            at += 4
        assert durations == ([0] if selector < 3 else [7] * 4)

    assert len(rows) == 401
    assert [t["sample"] for t in rows] == list(range(401))
    assert all(t['map'] == 0xd and t['class'] == 0 for t in rows)
    assert [t['frame'] for t in rows] == list(range(8175, 8576))
    frames = {t['frame']: t for t in rows}
    # Complete ordinary movements: down, up, right. Assertions are on actual
    # post-run_frame positions, not only velocity pointers or display durations.
    for first, last, selector, delta in [(8195, 8226, 3, (0, 16)), (8371, 8402, 4, (0, -16)), (8403, 8434, 5, (16, 0))]:
        prior = frames[first - 1]['e'][:2]
        expected = [[0, 0] for _ in range(32)]
        movement_selector = {3: 0x68, 4: 0x69, 5: 0x60}[selector]
        for axis in range(2):
            pointer = u(common, movement_selector * 4 + axis * 2)
            for i, value in enumerate(stream(common, pointer, 32)):
                expected[i][axis] = value
        actual = []
        for i, frame in enumerate(range(first, last + 1)):
            t = frames[frame]
            e, aux = t['e'], t['aux']
            actual.append([e[axis] - prior[axis] for axis in range(2)])
            prior = e[:2]
            assert e[7] == 7 - i % 8
            assert e[16] == i // 8 + 1 and e[17] == 1
            assert aux[4] == selector
            assert e[20:22] == [0, 0]
            assert aux[12:14] == [0, 0]  # applied and cleared, not stuck accumulators
        assert actual == expected
        assert tuple(sum(v[axis] for v in actual) for axis in range(2)) == delta

    # One full class-0 refusal/idle interval: a zero-duration single-record list
    # repeated sixteen times, consuming one repeat every native frame.
    for i, frame in enumerate(range(8227, 8243)):
        e, aux = frames[frame]['e'], frames[frame]['aux']
        assert e[:2] == [72, 688]
        assert e[7] == 0 and e[16] == 1 and e[17] == 16 - i
        assert aux[4] == 0 and aux[8:12] == [0, 0, 0, 0]
    assert frames[8243]['e'][17] == 16



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rom", required=True, type=Path)
    parser.add_argument("--decoder", required=True, type=Path, help="built crysta-cadence-decode binary")
    parser.add_argument("--wram", required=True, type=Path, help="native settledD.wram")
    parser.add_argument("--samples", required=True, type=Path, help="probe stdout JSONL")
    args = parser.parse_args()
    # Authentication and source binding happen in Rust BEFORE decoding; never
    # trust stale resource binaries supplied separately from the current ROM.
    decoded = json.loads(subprocess.check_output([str(args.decoder.resolve()), str(args.rom)]))
    rows = [json.loads(line) for line in args.samples.read_text().splitlines()]
    rows = [row for row in rows if "sample" in row]
    verify(bytes(decoded["common"]), bytes(decoded["body"]), args.wram.read_bytes(), rows)
    print("PASS: authenticated JP ROM; all 6668 common-resource bytes == native WRAM; "
          "three exact 32-frame translations; 16-frame zero-record idle/refusal.")


if __name__ == "__main__":
    main()
