# Trace the pots' open edges

## Summary

The pots' throws, dashes, jumps, drops and fragments match native traces
([Throw pots while dashing or jumping](pot-extras.md)). Some inputs and
cases inside them are not traced; the model guesses.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Trace each case and fix the model where it differs:
  - the carry dash's control word (`$0980`: the model keeps the walk's
    `$20`, natively `$10`) and its last frame's pose (the model stands,
    natively the walk's pose);
  - the carry dash jump: the ground test from J+24 to J+29 (skipped), a
    stream restart after the landing, B again in the landing's frames, one
    native frame of the other vertical list after the landing;
  - after a dash-throw: A and B in its dash, a script's pad lock while its
    pot flies, a perpendicular or opposite way or a diagonal in it;
  - the exit drop facing Up, Left or Right (the model takes Down's point);
  - the rope's drop (`$84:C5CB` too; the model puts the pot away at once).
  - a thrown pot or a dropped one while Ark walks out of an exit (the model
    flies it on, and it can strike actors), and broken pots' fragments in
    frozen frames (the model animates them on).

## Acceptance Criteria

- Each case matches a native trace.
