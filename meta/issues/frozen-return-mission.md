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
- The frozen-return player handoff is now source-pinned in both revisions. The
  guide's `COP DF` is `$88:AF30` in Japanese and `$88:B7B4` in European; its
  player entry is `$88:AF72` / `$88:B7F6`. The complete 52-byte stream is
  identical modulo relocation. Only its five exact `COP 84` sites, four direct
  Ark pose selections, resource pointers, display-list lengths and two movement
  streams are admitted; a changed guide target or profile freezes instead of
  falling through the generic skipped-service path.
- The first moving pose runs 36 ticks and moves down 84 pixels, from Y=368 to
  452. The second runs 16 ticks and requests another 15 pixels; the ordinary
  special-player collision resolver applies 12, including a clipped +1 at
  Y=463, then rejects the remaining positive samples at Y=464. This is
  per-displacement collision projection, not a coordinate snap. Script-owned
  motion suppresses manual input, then control releases at `(136,464)`;
  retained manual Left and Up inputs reach `(120,464)` and `(120,448)`.
- A fresh European native witness pins the complete stream, raw and resolved
  movement on every tick, Y on every completed frame through neutral release,
  `$FE/$23`, the zero input mask and Y=464 through frame 64835. Focused Japanese
  and European runtime tests and the continuous European route pin the same
  release/manual boundaries. The whitening's particles still stay frozen and
  its 37 nested frames are not waited.
- The way back up (selector 13 stairs), the Elder at D's door (`$21`, `$296`)
  and the town scene (`$3C`, an `FB` compact actor) run.
- Open against the broader acceptance criteria: ordinary door arrivals use the
  raw placement (D: 8 pixels left; selector 5 loads at raw + (8,0) natively),
  and the stairs up into `$20` land 8 pixels low (natively `(360,872)`). The
  frozen residents' variants are not checked.
