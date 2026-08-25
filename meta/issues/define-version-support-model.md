# Define the ROM revision and version-support model

## Summary

Choose the executable behavior reference and define how code addresses, content identifiers, and asset schemas relate across the Japanese and European English revisions.

## Dependencies

- [Implement safe ROM normalization and validation](safe-rom-validation.md)

## Requirements

- State which normalized revision drives M1–M6 differential tests.
- Define whether European executable behavior is supported before the first release or used only as a localization source.
- Model source ROM revision, asset schema version, and portable save/snapshot compatibility separately.
- Establish how cross-version address and semantic-ID correspondences are recorded without assuming identical ROM layouts.
- Publish a support matrix for classic behavior, extraction, and localization.

## Acceptance Criteria

- README and architecture documentation name one unambiguous initial behavior reference.
- Asset manifests and snapshots have documented revision/version identity fields.
- Later milestone acceptance criteria do not imply unsupported cross-version behavior.
- Unknown or mixed incompatible versions have a defined rejection path.

## Notes

- Milestone: [M0 — Safe foundation](../milestones.md#m0-safe-foundation)
- The Japanese ROM is currently proposed as the behavior reference and the European English ROM as the localization reference; this issue must confirm or revise that split.
