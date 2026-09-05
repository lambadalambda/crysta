# Implement and verify the compression codec

## Summary

Create a safe, tested implementation of Terranigma's compressed packet format.

## Dependencies

- [Bootstrap the Rust workspace and quality gates](bootstrap-rust-workspace.md)
- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)

## Requirements

- Validate existing community algorithms against both owned reference dumps.
- Define bounds and malformed-input behavior.
- Implement compression when byte-exact round trips are feasible.
- Document packet structure and variants.

## Acceptance Criteria

- Representative graphics and map packets decode to expected sizes and hashes.
- Malformed and truncated packets fail safely.
- Encode/decode round trips reproduce source packets or document canonical differences.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Do not copy third-party code without confirming its license; algorithms may be reimplemented from documentation and tests.

## Completion

Completed 2026-09-05 in `assets::compression`. The library has no runtime
dependencies or file I/O. Decoder and encoder APIs were added with failing
synthetic tests first; research was qualified by independent local decoding and
hash comparisons rather than TDD.

- All six graphics/map packet cases across authenticated JP/EU dumps match the
  independently run community decoder's expected lengths and SHA-256 values.
- The deterministic greedy encoder reproduces all six source packets byte-for-
  byte. Canonical differences for other legal encodings are explicitly tested
  and documented.
- Malformed/truncated input, output budgets, control-bit boundaries, overlapping
  references, maximum distances and output size have synthetic coverage.
- Nonzero headers and zero-length packets remain unqualified and fail explicitly.
- No third-party implementation or extracted bytes were committed. Format,
  provenance, addresses, and reproduction steps: [compression](../../docs/compression.md).

Verification: workspace formatting, Clippy with warnings denied, all workspace
tests (including both owned dumps), rustdoc with warnings denied, repository
safety, and tracker checks passed. A clean `git archive` export also passed
`cargo test -p assets --locked -- --nocapture`, with explicit JP/EU absence skips.
Independent decoder and encoder reviews found no blockers.
