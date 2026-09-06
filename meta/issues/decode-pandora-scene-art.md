# Decode the required Pandora actors and carrying poses

## Summary

Compile source-derived presentation for required town/map13/cellar/box/tutorial actors, carried/thrown pots and Ark carrying poses, without introducing general NPC simulation.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Qualify only required source actor identities, scene/phase membership, palettes, frame bounds, anchors and draw ordering from source and retained native captures.
- Dynamic scripted phases are explicit presentation data, not an invented wandering scheduler. Keep collision ownership with the portable core/source profile compiler.
- Preserve existing Ark and frozen-house art. Account explicitly for priority/layer omissions and distinguish source composition from whole-scene native equality.
- Export no embedded copyrighted sprites/palettes; retain raw assets under ignored local paths. Positive/negative tests and independent review precede substantial commits.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
