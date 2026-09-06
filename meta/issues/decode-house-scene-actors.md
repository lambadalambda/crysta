# Decode the complete ordinary house actor set

## Summary

Extend the shared sprite pipeline to the complete qualified house roster without duplicating a bespoke loader for every NPC.

## Dependencies

- [Qualify the fresh house room and actor roster](qualify-house-scene-roster.md)

## Requirements

- Derive all included actor graphics, palettes, frame compositions, anchors and palette/tile relocations from the authenticated ROM.
- Preserve actor identity, source spawn position, ordinary facing and qualified draw-order tie metadata for host scene assembly.
- Reuse existing composition primitives, qualifying any newly encountered tile-layout cases rather than assuming the first resident's restrictions generalize.
- Keep scripts/AI/dialogue out of the decoder; explicit bounded source projection is acceptable.

## Acceptance Criteria

- Every included resident/scene object has a reproducible ROM-only rendering contract and selected fresh native validation.
- Synthetic red-green tests and authenticated frame/palette/tile/placement comparisons pass, with raw assets ignored.
- Independent correctness/architecture review passes.

## Notes

- Subissue of [complete house setup](complete-house-scene-setup.md).
