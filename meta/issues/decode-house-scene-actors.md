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

## Completion

- Shared `HouseScenes` decodes all nine residents and F's table child, retaining 28 bounded list records, resource/source identities, placements, mirrors, palettes and native tie ranks. The first-NPC compatibility export remains byte-identical.
- Synthetic red/green malformed-source tests and separate production/qualification reviews pass. Parent independently repeated the two fresh art runs plus two D creation probes at ignored `local/house-scene-qualification/art-rA75pW`; normal/optimized checks passed.
- D is frozen at its source creation origin/pose, with explicit creation-only qualification; its initialized pose is not claimed as an AI-free native OAM sample. Shadow, transient labels and gated scene visuals remain excluded. Full evidence and pixel boundaries are in `docs/house-scene.md`.
