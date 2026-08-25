# Implement browser ROM and asset bootstrap

## Summary

Let users select a local cartridge dump and derive or load compatible assets entirely in the browser.

## Dependencies

- [Implement safe ROM normalization and validation](safe-rom-validation.md)
- [Build the reproducible local asset pack](local-asset-pack.md)
- [Validate extraction and the core boundary in WebAssembly](validate-web-extraction-spike.md)

## Requirements

- Use local file selection; never upload ROM bytes.
- Validate and normalize supported dumps in WebAssembly.
- Cache versioned derived assets with clear invalidation and deletion controls.
- Report unsupported inputs without retaining them.

## Acceptance Criteria

- Every ROM revision enabled for browser extraction by the support matrix follows the documented local flow.
- Network inspection confirms that ROM and extracted asset bytes are not transmitted.
- Cache migration, corruption, and deletion paths are tested.

## Notes

- Milestone: [M7 — Desktop and web releases](../milestones.md#m7-desktop-and-web-releases)
- Browser storage quotas and private-browsing failures need actionable fallbacks.
