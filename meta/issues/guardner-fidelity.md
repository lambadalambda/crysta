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
