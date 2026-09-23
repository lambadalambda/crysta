# Lift and throw pots to break the blue door

## Summary

The blue door in C breaks after two pot hits, counted by controller `$838C32` in `$0640`, and the second hit sets `$292` and patches the stairs.

## Dependencies

- [Run scripted movement, entry scenes and map transfers](scripted-movement-scenes.md)

## Requirements

- Lift, carry and throw a pot with the player's inputs, from source.
- Door hit detection, counter, `$292`, stair patch and the reaction scene.
- The blue door and stairs metatile/collision patches (from the geometry issue).

## Acceptance Criteria

- Throwing from (184,368) facing up hits; (136,368) misses, as natively.
- After two hits the stairs open; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Source specification (research, verified unless marked)

- **Pots are tiles**, not actors: C cells (3,21) `18FA`, (4,21) `18FB`,
  (5,21) `18FA`. Lifting (`$87:9683`, native player code on A facing the
  cell) replaces the cell with `$00F8` (`COP 45`), hands a shared carry actor
  (`$0DF4`) the pot, and returns control 23 frames after A.
- **Throw**: A while carrying; the pot stays in the hand 18 frames, then
  flies (Up: `$84:C4A0`, vector `$7C`) from launch Y-11 at -3 px/frame until
  a wall stops it (`COP DA` hook), then shatters. Hitting the door does not
  stop it. Native: from (184,368) samples y 357, 354, 351, 348, break at
  (184,321); from (136,368) it misses.
- **Hit**: `$85:D281` pairs attackers (`+$04 & $0400`) with targets (`+$04 &
  $0200`, set by `COP 65`) by frame rectangles; a hit sets the target's
  16-frame cooldown and redirects its script to `COP 65`'s target.
- **Door** `$83:8C32` (`$88:AAEE`): hit path `$AB81` adds 1 to `$0640`
  (`COP 4B`), dispatches on the count (`COP 4A`): hit 1 patches (11,21)
  `$0581` and (11,20) `$05A7` (`COP 44`), shows `$88:A15F`; hit 2 patches
  `$04CB`/`$04F6`, stamps both cells (`COP 3D`), **sets `$292`** at
  `$88:ABEE`, spawns a cosmetic helper (`COP 6A`), waits for local 6, lifts
  the stamps (`COP 3E`) and deletes itself.
- **Reaction**: the friend (`$88:9B73`) and residents `$88:A21D`,
  `$88:A3A0` hand off through locals 4..9 with a spawned fade child
  (`COP A2`, `$88:9CD8`); control returns natively at 24853.
- `COP 44` (`$80:949B`): x, y (signed tiles from the actor's cell), word:
  bits 0-8 tile, bit 9 layer path, high byte >> 2 a delay; stores
  `(attr & $7F) << 9 | tile` and queues the metatile.

## Progress

- Tile patches (`COP 44`/`42`) run and are drawn; patches survive moves
  between maps sharing the first layer, derived from the layer's source.
- The door's services run: hit target and return (`65`/`66`), counter
  branch (`4A`), stamps (`3D`/`3E`), spawns (`A2`), cosmetic helpers
  (`31`/`32`/`33`/`37`/`6A`) and display-only native code. A test strikes
  the door twice (host `World::strike`): `$292`, the stair patches, the
  reaction to local 9 and control return; the patches are still there in B.
- Text `$C4 0` (a page without its window) now decodes outside Pandora.
- Remaining: lifting, carrying and throwing pots with real presses, and the
  pot's hit test against the door. Also flag-gated loader patches, which a
  layer reload (for example from `$21` back to `$20`) needs to show the
  opened door.
