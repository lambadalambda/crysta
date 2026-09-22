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

## Completed: desktop playback confirmed

[Run instructions, controls and licenses](../../crates/crysta-app/README.md).

- Source-only extraction authenticates the Japanese ROM, resolves fresh Crysta
  selection3 (not fallthrough5), and reconstructs driver/sequence/sample transfers.
  Exact ranges, destinations, bank skips and startup protocol are recorded in the
  [source recipe](../../tools/native-music-qualification/README.md). No snapshots
  initialize production state; payloads cross physical APU ports.
- The retained MIT LakeSnes SPC/APU/DSP runs without a game CPU/PPU. A narrow
  negative-BRR signed-shift UB fix is regression-tested under UBSan. Native PCM
  production and the device live on one worker, outside simulation state/timing.
- M pauses/resumes music; -/+ adjusts volume; focus/suspension pauses preserve
  manual pause. No-device failure continues silently. This first pass keeps the
  one Crysta track across maps; no map-driven changes or effects are claimed.
- TDD: 31 app tests pass, with three explicitly opt-in tests. Both owned-ROM
  extraction and ten-second chunk-deterministic/non-silent PCM checks pass.
  Backend9 tests plus the thread-ownership compile-fail doctest pass, including
  UBSan; the real-ROM startup/render test also passes with C UBSan instrumentation.
  Removing the32-cycle post-ACK gap fails upload verification at `$0FE8`; restored
  code passes. Separate backend/source/integration reviews found no blockers.
- Root release workspace tests, strict Clippy and formatting pass; native app
  release build and strict Clippy pass. Changed app Rust files are formatted.
  The full standalone app formatting check still reports two pre-existing
  assertion-wrapping differences in untouched `frame.rs` tests.
- Native process launch stays running and reports silent fallback; the headless
  screenshot still renders bedroomF at `(304,112)` without initializing audio.
  **Real-device smoke fails at device creation with “No matching default audio
  unit found” in this restricted session.** Deterministic PCM is not audible
  hardware verification. Run the documented app/device test in a desktop terminal.

The user confirmed **“it works!”** when running the native app on the desktop,
and confirmed that music continues during the separately reported outdoor
gameplay stall. This supplies the manual audible-playback smoke check; the
automated device pause/resume/shutdown test remains unavailable in the restricted
session, not a claimed passing run. Together with the source, PCM, control and
backend tests above, this completes the bounded native-music issue. Full
SPC-backend/protocol work remains open.
