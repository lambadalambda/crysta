# Jump with B

## Summary

Ark's jump (B) is not modelled: `Presses` has no jump, and Ark is never in the air. The ROM's jump scripts (from `$84:9252`) clear Ark's `+$04` bit `$10` (on the ground) while he is in the air; `$80:CB97` keeps that, and the fall's `$097C & 4`, in `$0986` bits 10-13 for this and the previous frame.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Trace the jump: its height stream, its poses, its frames, and what it may cross.
- On a rope (`$84:9C95`): in the air, Ark wobbles off and falls (`$84:9BC6`); landing (`$0986 & $3C00`), he leans by frame parity (`$84:9BF8`).
- The jump from a dash and the throw from a jump: [dash-diagonals-attack-jump](dash-diagonals-attack-jump.md), [pot-extras](pot-extras.md).

## Acceptance Criteria

- B jumps Ark as natively, on both ROMs, and the rope reacts to it.
