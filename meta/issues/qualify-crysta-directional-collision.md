# Qualify the directional resolver for Crysta's remaining collision types

## Summary

The table traced by the movement-admission investigation is not a binary
walking predicate: `COP CA` gates a Right accelerated-action branch. The
ordinary motion resolver at `$80:D107` uses directional/pair dispatch, old-edge
slopes and additional state. Qualify that behavior before extending free roam
beyond its current 19 of 24 reachable maps.

## Dependencies

- [Trace the player's movement admission routine](trace-movement-admission-routine.md)

## Requirements

- Continue from the new input-only type6/7 witnesses produced by
  `tools/collision-qualification/slope_route.py`, not the old oscillating `goto`
  experiment. Extend coverage to all four directions, old/new edges, both
  sample positions, neighbour classes, aligned/unaligned positions and 1/2px
  steps. Trace any previously unaccounted direction before implementing it.
- Decode the slope diversions before the dynamic-bit override, preserving
  their ordering. Right's type6 witness reaches `$80:DF74 → DFA2`; Left's
  type7 witness reaches `$80:DBF8 → DC26`. Both change Y while holding a
  horizontal direction; simple Open/Solid aliases are insufficient.
- Account for type8's Up handler `$80:D506`, including its `$097C & 4` input.
  Do not infer a global class from its Down/Open handler equality.
- Admit types5/21 only where source-equivalent geometry and passive action-hook
  conditions are established. Type5 matches P16, type21 matches S12 across
  sixteen tables; that alone does not qualify gameplay side effects.
- Preserve type29's Right S-first exception: it blocks without the nudge Open
  would produce. Existing narrowly qualified Up stair aliases stay valid.
- Keep the current runtime conservative until reference samples and mutation
  controls qualify each extension. Do not rewrite unknown words to Open or
  remove the sample checks to obtain connectivity.

## Acceptance Criteria

- A pure directional resolver matches contiguous reference trajectories for
  every newly admitted case, with controls for wrong type, pair order, direction,
  mask, neighbour stride, branch sense and dynamic-bit precedence.
- Existing house trajectories and the map-$41 envelope agree, or each mismatch
  has an identified additional input and remains outside admission.
- Runtime traversal tests reach all 24 maps through real movement and exits,
  preserve collision boundaries and round trips, and do not silently discard
  unknown-material errors along the claimed route.
- Raw ROM/layers/traces stay ignored; committed evidence is tooling, input logs,
  selected source metadata and hashes. Remaining unsupported modes are explicit.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Trace the player's movement admission routine](trace-movement-admission-routine.md)
- Evidence and reproduction: [collision](../../docs/collision.md).
- This is the implementation/qualification follow-up, not completed by the
  probe-table decoder or the two slope discovery frames.

### Implementation checkpoint

- Added an explicit opt-in, pure directional candidate for 6/7 and passive
  5/21/29 geometry; defaults and production room builders remain unchanged.
- Six fresh input-only captures replay 1,972 contiguous frames, native attempted
  velocities and final XY, using per-frame live layers and uninterrupted walking
  histories. Player-only PC coverage requires both slopes' adjustment handlers
  in all four directions. Existing house candidate replays add 2,223 matching
  transitions. Synthetic controls include every ordered O/S/P pair.
- A diagnostic of the first 880 ordinary map-$41 discovery frames also agrees,
  but is not a pinned/full-envelope gate and lacks per-frame layers/velocities.
- Independent implementation and evidence reviews completed; reversed/equal
  material-pair test coverage was expanded and NPC PCs excluded from slope
  coverage. Frame-end XY and native-provided layers limit the equality claim.
- Still open: exhaustive native branch controls, broad 5/21/29 qualification,
  type8 state handling, full map-$41 envelope, production opt-in and retained
  error-checked 24-map movement/round-trip routes. Production remains **19/24**.

### Clear-bit type8 and checked host traversal

- Added a separate type8 opt-in asserting `$097C & 4 == 0` plus the complete
  ordinary-player/passive contract. Sixteen table entries, raw slope probes and
  Down6/raw8 ordering have synthetic controls; old constructors still reject8.
- Nine pinned native windows now replay **5,541 frames**, plus **2,223** legacy
  transitions. Added the 2,482-frame town cap window (Up-first8, Up Open/8,
  Down-first8), 207 passive closed-door5 frames, and a fresh 880-frame map-$41
  window with live layers/velocities. Horizontal8 and several pair/flag branches
  remain synthetic-only; the full map-$41 envelope is not claimed.
- Further town descent exits ordinary player mode at14469→14470 (flags
  `$0415→$0431`, special1, no player D107). The bounded window ends before this;
  the probe refuses the continuation rather than dropping that frame.
- Added explicit candidate room/world construction preserved across occupancy
  and transitions, checked movement/interaction, and actual events at destination
  construction. Legacy resident-decoding fallback is documented, not broadened.
- Candidate discovery now reaches **24/24** and replays every accepted action
  without core refusal/load error. **21/23** non-opening destinations have checked
  returns from their real outbound endpoints. Production remains conservative
  **19/24**; no global enablement or native progression equivalence is claimed.
- Two return gaps (`$19→$17`, `$1E→$0A`) align into the reverse doorway before
  arrival completes. The host omits selector-driven arrival ownership/coordinate
  conversion and native exit-scan suppression. Capture those actual native edges
  before changing arrival behavior; see the detailed collision document.
- Acceptance remains open: remaining native material/branch/mode qualification,
  full envelope, two arrival/round-trip cases, then justified production enablement.
