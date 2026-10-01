# Play tower 1's intro pan and text

## Summary

On the first visit `$90:8F28` locks the pad, pans the camera down from 768 pixels above Ark (`COP DD`, `COP DE`, 384 frames), shows two pages of Ark's words and unlocks the pad. Our interpreter stops at `COP DD`, so the pad stays locked.

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Model `COP DD` and `COP DE` (camera actor and pan) and run the intro to its end.

## Acceptance Criteria

- The intro's camera, text and unlock match native timing (JP 518 frames from load to unlock with A held).
