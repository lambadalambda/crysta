# Render bounded Pandora source scenes in the preview

## Summary

Consume the opt-in source atlas/phase/background artifact in the existing canvas frontend, preserving ordinary house behavior. This is a presentation component, not story or movement admission.

## Dependencies

- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Decode the required Pandora actors and carrying poses](decode-pandora-scene-art.md)

## Requirements

- Admit only the four fixed additional sheet routes/dimensions and eight source map mappings; preserve the separate cellar palette and existing house door isolation.
- Validate exact phase/map/instance membership against `pandora_scenes`, using explicit core-supplied `scene_phase`; no frontend flag-to-story inference or actor scheduler.
- Preserve source OBJ painter order, anchors/mirroring and priority2/3 versus opaque-high first background. OBJ priority must not reorder overlapping actors; a winning low-priority OBJ hidden by BG must not reveal a rear high-priority OBJ.
- Keep rendering bounded, deterministic and independent of original CPU execution; do not add dependencies, a full SNES renderer, windows/color math or unqualified effects.
- Existing house-only and legacy helper fixtures remain compatible. Unsupported scene/priority/background data fails visibly, not via fallback.

## Acceptance Criteria

- Red → green tests cover exact manifest/phase admission and actual nonvacuous overlapping pixel outcomes, including transparent BG, OBJ2/3 ordering and map mismatch.
- Existing house/dialogue browser harness/verifier suites pass and an independent correctness/architecture review approves the component.
- Runtime timing, carry overlay/pose selection and full Pandora browser acceptance remain explicitly parent-owned until integrated.
