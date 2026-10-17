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
- `COP E4`/`E5` in a callback yield the actor instead of ending the
  callback; a foe's wake does not reset their countdown, which the native
  knockback rewrites; `E5` (as `8F`) does not restart the streams each pass.
- `COP 3C` drops the actor's mark wherever it was set; natively it clears
  the current cell (`$80:BF0E`), also a `COP 3D` mark there. `COP AF`'s
  streams restart on their own list clock, not the running list's.
- Effects (`COP AA`): `7F:0100` (the parent) is not kept; a group deletion
  can remove one, natively not; `COP EC` with a count of 0 runs once and a
  total of 0 divides to `$FF` natively; on a non-effect actor `EC` never
  ends. `COP 93` waits for good (the light room's cycle end is not
  modelled). The ring: the `$24 == $06` and `$24 < $06` cases
  (`$87:A340`, `A307`), and the stale row past the bottom table.
- The ring's light (`frame::lighten_in_ring`) lights the composed
  background: natively a BG3 pixel there stays unlit, a clear BG1 pixel
  shows the backdrop alone, and outside the ring the clear pixels show the
  raised backdrop. `backdrop_addition` ignores `W12SEL`'s invert bit,
  `CGADSUB`'s half bit and `CGWSEL`'s clip and prevent bits; the light is
  drawn before the mosaic, natively after. The runtime's backdrop is a step
  behind (`$42` phase): 21 steps where the native frame has 22.

## Acceptance Criteria

- Each case is traced against the ROM and fixed or confirmed.
