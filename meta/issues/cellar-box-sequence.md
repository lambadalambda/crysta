# Open the box in the cellar

## Summary

The stairs lead through E and `$20` to `$21`, where a voice calls for help, a warning plays, and the box opens once both locals are set and Ark stands in its polling rectangle.

## Dependencies

- [Lift and throw pots to break the blue door](pot-throw-door.md)

## Requirements

- Admit `$20` and `$21` with their entry controllers.
- Contact callback, warning, polling gate, `$22` and the map reload.

## Acceptance Criteria

- The box opens at the same step of the native route; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Progress

- Stairs (selector 14) settle at the raw anchor plus (8,16): C → E
  (152,880) → `$20` (408,880) → `$21` (136,128), as natively.
- Spawn lists now run their `FE` controllers (the box room's entry and
  warning controller `$88:AD89`, and one in C); the player's own `FD` record
  (`$84:A129`) is skipped.
- Contact callbacks (`$7F:1010`), the player's recoil, `COP 0D` / `DF` /
  `14` and the transfer run. The native route's presses from `21-toward-box`
  reproduce the contact at 370, the recoil to 359, the warning, `$22` at
  (136,368) and the reload.
- The host doorway action no longer opens C's stairs through the closed
  blue door; E, `$20` and `$21` need the door broken.
- Map loads apply the flag-gated patch table `$96:CD9D` (`$8D:8FB4`), so C
  reopens its stairs from `$292` after a layer reload, as natively.
- Not modelled: fades (the reload comes at once, natively about 230 frames
  later), the box's `$22` path (`COP 99`), which the tour issue takes up.
- New controllers stop early: `FE` `$88:8062` in E and `$20` at `COP 99`,
  the `FE` in `$1D`/`$1E` at `COP DA` (`$92:CC8A`); C's `FE` `$88:ACAE`
  runs but its warning before `$28` does not fire yet. None has a body or
  locks the pad.
