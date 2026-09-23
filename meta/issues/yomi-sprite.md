# Draw Yomi instead of a placeholder

## Summary

Yomi is drawn as the pink placeholder: the art decoder refuses its sprite.

## Requirements

- Decode Yomi's sprite resources from source and draw it.

## Acceptance Criteria

- Yomi is drawn with its native frames in the box's rooms.

## Notes

- Reported by the user while playing the slice (2026-09-23).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Progress

- Yomi's descriptor mode `$0023` (and `$00A3`) is admitted: Yomi draws in
  the tour rooms. The spear's display `$83:957C`, whose script sets its own
  art base (`COP D8 $A2C000`), is refused rather than drawn with Yomi's body;
  it needs the Pandora direct-list art (`tour_object`). Pandora's Box in
  `$21` (mode `$0004`, `box_art`) is also still a placeholder.
