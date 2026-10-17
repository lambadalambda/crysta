# Dash diagonally, and attack and jump from a dash

## Summary

The dash (`room_core::run`) runs only in the four directions. Diagonal dashes, the dash attack and the dash jump are not modelled.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)
- [Walk diagonally](walk-diagonally.md)

## Requirements

- Model each from the native code and a probe.

## Acceptance Criteria

- Each matches native frames.

## Notes

- Pot throws while dashing or jumping: [pot-extras](pot-extras.md).

## Progress

- The dash jump runs (`world/jump.rs`, `b_in_a_dash_jumps_on_with_the_dash`, the native trace `d_jp_right_held`).
- The dash attack runs (`world/attack.rs`, `a_in_a_dash_is_the_dash_attack`, the native traces `g_jp_*`, all four ways).
- Diagonals run (`room_core::run`, `ark_dashes_diagonally_as_natively`, 15 native traces `h_*`/`q_*` on `$101`; `a_diagonal_dash_attacks_straight_and_jumps_on_diagonally`, `q_attack_ur`/`q_jump_ur`). The same traces fixed the plain dash's timing: a still frame more at the start, turns without a still frame, the dash attack from the frame after A, the stream's restart after a dash jump, the wall's `$4F`; cell type 15 is solid.
- Done; the untraced inputs inside it: [Trace the dash's open edges](dash-edges.md).
