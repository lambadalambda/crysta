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
- The scene's player scripts (`COP DF` with `COP 84` streams) are not
  modelled. New fixture-boundary assertions separate the states that were
  previously conflated: native releases the pad at `(136,464)` after its
  96-pixel scripted descent, then retained manual Left and Up inputs move to
  `(120,464)` and `(120,448)`. Portable releases at `(136,368)`; its immediate
  Left attempt cannot traverse that lane. Japanese movement waits are known at
  `$88:B32E/$88:B34C`, but the exact European map-`$21` player-stream entry and
  complete `COP 84` sequence are not source-pinned, so no coordinate snap or
  falsely general scripted-movement implementation was added. The particles
  stay frozen and the whitening's 37 frames are not waited.
- The way back up (selector 13 stairs), the Elder at D's door (`$21`,
  `$296`) and the town scene (`$3C`, an `FB` compact actor) run.
- Open against the acceptance criteria: positions. The fresh native route now
  pins `(136,464)` as the scripted release and `(120,448)` only after later
  manual input; portable still releases at `(136,368)`. Ordinary door arrivals
  use the raw placement (D: 8 pixels left; selector 5 loads at raw + (8,0)
  natively), and the stairs up into `$20` land 8 pixels low (natively
  `(360,872)`). The frozen residents' variants are not checked.
