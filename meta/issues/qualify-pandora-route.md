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

## Qualification in progress

- Isolated source task owns only this detail, `tools/pandora-qualification/` and `docs/pandora-progression.md`; parent owns tracker closure and portable integration.
- Start from the accepted conversation route's 77 commands, preserving every synchronizing capture, omit its finish command, then extend the same empty-SRAM process. Independent replay must preserve that observation schedule.
- Historical scenario inventory established no accepted Pandora endpoint. The new fresh route, not those scenarios, now reaches C → E → 20 → 21 and the forced box interior tour.
- [Source/reference contract and reproduction](../../docs/pandora-progression.md): direct map13 result1 → `$28`, C result1 → `$2E`, two actual pot hits → `$292`, second box contact → `$22`, mandatory 41 → 44 → 42 → 43 → 41 tutorial → `$243/$244`.
- Named neutral-stability/control witness: `pandora-tour-control`, completed41344, map41 `(136,208)`. The preceding `tutorial-052` at completed41224 already retains completed-tour control. Left/Up and neutral controls finish at completed41788 `(120,192)`. Later field return, `$21/$23/$FE`, equipment acquisition and combat are not claimed.
- Source-only projection received independent review before signed commit `380d0b1`. Two fresh processes completed without restore/patch/seed; both retained evidence checkers pass normally and under `-O`. Six source-helper tests and seven grouped checker tests pass; disabling semantic validators causes 34 expected mutation failures before restoring green.
- Independent final source/evidence/architecture review approved the declared scope. The parent's independent fresh replay remains an acceptance gate. No oracle tooling fix was needed. Required new map/asset/action boundaries are documented; portable integration and tracker closure remain parent-owned.
