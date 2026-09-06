# Qualify bounded Pandora navigation and contact admission

## Summary

Derive immutable collision/occupancy profiles and transition/contact contracts needed to connect the accepted house route to map13, the cellar and completed Pandora tour. Asset availability alone does not qualify movement.

## Dependencies

- [Qualify the native Pandora route and state changes](qualify-pandora-route.md)
- [Decode the required Pandora route backgrounds](decode-pandora-backgrounds.md)
- [Qualify and implement bounded cellar pot actions](qualify-pandora-pot-actions.md)

## Requirements

- Authenticate source grids, material/sample admission, actor occupancy, ordered exits and settled destinations for wider A, map13, changed C, E/20/21 and final map41 control.
- Separate source stairs/forced transfers from ordinary wooden-door motion; explicitly document portable pacing and history-reset policy rather than interpreting snapshot frame labels as delay constants.
- Qualify box contact bounds/order and shared-house patch/pot lifecycle across required loads. Do not add a general collision/event dispatcher or initialize from captures.
- Preserve narrow existing house profiles until the new aggregate is explicitly enabled; unsupported materials/branches remain atomic errors.

## Acceptance Criteria

- Source-derived compiler/data contract plus independent native sample/transition evidence cover the continuous admitted route and boundary rejection tests.
- TDD, mutation controls, affected regressions and independent correctness/architecture review pass.
- Exact fidelity limits are documented; no raw captures or extracted grids committed.

## Notes

- Parent: [Port the bounded Pandora route and sequence](port-pandora-sequence.md).
- Reverse-engineering/source discovery may precede tests; implementation must use red → green.
- Source discovery: [bounded navigation](../../docs/pandora-navigation.md) records material aliases, actual edge/actor samples, selector14 adjustment, box polling and pointer-cache lifecycle. Independent read-only correctness/evidence/architecture review approved the discovery-only increment; requested spatial-coverage and residue provenance clarifications were applied.
- Still open: exact profile/sample qualification, first-contact participant completion, tested compiler/checker, and parent reproduction. No existing house profile or source-observer contract changes.
