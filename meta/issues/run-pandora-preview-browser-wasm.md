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

## Completion evidence

Completed on branch `task/pandora-browser-wasm` with the facade in `a0e7d48` and
the worker, shared transport, static build, and regressions in `42c09d9`.

- `PandoraPreview` owns one authenticated Rust session in a dedicated Worker;
  the browser adapter exposes only the closed state/step/New Game/reset/art/BMP
  protocol and transfers ROM bytes from `File.arrayBuffer()` without backend
  upload, persistence, cache, or service-worker paths.
- Native facade tests, Wasm release build and Clippy, the no-`oracle` target-graph
  check, seven worker/transport tests, and the shared native/injected UI
  regressions pass. Workspace Clippy, tracker, safety, formatting, and diff checks
  pass as well.
- A real Chromium smoke at `http://127.0.0.1:8888/` covered load, explicit New
  Game, movement, explicit off-target interaction, reset, malformed replacement,
  and valid recovery. After startup, New Game, movement, interaction, and reset
  issued zero network requests.
- The generated Wasm and native facade matched complete state, art SHA, and all
  six BMP SHAs over the 5,707-command focused route through map 12. The
  visible-unready acknowledgement was snapshot-identical and neutral input
  advanced arrival by one tick.
- Local measurements were 346–370 ms for the file read, 540–657 ms for worker
  compilation, 289,734,656 bytes of Wasm linear-memory capacity (not total
  browser peak), and a 1,222,437-byte release Wasm.
- Independent `openai/gpt-5.6-sol` correctness and architecture review completed
  with no blocking findings.

`cargo test --locked --workspace` remains intentionally red only at
`fixture_current_producer_sources_match`: the gate first reports the documented
`Cargo.lock` identity change. No producer, observation, epoch, or output pin was
renewed. The full accepted 11,409-input/11,410-canvas browser journey, producer
revalidation, caching, and deployment remain follow-up work.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Full 11,409-input/11,410-canvas browser qualification, persistent caching, production deployment, and producer-pin renewal remain separate follow-up work.
- [Browser ROM bootstrap](browser-rom-bootstrap.md) and [WebAssembly frontend](webassembly-frontend.md) remain open.
