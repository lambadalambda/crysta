# Qualify the native Pandora route and state changes

## Summary

Identify and reproduce the minimum original-game path from the accepted fresh house progression to opening Pandora’s Box and regaining player control. This is a source/reference evidence gate, not portable implementation.

## Dependencies

- [Qualify the room B conversation and exterior progression](qualify-house-conversation-progression.md)
- [Qualify the first exterior landing profile](qualify-house-exterior-profile.md)

## Requirements

- Audit existing opening scenarios and any historical desync before treating their later state as evidence.
- Use the owned Japanese ROM, a fresh empty-SRAM Session and actual inputs from the menu/intro onward. No warps, state restoration or memory patches in the qualifying journey.
- Record exact required maps, exits, interactions, request/choice boundaries, event changes and immediate player/world effects. Separate necessary story steps from incidental movement/presentation.
- Pin relevant source bytes/operands and retained semantic checkpoints; source metadata must not initialize production state from captured WRAM.
- Explicitly record synchronization effects of observations such as save_state, timing policies and any oracle/tooling blocker.
- Keep raw ROM/text/graphics/traces/captures ignored; commit only reconstruction tooling, selected source metadata and hashes.

## Acceptance Criteria

- A reproducible input-only fresh route reaches a named post-Pandora player-control checkpoint.
- The prerequisite and effect contract is source-backed, with controls distinguishing missing prerequisites and reordered/omitted interactions where practical.
- A checker rejects mutated semantic evidence and an independent replay/review confirms the declared scope.
- Required implementation/asset boundaries and unresolved blockers are documented before the portable sequence is enabled.

## Notes

- Parent: [Open Pandora’s Box in the portable slice](open-pandora-portable-slice.md).
- RE discovery may precede executable tests; retained evidence/checker development must still demonstrate red → green mutation controls.
