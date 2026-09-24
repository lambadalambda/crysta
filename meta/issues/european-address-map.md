# Map the European ROM to the Japanese one

## Summary

Find, for every routine and table the slice reads, its European address.
Only a few byte windows match as they are: the banks moved and pointers
changed.

## Dependencies

- [Port the slice to the European English ROM](european-port.md)

## Requirements

- A tool that matches code by instruction shape (operands masked) and data
  by structure, and a recorded, reviewable correspondence table.

## Acceptance Criteria

- Every address the assets, runtime and app use has a European value or a
  recorded reason why it has none (a different system, such as text).

## Resolution

- `tools/eu-map/map.py` maps all 579 addresses the code names (exact runs,
  aligned gaps, masked instruction shapes, pointer references, LZ packets,
  bank bases) into `jp-to-eu.tsv`, each with its method and confidence;
  `emit.py` writes the high and medium rows into
  `crates/assets/src/layout/europe.rs` (2026-09-24). The low rows are text
  addresses, set by hand where used (`docs/european-text.md`).
