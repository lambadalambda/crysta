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

## Qualification

Done. Recorded as a **fourth stage**, `oracle-repin-producer.json`
(`d57368e0…f342`) with its report `oracle-repin-producer-bridge.json`
(`19c28e8a…8b07`), over the frozen repin stage. No earlier descriptor, report,
epoch or output pin changed.

**Scope.** Two pinned sources changed, not one: `crates/oracle/src/lib.rs`
(`f6dede52…` → `c0865438…`) and `vendor/ares/shims.cpp` (`1bc5dc60…` →
`ecf2b990…`). The gate stopped at the first. `vendor/ares/ares/sfc/cpu/memory.cpp`
also changed but was never pinned; the isolated builds compiled it.

**Output neutrality.** Two fresh isolated builds
(`local/map-inspector-oracle-repin/{oracle-a,oracle-b}`, own clean target,
build log, copied binary, one singleton capture each) reproduce all ten capture
files at 855,207 bytes, byte-identical to each other, to the frozen inventory
and to the retained repin capture. Manifest `a7f23508…664a`, nonpixels
`7998be25…0cb22`. The report reproduces byte-exactly in normal and `-O` Python.

**Gates.** The Rust sourcegate walks an ordered stage list instead of adding
another `.or_else()`; each stage must name the one before it, declare its
identity as the pre-state, and ship a report whose envelopes bind to it. The
lockfile projection is unchanged and passes. Negative controls: 28/28 oracle
Python mutations red in both modes; ten Rust gate removals each red; the
library (10/10) and repin (18/18) harnesses run again after a missing
`projection.py` dependency was fixed. Records under ignored
`local/map-inspector-oracle-repin/`.

**Review.** An independent review reproduced the report from the retained
roots, compared all ten files itself, confirmed every earlier pin byte-unchanged
and every old Rust assertion kept. Two nits were fixed (a stale README name, a
missing bridge-hash chain check); the rest went to
[Consolidate the qualification stage template](consolidate-stage-template.md).
