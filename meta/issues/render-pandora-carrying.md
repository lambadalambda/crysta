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
