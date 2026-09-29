# Repin the oracle producer source after the audio-port capture

## Summary

`crates/oracle/src/lib.rs` is a pinned library-producer source of the
map-inspector capture gate. The European work added bounded native frame
audio capture and CPU audio-port tracing to it (`1565a13`, `ae1d8aa`, and the
clippy fixes in `3553234`), so `fixture_library_producer_sources_match`
fails. Record the change through the established repin path.

## Dependencies

- [Repin the formatted library producer source](repin-formatted-producer-source.md)

## Requirements

- Show the capture producer's output is unchanged with the new source: fresh
  isolated builds reproduce the pinned capture files byte for byte.
- Record the replacement as a new stage over the frozen descriptors, as the
  formatted-source repin did; do not rewrite existing pins.
- Keep the mutation and negative controls strict.

## Acceptance Criteria

- `cargo test --workspace` passes, including
  `fixture_library_producer_sources_match`, with the evidence retained under
  ignored `local/`.
- The lockfile projection keeps `map-inspector`'s dependencies unchanged (the
  oracle no longer depends on the runtime, `6a63417`).

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
