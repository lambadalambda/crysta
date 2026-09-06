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
