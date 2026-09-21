# Trace the player's movement admission routine

## Summary

The collision predicate is inferred from movement in two maps and fails
closed on six attributes, which fragments the town and costs five of the
slice's maps. Find the routine by differential CPU trace, read it, and
replace the inferred partition with the table the game reads.

## Dependencies

- [Trace and qualify the movement collision predicate](qualify-collision-predicate.md)

## Requirements

- Trace one frame where a held direction moves the player and one where the
  same direction at the same facing is refused, with nothing else different,
  and bound the admission code by the executed-address difference.
- Read the routine from the ROM and state what it reads: the buffer, the
  bits of the cell word, the table, and how the lookup maps to admitted or
  refused.
- Provide a pure decoder of that table and check it against the measured
  partition and the recorded sweeps, with mutation controls.
- Record the result in `docs/collision.md`, and say what the decoder does
  not cover.

## Acceptance Criteria

- The admission routine is named by address and both outcomes are shown
  reached in the traces.
- The decoder reproduces walkable `{0, 2, 22}` and solid `{12, 14, 16, 25}`
  from the ROM alone, and every cell the sweeps stood on or stalled against
  agrees with it.
- The six undecided attributes have a verdict from the table, and the
  verdict for the town's `6`, `7` and `8` is stated together with what it
  means for the five missing maps.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Trace and qualify the movement collision predicate](qualify-collision-predicate.md)
- The probe is `tools/collision-qualification/trace.rs`, built beside the
  movement probe by `build.sh`; a `{"trace": name}` route line records one
  frame's instruction addresses under `local/`.
