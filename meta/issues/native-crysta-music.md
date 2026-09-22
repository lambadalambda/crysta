# Play Crysta music in the native app

## Summary

Add a bounded original-music playback path to `crysta-app`, using the user's
verified local Japanese ROM. Native playback is the current priority; browser
integration and sound effects remain separate.

## Dependencies

- [Select the project license and contribution policy](select-project-license.md)
- [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Requirements

- Derive the opening Crysta music inputs from the owned ROM; no distributed
  music assets or full-game emulator running behind the portable simulation.
- Use a license-compatible audio-only backend isolated from gameplay timing.
- Provide pause/resume and volume controls; fail gracefully without an audio device.
- Keep raw audio, driver/sample/sequence data and discovery captures ignored.
- Document the bounded track scope and reproduction steps, without claiming the
  complete CPU-to-SPC protocol or all map-driven audio transitions are solved.

## Acceptance Criteria

- The native app plays non-silent original Crysta music from local ROM-derived inputs.
- Tests cover extraction bounds/provenance and playback controls where practical.
- Audio does not change deterministic simulation state or require reference CPU execution during gameplay.
- Backend licensing and local run instructions are documented; independent review
  and native smoke verification pass (with any device limitations stated).

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- Bounded child of [SPC backend](spc-audio-backend.md) and
  [audio protocol discovery](reverse-audio-protocol.md); neither parent is closed by this work.
