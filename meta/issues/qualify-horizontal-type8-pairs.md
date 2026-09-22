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

## Source survey and route blockers

The reproducible [source survey](../../docs/collision.md)
attempts all1,104 loading IDs:496 resolve,404 are skipped for missing/duplicate
selected resources,204 fail resolution. It counts AllClear static BG1 only,
not actor occupancy, event patches, unresolved projections or other story states.
Survey SHA-256:
`aa935272e00a76d60ec9e228c9a297f1b71b2927733e975b82e0edab7f8945bc`.

- Direct Partial/8: zero pairs with first indices5/9/16/27/31 in resolved
  projections. Slope promotion can still enter the Partial table; the separate
  slope pass found no actual native second8 dispatch there either.
- Solid/8:802 pairs across61 projections, but none in the24-map Crysta baseline.
  Early navigation hints are map `$24` second cells `(12,51)/(12,54)` (first14),
  `$4E` `(9,5)` (first12), `$50` `(13,6)` (first14), and `$51` (first12/14).
- Static direct-exit searches from `$0A` and `$41` did not reach those early
  candidates. This omits scripted/forced transfers and is not an unreachability
  proof. No input-only native boot route to them is currently established.
- Further progression is a separate unresolved
  [first-tower route issue](qualify-tower-approach-route.md). Its known failed
  map41 sweeps were not repeated or silently expanded into a whole-game run.

Both directions of both pair classes remain **unqualified and open**. The
survey's four synthetic unit tests cover pair geometry, bit15 separation, exact
resource selection and ambiguity (red→green); they are tooling tests, not native
collision evidence. No fabricated ROM contact or runtime change was introduced.
