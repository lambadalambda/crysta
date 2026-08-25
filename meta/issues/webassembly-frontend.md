# Ship the WebAssembly frontend and web platform services

## Summary

Integrate browser rendering, input, audio, storage, lifecycle, and deployment around the portable core.

## Dependencies

- [Qualify the full classic-mode replay suite](qualify-classic-mode.md)
- [Implement browser ROM and asset bootstrap](browser-rom-bootstrap.md)
- [Implement the classic renderer](classic-renderer.md)
- [Integrate a compatible SPC audio backend](spc-audio-backend.md)

## Requirements

- Support keyboard and Gamepad API input.
- Integrate WebAudio with explicit user activation.
- Persist saves through a browser storage adapter.
- Handle visibility, resize, pause, and device restoration.
- Build a ROM-free static deployment.

## Acceptance Criteria

- The complete classic game is playable in a supported browser from local ROM selection through save/load.
- Simulation replay hashes agree with native for the same inputs.
- Deployment artifacts and source maps contain no ROM-derived content.

## Notes

- Milestone: [M7 — Desktop and web releases](../milestones.md#m7-desktop-and-web-releases)
- WebGPU may be used with a documented WebGL2 fallback if browser support requires it.
