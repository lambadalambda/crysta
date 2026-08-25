# Bootstrap the Rust workspace and quality gates

## Summary

Create the initial crate structure and local quality commands without prematurely selecting presentation libraries.

## Dependencies

- None.

## Requirements

- Create workspace manifests for ROM tooling and shared test support first.
- Configure formatting, Clippy, unit tests, and documentation checks.
- Document the supported Rust toolchain policy.

## Acceptance Criteria

- A clean checkout builds without a ROM.
- Formatting, Clippy with warnings denied, and tests pass.
- The workspace does not embed desktop or web dependencies in the deterministic core.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Use TDD for the first executable behavior; keep empty placeholder crates to a minimum.
