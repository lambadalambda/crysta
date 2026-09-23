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
