# Jump from a dash, with a pot and on a rope

## Summary

The dash jump, the carry jump and the rope jump differ from the ground jump (`docs/jump.md`, "Other jumps").

## Dependencies

- [Jump with B](jump.md)

## Requirements

- The dash jump: stream `$3A`, the dash's speed, lists `$0D`/`$0E`, the slide.
- The carry jump: no crouch, lists `$1B`..`$22`, the pot's height.
- The rope jump and the landing on a rope: the lean by `$0042` parity.
- The shadow under a jump (`$84:A917`, its list by `|h|`).
- A re-jump only while B is still held at the landing (`$0454`).

## Acceptance Criteria

- Each matches `docs/jump.md` on both ROMs.

## Progress

- The dash jump runs, frame for frame with the native trace on both ROMs (`b_in_a_dash_jumps_on_with_the_dash`). A new dash jump in the grace takes B alone (natively B with the dash's way).
- The carry jump runs, frame for frame with the native traces (Right, Left) on both ROMs (`b_jumps_with_the_pot_as_natively`). An exit in the air puts the pot down. Open: the jump-throw (A in the air, [pot-extras](pot-extras.md)), the rope jump, the shadow, the held-B re-jump.
- The shadow is drawn (`world/shadow.rs`): its lists match the native traces of the ground jump. Open: its row move over lips (`$84:A957`, hidden here instead).
