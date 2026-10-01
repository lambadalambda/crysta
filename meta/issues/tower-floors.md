# Load every tower floor with its stairs and chests

## Summary

The towers' floors (`$100-$123`, `$106`) do not load: backgrounds, collision, camera, stair exits, chests (flags `$900+n`) and their hint texts.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Admit and decode every tower floor on both ROMs; open chests and grant their items.

## Acceptance Criteria

- Every tower floor can be walked and left by its exits; chests open as natively.

## Notes

- 2026-10-02: all floors load (`$106` too). Chests open from the table `$96:D10F` (`docs/chests.md`; the `$0F9` kind and "I have enough" are open). The Magirocks (`$84:DD7E`) show and are taken. Hint texts are not checked.
