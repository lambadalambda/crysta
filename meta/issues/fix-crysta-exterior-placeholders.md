# Resolve remaining Crysta exterior resident placeholders

## Summary

The native Crysta screenshot still shows a purple resident placeholder at the
right edge of the exterior near the pond/path. Determine which source resident
and art-resolution refusal it represents, then fix the bounded supported case.

## Dependencies

- [Derive resident art from spawn records across the Crysta slice](crysta-resident-art.md)
- [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Requirements

- Identify the exact source record and reason for the exterior placeholder.
- Decode/render its real source art when justified; do not hide unknown residents
  or substitute unrelated art just to remove purple blocks.
- Preserve explicit refusal for unsupported art beyond the demonstrated case.
- Keep raw sprites, snapshots and ROM data ignored.

## Acceptance Criteria

- A failing regression names the relevant resident/art path before the fix.
- The bounded source resident renders correctly rather than as a placeholder.
- Relevant art and native rendering tests pass with independent review.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Outdoor stalls are tracked separately in
  [native Crysta outdoor stalls](fix-native-crysta-outdoor-stalls.md).
