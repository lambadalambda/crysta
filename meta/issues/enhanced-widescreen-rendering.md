# Add enhanced and widescreen rendering

## Summary

Provide genuinely expanded presentation rather than stretching the classic framebuffer.

## Dependencies

- [Qualify the full classic-mode replay suite](qualify-classic-mode.md)
- [Ship the desktop frontend](desktop-frontend.md)
- [Ship the WebAssembly frontend and web platform services](webassembly-frontend.md)

## Requirements

- Separate logical viewport expansion from display aspect ratio.
- Audit map streaming, actor spawning, culling, effects, HUD placement, and camera boundaries.
- Preserve classic rendering as the default regression path.
- Add higher-resolution or shader options behind presentation flags.

## Acceptance Criteria

- Selected areas render wider without exposing invalid map data or changing classic simulation.
- Classic framebuffer tests remain unchanged when enhancements are disabled.
- Viewport-edge behavior has focused tests.

## Notes

- Milestone: [M8 — Enhancements and extensibility](../milestones.md#m8-enhancements-and-extensibility)
- True widescreen is a game-logic integration problem as well as a renderer feature.
