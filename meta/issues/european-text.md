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
  admits the 12-pixel advance and clips its four-pixel cell overhang. A fresh
  empty-SRAM native child now replays the full input fixtures to the `$0D`
  doorway Elder at `(120,704)`, with `$21` set and `$296` clear. It matches
  13,568 indexed pixels on the 216×64 page containing that edge glyph
  (excluding only its source-positioned 16×16 D5 prompt), then 13,696 pixels
  in each of two unanswered mission-choice states (excluding only the selected
  8×16 cursor source cell). Both decoded option rows contain ink outside the
  cursor columns. This is native indexed-raster evidence, not prompt/cursor art,
  uploaded attributes or composed RGB.
- The first-bedroom native witness still matches all 4,992 pixels in the first
  page's 26 glyph cells and all 4,864 indices in its 76 perimeter frame tiles.
  It now also matches eight stable native CGRAM entries (1, 2, 4, 5, 6, 8, 9
  and 10) to the ROM-derived window/speaker colours. Entry 0 is untested, and
  the three colour-3 slots rewritten by HDMA are excluded. At the correctly
  mapped PAL framebuffer rows (`logical y + 29`), 3,401 bounded non-shade
  candidates remain unchanged across two settled native frames; 4,512
  shade-index candidates are excluded. That latter check is only native
  framebuffer self-consistency—it does not attribute composed RGB to
  BG3/CGRAM or establish portable RGB/HDMA parity.
- The first room-Elder choice witness still matches 14,208 indexed page pixels
  at each selection. It now samples both visible and hidden BG3 cursor states
  for both choices, compares their indexed cursor/interior art with ROM-decoded
  `WindowArt`, and verifies all 224 words in the uploaded 224×64 BG3 content
  region equal the WRAM staging words, including tile and attribute bits. This
  proves the complete staged words were uploaded; it does not independently
  prove every staged attribute, exact blink cadence or composed video.
- A separate natural-input witness reaches map `$0A` and checks one settled
  source-motion phase of the six-letter **Crysta** area title: six current 16×16
  OAM pieces, uploaded OBJ glyph pixels, and palette-2 selection plus its two
  used colours (entries 1 and 2) agree with the European ROM. It does not claim
  framebuffer drawing or animation cadence. Representative English titles and
  item names—including dictionary-expanded
  “Crystal Thread”—already have source-decoding/portable-art coverage. Native
  item-name presentation remains unwitnessed because item `$32` requires a
  separate naturally played shop route outside this bedroom-to-world-map slice.
