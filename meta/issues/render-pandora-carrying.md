# Render bounded Pandora carrying and pot flight

## Summary

Connect the accepted pot component's read-only phase/ownership/flight projection to source-derived Ark/pot poses and the phase-aware canvas renderer. Keep the simulation and dynamic state authoritative in the core.

## Dependencies

- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)
- [Decode the required Pandora actors and carrying poses](decode-pandora-scene-art.md)
- [Render bounded Pandora source scenes in the preview](render-pandora-source-scenes.md)
- [Implement bounded Pandora story state and continuation](port-pandora-story-state.md)

## Requirements

- Select the exact source Ark/pot list pairing for lift, held standing/walking, throw and admitted free flight. Preserve FA/FB art identity, anchors and independent mirrors; never add a second hardcoded carry elevation.
- Hand ownership differs from flight reservation; no pot may remain drawn after the admitted break. Ark throw recovery can continue without a pot sprite.
- Extend scene validation narrowly for this one core-supplied dynamic overlay, without accepting arbitrary actors, positions, priorities or reordered source residents.
- Frame0 finite presentation is permitted as an explicitly documented semantic policy; do not infer native animation timing from source duration bytes without qualification.
- Preserve overlapping OBJ/BG behavior, old house rendering and hidden/disabled live Pandora capability. No world-patch or story integration is implied by this component.

## Acceptance Criteria

- Focused red → green source-key/ownership/phase tests, actual nonvacuous canvas composition checks and existing frontend/host regressions pass.
- Independent correctness/architecture review approves the narrow host/core/renderer contract.
- Parent can select the overlay from existing public GameState/PotState getters without exposing private state or loading captures.

## Work boundary

Alice owns the pure carry adapter, minimal art hooks, narrow frontend overlay validation,
and focused tests/docs only. Parent owns live wiring, finite BG patches and aggregate
acceptance. Keep this issue open; do not enable the capability. Planned host seam:
`CarryInput::from_state(&PotState, (hand, reservation))`, then `Art::carry(map, input)`
returns an optional exact Ark key plus typed overlay; source NPC scenes remain unchanged.

### Host adapter checkpoint

- Red: six public API/type references absent (focused suite initially did not compile).
  Green: six focused tests pass, including all 32 source pairs, complete phase tick /
  both-slot lifetimes, bad lanes/samples and actual public PotState lift→walk→throw→
  recovery projection. ROM-backed opt-in catalog completeness/nonempty raster checks
  run with the own local ROM symlink. Strict all-target host Clippy passes.
- Independent host correctness/architecture review approved the pure contract.
  Its suggested reservation mutations and actual-core throw projection were added
  and pass. Native transient priority fidelity is explicitly excluded, not deferred
  into this patch. Frontend/evidence acceptance remains in progress.

### Frontend/evidence checkpoint — open for parent acceptance

- Red: the new overlay test returned2 actors instead of3 before implementation.
  Green:160 finite pair/lifetime cases plus strict catalog, roster, map, slot,
  flight, mirror and extra-field controls; inline invalid overlays visibly pause.
  Prepared-catalog alias controls exercise both held-pose and flight entries.
- Independent frontend correctness/architecture review approved exact source
  roster preservation and narrow overlay insertion. Independent evidence review
  approved the separate full-RGBA oracle and five detected mutations; follow-up
  pins the depth mutation to actual NPC-only differing pixels. Reviews were
  read-only source inspections, not independent reruns of owner executions.
- Own server8876/session `pandora-carry-preview`:160×57,344 real Canvas2D pixels
  compared,14,805 visible pot pixel-samples across FA/FB hand/flight,146 exact
  pot/NPC depth witnesses,47,922 hidden-winning-OBJ samples, five controls detected.
  ROM source rasters; diagnostic BG and finite states are synthetic. Not native
  whole RGB, source timing, live GameState wiring or a continuous journey.
- Fresh private evidence: `local/carry-evidence-wHKcN1/` (pointer in
  `local/carry-evidence-root.txt`), non-skipped export log, browser script/result,
  art SHA-256 `325a0dda2509b6a526b4ac12fcf8a0306960be9146967c809e2ba7bf6ea5dcdf`.
  No raw art/capture data committed. Fresh-file check prevents ROM-less skips
  from being used as browser evidence.
- Regression gates passed: old15 Node frontend tests; inline page harness;
  renderer48 pure cases plus16 full-canvas browser cases and mirror check;
  all16 host art tests including unchanged house/482-frame source pixels;
  full map-inspector suite (optional non-art fixtures may skip); strict Clippy.
  House-only browser has no error and pot button remains hidden.
- **Issue stays open.** Parent owns acceptance, real GameData/navigation, finite
  BG patches and live wiring. No room_preview/core/assets/server/world-patch/
  shared-index/backgrounds/door or action-controller changes were made.

### Parent component reproduction

- Parent independently rebuilt/exported the opt-in source art and passed all six
  carry unit tests within 65 host unit passes (three separate diagnostic tests
  intentionally ignored). Full workspace Clippy passes.
- Parent `parent-carry` browser independently reproduced all160 full-canvas cases,
  14,805 visible pot samples,146 pot/NPC depth witnesses and all five negative
  controls using that fresh parent export. Artifact/logs:
  `local/map-research/pandora-carry-parent-{art.json,browser.json}` and
  `pandora-world-host.txt`. Synthetic finite poses/diagnostic BG, not a journey.
- Latest live house-only UI passes511,2244 and1671 real-input regressions; its pot
  action remains hidden. Aggregate host wiring/final issue acceptance remain open.
