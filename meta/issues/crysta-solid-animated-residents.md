# Solid residents, the bedroom start, and resident animation

## Summary

Three things the native app gets wrong against the game: the player walks
through residents, the run starts in the wrong room, and residents stand
frozen on one frame.

## Dependencies

- [Derive resident art from spawn records across the Crysta slice](crysta-resident-art.md)

## What is already known

- `world::occupy` blocks a resident's collision cell and is off by default
  because a spawn list is not a cast list: blocking all 115 records took
  reachability from 19 maps to 2. The record-driven loader now tells bodies
  from scripts, so occupancy can be limited to bodies.
- Fresh startup places the player at `(304,112)` in bedroom `$000F`
  (`docs/new-game-bootstrap.md`).
- A resident's pose list holds several records with a duration each; the
  frozen loader keeps four for some residents at seven frames each, and the
  ordinary loop re-selects the pose and waits for it to resolve, so the list
  cycles.

## Requirements

- A resident whose record decodes to a body blocks their collision cell.
  Script-only records and refused records do not.
- The app starts in `$000F` at `(304,112)`.
- A resident's whole pose list is decoded, and the renderer cycles it by
  record duration.

## Acceptance Criteria

- Walking into a drawn resident stops the player.
- Reachability across the slice is measured with bodies solid and recorded
  here.
- The frozen four-frame residents decode four frames with the frozen
  durations and compositions.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Progress: done, and reachability did not move

- `Resident::body` is true when the record-driven loader decodes the
  record's art. `World::enter` blocks those residents' collision cells and
  no others. The reachability walk from the opening house still reaches 19
  of 24 maps, missing the same five as before, so bodies do not sit on
  doorway approaches; the script-only records that cost 17 maps last time
  are the ones left open.
- The app starts at `(304,112)` in `$000F`. The bedroom's three records are
  all script-only, so nothing but the player is drawn there.
- `HouseActor::from_records` decodes each pose list to its terminator with
  per-record durations, and the frozen four-record residents come back with
  four records at seven frames each, composition for composition. The app
  cycles a list by duration from a frame counter; a single zero-duration
  record holds.

Not done: residents do not walk about. The wanderer in `$000D` and the
map-`$0011` intro are actor-VM behaviour, and the walker does not execute
scripts.
