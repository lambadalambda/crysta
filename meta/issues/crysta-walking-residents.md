# Walking residents: execute the ordinary loop

## Summary

Residents stand where their record puts them and cycle one pose. In the game
several of them wander. Their scripts do it with a random-walk service inside
the ordinary loop, so the runtime needs to execute that loop rather than read
it once.

## Dependencies

- [Solid residents, the bedroom start, and resident animation](crysta-solid-animated-residents.md)

## What is already known

- Four drawn residents share the walking loop: `COP 59 n`, then `COP 23
  <loop>`, an optional mirror, `COP 26 c0 c1 r0 r1`, `COP 8F`, `COP 3B` and a
  branch or `BRA` back to the loop.
- `COP 26` (`$80:8E56`) draws a byte from the RNG at `$86:8236`. Bit 2 set
  means stand still; otherwise the low two bits pick down, up, left, right.
  The four operands are a tile rectangle, minimum and maximum column then
  minimum and maximum row, and a step that would leave it is refused. The
  handler's compares let the actor one cell past the far operands, so the
  reachable range is one wider and one taller than the operands read. The
  destination is probed through `$80:C092`/`C0AD`/`C0FB`/`C116`, which
  accept a cell whose attribute (word bits 9 to 15, so an occupied cell's bit
  15 counts) is 0, 1 or 22 and refuse anything else.
- An accepted step sets the actor's sequence from a table at `$80:8F6D`:
  down 3, up 4, left and right 5, with bit 14 of the facing word set for
  left. `COP 8F` then waits for the animation to resolve before the loop
  continues. A refused step or an idle draws zeroes the velocity.
- Standing sequences are 0, 1 and 2 by facing, as the player's are.

## Requirements

- A bounded interpreter runs each present resident's entry script from its
  first command: pose, mirror, waits, the random walk, flag branches and
  despawns, and the native `BRA`/`JMP` that close the loop. Any other native
  opcode or unaccounted service freezes that resident where they stand.
- A walking resident moves one tile per accepted step, faces the way they
  walk, shows the walking sequence for that direction, and stands facing the
  same way afterwards.
- Bodies block each other and the player, and cannot step onto the player.
- The RNG is the runtime's own; the game's sequence is not reproduced.

## Acceptance Criteria

- Over a long run the map-`$000D` wanderer moves, never leaves their
  rectangle and never stands in a cell the probe would refuse.
- Static residents do not move.
- Reachability across the slice is unchanged with residents walking.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)
- [Cadence qualification](../../docs/native-crysta-timing.md) now establishes
  the map-D ordinary class-0 action:32 ticks per tile with signed1/0 deltas,
  16 ticks idle/refused. `$8F32` selects a velocity stream and one **list
  repetition**, not a one-record countdown. Other movement profiles remain
  approximate.

## Progress: the loop runs

`crysta_runtime::actors::Actor` executes each present resident's entry script
from its first command, one frame at a time: pose and mirror selection, the
two waits, the random walk, the flag branches and despawn, the near-player
branch, and the native `BRA`/`JMP` that close the loop. Anything else
freezes the resident where they stand. The world ticks every actor after the
player's step, feeds each one the player's cell and every other body's cell
and destination as occupied, and rebuilds the collision room from the base
whenever a body's cell changes.

Over 4,000 frames the map-`$000D` wanderer visits several columns of their
rectangle, columns 4 to 11 on rows 41 and 42, and never leaves it, shows walking sequences while moving,
and is solid wherever they stand; the residents without a walk loop do not
move; reachability across the slice is still 19 of 24 maps.

### The flag branch was inverted

Running the wanderer's loop exposed it: `COP 08` had been evaluated with the
spawn stream's `$FA` rule, which is the opposite sense. `$80:8678` branches
when the flag is clear for a word without bit 15 and when it is set for a
word with it. The documented resident's six-way dispatch therefore falls
through to the documented pages on a **new game**, which is what
`docs/house-dialogue.md` recorded all along; the "already met" premise the
tests carried was an artefact of the inversion. The talk census moved from
16 to 22 speaking residents.

### Two more services, one of them modelled

`COP 0F id tx ty target` (`$80:888A`) branches on whether the player stands
within sixteen pixels of a map position given in tiles, with bit 7 of the
id inverting the sense; the wanderer uses it to notice the player at the
room's entrance. `COP 2E word target` (`$80:90AC`) branches on a bit of
`$0454`, which is not identified; the runtime never takes it, so the
wanderer's greeting on approach is not shown.

### Not reproduced

- Other movement profiles still use an approximate eight-frame tile step;
  only the source-bound map-D ordinary action has qualified velocity/list timing.
- The RNG: the runtime's own xorshift, seeded per map and record.
- `COP 59` turned out to be the scene pause, not a talk pause, and what
  stops a walker for the player is `COP 23`; both are executed now. See
  [walkers stop and face the player](crysta-walker-stops-for-player.md).
