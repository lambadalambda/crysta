# Play tower 1's intro pan and text

## Summary

On the first visit `$90:8F28` locks the pad, pans the camera down from 768 pixels above Ark (`COP DD`, `COP DE`, 384 frames), shows two pages of Ark's words and unlocks the pad. Our interpreter stops at `COP DD`, so the pad stays locked.

## Dependencies

- [Enter the towers from the world map](enter-the-towers.md)

## Requirements

- Model `COP DD` and `COP DE` (camera actor and pan) and run the intro to its end.

## Acceptance Criteria

- The intro's camera, text and unlock match native timing (JP 518 frames from load to unlock with A held).

## Notes

- `COP DD`/`DE` move the camera from 768 pixels above Ark back to him at 2 a frame; then the two pages and the unlock (`local_towers.rs` `the_first_visit_pans_down_to_ark_then_he_speaks`, both ROMs: 383..387 frames of pan). The title sprites that fly in are not drawn (see the backdrop issue).
