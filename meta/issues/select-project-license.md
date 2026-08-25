# Select the project license and contribution policy

## Summary

Choose licensing and contribution terms for original project code while clearly excluding rights to Terranigma content.

## Dependencies

- None.

## Requirements

- Compare permissive and copyleft options for the intended community and platform goals.
- Document the publication/provenance boundary for reconstructed assembly and data, including whether annotations, patches, or matching source may be committed.
- Document how third-party emulator, audio, and recompilation dependencies affect licensing.
- Add the selected license and update contribution documentation.

## Acceptance Criteria

- A root license file exists.
- README and contribution documentation state the policy consistently.
- The repository has an explicit publication/provenance policy for reconstructed source and ROM-derived data.
- Any incompatible dependency constraints discovered during review are captured as follow-up issues.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Do not assume that a license on original code grants rights to ROM-derived content.
