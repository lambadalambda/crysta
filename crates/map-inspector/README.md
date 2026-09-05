# Local map inspector

A native oracle capture CLI plus a dependency-free browser viewer. This is a
**map research tool**, not a general map loader or a playable port.

From the repository root:

```sh
cargo run -p map-inspector -- capture \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

Open the printed `local/maps/run-*/index.html` file. It shows map `$0128` (portal
cavern), original before/after viewports, a full raw layer, collision/grid/player
overlays, zoom, and cell inspection. Collision meanings and low-nine-bit metatile
indices are explicitly provisional. No server or network access is needed.

Inputs must match the Japanese reference ROM and pinned three-slot SRAM. Neither
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

# Fixed cavern experiment; asserts intermediate equality and reports final differences.
cargo run -p map-inspector -- qualify-loader \
  'local/Tenchi Souzou (Japan).sfc' 'local/saves/Terranigma.srm'
```

Offsets are hexadecimal and normalized/headerless. Redirect extracted JSON only
to ignored `local/`. See [static map layers](../../docs/static-maps.md) for source
ranges, pointer provenance, attribute lookup, trace stops and remaining gaps.
The viewer remains a runtime view; these commands do not replace its capture data.

## Tests

```sh
cargo test -p map-inspector
```

BMP/HTML tests are synthetic. Local tests authenticate
inputs, check ROM-only extraction, and qualify runtime checkpoints and loader
stages in fresh subprocesses. They skip explicitly without their local inputs. Browser QA additionally checks interaction and
responsive rendering. With `agent-browser` installed, run the atlas draw-order
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

Workspace crates `assets`, `oracle`, and `rom`, plus `serde_json` (MIT/Apache-2.0).
The viewer has no third-party JavaScript or styles. Community format leads were
studied without copying an unlicensed implementation; see the format document.
