# Fix actor-script rustdoc links to private constants

## Summary

Strict public documentation fails because actor-script API comments link to
private `CHAIN_CONTINUES`, `CHAIN_NEGATES`, and `MAX_CHAIN_WORDS` constants.

## Dependencies

None.

## Requirements

- Keep the constants private and preserve runtime behavior.
- Render the internal names as code rather than unresolved public-doc links.
- Mark the two coordinate-array examples in `room-core` as code; the full
  workspace build exposed those additional broken-link errors after the
  private-item links were repaired.

## Acceptance Criteria

- Reproduce the private-intra-doc-link failures before the change.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` passes afterwards.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)

## Verification

Reproduced all three private-link failures, then the two coordinate-array
broken links exposed by the full workspace build. All five doc comments now
render those references as code. Constants remain private; no runtime or API
behavior changed.

The strict workspace rustdoc build, formatting, Clippy with warnings denied,
and full workspace tests pass. The issue is complete and archived.
