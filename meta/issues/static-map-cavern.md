# Qualify static map loading and decode the cavern layer

## Summary

Advance beyond runtime inspection by tracing the Japanese map loader, resolving
its ROM-backed metadata and compressed layer, and comparing static output with
the qualified portal-cavern checkpoint.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md) (parent)
- [Build a local loaded-map inspector](loaded-map-inspector.md)
- [Implement and verify the compression codec](compression-codec.md)

## Requirements

- Trace loader execution to establish map identity, pointer resolution, dimensions
  and relevant source packet boundaries rather than inferring pointers from hashes.
- Implement a pure, bounded reader for the qualified static layout with synthetic
  malformed-input tests and lossless raw data preservation.
- Compare decoded cells with the qualified map `$0128` runtime layer, explicitly
  separating any dynamic modifications from static content.
- Keep all raw ROM data, traces and extracted artifacts under ignored `local/`.

## Acceptance Criteria

- Reproducible local qualification identifies loader PCs and pointer/data provenance.
- A static reader authenticates through its caller and rejects invalid pointers,
  dimensions and truncated/unsupported data without panics.
- Local tests pin packet extents and decoded output and explain runtime differences.
- Documentation states supported maps and remaining parent scope without claiming
  general indoor/outdoor/dungeon/world or collision behavior coverage.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- This is a bounded subissue; the parent remains open for broader map coverage,
  transitions, placements and behavior-qualified collision semantics.

## Completion record

- Qualified the real loader rather than the unused `$80:F690` candidate. Map
  `$0128` resolves an initial script pointer through `$86:959C + 3 * map_id`;
  the traced layer command supplies `$C9:0000` and packet `$C9:0002`.
- Added `StaticLayer`, a pure bounded dimension-prefixed packet reader preserving
  source bytes and raw words. Synthetic tests cover malformed dimensions,
  offsets, truncation, bank/output bounds and noncanonical encoding preservation.
- Static cavern output (5,120 bytes) exactly matches `$7E:A000` immediately after
  decompression. A separate 512-byte ROM attribute table and pure lookup exactly
  reproduce all initialized words at a second loader stop.
- Of 657 raw-source/gameplay word differences, 656 are attribute initialization.
  The sole later change is cell 448 `$0007 → $8007`; its writer/gameplay meaning
  is deferred, not encoded into static content.
- Added `decode-layer` (authenticated ROM only, no SRAM or emulator boot) and
  `qualify-loader` (fixed authenticated ROM/SRAM trace experiment). Local tests
  actually executed with owned inputs and passed; missing inputs skip explicitly.
- Independent correctness/architecture reviews approved the pure reader,
  attribute transform and qualification pipeline. Full workspace tests, format,
  Clippy and rustdoc with warnings denied passed. The full workspace suite also
  passed in a clean detached worktree with explicit ROM/SRAM test skips.
- Reproduction, source ranges/hashes, ten trace stops and qualification limits
  are recorded in [static map layers](../../docs/static-maps.md).

This subissue does not implement a general map-script/pointer resolver, arbitrary
map coverage, terrain graphics or movement collision semantics. Those remain in
the parent issue and the event-bytecode work.
