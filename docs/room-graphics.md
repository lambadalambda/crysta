# Qualified Crysta room first backgrounds

`assets::maps::visual::StaticBackground::from_rom(image, map_id)` reconstructs
**only the first background** for the Japanese maps `$000F`, `$0010`, and `$0128`.
The caller authenticates the ROM. `CavernBackground::from_rom(image)` remains a
compatible wrapper for `$0128`; its existing resource and indexed-image fixtures
are unchanged.

This is an allowlist of three profiles, **not support for arbitrary game maps**.
It does not execute gameplay events, evaluate conditional audio state, or compose
the SNES scene. No emulator state is needed to extract or view these backgrounds.

## Room loading recipe and bounds

The two room IDs use the same first-background sheet: **32×64 metatiles,
512×1024 pixels**, containing multiple interiors. They are not separate
256×256 resource images. At the qualified gameplay checkpoints, map `$000F` uses
camera `(256,0)` and `$0010` uses `(256,256)` within that sheet.

The root pointers are `$98:8496` / `$98:84AD`. Both defer subscript 1 at
`$98:8405`, whose first-layer, graphics, palette and definition loads are shared.
The following normalized, headerless source ranges are retained losslessly:

| Returned resource | Encoded/raw source range, half-open | Decoded extent |
|---|---|---:|
| Main graphics | `$23B11D..$23E222` | `$6000` bytes / 768 4bpp tiles |
| Main palette | `$31E945..$31EA05` | 192 bytes / colors 32–127 |
| BG1 definitions | `$2ABA43..$2AC506` | `$1000` bytes / 512 four-word metatiles |
| Attributes | `$30FF53..$30FFFC` | 512 bytes |
| Shared palette | `$328B78..$328BB8` | 64 bytes / colors 0–31 |
| First layer | `$2FCBB3..$2FCFE1` | 4096 cell bytes, plus source dimension prefix |

Unlike the cavern's `$4000`-byte graphics and 80×32-metatile layer, these rooms
load `$6000` graphics bytes and a 32×64 layer. The BG1 definitions still address
only tiles 0–511 with zero graphics adjustment. Having 768 decoded tiles does
**not** qualify additional definition-bit semantics: definition tile bit 9 and
cell bits above the low nine remain rejected. Raw priorities and flips are
preserved through the shared graphics sampler.

Returned compressed resources have exact decoded sizes and cannot cross their
source 64-KiB bank; the layer uses the existing bounded dimension-prefixed codec.
Resource source pointers come from the validated instructions, not fixed asset
addresses. Tests relocate them to synthetic resources.

### Conditional tail: profile recognition, not another interpreter

The general `scripts::resolve_map` deliberately rejects `$08 FD`, encountered
at `$98:8390` on these maps. It is not weakened or bypassed with a modified ROM.
Instead, the room profiles recognize fixed instruction windows and exact table
entries. Every control byte is checked, including the seven FD records, the
alternative audio selections, their jumps, and the common shared-resource tail
at `$98:819C`. Only known three-byte packed resource-pointer fields can vary;
these are decoded by the existing `scripts::unpack_pointer`.

The audited alternatives select audio and converge on subscript `$10`, whose
shared palette is needed by the first background. The extractor does not decide
which FD condition holds, select a soundtrack, or execute the global audio list.
There is no instruction cursor, new opcode parser, branch evaluator, or fallback
that silently skips unknown controls. Changed control windows/table pointers
fail. This fixed specialization can be replaced later by a separately qualified
loader API; it does not establish general FD support.

The same scripts also reference BG2 and sprite/shared graphics:

- BG2 layer `$31E263..$31E32E` and definitions `$2DFD48..$2DFFFD` are **not
  returned or composed**.
- `$000F`'s extra graphics `$29F02F..$29FCFE` target VRAM word `$4000`;
  its palette `$328B38..$328B78` starts at CGRAM color `$90`.
- Shared graphics target VRAM word `$7000`.

These fixed destinations do not overlap the returned first-background graphics
or colors. For **omitted** resources only instruction shapes and source start
addresses are checked; payload contents and full extents are not validated.
Success therefore qualifies BG1 extraction, not the successful execution of
every transfer in the full map-loading recipe.

## Oracle comparison without state patches

Evidence uses the known Japanese ROM SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`
and read-only SRAM SHA-256
`709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
The supplied SRAM menu defaults to slot **3**; two Up taps select actual slot 1.

From a fresh `Session::new_with_sram`, set inputs before each frame step using
zero-based, half-open intervals; all other buttons are released:

```text
Start [400,408)
Up    [900,908)
Up    [950,958)
A     [1100,1112)
Left  [1601,1657)
Down  [1657,1682)
```

After **1601 completed frames**: map `$000F`, player `(472,176)`, camera `(256,0)`.
After **1801 completed frames**: map `$0010`, player `(392,353)`, camera `(256,256)`.
The genuine doorway changes map ID at frame 1698; that frame is still mid-load
and is not used as a settled graphics checkpoint. One boot per process, no RAM
patches or snapshots, and explicit process exit after capture are required.

`local_visual_maps.rs` optionally reads ignored `local/room-runtime/f1601.*`
and `f1801.*` captures. `.wram` is `Session::wram_image()`; `.vram` and `.cgram`
are the corresponding word arrays serialized little-endian. Every file is
SHA-256 authenticated before comparison. Missing capture directory skips this
additional check; ROM-only fixture tests remain independently runnable.

Observed equality and intentionally excluded runtime changes:

- All 512 BG1 definitions match WRAM `$7E2000..$7E3000` byte-for-byte.
- Every first-layer low-nine-bit index matches runtime at `$7EA000`; higher
  runtime bits differ and are not inferred as collision behavior here.
- ROM colors 32–127 match CGRAM exactly. Shared colors 1–31 match; color 0 is
  `$28CD` in the shared ROM palette versus runtime backdrop `$1D6B`.
- Graphics match VRAM byte range `$0000..$6000` except animated artwork in tiles
  37–40 (20 differing bytes at frame 1601, 11 at frame 1801).
- Attributes match except entry 0 is runtime 22 at frame 1601 rather than ROM 0;
  the full table matches at frame 1801. This extractor retains the ROM table.

This is resource equality evidence, **not framebuffer equality**. Sprites,
animated artwork, BG2, priority composition, sunlight/windows, color math,
brightness, and transition effects are not rendered. The image uses natural
full-brightness ROM colors with checkerboard transparency.

## Full-layer fixtures and local viewer

Both room IDs produce the same complete static BG1 artifact:

- Indexed SHA-256:
  `4adec38bf192483ec43e62feaeb9219cc1e0d02b4860bc3b682dd5cc11c8886d`
- Priority SHA-256:
  `689d88232a9fc9a10d4550ffc8ff31a55ef0826b28046e8b14933cc902968174`
- Natural RGB plus checkerboard SHA-256:
  `5fa1f26738451e0b3feb350dc21f1edf587c18a9b43fbc318af7c7fc220160b2`

```sh
cargo test -p assets --lib maps::visual::tests
cargo test -p assets --test visual_maps --test local_visual_maps
cargo test -p map-inspector --test local_visual
cargo run -p map-inspector -- render-map 'local/Tenchi Souzou (Japan).sfc' f
cargo run -p map-inspector -- render-map 'local/Tenchi Souzou (Japan).sfc' 10
```

The existing local static viewer and manifest schema/key set are unchanged.
Cavern manifests retain `kind: static-cavern-background`; room manifests use
`kind: static-room-background`. Each export creates its own ignored directory
under `local/static-maps/`, retaining the BMP, index/priority planes, resource
hashes and source ranges, raw cells/metatiles/palette, and explicit limitations.

Qualification-worktree artifacts were retained under
`/Users/lainsoykaf/repos/ilar-task-opening/local/`: `room-runtime/`,
`room-resources/`, `room-resources.log`, and `static-rooms.log`. The ignored
`src/main.rs` replay probe captures VRAM/CGRAM as well as WRAM/PPM; the focused
`src/bin/resources.rs` and `src/bin/static_rooms.rs` probes retain resource and
full-sheet comparisons. No extracted resource or screenshot is committed.
