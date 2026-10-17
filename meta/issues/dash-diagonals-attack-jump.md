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
- Diagonals: measured (`h_*` on `$101`): a diagonal double tap dashes with the vertical's facing and list, the vertical axis 3, 2, 2 and the horizontal 1 a frame; the 8-frame grace runs on diagonally; the brake slides on the vertical only; a wall stops it with `$4F`. They need diagonal input first: [Walk diagonally](walk-diagonally.md).
