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
- Open: the landing's drop on `$114` (`$90:FA4E`, from 256 pixels up); a hit's lean on the rope; the drop while carrying (`$84:9F13`).
