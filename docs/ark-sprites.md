# ROM-backed ordinary Ark sprites

`assets::sprites::ArkSprites::from_rom(authenticated_rom.image())` decodes a
bounded **21-frame** ordinary standing/walking family. No CPU execution, captured
atlas, SRAM, WRAM or oracle is needed by the decoder. Left reuses horizontal
frames with actor H-flip. This is not an animation scheduler or general sprite VM.

## Host integration contract

```rust,ignore
let sprites = assets::sprites::ArkSprites::from_rom(rom.image())?;
let frame = sprites.frame(0xA4A54E).expect("qualified standing down");
let tiles = sprites.graphics(frame.resource()).unwrap();
let bounds = frame.composition().bounds(hflip, false); // left,top,right,bottom
let pixel = frame.composition().sample(tiles, hflip, false, relative_x, relative_y)?;
// Transparent: leave destination alone.
// Opaque {palette_index, priority, component}:
//   RGB = sprites.palette()[usize::from(palette_index - 128)].rgb8()
//   place at player_world + relative_xy - camera; no baked camera or shadow.
```

Bounds are half-open and actor-relative, **not collision bounds**. Natural BGR555
colors use the existing graphics primitive. Color zero stays transparent. Palette
indices retain the OBJ base 128; the qualified family uses palette 0 exclusively.
Priority is the raw two-bit OBJ priority (these frames use 2). Earlier components
win opaque overlap **regardless of priority**. Keep that metadata for later scene
composition; do not sort parts by priority. Host owns atlas packing/transport and
BG/OBJ scene policy. No fixed 32×32 crop is necessary: use per-frame bounds.

Runtime frame IDs are composition pointers, not animation IDs or VRAM tile slots:

| Family | Frame IDs, source list order |
|---|---|
| Stand down | `$A4:A54E` |
| Stand up | `$A4:A597` |
| Stand horizontal | `$A4:A5E0` |
| Walk down | `$9A:D48A,D4C5,D507,D549,D584,D5C6` |
| Walk up | `$9A:D608,D64A,D67E,D6B9,D6FB,D72F` |
| Walk horizontal | `$9A:D76A,D7AC,D7E7,D822,D86B,D8A6` |

Right has H-flip false, left true. Timing/facing selection belongs to the separate
animation implementation. `frame(id)` returns `None` for everything else; **no
fallback or reinterpretation of unknown IDs**. In particular, fresh completed
6800 is a special idle gesture `$A5:F839`, resource 5, not this ordinary standing
family. A semantic startup renderer may explicitly select ordinary standing,
but must not claim that it reproduces that special idle pose.

## Source composition and loading

Frame source bytes begin **four bytes before** the composition pointer:

- Anchor bytes: normal X, mirrored X, normal Y, mirrored Y. X is sign-extended.
- Twelve prefix bytes retained losslessly; their collision/other semantics are
  not inferred.
- One count byte, followed by exactly count seven-byte components.
- Component: size (`0=8×8`, `1=16×16`), normal/mirrored X, normal/mirrored Y,
  little-endian tile/attribute word.
- Low nine word bits are **ROM source tile IDs**, not final OAM tile IDs. Bits
  9–11 palette, 12–13 priority, 14/15 H/V flip. Actor flips choose alternate
  offsets/anchors and XOR component flips. Large parts flip as whole 16×16 units.
- Large-part source tiles are `n,n+1,n+16,n+17`, not packed `n..n+3`.
- Effective pixel Y is **OAM Y + 1**. Frame sampling already returns effective
  actor-relative pixels; do not add another one in the host.

The loader reads the first three fixed-length frame lists at `$A4:A1E4` and
`$9A:D064`: one standing record or six walking records and a `$FFFF` terminator.
Each four-byte list record has uninterpreted duration/direction bytes and a
relative pointer to the four-byte anchor. This is bounded data extraction, not
input/timing research. It follows pointers and rejects changed shapes, bank
crossings, out-of-resource tiles and unsupported palettes. Caller authenticates
the Japanese ROM, just as with `StaticBackground::from_rom`.

| Source/consumer | Role |
|---|---|
| `$80:ED75..EE14` | Resolve relative frame pointer and select/sign-extend anchors |
| `$80:EE1A..EE3C`, `$80:EF70..EFF2` | Place and emit player components to WRAM `$0A00` |
| `$80:F08A..F0B1`, `$80:F136..F22E` | Deduplicate/remap source IDs and queue planar tile uploads |
| `$80:A252..A255` | Resource-0 graphics pointer → `$9E:885E` |
| `$80:A258..A25B` | Resource-1 graphics pointer → `$9F:8000` |
| `$9E:885E..C85E`, `$9F:8000..C000` | Raw 512-tile 4bpp source windows, not compressed images |
| `$80:F941..F948` | `COP $5A`, bank `$B1`, pointer `$D831`, CGRAM index `$80`, count `$10` |
| `$80:9AEB..9B29` | COP handler copies count×2 bytes into `$7F:0600 + index×2` |
| `$B1:D831..D851` | Sixteen natural player colors |
| `$85:F9A9..F9C8`, `$85:F9DF..FA44` | CGRAM and OAM staging DMA |

All ranges are half-open. `source_ranges()` exposes the exact headerless data
extents used, including relocated graphics/palette pointers and every frame.
`SpriteFrame::source_bytes()` preserves anchors, prefix and component bytes.

## Qualification and limits

Synthetic TDD was red before each production layer: missing composition module,
then missing resource loader. Seven tests cover transparency, placement, whole-part
flips, stride-16 tiles, priority/order, exact extents, bounds and pointer relocation.
`local_sprites` authenticates the owned ROM and checks 14 selected indexed
compositions against the independently reconstructed capture evidence.

Experimental RE used reproducible fresh captures instead of synthetic TDD. The
probe reuses `tools/new-game-qualification/bootstrap.rs`, starts `Session::new`,
and reaches real control at 6800. It never restores state, patches RAM or calls
`save_state` (which synchronizes execution). Two independent processes took the
same route through F → 10 → F. Selected ordinary samples match component
placement/size/attributes/order in both WRAM draw records and read-only hardware
OAM, every uploaded source tile row in VRAM, and all sixteen CGRAM words. OBJSEL
is 2 (VRAM word base `$4000`), first sprite 0. Raw assets stay under ignored
`local/player-sprite-qualification/`.

Selected natural-source composition comparisons also cover the output image:
7050 standing down **390/390**, 7250 standing up **396/396**, 7360 standing left
**362/362**, 6915 walking down **368/368**, 7105 walking up **366/366**, 7315 walking
left **389/389** opaque pixels agree. The comparison applies ares's documented
Deep Black Boost gamma table, **not** a production palette modification.

These are translated isolated-pose comparisons, not full-scene equality. Ares
adds eight output rows in non-overscan mode (`ppu/dac.cpp`). Current WRAM/OAM and
`pixels()` are not one atomic latch; moving samples need reported ±2/3-pixel
translations relative to current actor position. Exact OAM placement is checked
separately without translation. Completed 6811 even stops mid-upload (complete
by 6812); chosen samples avoid that boundary. Sunlight/color math causes explicit
remaining image differences in the initial/rightward poses. BG occlusion,
windows, shadows, equipment effects, special idle gestures, transition animation,
OAM scanline limits and scene-effect reproduction remain unsupported.

One complete framebuffer hash at 6825 differed across fresh processes despite
identical WRAM/VRAM/CGRAM and selected pose comparisons; the evidence pins the
hardware/source surfaces, not a claim of identical unselected framebuffer bytes.

Run dedicated tests with:

```sh
cargo test -p assets --test sprites --test local_sprites
sh tools/player-sprite-qualification/replay.sh 'local/Tenchi Souzou (Japan).sfc'
```

The source/capture metadata and reproducible checker live in
`tools/player-sprite-qualification/`. No raw ROM, tile, palette or framebuffer
bytes are distributed.

## Local evidence/export tools

`replay.sh` builds the capture and ROM-only export binaries in ignored `local/`,
runs the shared fresh bootstrap twice in separate processes, recomputes selected
hardware/image comparisons and checks `reference.json` plus `sources.json`.
The final qualification run was `local/player-sprite-qualification/replay-F6SsYO`;
its selected reports also match the earlier independent pair `replay-5vVdX5`.

The export is deliberately not a browser atlas. It writes **28** tightly bounded
RGBA buffers (21 normal frames plus the seven horizontal mirrors), parallel
one-byte priority buffers (`255` means transparent), and `export.json` containing
opaque frame IDs, resources, bounds, sizes and source/output SHA-256 pins. Output
RGB uses natural `Bgr555::rgb8`, not ares's display gamma. Everything generated
stays local; the committed JSON contains metadata/hashes only.

`check.py ROM CAPTURE` authenticates supplied artifacts; freshness comes from the
separate-process replay, not from an artifact checker. It fails closed under
Python optimization (`-O` or `PYTHONOPTIMIZE`), which would disable its assertions.
`test_checks.py` was red before that guard: a tampered export incorrectly passed
with optimization enabled. It is now green for optimized imports and for tampered
exports under both modes. The latter regression uses an optional existing local
replay and never changes that replay's files.
