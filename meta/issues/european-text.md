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
- A fresh-child empty-SRAM European text test now waits at the first bedroom
  page's native `$D5`, verifies the `$04C4`/224×64 content anchor, and matches
  its live WRAM tilemap/VRAM glyph cells against all 4,992 pixels of the
  decoded first page's 26 glyph cells. The same snapshot compares all 76
  perimeter frame tiles (4,864 2bpp indices) against the ROM-decoded window
  art. This tests one fully typed page's indexed content and WRAM-staged
  frame layout, not uploaded BG3 tilemap attributes, native RGB/HDMA, prompt
  art or the other slice windows, choices, titles and item names.
- A second empty-SRAM European native child now replays the first 53 rows of
  the existing input-only tour fixture and stops at the **unanswered first
  room Elder choice** in `$0B`, with `$26` set and the European `$04C4`
  window anchor. It compares the retained `$88:91C6` page to the live
  WRAM-staged tilemap and 2bpp VRAM at both selection states: 14,208
  matching indexed pixels per state, excluding only the selected cursor's
  source-positioned 8×16 cell. Both option labels contain foreground ink;
  a Down edge changes `$0DCE` from 0 to 1 while `$0DC2` stays unanswered.
  Catalog 2 uses cursor-source y32/48, not the standard choice y24/40.
  See `crates/map-inspector/tests/local_european_choice_native.rs`. This
  extends native indexed-page evidence to one choice, **not** native cursor
  artwork/animation, BG3 attributes, RGB/HDMA, the later 216-pixel mission
  window, or all slice text.
