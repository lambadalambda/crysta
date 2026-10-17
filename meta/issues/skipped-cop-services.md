# Model the COP services the runtime steps over

## Summary

`docs/cop-inventory.md` lists 7 services that the underworld scripts reach
and that the runtime steps over without effect: `3C`, `5F`, `64`, `AA`,
`AF`, `E4` and `E5`.

## Dependencies

- [Model COP E4 and E5](cop-e4-e5.md)
- [Model COP AA and AF](cop-aa-af.md)
- [Model COP 3C, 5F and 64](cop-3c-5f-64.md)

## Requirements

- Each of the 7 services is modelled, or shown to have no effect the
  runtime must show.

## Acceptance Criteria

- `docs/cop-inventory.md` shows no stepped-over service, or each one left
  has a note in its sub-issue why stepping over is right.

## Notes

- Sites are in `docs/cop-inventory.md`, section "Sites of the services not
  modelled".

## Progress

- Done: `docs/cop-inventory.md` lists no stepped-over service (114 of
  114 modelled).
