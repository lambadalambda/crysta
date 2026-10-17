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
