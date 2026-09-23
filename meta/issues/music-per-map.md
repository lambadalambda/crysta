# Select music per map load

## Summary

Each map's loading script selects a track (`08 FC nn`), with flag-dependent
branches (Crysta's bedroom: selection 3 while `$23` is clear). The app plays
only selection 3.

## Requirements

- Extract any selection's sequence and samples from source, as selection 3 is.
- Evaluate the map's selection with the current flags on every load, and
  switch tracks through the driver (`F0` stop, uploads, `F4` play) only when
  the selection changes.

## Acceptance Criteria

- Source-derived tests give the selection for each slice map and state.
- The app switches at map loads; tests stay silent.

## Notes

- Parent: [Play the slice's music and sound effects](crysta-music-and-sounds.md)
