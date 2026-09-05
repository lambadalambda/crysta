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
