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
