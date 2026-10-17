# Model COP E4 and E5

## Summary

`COP E4` (7 sites: `$90:8CF6`, tower 5's `$97:C74D`..`C934`) and `COP E5`
(6 sites, `$97:C78E`..`C96A`, the boss room `$113`) are stepped over. The
docs name them in sprite flicker and hiding (`docs/enemy-scripts.md`,
knockback `$85:E03D` uses `7F:200A` for `COP E4`).

## Dependencies

- [List the COP services the underworld scripts use](cop-inventory.md)

## Requirements

- Read the handlers and model both services in the runtime.
- Check the result against a native trace of one site.

## Acceptance Criteria

- A unit test per service, and a local test that matches the native frames
  at one site.

## Progress

- Done: `Actor::own_wait` (the script runs each frame, back from its
  continuation, until the list has played); the High Cadet's copies on
  `$113` match the native trace (150 frames for 3 passes of 50).
