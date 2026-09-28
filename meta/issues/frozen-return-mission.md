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
  and the town scene (`$3C`, an `FB` compact actor) run. On `$21` → `$20`, Ark
  loads at `(368,874)`, passes the transition-owned `(360,872)`, and settles
  with free control at `(360,880)`.
- C's exact `$818DCD` ordinary door record now places synchronous checked exits
  at the native `(120,608)` rather than the raw `(112,608)`. Every operand and
  the normalized source are pinned and mutations fail closed; explicit entry
  stays raw and the animated path still settles at `(120,625)`.
- The nine frozen-return source residents are now pinned by a fresh empty-SRAM
  European native census. It forks the retained direct-acceptance (`$2E`) route
  at C, follows `C → 10 → 11 → 10 → C → B → C → D`, and stops at
  `(120,625)` before talking to the Elder. Initialization is bound from the
  advanced source cursor at `$80:F5D3` to its entity slot through completion at
  `$80:F5E4`; linked membership follows `$0DFC → entity+$2C`, not stale slots.
  The witness checks creation and settled state, composition SHA-256, selected
  OBJ palette and all sixteen CGRAM words for C `$83:8C12/1C/26/30`, D
  `$83:8CBC`, 10 `$83:8D84/8E`, and 11 `$83:8DEA`. B `$83:8B9E` is likewise
  source-bound at creation and then correctly unlinked by `$27 XOR $21`.
- D's doorway Elder `$83:8CC6` is checked separately from those nine. The
  continuous portable direct route matches each visited resident's record,
  position, script, selector, flip and descriptor, and joins C `$83:8C12`'s
  live selector 1 to the qualified `32×32` RGBA raster (offset `(-16,-32)`).
  The continuous browser retry route correctly has only `$83:8C1C` in C under
  `$2F/$3F/$42`, while matching the visited 10/11/D variants and the separate
  doorway Elder before completing `$21/$296/$3C` and reaching the world map.
  The full native direct route, portable route, rebuilt Wasm browser replay,
  focused witnesses and strict scoped gates all pass.
