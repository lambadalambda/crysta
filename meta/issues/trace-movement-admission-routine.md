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

## Progress: the table is traced, but the single-predicate premise is false

Fresh input-only replay reaches both outcomes of `$80:ADAD`: Right moves
`(331,112) → (332,112)` in one witness and remains at `(472,112)` in the other.
`admission-route.jsonl` and `check_trace.py` retain the route and validate the
complete-frame observations, call sites and branch successors. These witnesses
hold direction/facing/map/control constant, not every machine-state variable.

Reading the caller corrects the initial interpretation: `COP CA` at `$84:8E6C`
branches past a conditional accelerated-action entry and continues ordinary
`COP61`. It is **not** the routine refusing ordinary walking. Its `$E85C` table
is decoded by `assets::maps::collision::ProbeTable`, with the dynamic-bit bug
fixed: substituting high-byte 6 before shifting selects **entry 3**.

All six undecided attributes are nonzero in that table, but calling them solid
would be wrong. The ordinary resolver `$80:D107` dispatches directionally:
5 matches partial16, 21 matches solid12, 29 differs from Open in Right S-first,
8 has a state-sensitive Up handler, and 6/7 take slope paths. Owned-ROM tests
check these source distinctions. The diagnostic scorer now rejects missing
layers/malformed actor evidence and reports mismatches instead of returning an
unconditional success; `ext3` has 33 projection disagreements, not a green gate.

New fixed-input slope routes reach 6/7 without exact-position `goto` retries.
Two independently replayed probes agree frame by frame. At 11979 → 11980,
Right moves `(509,923) → (510,922)` through `$80:DF74 → DFA2`; Left moves
`(499,923) → (498,922)` through `$80:DBF8 → DC26`. Both are ordinary-control
frames. The leading edge, not the top-left point, contacts the slope.

Details, limits, hashes and commands: [collision](../../docs/collision.md).
The issue **remains open**: a probe-table match cannot satisfy the final-movement
acceptance criteria. [Directional resolver qualification](qualify-crysta-directional-collision.md)
is the bounded follow-up. Runtime reachability remains 19/24; no permissive
fallback or material-policy change was made.
