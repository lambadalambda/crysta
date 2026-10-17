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
