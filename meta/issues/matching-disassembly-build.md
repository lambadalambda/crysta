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
- Matching is an archival/reference property; the portable Rust build must not depend on assembling a ROM.
