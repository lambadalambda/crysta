# Lift and throw pots to break the blue door

## Summary

The blue door in C breaks after two pot hits, counted by controller `$838C32` in `$0640`, and the second hit sets `$292` and patches the stairs.

## Dependencies

- [Run scripted movement, entry scenes and map transfers](scripted-movement-scenes.md)

## Requirements

- Lift, carry and throw a pot with the player's inputs, from source.
- Door hit detection, counter, `$292`, stair patch and the reaction scene.
- The blue door and stairs metatile/collision patches (from the geometry issue).

## Acceptance Criteria

- Throwing from (184,368) facing up hits; (136,368) misses, as natively.
- After two hits the stairs open; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)
