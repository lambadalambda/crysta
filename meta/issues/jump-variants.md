# Jump from a dash, with a pot and on a rope

## Summary

The dash jump, the carry jump and the rope jump differ from the ground jump (`docs/jump.md`, "Other jumps").

## Dependencies

- [Jump with B](jump.md)

## Requirements

- The dash jump: stream `$3A`, the dash's speed, lists `$0D`/`$0E`, the slide.
- The carry jump: no crouch, lists `$1B`..`$22`, the pot's height.
- The rope jump and the landing on a rope: the lean by `$0042` parity.

## Acceptance Criteria

- Each matches `docs/jump.md` on both ROMs.
