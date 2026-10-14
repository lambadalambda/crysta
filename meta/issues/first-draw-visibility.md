# Keep the header's off-screen bit until the first draw

## Summary

A record header's `+$04` can set bit 14 (off screen; the Guardner `$97:C345`: `$C220`, the flyers `$97:BD3E`: `$4204`). Natively the draw pass (`$80:E9A2`) clears it only after the actor's first draw, so its first `COP 59` sleeps; the runtime reads only the actor's place, and the Guardner wakes 31 frames early (`docs/enemy-scripts.md`).

## Dependencies

- [Match the Guardner's sleep and vacuum](guardner-fidelity.md)

## Requirements

- Keep the bit per actor from the header; clear it as the draw pass does; let `COP 59` and the off-screen test read it.
- The draw pass's rule: the sprite box `E+$18`..`$1E` against the 256x224 screen, an edge on it, or the first edge before it and the far one on it; hidden actors keep the bit.
- The Guardner's sprite box: `foe::Boxes` gives (-8,16,-16,16) for its selectors 3, 5 and 7; natively it changes from (6,5,2,3) to (16,40,15,8) through pose 5.

## Acceptance Criteria

- On `$117` the Guardner first shows selector 5 at frame 31 for Ark at 570, 576, 590 and 600, and never for Ark at 604 (native traces in the job's `tmp/guardner2/`).
- The two tests the change touched in a trial (`qualified_wanderer_actions_match_native_cadence_without_gap_ticks`, `european_single_world_replays_bedroom_to_underworld`) are checked against native traces and pass.

## Progress

- Enemies keep the header's bit until the draw pass after the actors sees them on the screen (`Actor::drawn`); `COP 59` and the off-screen test read it. The Guardner wakes at frame 31, 7 at 93, 4 at 125 on both ROMs (`the_guardner_sleeps_its_first_thirty_frames`); the full suite passes. Only enemies take the bit: other records' header words are not `+$04` (a trial on all records broke the Crysta wanderer).
- Open: the sprite box (the Guardner wakes natively with Ark at 600, not here).
