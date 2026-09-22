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
the opening including `x=496`. No transition occurs. (That sweep is in any
case not a control for this door: the probe formula puts `x=496..503` in tile
column 30, and the door is column 31.)

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
which `ExitList::probe_origin` now reproduces. The scan requires `$097C` bit 4
**clear**: `$8D:87A5 BEQ $87AA` continues when clear; otherwise `$87A7`
jumps to the return tail. `$0480` must also be nonzero. This corrects the
original opposite-polarity annotation (whose `$879F` was an operand byte),
verified directly against the owned ROM during return-arrival qualification.
Frame-end samples do not establish the gate value at every scan invocation.

**This makes the conflict exact.** The house's own front door is the `1x1` rect
at tile `(31,46)`. Firing it needs `probe_y` in `736..=751`, so player `y` in
`752..=767`. Holding Up there stops at `y=768`, one pixel short, because the
collision sample and the exit probe are the same bounding corner: moving to
`y=767` would put the corner at `751`, inside tile `(31,46)`, which is the
solid attribute-14 door cell. The exit requires the corner to be exactly where
collision forbids it.

So these doors are not entered by ordinary walking, and no amount of alignment
fixes that. The already-qualified `room-core` collision agrees independently:
asked how far Up the player can reach at that door, it stops at `y=768`,
probe tile `(31,47)`, from every starting x.

## The mechanism: face the door and interact

Measured live. Standing at `(504,768)` in the town facing Up, pressing the
action button and then holding Up walks the player **through** the solid
attribute-14 door cell:

```text
f12015  Up   pos (504,768)  ctrl 160  probe tile (31,47)
f12016  Up   pos (504,767)  ctrl 160  probe tile (31,46)   <- past the stop
f12026  Up   pos (504,752)  ctrl 160  probe tile (31,46)
f12027  Up   pos (504,751)  ctrl   0  gate $8000           <- transition begins
f12043  Up   pos (504,735)  ctrl   0  gate $8000
f12044       map $000A -> $000D
```

Collision is suspended for that walk: `y=767` and everything below it are
positions ordinary movement refuses. Holding Up for 150 frames *without* the
action press leaves the player at `y=768` indefinitely, so the interaction is
what opens the door.

This is the same pattern `docs/playable-house.md` already documents for the
house's interior wooden door — face it, release movement, press Interact —
which is corroboration rather than a new mechanic.

**Modelling an exit rectangle as enterable from an orthogonally adjacent
walkable cell reaches all 24 maps**, up from 6, asserted by
`doorway_interaction_connects_the_whole_slice`.

Measured on that predicate, **entry from below alone also reaches 24/24**;
lateral-only and above-only each reach 6. So no map in the slice needs a
non-below entry, and the lateral and above arms are unexercised. The real
untested generalization is therefore from the measured `1x1` door to the `1x5`
rectangle that `$000C` uses for `$000B` — not from below to four sides. `$000B`
is itself entered from below, at `$000C` cell `(8,21)`.

One observed lead for the alignment question: walking into the *working*
`$000F` doorway snapped the player's x from 399 to 392 with only Down held, so
the game does move the player during a doorway approach.

## Remaining

- Only one door was confirmed live, a `1x1`. The other seven town entrances
  and the `1x5` rectangle into `$000B` are covered by the model but not
  measured, and the fine test behaves very differently across those sizes: a
  `1x5` admits a 65-pixel window where a `1x1` admits one pixel.
- The ~100-frame `control == 0` window after the action press is **not**
  door-specific: the same signature appears when interacting with a plain wall,
  where nothing opens. It is not evidence on its own.
- The routine that suspends collision and drives the walk-in is not located.
  `$097C` bit 15 is set throughout it, and `$097C` bit 4 suppresses the exit
  scan, so that word is where to look.
- Nothing is implemented in the portable core yet: `room-core` reproduces the
  refusal, not the interaction.

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
