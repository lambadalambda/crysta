# Loaded-map inspection

The first map deliverable is a **qualified runtime snapshot inspector**, not a
complete static ROM map decoder. It combines the original captured viewport with
a full raw tile/collision layer. Original graphics are shown only in the captured
viewport; the rest of the map uses structural colors, not reconstructed terrain.
The parent [map-format issue](../meta/issues/decode-map-collision-formats.md)
remains open. A subsequent [static-layer qualification](static-maps.md) now
decodes this cavern directly from ROM and reproduces its attribute initialization;
the viewer described here continues to display runtime captures. The new
[ROM-only static graphics viewer](static-graphics.md) separately draws the full
cavern from decoded assets and qualifies its metatile indexing. Historical
candidate labels in this runtime viewer are unchanged; collision is still open.

## Generate a local viewer

From the repository root, with the oracle's C++ build prerequisites installed:

```sh
cargo run -p map-inspector -- capture \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

Open the printed `local/maps/run-PID-SEQ/index.html` path in a browser. No web
server, external libraries, network requests, or uploads are needed. Keep the
BMP files next to the HTML. All generated files stay under ignored `local/` and
must not be committed or redistributed.

This command authenticates the Japanese ROM against the project's reference
SHA-256 and requires the existing qualified three-slot SRAM, SHA-256
`709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
The save is not distributed. Arbitrary saves and other revisions are rejected;
this is deliberately a reproducible research scenario, not a general save loader.

The viewer offers:

- before/after checkpoints, original viewport, grid, collision and player overlays;
- full runtime layer colored by collision code or **candidate** metatile index;
- 25–200% zoom, scrolling, locate-player, click and arrow-key cell inspection;
- raw word, masked fields, WRAM address, other-checkpoint word, and provenance;
- changed-cell highlighting (the qualified pair has **zero changed cells**).

Each new run directory contains `capture.json`, `index.html`, two 256×240
`reference-*.bmp` images, and raw WRAM/VRAM/CGRAM snapshots for both checkpoints.
JSON schema version 1 records source hashes, scenario, label and actual frame,
map dimensions in **cells**, camera/player positions in **pixels**, row-major raw
words, and SHA-256 hashes. VRAM and CGRAM words are serialized little-endian.
`verify` in place of `capture` emits the same JSON to stdout without writing files.

## Qualified scenario and evidence

The oracle boots with SRAM before power-on. For labels `0..=1840`, it holds Start
at `400..408`, A at `1100..1112`, and Right at `1800..1840` (half-open ranges).
A frame runs before each capture, so labels and core frame counters differ by one.

| Checkpoint | Label | Actual frame | Camera (pixels) | Player (pixels) |
| --- | ---: | ---: | --- | --- |
| Before movement | 1600 | 1601 | 648, 0 | 776, 112 |
| After 40 Right frames | 1840 | 1841 | 706, 16 | 834, 128 |

This Start/A-only replay selects default **save slot 3**, not slot 1 as older
metadata claimed. Actual slot 1 and its [Crysta doorway](opening-doorway.md)
are now separately qualified. The capture scenario ID is corrected to
`qualified-slot-3-right-movement`; checkpoint hashes and inputs are unchanged.

Both checkpoints are map **`$0128`, the portal cavern**, not Crysta town. The
layer is 1280×512 pixels, or 80×32 cells: 2560 words / 5120 bytes. Both copies hash
to `c3c7af3a0ef3c6c53e641b058ccaccad8a9b5ea42dca79c28e41c41ecaa450c5`.
Selected cell indices and words are 364=`$0E11`, 448=`$8007`, 608=`$0005`,
2559=`$1C39`. Movement changes player/camera and framebuffer, not these layer bytes.

RGB SHA-256 (row-major RGB8, not the BMP container):

- Before: `78c20d6a5dca2a006815c13f6577b33bcdc1ddb788ddad9949bdf889eaa7b0ea`
- After: `93a224b980b538bf5bbed26c7d7052a6c14777608e6d02162eb55cfb2ef74c8a`

The vendored ares `Screen::refreshPalette` produces ARGB8888. The oracle shim
preserves RGB channels and removes alpha; compile-time assertions cover red,
black/alpha removal, and mixed channels. The fixed 512×480 ABI is sampled at even columns in
the first 240 rows for this qualified noninterlaced, low-resolution scene.
The overlay assumes an **eight-row top border**: screen Y is map Y − camera Y + 8.
Placement on the atlas crops rows 8..232 (224 active rows). This is a
scene-qualified presentation assumption, not a general PPU geometry API.

## Runtime model

`assets::maps::LoadedMap` is a pure parser over a caller-owned full 128 KiB WRAM
image. The caller establishes ROM revision and checkpoint identity; structural
validation alone cannot prove that a map has finished loading.

| Address | Representation | Interpretation / qualification |
| --- | --- | --- |
| `$7E:047E` | u16 LE | Full runtime map ID |
| `$7E:0826`, `$7E:082A` | u16 LE | Pixel width/height; reader accepts nonzero multiples of 256 |
| `$7E:080E`, `$7E:0812` | u16 LE | Camera X/Y in pixels |
| `$7E:1000`, `$7E:1002` | u16 LE | Player X/Y in pixels |
| `$7E:A000` | row-major u16 LE | First runtime layer; 16×16-pixel cells |

Cell address is `$7E:A000 + 2 * (y * width_in_cells + x)`. The reader caps the
layer at `$4000` bytes, before the next imported buffer boundary `$7E:E000`;
this is a conservative supported-layout bound, not proof of all runtime buffers.
Raw words are preserved losslessly. `raw & $01FF` is a candidate metatile index,
not an SNES VRAM tile number. Initialization writes the metatile attribute into
bits 9..15, and the community overlay code `(raw >> 8) & $FE` is exactly twice
that value, so the two are the same field in different scales. Bit 15 is then
reused during play, so only bits 9..14 are unambiguous on a runtime layer.
The [measured movement classes and source-derived probe lookup](collision.md)
are bounded: the traced controller uses bits 9..13 and a dynamic-bit override,
while ordinary movement also depends on directional/pair and slope dispatch.
The community field is **not a passability predicate**. Raw words are preserved.
The conflicting imported field `$7E:081E` is intentionally not used.

## Provenance and unresolved work

- Canonical [memory-map documentation](memory-map.md) records imported address
  evidence and conflicts; the qualified replay supplies the observations above.
- The [community collision overlay](https://gist.github.com/Skarsnik/547e36239987adf65e888ca0de6d91b9)
  hosted by Skarsnik (crediting Gunty and `@wzl_`) supplies the high-byte mask and
  open/solid/water leads. It was studied, not
  copied. No license was established for its implementation. Its mask is now
  known to be exactly `2 * attribute`, so it reads the right field; the
  open/solid/water *labels* remain hypotheses. Measured play has since qualified
  four attribute values in one map, including one the raw position word walks
  over but which actually stops movement. See [collision](collision.md).
- A Japanese static-ROM scan found `STA $0826` at normalized `$00F690` and
  `STA $082A` at `$00F69A`. Surrounding instructions read bytes through `[$6E],Y`,
  mask with `$00FF`, then `XBA`. A probe from label 1100 failed to reach
  `$80:F690` before its frame limit (1,765,420 entries). That probe qualified
  nothing. The later [loader trace](static-maps.md) found the actual cavern path
  at `$86:8ADA` / `$86:9538`, not this candidate.
- Public terranigma.be ROM-layout, map-script, map-flags and map-exit pages were
  inaccessible during this research attempt. No new format claims rest on them.

The [map-ID script projection](map-scripts.md) now handles a bounded loading
subset. Still required: conditional loading and final composition, broader metatile/graphics reconstruction,
indoor/outdoor/dungeon/world-map coverage, placements/regions/transitions, and
trace-qualified collision behavior — [collision](collision.md) separates the
traced controller probe from the ordinary motion resolver and its remaining
slope/directional gaps. This tool is a visual aid for that work,
not a portable gameplay collision API or an asset pack.

## Validation

```sh
cargo test -p assets --test maps
cargo test -p map-inspector
```

Synthetic tests cover malformed WRAM, dimensions/bounds, row-major lookup,
lossless layer export, BMP channel/row encoding, and safe inline JSON. The
optional ROM-backed test runs a fresh oracle subprocess and pins both checkpoints,
selected cells, layer hash, and RGB hashes. It explicitly skips if the local ROM
or save is absent. The process-global oracle must not be rebooted in one process.

Browser QA uses generated local files: desktop (1280 px) and mobile (390 px),
checkpoint switching, overlays, zoom, locate-player, direct clicks, keyboard
selection, and console-error checks. Screenshots remain under ignored
`local/map-research/`; no original game art is included in this document.
