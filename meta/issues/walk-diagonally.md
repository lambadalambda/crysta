# Walk diagonally

## Summary

The runtime takes one cardinal direction a frame (`room_core::FrameInput`),
so Ark cannot walk diagonally, and the diagonal dash cannot start. The
hosts send one direction too.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Diagonal input from the hosts to the world.
- Diagonal walking with its collision and facing, from native traces.
- The diagonal dash is done (`meta/issues/dash-diagonals-attack-jump.md`).

## Acceptance Criteria

- Diagonal walks and dashes match native frames.

## Progress

- Done: diagonal walking in `room_core` (`PadInput`, snapshot
  version 4), the world (`World::update_with`) and the native app (two keys,
  the d-pad, the stick on both axes); six native traces on `$101` match frame
  by frame (`ark_walks_diagonally_as_natively`).
- Open: the web hosts still send one direction; jump steering, the rope, the
  plane and the pots take only the lead.
- Not traced yet: releasing the lead (not the cross) of a diagonal; a
  diagonal double tap (here a dash along the lead); a diagonal into a corner,
  where both axes' nudges can add up.
- Edges from the review: a pot's lift or rebase keeps a stale delayed cross
  for one frame; the app's edged presses and its trace log take the main
  direction only, and the app hands the cross to the session in a field, not
  with the frame; `room_core::run::step` reads the raw direction, not the
  lead (the world gives it the lead already).
