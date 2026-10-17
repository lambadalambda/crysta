# Model COP 3C, 5F and 64

## Summary

`COP 3C` (`$84:DDEC`, `$90:92AC`, `$90:9995`, `$90:9E79`), `COP 5F`
(`$84:DDEE`) and `COP 64` (`$90:A115`, map `$11D`: the circle window effect,
`docs/tower-five.md`) are stepped over.

## Dependencies

- [List the COP services the underworld scripts use](cop-inventory.md)

## Requirements

- Read the handlers and model each service, or show that stepping over is
  right.

## Acceptance Criteria

- A unit test per modelled service, checked against a native trace.

## Progress

- Done: `5F` (by Ark's facing, in `player_test`) and `3C` (drops the mark),
  with unit tests. Open: `64`, the guardian's ring window (`$87:A26B`, its
  HDMA tables from `$87:A91D`), display only.
- Done: `64`, the ring window (`world/circle.rs`, `World::ring`): the
  tables match the native ones on `$11D`, lagging one raise of `$0474`.
  Drawing it is with [Draw colour addition](colour-addition.md).
- Checked: `3C` on `$118` (`$90:9E79`): natively the guardian's flicker
  takes 122 frames, then `3C` leaves `$7E:A000` as it was (no mark); the
  runtime's loop takes the same 122 frames and holds no mark. `5F` runs
  only in the Magirock's callback (`$84:DDEE`), which `world/magirock.rs`
  models (`ark_takes_the_magirock_on_the_second_floor`).
