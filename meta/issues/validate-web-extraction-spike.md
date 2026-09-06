# Validate extraction and the core boundary in WebAssembly

## Summary

Run an early browser feasibility spike before the full game depends on assumptions that do not hold for `wasm32-unknown-unknown`.

## Dependencies

- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Implement and verify the compression codec](compression-codec.md)
- [Define the deterministic portable core model](deterministic-core-model.md)

## Requirements

- Compile ROM validation, one representative decoder, and a minimal deterministic core step to WebAssembly.
- Exercise browser-local file input without transmitting selected bytes.
- Measure peak memory and processing time for representative extraction.
- Avoid threads, blocking filesystem APIs, and host-specific assumptions.
- Record browser API and fallback decisions without building the full frontend.

## Acceptance Criteria

- A local browser smoke page validates a supported dump and decodes one representative packet.
- Network inspection confirms that selected ROM bytes are not uploaded.
- The same synthetic core replay produces the native and WebAssembly state hash.
- Findings update architecture constraints or create focused blocker issues.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Full browser caching, audio, input, rendering, and release packaging remain in M7.

## Bounded spike — awaiting independent parent acceptance

Owned in `task/wasm-spike`; leave this issue open for parent acceptance. Scope is
one JP packet and the existing six-frame synthetic walking/snapshot replay,
not the room preview or a production frontend. The preview on port 8765 is
untouched. No shared tracker indexes or milestones are changed.

Tool inspection found the Rust Wasm target and agent-browser 0.30.1 installed,
but no wasm-bindgen CLI or wasm-pack. Use wasm-bindgen 0.2.126 (MIT/Apache-2.0),
with its matching CLI installed under ignored `local/toolchain/`; no bundler,
WASI, threads, or new decoder/core dependencies. Browser automation follows the
agent-browser skill and uses a separate named session and port.

Implementation: `cd7a8f9`. [Reproduction, evidence and architecture constraints](../../tools/web-spike/README.md).
Observed 2026-09-06 using separate port 8876/session `wasm-spike`:

- Browser-local JP authentication and existing intro Earth packet decode pass:
  normalized offset `0x2D0000`, 18,112 compressed → 32,768 decoded bytes, matching
  the independently pinned codec output hash. No extracted content is returned.
- Native and actual browser Wasm runs match the complete metadata report,
  including the six-frame canonical final state and all-frame trace hashes.
- HAR/CLI network logs show six startup GETs with zero request bodies; selection
  adds no requests. Two reselections and same-size invalid-image rejection add
  zero requests in a second HAR. Static/glue inspection confirms no upload path.
- Fresh-instance read/probe: 2.5/90.5 ms; warm probes: 18.3 and 15.1 ms. Linear
  memory capacity high-water: 9,699,328 bytes (9.25 MiB), **not total browser peak**.
  Chromium heap samples are coarse/non-peak; true total peak is unmeasured and
  must not be inferred from Wasm capacity. No production memory budget is claimed.
- Modern File.arrayBuffer + ES module/Wasm APIs suffice without isolation,
  shared memory, threads, WASI, or host-service imports. Synchronous probe responsiveness
  and legacy-browser/full-extraction fallbacks remain outside this tiny spike.
- Ignored worktree `local/web-spike/` retains captures, runtime/native/browser
  reports, timing samples, screenshot, artifact hashes and `acceptance.json`.
  The owned ROM is only an ignored link to the parent dump; nothing raw is committed.

TDD and independent correctness/architecture review completed; reviewer found
no must-fix issues. Workspace tests/Clippy, Wasm Clippy, safety/tracker checks and
spike formatting pass. Full workspace formatting reports pre-existing unrelated
assets/map-inspector differences, intentionally not changed. Issue/index closure
is reserved to the parent after independent acceptance, including explicit
acceptance of the browser-total-peak measurement limitation.
