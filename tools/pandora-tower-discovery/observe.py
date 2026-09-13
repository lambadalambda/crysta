#!/usr/bin/env python3
"""Read semantic observations out of a probe WRAM capture.

Discovery-only helper for the tower-approach route. It reads captures; it never
initializes production state from them, and it decodes no ROM content.

The probe's own JSONL observation only reports event flags below $200, so the
tour-completion flags ($243/$244) and the pot flag ($292) are invisible there.
This reads the full block instead.
"""

from __future__ import annotations

import sys

# Same addresses the accepted checker uses; keep in sync with
# tools/pandora-qualification/check.py if either moves.
EVENT_BLOCK = 0x6C0
EVENT_COUNT = 0x400
MAP = 0x47E
POSITION = 0x1000
FACING = 0x1014
CONTROL = 0x980

# The accepted endpoint set recorded in docs/pandora-progression.md.
ENDPOINT_FLAGS = frozenset(
    {0x20, 0x22, 0x26, 0x27, 0x28, 0x2E, 0xFB, 0x243, 0x244, 0x292}
)
MAP_HALL = 0x41


def word(wram: bytes, address: int) -> int:
    return int.from_bytes(wram[address : address + 2], "little")


def flags(wram: bytes) -> frozenset[int]:
    return frozenset(
        i for i in range(EVENT_COUNT) if wram[EVENT_BLOCK + i // 8] >> (i % 8) & 1
    )


def observe(wram: bytes) -> dict:
    return {
        "map": word(wram, MAP),
        "position": [word(wram, POSITION), word(wram, POSITION + 2)],
        "facing": word(wram, FACING),
        "control": word(wram, CONTROL),
        "flags": sorted(flags(wram)),
    }


def progressed(wram: bytes) -> dict:
    """Everything that would mean the continuation fired."""
    seen = flags(wram)
    return {
        "new_flags": sorted(seen - ENDPOINT_FLAGS),
        "lost_flags": sorted(ENDPOINT_FLAGS - seen),
        "left_hall": word(wram, MAP) != MAP_HALL,
    }


def main() -> int:
    for path in sys.argv[1:]:
        with open(path, "rb") as handle:
            wram = handle.read()
        state = observe(wram)
        change = progressed(wram)
        mark = (
            " *** PROGRESSED ***"
            if change["new_flags"] or change["lost_flags"] or change["left_hall"]
            else ""
        )
        print(
            f"{path}: map={state['map']} pos={state['position']} "
            f"facing={state['facing']} control={state['control']}{mark}"
        )
        if mark:
            print(f"  {change}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
