# Establish ROM-free continuous integration

## Summary

Add public automation that validates code, documentation structure, and repository safety without proprietary inputs.

## Dependencies

- [Bootstrap the Rust workspace and quality gates](bootstrap-rust-workspace.md)
- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Define the oracle artifact and publication policy](define-oracle-artifact-policy.md)

## Requirements

- Run format, lint, tests, and documentation checks.
- Validate issue-index and milestone links.
- Fail if tracked files use known ROM extensions, generated-asset locations, known full-ROM hashes, or suspicious large binary blobs outside reviewed fixture paths.
- Document how optional local ROM-backed tests are invoked.

## Acceptance Criteria

- CI passes on a clean checkout with no ROM.
- A deliberate tracked ROM-extension fixture causes the safety check to fail.
- Local ROM-backed tests skip clearly when no path is configured.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Keep CI deterministic and avoid downloading copyrighted fixtures.
