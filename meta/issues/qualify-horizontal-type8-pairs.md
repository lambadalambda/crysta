# Qualify native horizontal Partial/8 and Solid/8 pairs

## Summary

Extend native horizontal type8 coverage beyond Open/8 to ordered Partial/8 and
Solid/8 dispatch for both Left and Right.

## Dependencies

- [Directional collision qualification](qualify-crysta-directional-collision.md)

## Requirements

- Capture actual sampled pairs and dispatched table targets from input-only
  empty-SRAM native sessions, without warps, patches or save states.
- Preserve first/second order, tentative coordinates, old-edge slope precedence,
  dynamic-bit override and ordinary/passive admission. Stop at mode changes.
- Add failing coverage tests and mutation controls; replay every frame of each
  pinned window without resets/exclusions. Preserve conservative production.

## Acceptance Criteria

- Both directions and both pair classes have source-grounded native witnesses,
  or missing cases stay explicitly open with evidence-backed blockers.
- Contiguous captured trajectories match; existing candidate 24/24 outbound and
  23/23 returns remain green. Independent review and documentation completed.

## Notes

- Related: [first8](qualify-horizontal-first8.md) and
  [slope-mediated cases](qualify-slope-mediated-type8.md).
- Commit tooling, recipes, selected metadata and hashes, not raw ROM/layers.
