# Match the flyers' pose 7

## Summary

The flyers `$97:BD3E` show pose 7 (`COP 81 07`, `$97:BD86`) for 33 frames natively and 50 in the runtime, then natively pose 6 for 50 frames before the chase (`$97:BE80`). Their random steps then differ, and on `$117` with Ark at (376,570) the Guardner's callback takes the other branch natively (`$0408 & $80`: a push and no sleep).

## Dependencies

- [Match the Guardner's sleep and vacuum](guardner-fidelity.md)

## Requirements

- Trace the flyer's pose lists and fix the duration.

## Acceptance Criteria

- On `$117` with Ark at 570 the flyers step the generator at the native frames (9, 42, 92, 94, 110, 126, 127).

## Progress

- Done: not the pose's length. The flyers fly up into the wall at y 432; `COP DA` set their Up callback (`7F:1006` = `$97:BD41`), and the wall probe's wake restarts them at frame 42 (`$80:E15C`, `$80:C967`); `COP 80` then stops the old streams at the list's end (`Motion::repose`). With the native generator (`World::set_random`) the flyers match the native trace to frame 462 (`the_flyers_wall_wakes_end_their_pose_waits`).
- Not modelled: the tile classes that raise bits 6-8 instead of the side's bit (`$80:D642`).
