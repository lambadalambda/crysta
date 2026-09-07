# Run the Pandora preview browser-locally through Wasm

## Summary

Add a stateful `wasm-bindgen` adapter and local-file bootstrap around the accepted host-free Pandora preview so the existing playable UI can run without a simulation HTTP backend.

## Dependencies

- [Expose the Pandora preview as a host-free Wasm library](expose-pandora-preview-wasm-library.md)
- [Validate extraction and the core boundary in WebAssembly](validate-web-extraction-spike.md)

## Requirements

- Authenticate and compile a user-selected Japanese ROM once in Wasm; retain all gameplay, progression, state projection, cameras, art, and rendering in Rust.
- Adapt the existing room UI/controller through an injected transport and local asset resolver without changing native HTTP mode or accepted input/dialogue timing semantics.
- Expose bounded stateful commands for step, New Game, reset, state, art, and backgrounds with predictable lifecycle and error handling.
- Never upload or persist ROM bytes. Malformed replacement selection must not leave a stale playable session.
- Provide reproducible no-bundler build/serve tooling using `wasm-bindgen` 0.2.126 and focused native/Wasm/transport tests.

## Acceptance Criteria

- A local static server on an isolated port runs the actual room UI from local ROM selection with no simulation backend requests.
- Browser smoke covers load, New Game, movement, interaction, reset/reload error handling, and immutable art/background delivery.
- Native and Wasm state/output checks agree for focused input sequences.
- Startup time and Wasm linear-memory capacity are measured without claiming total browser peak.
- Independent correctness and architecture review finds no blocking issue.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Full 11,409-input/11,410-canvas browser qualification, persistent caching, production deployment, and producer-pin renewal remain separate follow-up work.
- [Browser ROM bootstrap](browser-rom-bootstrap.md) and [WebAssembly frontend](webassembly-frontend.md) remain open.
