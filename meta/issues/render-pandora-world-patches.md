# Render bounded Pandora world patches

## Summary

Render source-qualified door and consumed-pot tile changes from the core's effective room without duplicating progression state in the frontend.

## Dependencies

- [Qualify bounded Pandora navigation and contact admission](qualify-pandora-navigation.md)
- [Implement bounded Pandora story state and continuation](port-pandora-story-state.md)
- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Render bounded Pandora source scenes in the preview](render-pandora-source-scenes.md)

## Requirements

- Discover a finite replacement atlas from authenticated immutable profile deltas, unioned over the source-cache family; keep house and cellar palettes distinct.
- Diff authoritative effective raw tile IDs against the unpatched bitmap source grid, not the selected collision variant. Ignore occupancy-only attribute differences; reject unexplained cell/tile changes.
- Preserve existing wooden-door authority without competing patch paths. Include Town doors only once core exposes their source-backed effective writes.
- Emit a complete sorted sparse patch set; reuse one bounded working background/mask pair per sheet, restoring removed patches from immutable bases. No full-sheet copy each tick or cache of all combinations.
- Replace both tile color and all high-mask bits; retain winner-OBJ-before-BG priority semantics even in ordinary house scenes with no named Pandora phase.
- No captured grid initializer, frontend flag-to-story inference, broader movement admission or live enablement.

## Acceptance Criteria

- Source mutation/candidate/diff tests and every admitted replacement pixel/mask pass under both palettes.
- Actual pixel tests cover high↔low, transparency, restoration, shared-sheet retention, occupancy-only changes and overlapping OBJ2/3.
- Existing host/frontend regressions and independent correctness/architecture review pass; final runtime/browser route remains parent-owned.
