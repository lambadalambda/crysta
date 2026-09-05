# Decode map, metadata, and collision formats

## Summary

Model map metadata, tile arrangements, placements, regions, transitions, and collision semantics.

## Dependencies

- [Implement and verify the compression codec](compression-codec.md)
- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)

## Requirements

- Trace map-loading code to verify pointer and dimension fields.
- Decode representative indoor, outdoor, dungeon, and world maps.
- Connect collision values to observed player behavior.
- Support lossless import/export where possible.

## Acceptance Criteria

- A map-inspection tool renders structural layers from a local ROM.
- Collision fixtures agree with reference traces in selected rooms.
- Pointer and bounds validation rejects malformed data.

## Subissues

- [Qualify static map loading and decode the cavern layer](static-map-cavern.md):
  trace pointer provenance and compare static output to the loaded checkpoint.

- [Build a local loaded-map inspector](loaded-map-inspector.md): first visual
  deliverable using a qualified runtime checkpoint, without claiming complete
  static map decoding.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Convenience images are views; typed decoded structures remain canonical.

## Progress and remaining qualification

The loaded-map inspector subissue is complete: a local browser artifact shows
original cavern viewports beside a bounded, lossless 80×32 raw runtime layer.
The [format record](../../docs/maps.md) distinguishes observed checkpoint data
from imported collision hints. No static decoder completion is claimed.

This parent remains open. Static map pointers, indoor/outdoor/dungeon/world-map
coverage, placements/regions/exits, and behavior-qualified collision fixtures
remain. A probe did not reach the candidate dimension-loading instruction;
loader tracing is not yet qualified. The current before/after pair has identical
layer words, so it does not prove dynamic collision updates or wall semantics.
