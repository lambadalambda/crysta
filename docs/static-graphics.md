# Static cavern graphics and full-map rendering

The portal cavern **map `$0128`** can now be drawn entirely from the authenticated
Japanese ROM: **1280×512 pixels, 80×32 cells**, with no SRAM, oracle boot or
framebuffer samples. This is a first-background asset inspection view, **not a
complete scene renderer**. Other map IDs are rejected by this rendering command.

## Generate and inspect

```sh
cargo run -p map-inspector -- render-map \
  'local/Tenchi Souzou (Japan).sfc' 128
```

IDs are hexadecimal. Open the printed `local/static-maps/run-PID-SEQ/index.html`
file directly in a browser. Keep `map.bmp` beside it. No server, network, external
fonts, JavaScript packages or uploads are required. The tool still builds with
native oracle/C++ prerequisites because it is the same multi-mode binary, but
`render-map` never constructs an emulator session or reads SRAM.

The page offers zoom (25–400%), scrolling, a 16-pixel grid, click/keyboard cell
selection, an enlarged metatile preview, and each quadrant's raw SNES tile word,
graphics index, palette bank, priority and flips. Home/End select row edges;
Ctrl+Home/End select map corners. At the tested 1600-pixel desktop width, 75% zoom shows the whole map.

Each fresh output directory contains:

- `map.bmp`: natural/full-brightness BGR555 colors, with a neutral 8-pixel checker
  for transparent pixels; this image is a convenience view, not canonical data;
- `indices.bin`: row-major 1280×512 palette indices; zero means **transparent**,
  not an opaque CGRAM-zero backdrop (opaque 4bpp samples never have color zero);
- `priority.bin`: one byte per pixel, 0/1; zero for transparent samples;
- `map.json`: raw cells, four-word definitions, all 128 raw palette words,
  resource source ranges/hashes, pixel-plane hashes and explicit limitations;
- `index.html`: dependency-free viewer with the same embedded metadata.

All outputs contain ROM-derived material: **keep them under ignored `local/`;
do not commit or redistribute them**. The ROM remains unchanged. Input identity
is the Japanese SHA-256 in [reference dumps](../README.md).

## Pure model and supported recipe

`assets::graphics` supplies exact-size SNES 4bpp planar tile decoding, raw BGR555
colors, raw background tile words and indexed metatile sampling. Plane 0/1 bytes
interleave in the first 16 bytes of each 32-byte tile; planes 2/3 interleave in
the second 16. Bit 7 is leftmost. RGB conversion expands each five-bit channel
with `(v << 3) | (v >> 2)`; unused color bit 15 is preserved in the raw word.

Background tile words retain:

| Bits | Field |
|---|---|
| 0–9 | Graphics tile index |
| 10–12 | 16-color palette bank |
| 13 | Priority |
| 14 / 15 | Horizontal / vertical flip within the 8×8 tile |

Color zero is explicitly transparent, independent of palette selection. Priority
is metadata, not flattened into a guessed layer order. `CavernBackground` in
`assets::maps::visual` owns the game-specific recipe and exposes raw resources,
layer, tiles, definitions, palette and a bounded map-relative pixel sampler.
Authentication and local file/image export remain outside `assets`.

The recipe resolves map `$0128` through the [loading-script projection](map-scripts.md),
then requires exactly the seven ordered resource command shapes below. Packed
pointer bytes are resolved, not compared to hard-coded offsets. Unknown modes,
extra/reordered resources, unsupported high cell bits, nonzero definition bit 9,
truncated/bank-crossing resources and incorrect packet sizes fail explicitly.
This conservative specialization is intentional; it does not pretend to execute
all cached/partial transfers.

| Kind | Operands excluding packed pointer | Normalized source extent | Meaning |
|---|---|---|---|
| Graphics `$80` | `00 20 01`, suffix `00 00` | `$1C45FE..$1C77B5` | Compressed 16,384 bytes → 512 4bpp tiles, VRAM base zero |
| Palette `$40` | `00 60 20` | `$2B5426..$2B54E6` | Raw 96 colors → palette entries 32–127 |
| Definitions `$20` | `00 40 00 01` | `$2076CD..$207FFC` | Compressed 4,096 bytes → 512 four-word definitions |
| Attributes `$20` | `00 08 00 81` | `$2B439E..$2B4462` | Compressed 512-byte attribute table, retained but not used as graphics |
| Layer `$10` | `01` | `$090000..$0905F8` | Dimension header and compressed 80×32 cells |
| Shared graphics `$80` | `00 08 00`, suffix `70 00` | starts `$299000` | Cached/skipped in qualified load; **not extracted or rendered** |
| Shared palette `$40` | `00 20 00` | `$328B78..$328BB8` | Raw 32 colors → palette entries 0–31 |

Ranges are half-open. Original packet encodings and raw palette bytes are
preserved, rather than reconstructed by re-encoding.

## Loader evidence

The existing local cavern loader trace and ROM disassembly establish:

- Graphics handler `$86:876E`: normal-path operands are source start/end in
  `$200`-byte units, **not start/length**. `$86:88B5..88BC` writes the destination
  VRAM **word** address from `(destination & $70) << 8`. Buffered decompression
  at `$86:84E1` uses the WRAM `$5000..$7000` ring and DMA; the entire graphics
  payload is not expected to survive contiguously in WRAM.
- Palette handler `$86:8903`: start/end/destination operands are color indices.
  They are doubled and copied raw to staging base `$7F:0600`. Common helper
  `$86:927E` copies `end - start` bytes from `source + start` to `base + destination`.
- Definition handler `$86:8930`: start/end/destination use `$40`-byte units;
  the source-start high bit can bypass decompression. Flag `$80` selects the
  separate byte-attribute table. The cavern definition command decompresses to
  scratch then copies 4 KiB to `$7E:2000`.
- Post-copy `$86:92AC` traverses eight-byte records, adjusting each of four words
  as `(word & $FDFF) | adjustment`. The cavern has zero bit 9 and zero adjustment
  (`$7E:0846`), so source definitions and final definitions are identical here.
  The static recipe rejects rather than guesses nonzero adjustment behavior.
- Each cavern cell's low nine bits index a definition; offsets `0,2,4,6` are
  **TL, TR, BL, BR**. This ordering is independently checked against the actual
  circular BG1 tilemap, not inferred merely from the adjustment loop order.

This adds graphics meaning for the **qualified cavern**; it does not establish
universal map-cell graphics or collision semantics. Attribute initialization and
the later isolated cell `$8000` bit change remain described in [static maps](static-maps.md).

## Runtime equality and pixel comparison

```sh
cargo run -p map-inspector -- qualify-loader \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

The existing menu + cavern experiment retains all original loader stops. Its new
`visual` report checks the same final **frame 1601**, camera `(648,0)`:

| Check | Equality |
|---|---|
| Decoded graphics vs VRAM bytes `$0000..$4000` | all 16,384 bytes |
| Decoded definitions vs WRAM `$7E:2000..$7E:3000` | all 4,096 bytes |
| Cavern palette vs CGRAM bytes `$40..$100` | all 192 bytes |
| Definition expansion vs BG1 VRAM tilemap | 672 words (28×24 8-pixel tiles) |
| Static opaque patch vs original framebuffer | all 256 pixels, after the qualification-only effect profile |

BG1SC mirror `$7E:046D = $38` describes a 32×32 circular tilemap at VRAM **word**
`$3800`. World 8-pixel tile columns `[82,110)` and rows `[2,26)` map to
`$3800 + (y % 32) * 32 + x % 32`; every expected quadrant word matches.

The opaque, sprite-free pixel patch is world rectangle `(656,16,16,16)`:
viewport `(8,16)`, with the capture's eight-row top border and doubled horizontal
samples accounted for. The reference has a color effect; comparing its RGB
directly to natural ROM colors would be wrong. Qualification checks effect
mirrors `$046A..046D = 80 A1 00 38`, `$0471..0473 = CF 00 00`, and applies
saturating subtraction of **15 from green and blue**, followed by ares's default
**Deep Black Boost** ramp at full brightness. The register-copy path is visible
at `$87:88FD..$87:8941`; the ramp is from the licensed vendored
`vendor/ares/ares/sfc/ppu-performance/color.cpp`. This is a fixed-checkpoint
comparison, not a portable color-math implementation or a general PPU-state API.

The shared palette's first word changes at runtime (`$28CD → $0023`). We preserve
the ROM value in the static model; transparent pixels are not composited against
it. The remaining shared words agree at this checkpoint, but the formal palette
equality check above deliberately covers only the 96 cavern colors. No used
cavern definition references a nontransparent pixel from palette banks 0/1.

Key decoded SHA-256 values:

| Data | SHA-256 |
|---|---|
| Graphics | `06c37bb92a0571049642e701af19aba23739f5fb129f5b36fba5dcc9ef7cfef8` |
| Definitions | `81ea75bfce96d0a1dbc7fcff904a56baeaae023f11d38c39710072a4972abe1f` |
| Cavern palette | `0820e01ccbfa952c28f54317033a331db2fffe3e54a2bb07614b32c3b0d675ec` |
| Full indexed map | `bb590b388811f2471232fc58e6a081455bd701540d4cf7a962c8b1e71827eb68` |
| Effect-matched 16×16 RGB patch | `4ca438531618fd4c5d43b3d01796f26fb6ac1aed3b6f2c112df8028e07d5f043` |

The full indexed map contains 25,281 transparent pixels. These hashes are
qualification metadata, not embedded graphics fixtures.

## Verification and remaining scope

Synthetic red–green tests cover all planar bits, palette conversion, independent
word fields/flips, quadrant placement, transparency/priority, resource framing,
size/bank/input bounds, duplicate loads, unsupported adjustments and high cell
bits. Authenticated local tests check source extents/hashes, the complete indexed
map, CLI exports and fresh-process oracle equality. They skip explicitly without
local inputs. Browser regression covers real full-map loading, cell inspection,
zoom, grid, keyboard/click selection, preview and desktop/mobile layouts:

```sh
agent-browser --session static-cavern open 'file:///absolute/path/to/index.html'
agent-browser --session static-cavern wait --fn \
  "document.getElementById('atlas').dataset.ready === 'true'"
agent-browser --session static-cavern eval --stdin \
  < crates/map-inspector/tests/static-viewer-check.js
agent-browser --session static-cavern set viewport 390 844
agent-browser --session static-cavern eval --stdin \
  < crates/map-inspector/tests/static-viewer-check.js
agent-browser --session static-cavern close
```

**Not qualified:** other rendered map IDs, final scene composition, animated
palettes/graphics, shared graphics caching, sprites, windows, brightness/raster
behavior, gameplay events, transitions and collision. The moving frame 1841 also
showed a three-pixel difference between the captured background scroll and the
sampled camera metadata during exploratory comparison; it is not used for this
pixel-equality fixture. No camera correction was silently added to the viewer.
The [graphics](../meta/issues/decode-graphics-animation.md) and
[map-format](../meta/issues/decode-map-collision-formats.md) parent issues remain open.
