# Match the Guardner's sleep and vacuum

## Summary

The Guardners run natively, but the sleep's length (`COP 8F` count, `$97:C5BB`) and the vacuum's pull (`$97:C447`, a pixel a frame against the walls) were checked only against the longplay, not traced.

## Subissues

- [Keep the header's off-screen bit until the first draw](first-draw-visibility.md)
- [Run a spawn linked after its parent in the same frame](spawns-same-frame.md)
- [Match the flyers' pose 7](flyer-pose-seven.md)

## Dependencies

- [Play tower 4](tower-four.md)

## Requirements

- Trace the sleep and the vacuum in an emulator and compare.

## Acceptance Criteria

- The sleep's frames and the pull's path match a trace.

## Notes

- Markers: `world.rs` (`push_ark`), `actors/native.rs`.

## Progress

- Native JP traces (2026-10-08, `docs/native-warp.md`): the bolt's hit and its contact callback come together; Ark's script `$97:C5A9` holds list 7 for 132 frames and 8 for 32 (163 frames of `$097E & $0400`); the vacuum pulls a pixel every third frame (`COP BE` rest 2), x until within 2 of the Guardner (`COP D3` zone 2), then y until his probe is 26 below it (y 514), through the sleep and after it, then `COP 15` to `$118` (384,848). The runtime matches this (`a_guardners_bolt_puts_ark_to_sleep_and_its_vacuum_draws_him_in`).
- The sleep's list starts at H+2 as natively: the hit's push is Ark's turn, the callback's script runs from the next frame; Ark's push from a script applies at the start of the next actor pass (`$80:D0F0`).
- Research (2026-10-14, the job's `tmp/guardner2/`): runtime frame f is native 60504+f on `$117`. The vacuum's late start is not its own bug: the pull starts at the end of the Guardner's pose-4 cycle, late because it wakes 31 frames early. The damage 8 against 10 at Ark y 570 is the variance (`$85:DE3A`) on the frame counter `$42`, which the runtime starts at 0 and steps without the native lag frames.
- With the first draw and the same-frame spawns: on (376,576) the hit at frame 130, the sleep at 132, the first pull at 144, as the research's native model (`a_guardners_bolt_puts_ark_to_sleep_and_its_vacuum_draws_him_in`).
- Open: [flyer-pose-seven](flyer-pose-seven.md) (the random steps at 570); at H+1 natively Ark shows list `$0D`, here none; the frame counter `$42` without lag frames (the damage variance); a native-seeded check at 590.

