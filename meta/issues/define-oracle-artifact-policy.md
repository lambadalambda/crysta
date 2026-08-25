# Define the oracle artifact and publication policy

## Summary

Classify reference-emulator outputs and reconstructed source so contributors know what may be committed, what must remain local, and what requires review.

## Dependencies

- [Select the project license and contribution policy](select-project-license.md)

## Requirements

- Classify controller logs, hashes, snapshots, memory dumps, screenshots, audio captures, extracted data, reconstructed assembly, and generated binaries.
- Define the canonical ignored local workspace and tool defaults.
- Decide the publication/provenance boundary for reconstructed code and data before matching disassembly is committed.
- Document a review path for minimal derived fixtures whose status is uncertain.
- Avoid presenting this project policy as legal advice.

## Acceptance Criteria

- Contribution documentation contains a concise committed/local/review-required policy.
- Disassembly and oracle issues link to the publication policy before producing artifacts.
- Tool defaults and `.gitignore` agree on local artifact locations.
- Repository safety checks cover suspicious tracked blobs in addition to filename extensions.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- Raw save states and video/audio/memory captures can contain copied game content even when they are not ROM files.
