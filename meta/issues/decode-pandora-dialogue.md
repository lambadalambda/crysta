# Decode the required Pandora progression dialogue

## Summary

Extend the bounded ROM text decoder and qualify every request/page/choice context required by the direct Pandora route through the final tutorial acknowledgement.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)

## Requirements

- Preserve source page order, acknowledgement/auto-return boundaries, names and explicit choice catalog semantics; do not invent text or acknowledgements.
- Authenticate new control/font/layout behavior before admission. Existing house dialogue and its renderer transport remain compatible.
- Compare independently reconstructed pages and selected native font-cell pixels; distinguish source-only pages and finite presentation omissions.
- Keep raw text and font/bitmap exports ignored; commit only tooling, metadata and hashes.

## Acceptance Criteria

- Required source inputs and bounded behavior are authenticated and reproducible from the owned Japanese ROM.
- Focused red → green tests, source/native comparisons, existing affected regressions and independent correctness/architecture review pass.
- A documented compiler/component contract is usable by the parent portable integration without original CPU execution.
- Fidelity/admission limits and source-only evidence are explicit; raw assets/captures stay ignored.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Source contract: [Pandora progression](../../docs/pandora-progression.md). Parent fresh replay is an independent source acceptance gate.
