# Play the frozen return and the Elder's mission

## Summary

Ark returns to `$21`, Elle is frozen (`$FE`, `$23`), the village residents are frozen, the Elder at D's door sets `$21` and, after acceptance, `$296`, and the town controller moves Ark and sets `$3C`.

## Dependencies

- [Play the tour inside the box and take the spear](box-tour-and-spear.md)

## Requirements

- Return scene, frozen resident variants, the doorway Elder with choice, the town scene.

## Acceptance Criteria

- Flags and positions match the native route; tests and gates pass.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice).
- Parent: [Play the Crysta story from the wake-up scene to the world map](play-crysta-story.md)

## Progress

- The return scene in `$21` runs to `$FE` and `$23` with a free pad:
  `COP 00`/`01`, `COP 99`, `+$06` interaction writes and the whitening's
  native effect code.
- Gaps: the scene's player scripts (`COP DF` with `COP 84` streams) are not
  modelled, so Ark ends at (136,368), natively (136,464); the particles stay
  frozen; the whitening's 37 frames are not waited.
- Remaining: the frozen village, the Elder at D's door (`$21`, `$296`) and
  the town scene (`$3C`).
