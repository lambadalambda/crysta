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

## The scan is now decoded

`$8D:8797..8838` is the scan, and `$87:91B3` is what feeds it:

```text
probe_x = (player_x -  8) & ($085A | $0F)   -> $095E, tile $0962
probe_y = (player_y - 16) & ($085E | $0F)   -> $0960, tile $0964
```

The masks are power-of-two wrap masks, not the extent minus one. Measured live
in the town: standing at `(496,768)` writes `(488,752)` and tile `(30,47)`,
which `ExitList::probe_origin` now reproduces. The scan is additionally gated
by `$097C` bit 4, and needs a non-zero exit-list pointer at `$0480`.

**This makes the conflict exact.** The house's own front door is the `1x1` rect
at tile `(31,46)`. Firing it needs `probe_y` in `736..=751`, so player `y` in
`752..=767`. Holding Up there stops at `y=768`, one pixel short, because the
collision sample and the exit probe are the same bounding corner: moving to
`y=767` would put the corner at `751`, inside tile `(31,46)`, which is the
solid attribute-14 door cell. The exit requires the corner to be exactly where
collision forbids it.

So these doors are not entered by ordinary walking, and no amount of alignment
fixes that. What remains is to find the mechanism that does enter them.

One observed lead: walking into the *working* `$000F` doorway snapped the
player's x from 399 to 392 with only Down held, so the game does move the
player during a doorway approach.

## This one mechanism is worth 17 maps

Re-running the connectivity search while assuming a solid exit rectangle can be
entered from the walkable cell below it:

```text
                                 today:  6/24 maps
 + one-cell doors enterable from below: 23/24 maps
            + doors + attr 29 walkable: 23/24
       + doors + attr 29, 5, 21 walkable: 23/24
+ doors + every remaining attr walkable: 23/24
```

So the undecoded attributes contribute **nothing** to connectivity, and this
single mechanism is the whole gap. The one map it still misses is `$000B`,
whose only entrance is `$000C`'s `1x5` rectangle at `(8,16)` — rows of
`14,14,12,14,14`, solid all the way down, so it is the same class of doorway
rather than a different problem.

Note the fine test is far more permissive for a tall rectangle: `5 * 16 - 15`
gives a 65-pixel window, against a single pixel for `1x1`. Any proposed
mechanism has to explain both.

## Requirements

- Find the mechanism that enters a one-cell door, given that the scan itself
  is decoded and provably cannot fire from a walked position.
- Explain the doorway snap: entering `$000F`'s doorway moved the player's x
  from 399 to 392 under Down alone.
- Reconcile the collision sample with the exit probe. Sweeps measured collision
  at `dx` in `-2..=0`, `dy` in `-12..=-8` from the player word, while the exit
  probe is `(x-8, y-16)`. The door evidence suggests they are the same corner
  and the sweep bound is loose; settle it.
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
