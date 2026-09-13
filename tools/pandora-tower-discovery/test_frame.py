#!/usr/bin/env python3
"""Unit tests for the framebuffer reader. No ROM, capture or Pillow needed."""

from __future__ import annotations

import struct
import unittest
import zlib

import frame


def capture(pixel):
    """A full-size XRGB buffer whose every pixel is `pixel` = (r, g, b)."""
    red, green, blue = pixel
    return bytes([blue, green, red, 0]) * (frame.WIDTH * frame.SOURCE_ROWS)


def columns(pixels):
    """A buffer where each doubled column pair carries its own colour."""
    buffer = bytearray()
    for _ in range(frame.SOURCE_ROWS):
        for x in range(frame.WIDTH):
            buffer += bytes([x // 2 % 256, 0, 0, 0])  # blue channel = source column
    return bytes(buffer)


class TestRows(unittest.TestCase):
    def test_shape_is_the_visible_snes_frame(self):
        image = frame.rows(capture((1, 2, 3)))
        self.assertEqual(len(image), frame.VISIBLE_ROWS)
        self.assertEqual(len(image[0]), frame.OUT_WIDTH)

    def test_channels_are_unswapped(self):
        # Source is little-endian XRGB, i.e. B,G,R,X; a red pixel must stay red.
        self.assertEqual(frame.rows(capture((200, 10, 20)))[0][0], (200, 10, 20))

    def test_selects_even_source_columns(self):
        image = frame.rows(columns(None))
        # Column n of the output must come from source column 2n.
        self.assertEqual([pixel[2] for pixel in image[0][:4]], [0, 1, 2, 3])

    def test_rejects_a_wrong_sized_capture(self):
        with self.assertRaises(ValueError):
            frame.rows(b"\x00" * 16)

    def test_grid_only_tints_grid_lines(self):
        plain = frame.rows(capture((0, 0, 0)))
        grid = frame.rows(capture((0, 0, 0)), grid=True)
        self.assertNotEqual(plain[0][0], grid[0][0])  # on a 64 line
        self.assertEqual(plain[5][5], grid[5][5])  # off any line
        self.assertEqual(grid[0][0], (127, 0, 0))  # 64 lines are red
        self.assertEqual(grid[32][1], (0, 127, 0))  # 32 lines are green


class TestEncode(unittest.TestCase):
    def test_writes_a_decodable_png(self):
        data = frame.encode(frame.rows(capture((10, 20, 30))))
        self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
        width, height = struct.unpack(">II", data[16:24])
        self.assertEqual((width, height), (frame.OUT_WIDTH, frame.VISIBLE_ROWS))
        start = data.index(b"IDAT") + 4
        end = data.index(b"IEND") - 8
        raw = zlib.decompress(data[start:end])
        # Every scanline carries a leading filter byte.
        self.assertEqual(len(raw), frame.VISIBLE_ROWS * (1 + frame.OUT_WIDTH * 3))
        self.assertEqual(raw[0], 0)
        self.assertEqual(tuple(raw[1:4]), (10, 20, 30))


class TestDestination(unittest.TestCase):
    def test_rejects_output_outside_local(self):
        for path in ("shot.png", "/tmp/shot.png", "local/../shot.png"):
            with self.assertRaises(ValueError):
                frame.checked_destination(path)

    def test_accepts_output_under_local(self):
        self.assertEqual(str(frame.checked_destination("local/shot.png")), "local/shot.png")

    def test_missing_arguments_do_not_traceback(self):
        self.assertEqual(frame.main([]), 2)


if __name__ == "__main__":
    unittest.main()
