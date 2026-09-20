# Decode the door-entry trigger for one-cell exits

## Summary

Walking decoded geometry through decoded exits reaches **6 of the 24 Crysta
maps**. The other eighteen are gated by one unresolved mechanism: the town's
building entrances are `1x1` exit rectangles sitting on cells the player
provably cannot occupy, so no walked position ever selects them.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md)

## Evidence

Two kinds of exit appear in the slice, and only one of them works by walking.

**Multi-cell rectangles do work.** Map `$000F`'s exit to `$0010` is a `1x2`
rectangle at cells `(24,12)`, whose first row is attribute 2 and is walkable.
Driving the reference emulator into it transitions the map, and
`ExitList::select` agrees: at the transition the player was at `(392,225)`,
whose bounding origin `(384,209)` matches the record, allowing for the frames
of transition latency before the map id changes.

**One-cell rectangles do not.** All eight town entrances in map `$000A` are
`1x1`. For a one-wide rectangle the fine test
(`origin - corner < dimension * 16 - 15`) admits exactly **one** origin, so the
player would have to stand at `(x*16 + 8, y*16 + 16)`. Seven of the eight
rectangles sit on attribute-14 cells, and the measured collision offset puts
the player's sample inside that solid cell at the required position. Driving
the emulator to the house's own front door confirms it: from the arrival point
`(504,769)`, holding Up moves one pixel to `y=768` and stops, at every x across
the opening including the exactly-aligned `x=496`. No transition occurs.

The eighth entrance, `$000A -> $0015` at cell `(32,12)`, *is* on a walkable
attribute-2 cell. It is unreached for an unrelated reason: no decoded path
leads to it from the arrival point.

## Requirements

- Establish how a one-cell exit is actually triggered, from the routine that
  performs the check rather than by inference. The geometry scan is already
  located at `$8D:8797..8838`; find its caller and what it passes.
- Distinguish the collision sample point from the exit origin. They are
  currently known to differ — collision was measured at `dx` in `-2..=0`,
  `dy` in `-12..=-8` from the player word, while the exit origin is
  `(x - 8, y - 16)` — and that difference is the heart of this issue.
- Say whether door entry is ordinary movement at all, or an interaction with
  its own controller state. `docs/house-exterior.md` already assigns departure
  and arrival timing to the conversation/navigation owner, which is a hint.
- Extend the connectivity test as each mechanism lands, restating the reachable
  set rather than relaxing the assertion.

## Acceptance Criteria

- A player walking decoded geometry reaches all 24 Crysta maps, or each
  remaining map has a named, evidenced reason it cannot be reached yet.
- The trigger is explained from source, with a live transition confirming it.
- `ExitList::select`'s contract states which origin it takes and how that
  relates to the collision sample.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Make the Crysta slice fully playable](playable-crysta-slice.md)
- The measured frontier is pinned by
  `crates/map-inspector/tests/local_crysta_rooms.rs`, which asserts the exact
  reachable set so that decoding a new mechanism forces the frontier to be
  restated rather than drifting quietly.
- Attribute 29 sits under nine exits and is still undecoded; `room-core`
  qualifies it per-cell as open only when moving Up. A general rule for it is
  likely part of the same mechanism.
