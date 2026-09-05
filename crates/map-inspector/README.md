# Local map inspector

A native capture/extraction CLI plus dependency-free browser viewers. This is a
**map research tool**, with an explicitly limited CPU-free semantic room preview,
not a general map loader or a complete playable port.

## CPU-free room preview

```sh
cargo run -p map-inspector -- serve-room \
  'local/Tenchi Souzou (Japan).sfc' 8765
```

Open `http://127.0.0.1:8765/`, choose **New Game**, then **Resume** and walk with
Arrow/WASD/touch. This source-derived default-name start explicitly skips intro
presentation. No SRAM or original CPU execution is required.
The Rust core runs natively behind the local browser UI; its Wasm build is
validated but not wired into this frontend yet. Port `0` selects an unused port.

**Limited semantic preview:** reference-qualified flat walking and an
endpoint-qualified doorway policy, not native video-frame transition timing.
Open/solid corner responses are supported. Ordinary direction reuse is supported. Partial furniture and passive flagged walls are supported. Unknown materials and rapid
same-direction accelerated triggers still stop explicitly;
use Reset. The player is a bounds marker; no sprites, actors, combat or audio.
See [portable boundary, evidence and limits](../../docs/portable-room.md).
`verify-house ROM semantic-preview` verifies the fresh511-step house round trip.
`verify-room ROM semantic-preview` retains the saved-position diagnostic route.
See [playable boundary and verification](../../docs/playable-house.md).

## ROM-only full-map viewer

```sh
cargo run -p map-inspector -- render-map \
  'local/Tenchi Souzou (Japan).sfc' 128
```

Open the printed `local/static-maps/run-*/index.html`. It renders the complete
1280×512 portal cavern from decoded graphics, palettes and metatiles, not
framebuffer samples. No SRAM or emulator execution is needed. Natural palette,
checkerboard transparency, zoom/grid, cell/word inspection and metatile previews
are included; sprites, animation and final scene effects are not. Maps `$000F`,
`$0010` and `$0128` have explicit supported profiles. Use `f` or `10` instead
of `128` for the [Crysta first-background sheet](../../docs/room-graphics.md). See [static graphics](../../docs/static-graphics.md) for resource
formats, source/VRAM equality and the qualified 16×16 reference-pixel patch.
Generated assets remain local; do not commit or redistribute them.

## Qualified Crysta doorway replay

`qualify-opening ROM SRAM` runs seven pinned checkpoints from actual save slot 1
through the `$000F` → `$0010` doorway. `trace-opening ROM SRAM` independently
checks eight native trigger/controller/loader stops against the decoded exit.
See [inputs, hashes and scope](../../docs/opening-doorway.md). These commands
run the oracle; static `render-map` does not. Neither is a portable simulation.

## Runtime capture viewer

From the repository root:

```sh
cargo run -p map-inspector -- capture \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

Open the printed `local/maps/run-*/index.html` file. It shows map `$0128` (portal
cavern), original before/after viewports, a full raw layer, collision/grid/player
overlays, zoom, and cell inspection. Its historical candidate labels are retained;
low-nine-bit metatile indexing is now qualified for the cavern by the static
renderer, but collision meanings remain provisional. No server or network access
is needed.

Runtime capture/qualification inputs must match the Japanese reference ROM and
pinned three-slot SRAM. Neither
is provided. See [maps and qualification](../../docs/maps.md) for hashes, replay,
runtime layout, the eight-row border assumption, provenance, and remaining work.
Building the native oracle requires a C++20 compiler (see its
[build configuration](../oracle/build.rs)). `assets::maps`
owns the pure runtime model; this binary owns authentication, replay, and local I/O.

`capture` writes files to a fresh ignored `local/maps/` directory, never over
inputs. `verify` instead of `capture` prints the JSON manifest without exporting
files. Both modes run one process-global emulator session and exit the process.
All captures contain ROM-derived material: **do not commit or redistribute them**.

## ROM-only decoding and loader qualification

```sh
# JSON to stdout; no SRAM read or emulator boot.
cargo run -p map-inspector -- decode-layer \
  'local/Tenchi Souzou (Japan).sfc' 0x90000

# Map-ID lookup and resource provenance; no SRAM read or emulator boot.
cargo run -p map-inspector -- resolve-map \
  'local/Tenchi Souzou (Japan).sfc' 128

# Menu/cavern experiment; asserts intermediate equality and reports final differences.
cargo run -p map-inspector -- qualify-loader \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

Map IDs and offsets are hexadecimal; offsets are normalized/headerless.
See [loading-script projection](../../docs/map-scripts.md) for command coverage,
strict failures and multi-layer source-load output. Redirect extracted JSON only
to ignored `local/`. See [static map layers](../../docs/static-maps.md) for source
ranges, pointer provenance, attribute lookup, trace stops and remaining gaps.
The original capture viewer remains a runtime view; these inspection commands do
not replace its capture data. `render-map` creates a separate static viewer.
`qualify-loader` now also checks graphics, palette, definition and tilemap equality,
plus a fixed opaque background pixel patch after the reference effect profile.

## Tests

```sh
cargo test -p map-inspector
```

BMP/HTML tests are synthetic. Local tests authenticate inputs, check ROM-only
extraction and map-ID inspection, and qualify runtime checkpoints and loader
stages in fresh subprocesses. They skip explicitly without their local inputs.
Browser QA additionally checks interaction and responsive rendering. With `agent-browser` installed, run the atlas draw-order
and changed-count regression on a generated viewer:

```sh
agent-browser --session terramap open 'file:///absolute/path/to/local/maps/run-.../index.html'
agent-browser --session terramap eval --stdin < crates/map-inspector/tests/viewer-check.js
agent-browser --session terramap close
```

The browser regression temporarily changes one in-memory word (not capture files)
to exercise highlighting, since the qualified pair has no changed layer words.
It is a separate browser check, not part of `cargo test`.

## Dependencies

Workspace crates `assets`, `oracle`, `rom`, and `room-core`, plus `serde_json` (MIT/Apache-2.0).
The viewer has no third-party JavaScript or styles. Community format leads were
studied without copying an unlicensed implementation; see the format document.
