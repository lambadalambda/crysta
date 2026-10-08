# Match the Guardner's sleep and vacuum

## Summary

The Guardners run natively, but the sleep's length (`COP 8F` count, `$97:C5BB`) and the vacuum's pull (`$97:C447`, a pixel a frame against the walls) were checked only against the longplay, not traced.

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
- Open: the sleep's list starts a frame early (the callback's `COP DF` runs Ark's new script the same frame); the vacuum's first push comes at h+13, natively h+6; the Guardner wakes at y 592 or less on its first frame, natively at y 600 and 49 frames after it spawns (`COP 59 1E`).

