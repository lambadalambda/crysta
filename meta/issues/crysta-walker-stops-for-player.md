# Walkers stop and face the player who faces them

## Summary

A walking resident keeps walking when the player stands next to them and
faces them, so talking to a walker is a matter of luck. In the game the
walker stops, turns to the player and stands there for as long as the player
faces them. That is `COP 23`, which sits in every walk loop and has been
stepped over by its derived length.

## Dependencies

- [Walking residents: execute the ordinary loop](crysta-walking-residents.md)

## What is already known

- The loop head of every walker is `COP 59 n` followed by `COP 23 <head>`;
  the `BRA` at the bottom of the loop returns to the `COP 59`.
- `COP 23` (`$80:8D1E`) branches to its target, after turning the actor,
  when the player stands one cell away and faces the actor. Measured live
  on the room-B resident: the branch fired on the frame the player, walking
  up, reached sixteen pixels below them, and it wrote the resident's
  facing and set bit 9 of the slot word at +6.
- `COP 59 n` (`$80:9AC7`) tests bit 14 of the slot word at +4. When it is
  set the service rewinds the script pointer onto itself, stores `n` at
  +$0E, which the scheduler at `$80:C6F0` counts down before running the
  script again, and yields. That bit is set on every actor for a few frames
  around a map transition and never during a conversation, so it is a scene
  pause, not the talk pause the walking-residents issue guessed at.
- `$0454` is the held-button word: `COP 2B`, `COP 2D` and `COP 61` compare
  it against masks, and the trace shows `$0100` while Right is held and
  `$0400` while Down is. `$0956` is the player's facing, `$0966` their x and
  `$0968` their y less eight.

## Requirements

- `COP 23` is executed: when the player stands one cell from the actor on
  one axis, within eight pixels on the other, and faces the actor, the actor
  turns to face the player, selects the standing sequence for that facing,
  jumps to the target and yields for the frame. Otherwise the loop continues
  past the two-byte target.
- `COP 59` continues past its one operand byte. The runtime never raises the
  scene pause, and the service is recorded as that rather than left to the
  derived length.
- A walker who is stopped this way is on the grid, so the world's talk test
  finds them in the faced cell.

## Acceptance Criteria

- With the player placed one cell from the map-`$000D` wanderer and facing
  them, the wanderer does not leave their cell over a long run and faces the
  player. With the player facing away, the wanderer walks.
- The adjacency window matches the handler: nine to sixteen pixels on the
  approach axis, at most eight on the other; the facing must equal the
  direction from the player to the actor.
- The frame ends after a taken branch, so a player who stands facing a
  walker does not spin the interpreter into its budget.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Walking residents: execute the ordinary loop](crysta-walking-residents.md)
- Evidence: `tools/collision-qualification/trace.rs` run over the
  house-conversation route, which dumps every actor slot per frame, and the
  handler read with `tools/disasm65/disasm65.py`.
- `$0999` gates the branch in the handler and is zero through a whole
  conversation in the trace; the runtime reads it as zero.
- The handler has an overlap case for an actor within eight pixels of the
  player on both axes, which it resolves by trying Down or Up first and the
  horizontal side second. It is modelled because it is three lines, though
  solid bodies never reach it.

## Progress: done

`crysta_runtime::actors` executes both services. `COP 23` classifies the
player's offset as the handler does, in `approach`, and a taken branch turns
the actor, selects the standing pose and ends the frame; `COP 59` continues.
`Surroundings` carries the player's facing, which `World` supplies.

- Unit tests cover each side, the nine-to-sixteen and eight-pixel
  boundaries, the overlap case's two candidates, the facing-away case and
  the one-branch-per-frame yield.
- ROM-backed: with the player one cell right of the map-`$000D` wanderer and
  facing left, the wanderer stays in their cell for 600 frames and faces
  right; with the player facing up, the wanderer walks again.
- Reachability across the slice is unchanged: the world tests still pass.
