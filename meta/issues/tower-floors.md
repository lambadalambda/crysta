# Load every tower floor with its stairs and chests

## Summary

The towers' floors (`$100-$123`, `$106`) do not load: backgrounds, collision, camera, stair exits, chests (flags `$900+n`) and their hint texts.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Admit and decode every tower floor on both ROMs; open chests and grant their items.

## Acceptance Criteria

- Every tower floor can be walked and left by its exits; chests open as natively.
