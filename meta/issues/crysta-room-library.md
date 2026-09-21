# Promote the Crysta room builder into a library

## Summary

`crates/map-inspector/tests/local_crysta_rooms.rs` builds a `room_core::Room`
for every one of the 24 Crysta maps, installs the qualified material policy,
and walks the player to an exit approach in each. That is the free-roam
foundation, and it is only reachable from a test. Promote it into a library so
a runtime can use it.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md)
- [Trace and qualify the movement collision predicate](qualify-collision-predicate.md)

## What moves

From the test file, unchanged in behaviour:

- `crysta_room(rom, map) -> (Room, width, height)` — decodes the map's layer and
  512-byte attribute table through `StaticBackground`, builds the cell grid from
  `attributed_cells`, and applies the policy below.
- `qualified_policy(map, width, height)` — the `MaterialRule` set that opts maps
  into the aliases `room-core` leaves undecided by default: `TownSolid25` across
  map `$0A`, and `ClosedDoorPartial5` / `StairOpen29` scoped to the specific
  cells on `$0C`, `$0E` and `$020`. The alias scopes are validated by
  `room-core`, so a wrong cell is rejected rather than silently accepted.
- `arrival_starts`, `exit_approaches` and `landing_cell`, which the transition
  work needs.

No sample halo is installed. That is the point: a `Room` without
`with_sample_halo` admits every cell its material policy allows, which is what
separates free roam from the qualified corridor.

## Requirements

- The library is the single definition; the test consumes it rather than
  duplicating it, and its assertions keep passing unchanged.
- The policy stays data, not code paths: a map with no entry gets an empty rule
  set, and adding a map does not mean editing a `match` on behaviour.
- Decode failures stay fatal and name the map. A room built from a map whose
  attributes did not decode is worse than no room.

## Acceptance Criteria

- All 24 maps build a `Room` through the library.
- `local_crysta_rooms.rs` passes with its builder removed, consuming the
  library instead.
- The 296 cells across 6 attributes that remain unqualified are still refused
  rather than guessed, and the refusal names the attribute.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Play the Crysta slice free-roam in a native app](free-roam-crysta-app.md)
- Where this lands matters: `map-inspector` is a qualification tool and should
  not become a runtime dependency. A new crate is the likely home.
