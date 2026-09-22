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

## Completed

The root refusal was exterior record `$038A19`, descriptor `$83ED37`, mode
`$0022`. Its failed graphics allocation also refused reuses `$038A23/$038A2D/$038A37`.
The house loader now accepts the same no-extra-movement-pointer `$0022` layout
already supported by the native-qualified Pandora bird decoder. No resident is
hidden and no unrelated art is substituted.

- Red owned-ROM tests reproduced the exact root refusal before the one-line
  mode admission change. Root and all three reuses now match Pandora graphics,
  palettes, compositions, durations and facing for selectors0–8 in both mirrors.
- Synthetic layout tests and mutated unsupported `$0021` controls preserve
  refusal/predecessor propagation. Existing frozen-house equivalence still passes.
- Owned-ROM asset tests (4) and runtime art tests (7), assets/runtime strict
  Clippy and independent correctness/DRY review pass.
- The native headless render at mapA `(360,472)`, camera `(232,360)`, now shows
  the bird where the purple block was. All four exterior bird records report
  `drawn`; raw before/after images remain ignored under `local/crysta-outdoor/`.

This closes the demonstrated exterior placeholder chain, not every unsupported
actor/art mode in the game. The movement stall is a separate issue.
