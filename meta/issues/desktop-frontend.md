# Ship the desktop frontend

## Summary

Provide native window, rendering, input, audio, configuration, storage, and ROM-extraction integration.

## Dependencies

- [Qualify the full classic-mode replay suite](qualify-classic-mode.md)
- [Implement the classic renderer](classic-renderer.md)
- [Integrate a compatible SPC audio backend](spc-audio-backend.md)
- [Port menus, inventory, configuration, and saves](menus-inventory-save.md)

## Requirements

- Choose libraries using an architecture decision record.
- Support keyboard and standard controllers.
- Provide classic scaling, fullscreen, audio controls, and save locations.
- Package ROM-free builds for supported desktop systems.

## Acceptance Criteria

- A clean user flow validates a ROM, builds/loads assets, starts the game, and persists a save.
- Input and audio survive focus and device changes.
- Packages contain no prohibited content.

## Notes

- Milestone: [M7 — Desktop and web releases](../milestones.md#m7-desktop-and-web-releases)
- Keep platform event handling outside the core crate.
