# Apply flag-gated doors, blockers and tile patches

## Summary

Progress in Crysta is gated by hidden occupancy actors (D's gate needs `$26`), metatile and collision patches (the blue door, the cellar stairs) and COP3B stamps. None of these exist natively, so the player can walk anywhere.

## Dependencies

- [Run scene dialogue, choices and flags from scripts](script-dialogue-choices.md)

## Requirements

- Hidden bodies block by occupancy even without art.
- Loading-script and script-driven tile patches apply to background and collision.
- A flag write in the current map refreshes roster and patches as the game does.

## Acceptance Criteria

- Before `$26` the D gate blocks; after it, it opens; the blue door and stairs follow `$292`.
- The slice's reachability without progression drops to what the game allows; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Result

- `COP 3B` stamps the actor's cell; a scripted leg lifts it. Blocking cells
  are visible bodies plus artless actors' stamps; the room is rebuilt when
  they change, and the host doorway action refuses a blocked faced cell.
- D's gate now holds the house: before `$26` only `$0B`–`$11` are
  reachable (new test); with `$26` the gate deletes itself (`COP 48`) and
  reachability is unchanged (19 of 24). The traversal tests now start after
  the Elder.
- In-map refresh: the gate is load-time only natively, so none is added.
- The blue door and cellar stairs (`$292` patches) move to
  [the pot-throw issue](pot-throw-door.md), where their trigger lives.
- Independent review approved. Applied: the doorway refusal counts only a
  blocked faced cell inside the matched doorway (a player standing in a
  doorway still opens it); entry uses the same blocking cells; an explicit
  test opens D's door with `$26` and refuses it without. Newly blocking
  artless stamps elsewhere (`$10` (27,22), `$1C` (19,6), `$11` (22,41),
  `$1B` (9,26)) do not change reachability and match the native grid bit.
