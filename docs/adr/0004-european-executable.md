# Run the European English executable

Status: decided 2026-09-24. Supersedes the "European executable behavior"
row of [ADR 0001](0001-version-support-model.md).

## Decision

The slice runs from the European English ROM (`Terranigma`, normalized
SHA-256 `93ba50d8…3edd38`) as well as from the Japanese one: its own code
addresses, data, text engine, font and timing, not Japanese logic with
English text.

## Consequences

- Every ROM address the assets, the runtime and the app read goes through a
  per-revision layout keyed by `rom::Revision`; the Japanese values stay the
  reference, and European ones come from a recorded correspondence.
- The European text engine gets its own decoder; pages, choices and labels
  keep one portable representation.
- Behavior measured on the Japanese executable is not assumed for the
  European one: the European slice is checked against its own native route.
- The Japanese lane stays the behavior reference for the differential
  tests; the European lane adds its own.
