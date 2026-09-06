# Decode the required Pandora route backgrounds

## Summary

Compile and independently qualify only the first-background resources, source camera profiles and attributed grids needed by wider A, map13, E/20/21 and the forced41–44 tour.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Authenticate resource provenance and state-dependent setup. Maps41–44 inherit the controller-supplied graphics and ordered bank-first COP5A palette transfers; they are not ordinary standalone house recipes.
- Keep full source-derived grids distinct from route admission and dynamic occupancy. Preserve house/exterior recipes and priority semantics.
- Compare selected native tile words/indexed pixels/priority and camera evidence; document omitted layers/effects instead of claiming whole-frame RGB equality.
- Keep extracted pixels/grids/palettes local; source metadata and hashes only are committed.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
