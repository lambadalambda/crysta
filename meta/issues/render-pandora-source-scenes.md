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

## Frontend component complete; parent integration pending

Alice changed only the inline renderer, its focused JS/browser tests and the
appended frontend section in `docs/pandora-scene.md`. The state contract adds
only optional `scene_phase`; source scene entries keep the adapter's
`id/key/position/priority`. Prepared rasters retain RGBA and opaque-high masks
for winner-first OBJ composition. No host enablement, inferred story phase,
record scheduler, carry membership or shared index edits.

### Verification

- TDD: the new focused check first failed on the missing admission/compositor
  API; it now passes. The original inline harness remains green and additionally
  checks all six sheet loads and visible phase/roster/priority rejection.
- Independent full-RGBA Node oracle: **48** cases, **348** opaque output samples
  and **612** absent/occluded samples, covering priorities2/3, both orders,
  transparent holes, anchors, diagnostic flips and camera clipping.
- Actual inline browser helpers: **16 × 57,344** exact RGBA pixel checks plus
  two distinct precomposed-mirror/anchor pixels. This includes a front OBJ2
  hidden by opaque-high BG with rear OBJ3 present; neither priority sorting nor
  masking each actor independently can satisfy that case.
- Built the unchanged house-only host from this worktree. Separate local port
  **8876**, browser session **pandora-renderer**: real UI house `default511`
  passed **512** full-canvas checks; real UI conversation passed **1671** steps
  and **1683** visual checks, ending in A at `(538,815)`, dialogue closed.
- House/conversation verifier unit suites pass (**8 + 7** tests). `git diff
  --check` passes. Private logs are under `local/pandora-renderer/`.
- Independent read-only reviewer **d97d684d-2f20-4077-8c5a-34dd6016e6ea** approved
  correctness, architecture/compactness and the nonvacuous pixel evidence with
  no blockers, conditional on the now-passed dialogue run. Reviewer inspected
  source/evidence logic but could not execute tools/tests independently.

This is presentation-component acceptance, **not live Pandora route acceptance**
or a whole-native-RGB claim. C windows/color math, opening-white palette,
secondary backgrounds, scripted timing, animation and carry/world-patch
integration remain omitted/parent-owned. Shared issue indexing/archival is left
to the parent, as requested.

## Parent component acceptance

- Cherry-picked the reviewed renderer and contract; independently reran 48 RGBA oracle cases, the inline controller harness and all 15 house/conversation verifier tests.
- On the rebuilt parent host, actual canvas checks pass: `local/map-research/pandora-render-parent-browser.json` records 16 full 256×224 comparisons and two distinct precomposed mirror/anchor pixels. These are synthetic composition controls, not native whole RGB.
- Real input-only house dialogue/exterior regression passes again: `local/map-research/pandora-render-parent-house.json`, 1671 ticks / 1683 visual checks. Host 51 unit tests and strict workspace Clippy pass; the full host's separate SRAM observer fixture is still tracked independently.
- Component archived. The host remains `compile_profile(false)`; dynamic carrying/world patches, script timing, core-driven phase selection and the complete Pandora browser route remain parent integration work. No runtime enablement is inferred from this acceptance.
