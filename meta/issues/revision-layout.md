# Read every ROM address through a per-revision layout

## Summary

Replace the fixed Japanese addresses in `assets`, the runtime and the app
with lookups in a layout chosen by `rom::Revision`.

## Dependencies

- [Map the European ROM to the Japanese one](european-address-map.md)

## Requirements

- One place that names each address per revision; the Japanese values
  unchanged, so every Japanese test stays green.

## Acceptance Criteria

- The slice's decoders run on both ROMs; the Japanese tests pass unchanged.
