# Run a spawn linked after its parent in the same frame

## Summary

`COP A2` links its child after the parent (`$80:BC7C`), and the list walk runs it in the same frame: the Guardner's bolt is at (384,483) the frame it spawns. The runtime runs spawns from the next frame (`World::spawn_actors`), so the bolt hits a frame late.

## Dependencies

- [Match the Guardner's sleep and vacuum](guardner-fidelity.md)

## Requirements

- In the actor pass, after each actor, link its `ListPlace::After` spawns and run them next.

## Acceptance Criteria

- The Guardner's bolt is at (384,483) on its spawn frame.
- `the_high_cadet_falls_after_three_real_hits`, which failed in a trial, is checked against a native trace and passes.

## Progress

- Done: after each actor its spawns join the list and the walk goes on from it (`World::walk_actors`); a group deletion before them applies first. The bolt is at (384,483) on frame 107 on both ROMs (`the_guardners_bolt_moves_on_the_frame_it_spawns`); the full suite passes, the high Cadet's test with it.
