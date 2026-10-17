# Throw pots while dashing or jumping, and show their fall and fragments

## Summary

The generalized pots (`docs/pots.md`) leave out a dash-throw (released at
once, 6 px a frame) and a jump-throw (the jump's height added), each
measured once natively; the fall a pot shows as an exit drops it
(`$84:C5CB`, stream `$6A`); and the fragments a break leaves (`$84:C7B8`,
frames `$A2:E2BD` onward).

## Dependencies

- [Lift and throw pots as the native game does](throw-pots-freely.md)

## Requirements

- Trace the dash-throw and jump-throw sources and measure them again; draw
  the drop's fall and the break's fragments.

## Acceptance Criteria

- The `throw-right-dashing`, `throw-left-jumping` and `exit-while-carrying`
  fixtures (`local/pots/`) pass frame by frame on both ROMs, and the
  fragments match native frames.

## Progress

- The jump-throw runs (`PotState::throw_in_air`, `world/jump.rs`): the fixture `throw-left-jumping` matches on both ROMs; Ark's `$2A`/`$2B`/`$2C`, stand at J+24, control at J+25.
- Open: the dash-throw (traced in the job's `tmp/throws/`: speed 6, the stream `$2D`, Ark's `$12`-`$14` of resource 3 for 17 frames, then the plain dash), the drop's own stream `$2E` (`$84:C6D3`, code only), the fragments.
- The carry dash exists natively: a double tap with a pot runs Ark's
  `$84:B5A7` with resource 1's `$1C`; A in it throws at speed 6 (`$84:B453`,
  resource 3's `$14`), then the plain dash. The trace `w_jp_dash` (the
  job's `tmp/throws/`) starts it against an obstacle (x +3 and y +6 over 8
  frames), so its stream needs a trace on open floor.
