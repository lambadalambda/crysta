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

## Frontend implementation and verification

Frontend implements strict six-sheet catalog/complete-state admission, immutable
base restoration and one working BG/high pair per sheet, bounded16×16 writes,
shared-sheet mask agreement, and consolidated world/wooden-door rendering. World
capability uses the existing winner-OBJ-before-BG compositor in ordinary house
scenes too; phase/carry validation remains unchanged. No Rust, main, core, host,
source catalog, flags inference or live enablement changes belong to this work.

Red-first `pandora-world-check.js` initially failed on the competing legacy door
path, then passed13 tile RGBA/all256-bit checks with52 bounded writes and8 initial
rasters (6 sheets + 2 actors), unchanged through repeated toggles. Node negatives
cover malformed catalogs/complete states and atomic no-write failure. Mixed high
bits and nonzero tile-row replacement/restoration were added after independent
correctness/architecture review identified those coverage gaps; no blocking code
issues or architectural refactor were requested.

An owned static server on127.0.0.1:8879 and named `pandora-world-render` browser
session verified the actual default Canvas2D updater,52 bounded writes and6
full-canvas57,344-pixel comparisons. Existing room/controller/door tests,
renderer48 Node cases plus16 full-canvas browser cases, carry160 Node cases,
and15 browser-helper Node tests pass. These are synthetic renderer fixtures,
not source authenticity, occupancy-only host diff, native RGB/timing or a live
Pandora journey. Details and reproducible commands:
[bounded renderer documentation](../../docs/pandora-world-patches.md).

Issue remains **open**: parent owns finite source candidate compilation, host/core
integration, both source-palette qualification and final runtime/browser route
acceptance. Frontend evidence does not complete those acceptance criteria.

## Acceptance Criteria

- Source mutation/candidate/diff tests and every admitted replacement pixel/mask pass under both palettes.
- Actual pixel tests cover high↔low, transparency, restoration, shared-sheet retention, occupancy-only changes and overlapping OBJ2/3.
- Existing host/frontend regressions and independent correctness/architecture review pass; final runtime/browser route remains parent-owned.

## Integration contract (in progress)

- Opt-in `art.world_backgrounds[key]` contains `tiles[tile_id] = {rgba, high}` and
  `candidates[cell] = [replacement tile IDs]`, with six finite sheet keys. Town
  atlas availability does not itself qualify a Town runtime write.
- `Art::world_background(map, &game.effective_room(data)?)` returns the complete
  sorted `state.world_background = {key, patches:[{cell,tile}]}`. The caller must
  use the effective room, not its immutable selected collision profile.
- This capability consolidates wooden-door rendering into the same sparse path;
  `wooden_door_open` remains inspection metadata, not a competing visual selector.
  Legacy bundles retain their existing two-tile door transport unchanged.
- Host red tests were recorded before implementing projection and atlas; raw
  source discovery precedes those tests. Four host world tests now pass, including
  5,120 source candidate/palette pixel/mask samples (all opaque low BG). Synthetic
  frontend controls separately exercise high/transparent replacements.
- Parent reproduced the 13-tile/52-write/6-full-canvas world checks and 160 carry
  full-canvas comparisons; source atlas export was independently rebuilt by the
  parent. Host units pass (65, with 3 explicit diagnostic tests ignored), strict
  workspace Clippy passes. Rust independent static review found no actionable
  defects. Full GameState→host→browser route remains pending.
- Registering the reusable navigation module changes the whole-file `main.rs`
  observer pin. That gate is intentionally not repinned yet: unchanged capture
  recipe/source and freshly reproduced outputs must be independently revalidated.
