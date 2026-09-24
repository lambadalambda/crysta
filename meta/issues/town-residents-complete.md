# Complete the town's residents

## Summary

A sweep of every slice map found residents whose scripts stop at
services the runtime did not model: six townspeople in `$0A` (`COP B0`,
`COP 22`), a watcher in `$11` and a resident in `$1D` (native reads of
`$097C` and `$07ED`), and the shopkeepers in `$1D`/`$1E` (a native shop
spawner).

## Dependencies

- [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Requirements

- `COP 22` (switch on the spawn parameter), `COP B0` (movement base),
  `COP 81`/`87` (poses with movement streams) as the handlers do.
- Native runs may read `$097C` and `$07ED`.
- The shops are a separate feature (menus, money, items).

## Acceptance Criteria

- No resident outside the shops stops at an unmodelled service in any
  story state; the town's two walkers follow the native route's path.

## Notes

- Native walker `$88:8336`: up 64 px in 123 frames, then four 16-px hops
  47 frames apart, a 438-frame cycle (departure journey, slot `$10C0`).

## Resolution

- Implemented; the runtime's walker matches the native cycle frame for
  frame (up from frame 129 to 251, hops at 255/302/349/396, next cycle at
  567). Shops stay open as their own issue.
