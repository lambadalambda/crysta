# Decode the European text engine, font and windows

## Summary

The English release has its own text encoding, font and window layout.
Decode them into the same pages, choices and labels the slice uses.

## Dependencies

- [Map the European ROM to the Japanese one](european-address-map.md)

## Acceptance Criteria

- The slice's dialogue, choices, titles and item names decode from the
  European ROM and draw as its native frames.

## Progress

- The frozen-return Elder's English request at `$88:8D6C` uses a 216-pixel
  custom content window whose last glyph starts at x204. The shared decoder
  now admits the 12-pixel advance and clips its four-pixel cell overhang in
  the portable bitmap; the dialogue proceeds to mission `$296`. This does
  not establish a native raster match for that window or complete the
  remaining text/timing acceptance.
