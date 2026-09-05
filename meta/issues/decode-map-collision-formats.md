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

The [static cavern subissue](static-map-cavern.md) is now complete as well.
The actual loader supplies a dimension-prefixed ROM packet and a separate
attribute table; pure static decoding matches both raw and initialized runtime
bytes exactly. One later cell-bit change is isolated, without claiming its
gameplay semantics. See [static maps](../../docs/static-maps.md).

This parent remains open. General map-script/pointer resolution, additional
indoor/outdoor/dungeon/world-map coverage, graphics, placements/regions/exits,
and behavior-qualified collision fixtures remain. The old dimension-store probe
was a false lead; the qualified cavern loader uses another path.
