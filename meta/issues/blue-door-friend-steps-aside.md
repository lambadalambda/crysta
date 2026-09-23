# Let the friend at the blue door step aside

## Summary

The friend standing in front of the blue door in C does not step aside, as he does natively.

## Requirements

- Find the script or proximity rule that moves him and run it.

## Acceptance Criteria

- He steps aside as on the native route; tests pin his positions.

## Notes

- Reported by the user while playing the slice (2026-09-23).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Resolution

- His script already steps aside (`$88:9C16` left to (120,416), `$88:9C21`
  unlock, `$88:9C25` down to (120,512), `COP A7`), at the end of the second
  hit's reaction. In the reported session an A press during the reaction's
  locked pause ran the host doorway action and took Ark down the stairs
  before that. Doors and the doorway action now wait while a script holds
  the pad; A still talks (`$FF50` leaves it free).
