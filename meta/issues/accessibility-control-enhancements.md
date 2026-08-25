# Add accessibility and control enhancements

## Summary

Add remapping and optional usability features through explicit frontend or enhanced-mode policies.

## Dependencies

- [Ship the desktop frontend](desktop-frontend.md)
- [Ship the WebAssembly frontend and web platform services](webassembly-frontend.md)

## Requirements

- Support complete keyboard/controller remapping.
- Evaluate hold/toggle, text-speed, contrast, screen-shake, and input-buffer options.
- Keep authoritative classic behavior unchanged when options are disabled.
- Document save/config portability.

## Acceptance Criteria

- Every enhancement can be enabled independently and reset safely.
- Classic replay hashes remain unchanged with default settings.
- Controls are operable without requiring a specific device type.

## Notes

- Milestone: [M8 — Enhancements and extensibility](../milestones.md#m8-enhancements-and-extensibility)
- Prioritize measured user needs; do not bundle unrelated gameplay changes under accessibility.
