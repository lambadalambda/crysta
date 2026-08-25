# Select and integrate the reference emulator

## Summary

Evaluate emulator/debugger options and integrate the smallest maintainable reference-execution boundary.

## Dependencies

- [Select the project license and contribution policy](select-project-license.md)
- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Define the oracle artifact and publication policy](define-oracle-artifact-policy.md)
- [Define the ROM revision and version-support model](define-version-support-model.md)

## Requirements

- Evaluate determinism, frame stepping, state export, scripting, license, and headless use.
- Record the decision and rejected alternatives.
- Provide a local command that boots a verified ROM through the selected boundary.

## Acceptance Criteria

- The selected approach is documented in an architecture decision record.
- A smoke test reaches a stable frame boundary from reset.
- Licensing is compatible with the project policy.

## Notes

- Milestone: [M1 — Reference oracle](../milestones.md#m1-reference-oracle)
- Snapshots, memory dumps, screenshots, and audio captures produced here are
  local-only artifacts under the classes defined in
  [CONTRIBUTING](../../CONTRIBUTING.md); public fixtures carry inputs, hashes,
  and schemas only.
- Mesen, bsnes-derived cores, and a dedicated headless harness are candidates; selection must be evidence-based.
