# Read every ROM address through a per-revision layout

## Summary

Replace the fixed Japanese addresses in `assets`, the runtime and the app
with lookups in a layout chosen by `rom::Revision`.

## Dependencies

- [Port the slice to the European English ROM](european-port.md)

## Requirements

- One place that names each address per revision; the Japanese values
  unchanged, so every Japanese test stays green.

## Acceptance Criteria

- The slice's decoders run on both ROMs; the Japanese tests pass unchanged.

## Progress

- `assets::layout` (2026-09-24): `Address::both`, `layout::at` over the
  generated table, `per_revision`. The load path is ported: all 28 slice
  maps load and run on the European ROM, compared with the Japanese
  (`crates/crysta-runtime/tests/local_european.rs`). Open: the Pandora
  sprites, the shop texts, the labels, the shop display, the window art,
  the music extraction.
