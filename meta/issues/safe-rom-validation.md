# Implement safe ROM normalization and validation

## Summary

Recognize supported cartridge dumps, normalize copier headers in memory, and reject unknown inputs safely.

## Dependencies

- [Bootstrap the Rust workspace and quality gates](bootstrap-rust-workspace.md)

## Requirements

- Detect headerless images and 512-byte copier headers by validated structure rather than filename.
- Calculate CRC32 and SHA-256 over normalized bytes.
- Recognize the documented Japanese and European English hashes.
- Use `local/` as the documented default for private ROM-backed inputs and outputs.
- Never rewrite the user's source dump.

## Acceptance Criteria

- Public tests cover header detection, normalization, and digest classification with synthetic inputs and injected expected digests.
- Unknown, truncated, and malformed inputs return actionable errors.
- Local optional tests verify user-provided dumps when configured.
- No ROM bytes or extracted content are committed.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Headered and headerless supported forms must normalize to the same internal offset model.
