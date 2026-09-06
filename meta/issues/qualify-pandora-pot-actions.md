# Qualify and implement bounded cellar pot actions

## Summary

Qualify and implement source-derived FA/FB lifting, carrying and throws sufficient for the direct C cellar-door route, including a miss and two actual hits.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Do not replace hit testing with a pot counter or interaction-distance shortcut. Preserve consumed-cell membership, carried-object ownership, resident occupancy and carrying movement semantics.
- Source/native qualification must distinguish lift/hold/throw/flight, a miss, first/second hit and control restoration. Door reactions/whole story graph remain the parent runtime responsibility.
- Provide a dependency-free, deterministic pure core component with atomic error handling and canonical snapshots/continuation. Only admitted carrying/throw behavior; no general combat/projectile system.
- Keep source/art/capture data local; use red/green native-fixture and synthetic tests and independent review.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
