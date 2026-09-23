# Witness a class-2 Crysta walker natively

## Summary

The exterior walkers `$83:8A19/8A23/8A2D` are class 2 (descriptor mode
`$22 & $0F`), not class 4. A local input-only probe in the town shows the
moving one walks 16 px in 32 ticks with 1,0 deltas and idles 16 ticks
(list `[7,7]` x 1). Retain that evidence as reusable tooling.

## Dependencies

- [Qualify the remaining Crysta walkers' movement cadence](qualify-remaining-crysta-walkers.md)

## Requirements

- Extend `tools/crysta-cadence-qualification` with a town itinerary (the
  arrival route up to `settleTown-open`) and a probe mode that samples a
  chosen slot, without warps, patches or save states.
- The verifier checks the class-2 slot's class, source-derived walk and idle
  windows and deltas against the decoded packet `$D4:7890` and common streams.
- Record the one-frame loop overhead of `COP 03` separately from the action.

## Acceptance Criteria

- Two fresh probes are byte-identical and the verifier passes on both.
- Synthetic verifier tests cover the new mode, red before green.
- Raw captures stay under ignored `local/`.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).

## Result

- `probe.rs` takes optional `SLOT-HEX SAMPLES`; defaults keep map D.
  `town-route.jsonl` is a literal prefix of the arrival route plus `finish`.
  `decode.rs` authenticates records `$83:8A19/8A23/8A2D`, descriptor
  `$83:ED37` and packet `$D4:7890`. `verify.py --town` checks every frame
  11394..11594 of slot `$1300`, including the common resource in the town
  WRAM, four full steps, two idles and seven one-frame `COP 03` loop gaps.
- Red: town tests errored before `verify_town` existed. Green: 5 synthetic
  tests (21 town mutation cases). Three fresh town probes are byte-identical
  and pass; a fresh map-D probe passes the unchanged map-D checks.
  Independent review approved; its should-fix items were applied.
