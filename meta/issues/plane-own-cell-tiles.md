# Model the plane's own-cell tiles

## Summary

Walking on a world plane (`plane.rs`) ignores own-cell tiles `$88..$8F` (`$80:C440`). `$80:C440` returns at once unless `$048A` bit 13 is set (the spawn list's header byte `$20`); with it, `$80:C469` inverts the plane's rule: `$A0..$BF` open, `$88..$8F` a special case (carry set, A = 0), the rest blocked (guess: the sea travel). The underworld `$03` (header `$40`) never sets it.

## Dependencies

- [Complete Chapters 2 and 3](complete-chapters-two-three.md)

## Requirements

- Find the maps with header bit `$20` and what reads `$80:C469`'s A = 0, then model the mode in `plane.rs`.

## Acceptance Criteria

- A test on a plane cell `$88..$8F` gets the native result.
