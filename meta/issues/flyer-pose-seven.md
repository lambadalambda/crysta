# Match the flyers' pose 7

## Summary

The flyers `$97:BD3E` show pose 7 (`COP 81 07`, `$97:BD86`) for 33 frames natively and 50 in the runtime, then natively pose 6 for 50 frames before the chase (`$97:BE80`). Their random steps then differ, and on `$117` with Ark at (376,570) the Guardner's callback takes the other branch natively (`$0408 & $80`: a push and no sleep).

## Dependencies

- [Match the Guardner's sleep and vacuum](guardner-fidelity.md)

## Requirements

- Trace the flyer's pose lists and fix the duration.

## Acceptance Criteria

- On `$117` with Ark at 570 the flyers step the generator at the native frames (9, 42, 92, 94, 110, 126, 127).
