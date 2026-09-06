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

## Bounded spike in progress

Owned in `task/wasm-spike`; leave this issue open for parent acceptance. Scope is
one JP packet and the existing six-frame synthetic walking/snapshot replay,
not the room preview or a production frontend. The preview on port 8765 is
untouched. No shared tracker indexes or milestones are changed.

Tool inspection found the Rust Wasm target and agent-browser 0.30.1 installed,
but no wasm-bindgen CLI or wasm-pack. Use wasm-bindgen 0.2.126 (MIT/Apache-2.0),
with its matching CLI installed under ignored `local/toolchain/`; no bundler,
WASI, threads, or new decoder/core dependencies. Browser automation follows the
agent-browser skill and uses a separate named session and port.
