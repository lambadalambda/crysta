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
