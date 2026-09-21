# Walk between Crysta maps through the real exit geometry

## Summary

Free roam needs the player to leave a map by walking into its exits and arrive
somewhere correct in the destination. The exit geometry is decoded and the
door-entry mechanism is measured; what is missing is a runtime that applies
them.

## Dependencies

- [Promote the Crysta room builder into a library](crysta-room-library.md)
- [Decode the door-entry trigger for one-cell exits](decode-door-entry-trigger.md)

## What is already known

- `assets::maps::exits::ExitList` gives each map's exit rectangles and their
  destinations.
- The scan at `$8D:8797` tests the probe written at `$87:91B3` as
  `(x-8) & (mask_x|$0F)` and `(y-16) & (mask_y|$0F)`, so an exit is matched
  against a wrapped probe rather than the player's raw position.
- Some exits are doorways that require facing and an interaction rather than
  walking through, measured live. Most town entrances sit on cells the player
  cannot stand on, which is why walking alone reaches only part of the slice
  and doorway interaction is what connects it.

## Requirements

- Transition on the probe geometry the game uses, not on the player's bounding
  box overlapping a rectangle.
- Distinguish walk-through exits from doorways that need an interaction, and
  drive both from the decoded data rather than a hand-written table.
- Place the player on arrival at a cell they can stand on, derived from the
  destination's own data.
- An exit whose destination is outside the slice is refused as movement — the
  player does not pass — rather than transitioning to an unloaded map.

## Acceptance Criteria

- Every bound doorway in the slice can be entered and leaves the player
  somewhere standable.
- A round trip returns the player to a cell adjacent to the exit they used, not
  to an arbitrary arrival point.
- The whole slice is connected: a walk plus doorway interactions reaches all 24
  maps from the opening house.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Play the Crysta slice free-roam in a native app](free-roam-crysta-app.md)
- `local_crysta_rooms.rs` already proves connectivity offline with
  `reachable_maps`; this is that logic made live.

## Progress: 19 of 24 maps reachable

`crysta_runtime::world::World` walks the slice and moves between maps.
Reachability is measured by breadth-first search over `(map, cell)` where every
edge is a real `World` step, so a map counts only if the player could walk
there. From the opening house: **7 maps by walking, 19 once doorways open.**

### Walk-through and doorway are different tests, and the data says why

A town entrance is a single cell the player cannot stand on, with exactly one
standable neighbour below it, and its trigger is **one pixel tall** — the fine
test is `origin - tile*16 < height*16 - 15`, which for a 1x1 record is `< 1`.

For the door into `$001D`, exactly one player position in the whole map
satisfies it, `(904, 752)`, and collision stops the player eight pixels short at
`(904, 760)`. It is not reachable by walking, and no amount of walking will fire
it.

But the player standing in front of the door already has their collision origin
*in the door's own tile*; it is only the sub-tile position they cannot reach. So
a doorway is matched on **tiles** and a walk-through exit on pixels. Making that
distinction took reachability from 7 maps to 19.

### Arrival must disarm the trigger

The player arrives standing on geometry that is usually an exit in its own
right, because a doorway leads back the way it came. An arrival therefore
disarms the trigger and stepping clear of every exit rearms it, or two maps
ping-pong forever. `an_arrival_does_not_immediately_bounce_back` asserts this
for every arrival the slice declares.

## Remaining: the five missing maps are one collision gap, not five problems

Chased to a single cause. **The doors are fine** — placing the player at the
approach cell and interacting opens every one of them, including `$1B` and
`$20`. What fails is *walking to* the approach.

The town is fragmented by undecided collision attributes. Probing every
standable cell in every direction:

| Map | Clean probes | Refusals |
| --- | ---: | --- |
| `$0A` town | 14,247 | `UnsupportedType(6)` 647, `(7)` 518, `(8)` 261 |
| `$1A` | 2,288 | none |
| `$12` | 3,417 | `UnsupportedType(29)` 27 |

`room-core` fails closed on an undecided attribute, so those 1,426 refusals are
walls. They split the town into disjoint walk components: from the `$000D`
arrival the player reaches 1,671 of the town's 3,951 standable cells, and the
component containing the `$1B` door approach is a separate 265-cell pocket with
**zero** overlap. `$1A` and `$1C` are only reachable through `$1B`, and `$1B`
only from the town, so one pocket boundary costs three maps.

A note on an earlier claim: `docs/collision.md` records the six undecided
attributes as affecting 296 cells, and I had read that as not affecting
connectivity. That was wrong. It is true of the *cell-level* model, where
`regions()` puts both components in one region; it is false of movement, where
the same attributes fail closed.

### The existing sweeps cannot settle it

Attributes 6, 7 and 8 are not merely unreported — they were never visited. Over
**20,071 walking frames** across all five town recordings the collision point
stood only on attributes 0 and 22, and sustained presses contacted only 0, 12,
14 and 25. Neither occupancy nor contact evidence exists for 6, 7 or 8.

So this needs new measurement, not re-analysis: sweeps that drive the player
into those cells from the reachable side. That is
[the collision predicate issue](qualify-collision-predicate.md), and it is what
blocks the last five maps.

`$20` and `$21` are the same shape of problem one map deeper: the `$0E` door
into `$20` opens when the player stands at its approach, so what is missing is
walk-reachability inside `$0E`.
