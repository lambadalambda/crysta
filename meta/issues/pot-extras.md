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
