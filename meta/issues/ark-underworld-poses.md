# Show Ark's poses in the underworld scenes

## Summary

Several of Ark's poses are not drawn; the logic runs, but his art stays as it was.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Sleep (`COP 84`/`89` on his own script, the Guardner's sleep).
- Falling, rope, lean and landing; the landing's drop from 256 pixels up on `$114` (`$90:FA4E`); a hit's lean on the rope.
- The lip jump.
- Victory and recovery at a level-up, the both-hands lift at a chest, the Magirock lift.

## Acceptance Criteria

- Each pose matches native frames.

## Notes

- Markers: `world/fall.rs`, `world/levelup.rs`, `world/chest.rs`, `world/magirock.rs`, `actors.rs` (`ark_pose`).

## Progress

- `World::ark_pose` names the list of Ark's resource each state shows, and the app draws it: the fall (0's `$18`), the lip drop (0's `$13`/`$15`), the rope (1's `$0F`, `$10` with the spear's colours, `Mode4Art::with_weapon`; the lean's 6/7), the burn (5's 4, 5), the level up (0's `$1D`, then 2's `$20`), the chest (3's `$39`, `$3A`, then 0's 3/4/5 by facing), the Magirock lift (3's `$18`..`$1A`), and his script's own lists (`COP 84`/`89`: the Guardner's sleep).
- The landing on `$114` (`$90:FA4E`, `world/landing.rs`): hidden 5 frames, 0's `$13` from 256 pixels up over the high tiles, 0's `$14` and sound `$0F` unless the cell is `$13`.
- The drop with a pot (`$84:9F13`/`9F33`): resource 3's `$1D`/`$20` and the pot's `$25`/`$26` (`PandoraCarryMotion::Dropping`). Natively unreachable on `$117` (its holes meet pits first), but its pits take a carrying Ark: the pot flies from 16 pixels ahead (`$84:C649`).
- "A hit's lean on the rope" was a wrong guess: `$0986 & $3C00` is Ark in the air or falling, now or a frame ago (`$80:CB97`), so the rope's lean on landing needs the jump: [jump](jump.md).
