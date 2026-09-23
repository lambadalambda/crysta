# Derive each walker's step and idle timing from source

## Summary

`actors/cadence.rs` admits one hard-coded record and uses fixed 32/16 ticks.
Derive the timing per resident from its own source chain instead, so every
class-0 and class-2 walker in the slice gets source timing.

## Dependencies

- [Qualify the remaining Crysta walkers' movement cadence](qualify-remaining-crysta-walkers.md)

## Requirements

- Class is descriptor mode `& $0F` (`$80:FAA4`); a zero descriptor reuses the
  previous record's class and packet. Movement base is `$4000 + (mode & $70) << 8`.
- Admit only the class-0 movement group (`class & ~3 == 0`) on the common
  `$6000` base with the audited class-0 streams. Walk ticks are the direction's
  walking list (sum of duration+1); idle ticks are the `$80:8FB5` count for
  `class & 3` times the facing's idle list. A walk must cover exactly 16 px.
- Replace exact-site skipped-service exceptions with a per-service allowlist
  justified by handler behaviour. Anything else keeps the approximate projection.
- Keep map `$0D` timing and tests unchanged.

## Acceptance Criteria

- Pure tests cover class and packet resolution, reuse, list sums (including
  a 31-tick list), idle counts and refusals.
- An owned-ROM test lists every drawn `COP 26` walker as admitted or refused
  with a reason; the town class-2 walker matches its native witness.
- Workspace fmt, Clippy and tests, and app tests with the ROM pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Source: `$80:F4EA` (record), `$80:F599..F5C3` (header), `$80:FA65..FB1A`
  (descriptor, class, base, reuse), `$80:8F32`/`$80:8F85` (class tables).

## Result

- `actors/cadence.rs` now derives `Cadence { walk: [u16; 4], idle }` per
  resident or returns a `Refusal`. It audits the `$80:8F6D` class-0 row, the
  whole `$80:8FB5` table, both common load sites and the three class-0
  streams, and decodes the resident's own packet lists.
- Actors keep `Option<Cadence>`; an admitted action lasts the derived ticks,
  and movement parity counts from the action's first tick, so a 31-tick list
  still covers 16 px. The exact-site exceptions became a per-service
  allowlist backed by the handler reads.
- All eight walkers derive as tabled in `docs/native-crysta-timing.md`;
  `$1A`'s up walk is 31 ticks. Refusals are tested for class 4, a private
  base, changed tables, streams and packets, an `$FD` record and a record
  outside the map.
- New tests did not compile against the old API (red), then pass: 39
  runtime tests, workspace fmt/Clippy/tests, 65 app tests with the ROM.
  Pure tests cover the reuse chain, pointer range and list sums; an
  owned-ROM census finds exactly eight drawn COP26-then-COP8F walkers in
  the 24 maps and all eight derive.
- Independent review approved. Applied: overflow-checked idle with zero
  refused, pure tests, stricter pointer range, exact refusal asserts. The
  sprite loader's reuse rule steps over `$00`/`$FD` records while this one
  refuses them; both are cross-referenced rather than merged.
