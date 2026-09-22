# Qualify native horizontal first8 dispatch

## Summary

Extend horizontal Open/8 evidence to actual Left and Right first-sample type8
contacts in the ordinary passive resolver.

## Dependencies

- [Directional collision qualification](qualify-crysta-directional-collision.md)

## Requirements

- Input-only empty-SRAM native boot sessions; no warps, patches or save states.
- Prove actual first8 dispatch, including tentative sampling, dynamic-bit state,
  old-edge precedence and player-only PCs; distinguish aligned/unaligned cases.
- Pin contiguous replay windows and recipes; add failing coverage tests and
  mutation controls before admitting evidence. Keep production conservative.
- Unsupported or unreachable cases remain explicit, not relabeled as qualified.

## Acceptance Criteria

- Left and Right native first8 witnesses replay without exclusions or state resets.
- Relevant source branches and qualification limits are documented; regression
  gates and independent review pass.

## Notes

- Related: [horizontal pair cases](qualify-horizontal-type8-pairs.md) and
  [slope-mediated cases](qualify-slope-mediated-type8.md).
- Raw ROM/layers/traces stay ignored; publish tooling, recipes, selected metadata
  and hashes only. Native modes outside ordinary walking remain out of scope.
