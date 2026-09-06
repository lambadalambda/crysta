# Decode the required opening dialogue presentation

## Summary

Decode only the ROM text/font resources and bounded text commands needed by the room B progression conversation. Reuse a compact data interface suitable for simple browser presentation; retain page/ack boundaries without requiring native window effects.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)

## Requirements

- Bounded subissue of [talk and leave the house](talk-and-leave-house.md).
- ROM/source-driven compilation, no capture-seeded production data or original CPU in simulation.
- Independent correctness/architecture review before substantial commits.

## Acceptance Criteria

- ROM-only dialogue presentation matches selected fresh text/font evidence, with synthetic malformed-source tests and explicit command limits. No copied text or glyph assets tracked.
