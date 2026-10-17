# Close the later review edge cases

## Summary

Edge cases from the reviews of the towers, the jumps, the pots and the COP
inventory. None breaks a tested path.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- `World::join_list` places a `Head`/`Last` spawn as if there were one list.
- An enemy whose death runs from a callback can restart the death on more
  hits.
- The Shadowkeeper routines are matched by their address only, not by
  their bytes (`actors/shadowkeeper.rs`).
- `$93:DE4A` subtracts without `SEC`; the model's centre (`x - 32`,
  `y - 20`) may be one pixel off natively.
- A B press for a jump during a door or chapter frame waits for that frame.
- Flying pots advance in the other holding frames of a jump.
- A hit during a pot's flight in the air skips one step of the pot.
- The snapshot's check of an aloft pot (byte 38) accepts more than the
  runtime writes.
- The COP walk (`service_sites`): a choice table is taken as 3 entries; a
  switch entry that points into its own table would be read as code; an
  address is walked once, with the register widths of the first path.

## Acceptance Criteria

- Each case is traced against the ROM and fixed or confirmed.
