# Trace the dash's open edges

## Summary

The dash, its diagonals, attack and jump match native traces
([Dash diagonally, and attack and jump from a dash](dash-diagonals-attack-jump.md)).
A few inputs inside them are not traced; the model guesses.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Trace each case on `$101` and fix the model where it differs:
  - input that changes on a turn's second frame (the model keeps the turn);
  - A on the dash's setup frame (the model lets it lift or talk);
  - a cross change: whether the dash pose starts over (the model counts on).

## Acceptance Criteria

- Each case matches a native trace.

## Progress

- Done (native JP traces `e_*` on `$101`, 2026-10-18): a turn moves on the
  old way until the new way is held (`Run::Dash::way`); A on the setup frame
  attacks, A on the tap's frame does nothing; a cross change keeps the pose
  counting. Tests: `a_turn_moves_on_the_old_way_until_the_new_is_held`,
  `a_on_the_setup_frame_attacks_and_on_the_tap_does_not`.
- Not traced, guessed: a dash jump after a turn (the facing counts as
  held), A on the setup frame with the way released (it attacks), turns
  and the opposite press of a carry dash after a turn (by the facing).
