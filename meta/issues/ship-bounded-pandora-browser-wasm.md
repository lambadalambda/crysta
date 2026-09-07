# Ship the bounded Pandora browser Wasm frontend

## Summary

Deliver a production-bounded, ROM-free static browser frontend in which the accepted Pandora opening runs in the actual Rust `room-core` WebAssembly build after local, authenticated ROM extraction.

## Dependencies

- [Implement browser ROM and asset bootstrap](browser-rom-bootstrap.md)
- [Ship the WebAssembly frontend and web platform services](webassembly-frontend.md)
- [Validate extraction and the core boundary in WebAssembly](validate-web-extraction-spike.md)
- [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md)
- [Verify the continuous Pandora browser journey](verify-pandora-browser-journey.md)

## Requirements

- Reuse the accepted Rust source-derived Pandora compiler, deterministic core, renderer projections, actual controls, and ready-versus-visible dialogue behavior; do not duplicate gameplay rules in JavaScript.
- Authenticate and compile a user-selected owned Japanese ROM entirely in the browser. Never upload, cache, distribute, or commit ROM bytes or extracted assets.
- Run the portable simulation entirely in browser Wasm after bootstrap, with no original CPU execution, simulation backend, per-step server request, state injection, or preseeded content fixture.
- Keep the implementation bounded to the qualified New Game → required town/house/cellar/pot interactions → Box opening → mandatory tour → regained map `$0041` control route.
- Preserve deterministic current-profile native/Wasm parity and legacy profile behavior.
- Provide a reproducible source-only build/static-server workflow and clearly document scope, fidelity, security/network boundaries, and measured bootstrap/runtime resource use.

## Acceptance Criteria

- Automated red-to-green coverage proves local ROM authentication/extraction, Wasm ABI behavior, dialogue readiness, deterministic snapshots, and rejection without mutation.
- A real browser selects the owned ROM locally and completes the authentic fixed route through final map `$0041` `(136,208)` player control using actual UI controls and full-canvas checks.
- Browser network evidence proves zero ROM upload and zero gameplay/backend requests after static startup; inspected deployment artifacts contain no ROM-derived content.
- Current-profile native and Wasm replay identities agree, and relevant legacy house/conversation/browser regressions pass.
- Strict workspace Clippy, tests, repository safety, tracker validation, targeted formatting, and an independent final correctness/architecture/security review pass.
- Runnable commands, URL, evidence identities, measurements, and bounded limitations are documented.

## Notes

- This issue is a production-bounded Pandora frontend, not completion of the broad browser bootstrap or full-game WebAssembly frontend issues; those remain open.
- Audio, caching, full distribution, general event VM, combat, and gameplay beyond the admitted route stay out of scope unless essential to this acceptance.
- All ROM-backed outputs, captures, traces, binaries, and extracted content remain ignored under `local/`.
