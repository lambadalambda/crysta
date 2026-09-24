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
