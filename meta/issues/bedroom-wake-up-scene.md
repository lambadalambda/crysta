# Play Elle's wake-up scene

## Summary

A new game starts in the bedroom after the wake-up. Play it instead: Elle's five pages with Ark held, Elle leaving, `$20` set, then control.

## Dependencies

- [Run scene dialogue, choices and flags from scripts](script-dialogue-choices.md)

## Requirements

- Start from the state the game reaches after the prologue load (`$FB` set, `$20` clear) and run the bedroom scene script.
- Elle's exit movement and despawn come from her script.
- The prologue title card is shown as a simple card or skipped, documented.

## Acceptance Criteria

- Inputs of `new-game-qualification` produce the same page count and control-return point relative to scene start.
- Tests red before green; gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Result

- The app starts a new game at the bedroom with `fresh_game_flags()` (`$FB`
  only); Elle's `$00` record now decodes (sprite loader accepts `$00`
  records and her descriptor's mode `$A0`, palette `$90`, graphics `$30`).
- New services: `COP 54` (items), `COP 3A`/`39` (scripted legs moved through
  `COP 8E` at the audited class-0 stream), `COP A7` (delete); `COP 19` is
  stepped over as benign. The text decoder ends a request at `$D5 $D4`.
- Owned-ROM test with real presses, against `new-game-qualification`: Elle
  speaks after 115..125 frames (natively about 124), five pages with A, `$20`
  right after the last, the pad unlocks 318..322 frames later (natively 320),
  she deletes herself 94..98 frames after that, and Ark holds `$7A`, `$A0`.
  Red first: Elle's art test failed with `NoDescriptor`; the story test
  failed on her text, then on the revoked cadence, before passing.
- Headless screenshots show her first page beside the bed and her walk out.
- The prologue title card (`序章 / 旅立ち`) is not shown.
- Independent review approved. Follow-ups: unit tests for every leg path
  (up, left/mirror, the bit-7 skip, refusals); `walking` is set during a
  leg. Admitting `$00` records made three more bodies; checked natively:
  `$13`'s `$83:8EC8` is visible, blocking and talkable (correct);
  `$13`'s `$83:8ED2` is real, unhidden, without occupancy or callback, at
  x=680 outside `$13`'s camera region, so unreachable; `$21`'s
  `$83:9271` hides itself with inline `+$04` bit-15 code and waits in
  `COP 05` for flag `$03` -- now modelled (hidden: not drawn, not blocking),
  with a test that fails without it.
- Also changed: map `$17`'s `$83:8FE5` now walks its `COP 39` legs between
  x=328 and 440 instead of standing still; no native capture of `$17`
  exists to confirm the pattern.
