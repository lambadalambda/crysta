# Build a local loaded-map inspector

## Summary

Deliver a first visual map-research tool using a verified local ROM and the
reference oracle. Preserve typed runtime map/collision data and distinguish it
from unproved static ROM formats and collision semantics.

## Dependencies

- [Decode map, metadata, and collision formats](decode-map-collision-formats.md) (parent)
- [Implement and verify the compression codec](compression-codec.md)

## Requirements

- Export a reproducible loaded-room checkpoint with map identity and player/camera metadata.
- Validate runtime map dimensions, buffer bounds, and selected collision-cell indexing.
- Provide a local browser viewer with a reference image, grid/collision overlays,
  cell inspection, and zoom; keep all generated content under ignored `local/`.
- Label imported collision meanings as hypotheses unless reference behavior is tested.
- Do not present a runtime snapshot as a complete static ROM map decoder.

## Acceptance Criteria

- A documented command produces a viewable local artifact from the owned Japanese ROM.
- Synthetic tests reject malformed buffers and validate coordinate/cell lookup.
- A ROM-backed test pins checkpoint identity, dimensions, and selected cell values/hashes.
- Browser QA verifies rendering, overlay toggles, zoom, and cell inspection.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- This first deliverable does not close the parent: ROM pointers, representative
  indoor/outdoor/dungeon/world formats, transitions, and trace-qualified collision
  semantics remain parent requirements.
