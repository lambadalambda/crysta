#!/usr/bin/env python3
"""Write a probe framebuffer capture to a PNG, optionally with a coordinate grid.

usage: frame.py CAPTURE.pixels local/OUT.png [--grid]

The oracle framebuffer is 512x480 XRGB8888, but this scene is rendered at SNES
256x224 with horizontal doubling only: columns are duplicated, rows are not.
Selecting every second column and keeping rows is the same convention as
`tools/new-game-qualification/screenshots.py`; that tool batches a directory
through Pillow, while this one writes a single annotated frame with no
dependency. With the camera at the origin, as it is throughout the hall, player
coordinates then land directly on image coordinates, which is what makes a
capture navigable. The player anchor is the sprite's bottom-right: its body
occupies roughly (x-16..x, y-24..y).

Output stays under local/: these are game rasters and must not be committed.
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

WIDTH = 512
SOURCE_ROWS = 480
VISIBLE_ROWS = 240
OUT_WIDTH = 256


def rows(pixels: bytes, grid: bool = False) -> list[list[tuple[int, int, int]]]:
    if len(pixels) != WIDTH * SOURCE_ROWS * 4:
        raise ValueError(f"expected {WIDTH}x{SOURCE_ROWS} XRGB, got {len(pixels)} bytes")
    out = []
    for y in range(VISIBLE_ROWS):
        row = []
        for x in range(0, WIDTH, 2):
            offset = (y * WIDTH + x) * 4
            blue, green, red, _ = pixels[offset : offset + 4]
            column = x // 2
            if grid and (column % 32 == 0 or y % 32 == 0):
                tint = (255, 0, 0) if (column % 64 == 0 or y % 64 == 0) else (0, 255, 0)
                red, green, blue = (
                    (value + tint[i]) // 2 for i, value in enumerate((red, green, blue))
                )
            row.append((red, green, blue))
        out.append(row)
    return out


def encode(image: list[list[tuple[int, int, int]]]) -> bytes:
    raw = b"".join(
        bytes([0]) + b"".join(bytes(pixel) for pixel in row) for row in image
    )

    def chunk(tag: bytes, data: bytes) -> bytes:
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", OUT_WIDTH, len(image), 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def checked_destination(destination: str) -> Path:
    path = Path(destination)
    if not path.resolve().is_relative_to(Path("local").resolve()):
        raise ValueError("rasters must stay under local/")
    return path


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__.splitlines()[2], file=sys.stderr)
        return 2
    source, destination = argv[0], checked_destination(argv[1])
    grid = "--grid" in argv[2:]
    with open(source, "rb") as handle:
        image = rows(handle.read(), grid=grid)
    with open(destination, "wb") as handle:
        handle.write(encode(image))
    print(f"wrote {destination}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
