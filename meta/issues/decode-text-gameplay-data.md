# Decode text and gameplay data tables

## Summary

Model official text plus item, equipment, enemy, store, level-up, and related gameplay tables.

## Dependencies

- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)

## Requirements

- Decode text dictionaries, control codes, pointers, and language-specific layouts.
- Define typed records for known gameplay tables.
- Cross-check table consumers in assembly.
- Keep localization content outside version control.

## Acceptance Criteria

- Named local exports can be generated for both supported dumps.
- Representative records agree with in-game observations and public maps.
- Unknown fields remain explicit rather than guessed.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- The European ROM is the official English reference; code behavior remains anchored separately.
