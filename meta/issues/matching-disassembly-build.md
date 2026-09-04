# Establish a byte-matching disassembly build

## Summary

Select an assembler and create a reconstruction that can replace opaque ROM ranges incrementally.

## Dependencies

- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Define the oracle artifact and publication policy](define-oracle-artifact-policy.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)

## Requirements

- Evaluate assembler syntax, exact-placement control, assertions, and cross-platform setup.
- Build from a verified local normalized ROM without modifying it.
- Permit opaque ranges while known ranges move into assembly or structured data.
- Compare reconstructed output byte-for-byte.

## Acceptance Criteria

- The normalized output matches the selected reference SHA-256.
- A one-byte intentional change is detected with an offset report.
- Build products and local ROM slices are ignored.

## Notes

- Milestone: [M2 — Matching disassembly foundation](../milestones.md#m2-matching-disassembly-foundation)
- Reconstructed assembly and data committed under this issue follow the
  commit-safe / local-only / review-required classes defined in
  [CONTRIBUTING](../../CONTRIBUTING.md).
- Matching is an archival/reference property; the portable Rust build must not depend on assembling a ROM.

## Verification

Verified locally on 2026-08-27 with cc65 tools reporting `V2.18 - N/A`:

- `cargo run -p disasm -- reconstruct <Japanese dump>` produced a 4 MiB image
  matching SHA-256
  `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- Temporarily changing the assembled `clc` at file offset `$008001` to `sec`
  failed with `built $38, expected $18` and reported both `$008001` and
  canonical SNES address `$C0:8001`; the source was then restored and the clean
  reconstruction rerun.
- The ROM-free workspace tests, formatting, clippy, rustdoc, tracker check, and
  repository-safety check pass.
- Generated ROMs, normalized slices, objects, listings, and maps are written
  only beneath ignored `local/disasm/` run directories.
