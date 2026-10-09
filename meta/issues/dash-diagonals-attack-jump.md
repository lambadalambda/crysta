# Dash diagonally, and attack and jump from a dash

## Summary

The dash (`room_core::run`) runs only in the four directions. Diagonal dashes, the dash attack and the dash jump are not modelled.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Model each from the native code and a probe.

## Acceptance Criteria

- Each matches native frames.

## Notes

- Pot throws while dashing or jumping: [pot-extras](pot-extras.md).

## Progress

- The dash jump runs (`world/jump.rs`, `b_in_a_dash_jumps_on_with_the_dash`, the native trace `d_jp_right_held`). Open: diagonals, the dash attack.
