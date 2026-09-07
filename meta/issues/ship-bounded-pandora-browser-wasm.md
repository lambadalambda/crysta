# Ship the bounded Pandora browser Wasm frontend

## Summary

Deliver a production-bounded, ROM-free static browser frontend in which the accepted Pandora opening runs in the actual Rust `room-core` WebAssembly build after local, authenticated ROM extraction.

## Dependencies

- [Run the Pandora preview browser-locally through Wasm](run-pandora-preview-browser-wasm.md)
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
- Browser network evidence proves zero ROM upload and zero gameplay/backend requests after static startup; inspected deployment artifacts contain no ROM bytes or extracted asset files.
- Current-profile native and Wasm replay identities agree, and relevant legacy house/conversation/browser regressions pass.
- Strict workspace Clippy, the executed relevant test suites, repository safety, tracker validation, targeted formatting, and an independent final correctness/architecture/security review pass; the full workspace run may stop only at the separately owned strict current-producer identity gate.
- Runnable commands, URL, evidence identities, measurements, and bounded limitations are documented.

## Completion evidence

Completed with signed verifier commit `2593b3d` and the unchanged stage-2
runtime. A real Chromium session selected the owned ROM through the file input,
observed the authenticated saved checkpoint, clicked actual New Game, and drove
all 11,409 projected UI actions. All 11,410 complete 256×224 canvases and every
projected field/raw snapshot matched the independently reviewed native proof.
The final state was tick 11,409, map `$0041` `(136,208)`, walking/player, with no
dialogue or error and snapshot
`8aab7a37cb0c1115438e8f177c7213fabb0cee14f94d7eaed0cf58635c7484d5`.

The run retained 181 proved omissions, all 18 visible-unready updates, 187
semantic/readiness checkpoints, all 34 direct source invocations, and all
required source-raster/background/patch/carry/Ark evidence. An independent
post-run read-only worker state exactly matched the final verifier state.

The static server on isolated port 8890 was stopped after verifier launch and
the journey still completed. HAR evidence contains only static startup GETs and
12 browser-local Blob image reads: no POST, ROM upload, gameplay/asset endpoint,
or non-loopback request occurred. The generated static site contains no ROM,
BMP, art JSON, proof, trace, or saved state. File read was 362.3 ms, worker
compile 656.7 ms, and Wasm linear-memory capacity 289,734,656 bytes; the latter
is explicitly not total browser peak.

The relevant 43 JS helper/transport tests, shared native/injected UI regression,
Pandora facade tests, strict workspace and Wasm Clippy, release Wasm build,
no-`oracle` graph check, safety, tracker, and diff checks pass. The full workspace
test run stops at the separately owned current-producer identity gate; a
no-fail-fast diagnostic continued through later suites until its 120-second
harness bound, with no additional failure observed before termination. The prior full
legacy HTTP journey was not rerun; unchanged house/conversation/Pandora helpers
and native UI mode were rerun, while the existing accepted native journey
remains authoritative. `cargo test --locked --workspace` remains deliberately
red at the current-producer source identity gate; no pin was changed.
Independent `openai/gpt-5.6-sol` correctness, architecture, and security reviews
reported no blocking finding. Evidence hashes and reproduction details are in
[`docs/pandora-browser.md`](../../docs/pandora-browser.md).

The broad browser ROM bootstrap and WebAssembly frontend issues remain open for
general release scope. Final producer/source-bridge qualification remains owned
by the separate producer revalidation issue.

## Notes

- This issue is a production-bounded Pandora frontend, not completion of the broad browser bootstrap or full-game WebAssembly frontend issues; those remain open.
- Audio, caching, full distribution, general event VM, combat, and gameplay beyond the admitted route stay out of scope unless essential to this acceptance.
- All ROM-backed outputs, captures, traces, binaries, and extracted content remain ignored under `local/`.
