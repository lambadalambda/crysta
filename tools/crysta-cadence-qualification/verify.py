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


def display_lists(body):
    """Durations of the six ordinary display lists, independent of velocity."""
    lists = []
    for selector in range(6):
        at = u(body, selector * 2)
        durations = []
        while u(body, at) < 0x8000:
            durations.append(body[at])
            at += 4
        lists.append(durations)
    return lists


# Walking selectors 3/4/5 use class-0 row movement $68/$69/$60 (down/up/X).
MOVEMENT = {3: 0x68, 4: 0x69, 5: 0x60}


def check_step(frames, common, first, selector, delta, count=32):
    """One 32-frame list repetition of an ordinary walking action, or its
    first `count` frames at the end of a window.

    Assertions are on actual post-run_frame positions, not only velocity
    pointers or display durations."""
    prior = frames[first - 1]['e'][:2]
    expected = [[0, 0] for _ in range(count)]
    for axis in range(2):
        pointer = u(common, MOVEMENT[selector] * 4 + axis * 2)
        for i, value in enumerate(stream(common, pointer, count)):
            expected[i][axis] = value
    actual = []
    for i, frame in enumerate(range(first, first + count)):
        e, aux = frames[frame]['e'], frames[frame]['aux']
        actual.append([e[axis] - prior[axis] for axis in range(2)])
        prior = e[:2]
        assert e[7] == 7 - i % 8
        assert e[16] == i // 8 + 1 and e[17] == 1
        assert aux[4] == selector
        assert e[20:22] == [0, 0]
        assert aux[12:14] == [0, 0]  # applied and cleared, not stuck accumulators
    assert actual == expected
    if count == 32:
        assert tuple(sum(v[axis] for v in actual) for axis in range(2)) == delta


def check_rows(rows, first, count, map_id, klass):
    assert len(rows) == count + 1
    assert [t["sample"] for t in rows] == list(range(count + 1))
    assert all(t['map'] == map_id and t['class'] == klass for t in rows)
    assert [t['frame'] for t in rows] == list(range(first, first + count + 1))
    return {t['frame']: t for t in rows}


def check_common(common, wram):
    assert len(wram) == 0x20000, "expected full native WRAM image"
    assert len(common) == 0x1a0c and common == wram[0x16000:0x17a0c], "complete common resource differs"
    assert wram[0x44b:0x44e] == bytes.fromhex("37 f0 ab"), "cached common source differs"


def verify(common, body, wram, rows):
    check_common(common, wram)
    assert len(body) == 0x28b, "unexpected descriptor resource length"
    for selector, pointers, cycle in [
        (0x60, (0x6d18, 0), (1, 0)),
        (0x68, (0, 0x6fc8), (1, 0)),
        (0x69, (0, 0x6fd4), (-1, 0)),
    ]:
        assert tuple(u(common, selector * 4 + axis * 2) for axis in range(2)) == pointers
        axis = 0 if selector == 0x60 else 1
        assert stream(common, pointers[axis], 32) == list(cycle) * 16

    assert display_lists(body) == [[0]] * 3 + [[7] * 4] * 3

    frames = check_rows(rows, 8175, 400, 0xd, 0)
    # Complete ordinary movements: down, up, right.
    for first, selector, delta in [(8195, 3, (0, 16)), (8371, 4, (0, -16)), (8403, 5, (16, 0))]:
        check_step(frames, common, first, selector, delta)

    # One full class-0 refusal/idle interval: a zero-duration single-record list
    # repeated sixteen times, consuming one repeat every native frame.
    for i, frame in enumerate(range(8227, 8243)):
        e, aux = frames[frame]['e'], frames[frame]['aux']
        assert e[:2] == [72, 688]
        assert e[7] == 0 and e[16] == 1 and e[17] == 16 - i
        assert aux[4] == 0 and aux[8:12] == [0, 0, 0, 0]
    assert frames[8243]['e'][17] == 16


# Slot $1300's actions in the sampled window: (first frame, kind, selector,
# delta). Each is followed by one COP03 loop frame.
TOWN_ACTIONS = [
    (11409, "step", 5, (16, 0)),
    (11442, "idle", 2, None),
    (11459, "step", 3, (0, 16)),
    (11492, "idle", 0, None),
    (11509, "step", 4, (0, -16)),
    (11542, "step", 5, (16, 0)),
]


def check_idle(frames, first, selector, begin=0):
    """Two duration-7 records, one repetition: frames `begin..16` of an idle."""
    # At the window's start there is no earlier frame to hold position from.
    position = frames[first - (begin == 0)]['e'][:2]
    for i, frame in enumerate(range(first, first + 16 - begin), begin):
        e, aux = frames[frame]['e'], frames[frame]['aux']
        assert e[:2] == position
        assert e[7] == 7 - i % 8 and e[16] == i // 8 + 1 and e[17] == 1
        assert aux[4] == selector and aux[8:14] == [0] * 6 and e[20:22] == [0, 0]


def verify_town(common, body, wram, rows):
    """Town slot $1300, record $83:8A2D: class 2 (descriptor mode $22 & $0F).

    Class 2 uses the class-0 movement row and $80:8FB5's class-and-3 row 2,
    one idle repetition. Its packet's idle lists are two duration-7 records,
    so an idle also lasts 16 frames, for a different reason than class 0.
    The script closes its loop with COP03, which costs one frame per action
    with the repetition count at zero; that frame is not part of the action."""
    check_common(common, wram)
    assert len(body) == 0x71d, "unexpected class-2 packet length"
    assert display_lists(body) == [[7] * 2] * 3 + [[7] * 4] * 3
    frames = check_rows(rows, 11394, 200, 0xa, 2)
    # The window opens two frames into a facing-right idle.
    check_idle(frames, 11394, 2, begin=2)
    loops = [11408]
    for first, kind, selector, delta in TOWN_ACTIONS:
        if kind == "step":
            check_step(frames, common, first, selector, delta)
        else:
            check_idle(frames, first, selector)
        loops.append(first + (32 if kind == "step" else 16))
    check_step(frames, common, 11575, 5, None, count=20)
    for loop in loops:
        e, aux = frames[loop]['e'], frames[loop]['aux']
        assert e[:2] == frames[loop - 1]['e'][:2]
        assert e[7] == 0 and e[16] == 0 and e[17] == 0 and aux[8:10] == [0, 0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rom", required=True, type=Path)
    parser.add_argument("--decoder", required=True, type=Path, help="built crysta-cadence-decode binary")
    parser.add_argument("--wram", required=True, type=Path,
                        help="native settledD.wram, or settleTown-open.wram with --town")
    parser.add_argument("--town", action="store_true", help="verify the class-2 town slot")
    parser.add_argument("--samples", required=True, type=Path, help="probe stdout JSONL")
    args = parser.parse_args()
    # Authentication and source binding happen in Rust BEFORE decoding; never
    # trust stale resource binaries supplied separately from the current ROM.
    decoded = json.loads(subprocess.check_output([str(args.decoder.resolve()), str(args.rom)]))
    rows = [json.loads(line) for line in args.samples.read_text().splitlines()]
    rows = [row for row in rows if "sample" in row]
    if args.town:
        verify_town(bytes(decoded["common"]), bytes(decoded["town_body"]),
                    args.wram.read_bytes(), rows)
        print("PASS: authenticated JP ROM; all 6668 common-resource bytes == native WRAM; "
              "class-2 town walker: every frame 11394..11594, four exact 32-frame "
              "translations, two 16-frame two-record idles, seven one-frame loop gaps.")
        return
    verify(bytes(decoded["common"]), bytes(decoded["body"]), args.wram.read_bytes(), rows)
    print("PASS: authenticated JP ROM; all 6668 common-resource bytes == native WRAM; "
          "three exact 32-frame translations; 16-frame zero-record idle/refusal.")


if __name__ == "__main__":
    main()
