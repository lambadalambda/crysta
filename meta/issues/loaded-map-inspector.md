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

## Completion record

- Added pure `assets::maps` with WRAM/dimension/capacity validation, coordinate
  lookup, raw-word preservation and lossless layer export. Synthetic tests used
  red→green development.
- Added `map-inspector capture|verify` with authenticated JP ROM and pinned SRAM,
  two deterministic map `$0128` portal-cavern checkpoints, local BMP/JSON/raw
  memory exports, and a dependency-free browser viewer.
- Qualified frames 1601/1841, 80×32 cells, selected words, layer and RGB hashes in
  a fresh-process local integration test. There are zero changed layer words;
  player/camera movement is separate.
- Browser QA passed at desktop/mobile sizes, including checkpoint switching,
  overlays, 25–200% zoom, click/keyboard inspection and locate-player. A synthetic
  in-browser difference caught then verified the atlas overlay-order fix.
- Independent correctness/architecture review approved the model, capture tool
  and viewer after the draw-order correction. Workspace format, Clippy (warnings
  denied), tests and rustdoc (warnings denied) passed. A clean detached worktree
  also passed the full workspace suite with explicit ROM/SRAM-backed test skips.
- See [runtime layout and qualification](../../docs/maps.md) for the command,
  provenance, boundaries and unresolved parent scope.

Optional review follow-up: the eight-row framebuffer borders still accept
map-coordinate clicks and can receive overlays. Restricting them to active
terrain rows is deferred; no general display-geometry API is claimed.
