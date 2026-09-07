# Expose the Pandora preview as a host-free Wasm library

## Summary

Separate the accepted source-derived Pandora preview/compiler/art path from the native inspection host so a later `wasm-bindgen` facade can own browser transport without reimplementing gameplay.

## Dependencies

- [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md)
- [Validate extraction and the core boundary in WebAssembly](validate-web-extraction-spike.md)

## Requirements

- Expose a small Rust library API that constructs the current Pandora-enabled preview from authenticated ROM bytes, accepts the existing input protocol, and returns unchanged state and bitmap outputs.
- Build room backgrounds directly in memory; preview construction must not export or read files.
- Keep the Wasm library dependency graph free of the native oracle/C++ stack and add a `wasm32-unknown-unknown` library compile gate.
- Add focused native parity tests for the in-memory bitmap path and public preview behavior.

## Acceptance Criteria

- Native tests demonstrate byte-for-byte parity with the previously accepted static background rendering and unchanged deterministic preview state.
- `cargo build --locked --target wasm32-unknown-unknown -p map-inspector --lib` passes without compiling or linking the oracle.
- Independent correctness and architecture review finds no blocking issue.
- The concrete API and remaining facade/transport/qualification work are recorded without implementing browser bootstrap or a frontend.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- `browser-rom-bootstrap.md` and `webassembly-frontend.md` remain open; this is only their source-owned library prerequisite.
