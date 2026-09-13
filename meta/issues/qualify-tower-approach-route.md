# Qualify the route from the Pandora tour to the first tower

## Summary

Extend the accepted input-only reference journey from its current endpoint to
the first tower transition, and record the source contract for everything the
segment requires. This is the evidence gate that unblocks the remaining
portable work in
[Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md);
it is not portable implementation.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Implement bounded Pandora story state and continuation](port-pandora-story-state.md)

## Requirements

- Extend the same empty-SRAM Session used by `tools/pandora-qualification/route.jsonl`,
  preserving its commands and observation schedule, and continue from the named
  `pandora-tour-control` witness. No warps, state restoration, memory patches or
  save loading in the qualifying journey.
- Record the required maps, exits, interactions, request/choice boundaries, event
  flag changes and immediate player/world effects from that witness to the first
  tower transition. Separate required story steps from incidental movement.
- Establish explicitly whether the segment requires equipment acquisition,
  combat, or a frozen-town field return, since all three are currently outside
  every accepted claim. Record what is mandatory versus avoidable rather than
  assuming either.
- Name a stable post-transition control witness the way `pandora-tour-control`
  names the current endpoint, with its completed frame and position.
- Pin source bytes/operands and retained semantic checkpoints under the current
  `headless-sync-video-v1` observer epoch. Source metadata must not initialize
  production state from captured WRAM.
- Extend the existing checker with mutation controls that fail when the new
  semantic evidence is altered, and keep it passing normally and under `-O`.
- Keep raw ROM, text, graphics, traces and captures ignored under `local/`.
  Commit only tooling, selected source metadata and hashes.

## Acceptance Criteria

- A reproducible input-only fresh route reaches a named first-tower-transition
  control checkpoint, with an independent replay confirming the declared scope.
- The prerequisite and effect contract is source-backed, with controls
  distinguishing missing prerequisites from reordered or omitted interactions
  where practical.
- The checker rejects mutated semantic evidence; disabling a validator produces
  the expected failures before green is restored.
- The asset, runtime and rendering work the segment newly requires is written
  down as bounded follow-up scope before any portable integration begins.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md)
- Current endpoint per
  [the source/reference contract](../../docs/pandora-progression.md):
  `pandora-tour-control`, map `$41`, completed frame 41344, `(136,208)`; the
  final two-axis controls finish at completed 41788, `(120,192)`. Later field
  return, `$21/$23/$FE`, equipment acquisition and combat are explicitly not
  claimed there.
- Needs the owned Japanese ROM. RE discovery may precede executable tests, but
  retained evidence and checker work must still show red → green mutation
  controls.
- If the segment turns out to need combat, the actor/combat port becomes a hard
  prerequisite for the slice rather than a parallel track. Determining that is
  part of this issue's value, so resolve it early and report it.
