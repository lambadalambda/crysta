# Depth order: doors, Yomi, the weapon

Native research for [door-entry-depth](../meta/issues/door-entry-depth.md),
[box-yomi-depth](../meta/issues/box-yomi-depth.md) and
[spear-pedestal-depth](../meta/issues/spear-pedestal-depth.md). Addresses are
JP; EU is the same unless given. References: `local/depth/` (see its
`README.txt` for frames, states and inputs).

## Hardware facts

- NMI copies shadows at `$85:FC66`: `$0468`→TM/TMW, `$0469`→TS, `$046C`→BG12NBA,
  `$046D/$046E`→BG1SC/BG2SC, `$046A/$046B`→CGWSEL/CGADSUB.
- House C (`$0C`): TM `$17`, BG1SC `$3C`, BG2SC `$38` (first layer = BG2), mode `$09`.
- Box tour (`$41`..`$44`): TM `$15` (BG1, BG3, OBJ), BG1SC `$38`, char base 0.
  The first layer is hardware BG1 here. No BG2 on the main screen.
- OAM buffer `$7E:0A00`. OBJ vs OBJ: lower OAM index wins, priority ignored.
  The winning OBJ pixel is then ranked against BG by its own priority
  (mode 1: OBJ1 < BG low < OBJ2 < BG high < OBJ3).
- Entity `+$08` bits 12-13 override OBJ priority. `COP BA nn` (`$80:AA6F`)
  writes `(+$08 & $CFFF) | nn<<8`.

## 1. Door entry: a priority-1 mask actor

Native (both ROMs, house C north door, `local/depth/{jp,eu}-door`):

- Ark keeps priority 2. The wall above the door is **low** priority BG; no
  BG change happens.
- The player-helper actor (`$0DF6`, created at `$84:BDD4`, `+$06 |= $4000`,
  art base `$A2:C000`) is put in mask pose `$37` by Ark's door-walk state
  (`$84:B988`/`$84:BA19` → `LDA #$BDF7; JSR $BD83`). `$BD83` copies Ark's
  position to the helper.
- `+$06` bit `$4000` puts it first in OAM (indices 0-13). It is 14 tiles
  `$51` (all colour 3), palette 2, **priority 1**, relative to the helper:
  rows dy -64..-57 at dx -8..+7, rows dy -56..-33 at dx -16..+15.
- Effect: where the mask is opaque it wins OBJ arbitration over Ark, then
  loses to opaque BG low/high, so the wall shows and Ark is hidden.
- Observed: mask appears when Ark is 17 px into the walk (y 352→335, helper at
  (136,336)); it stays until 3 frames into map `$0B`.

Our rule: `draw_sprite` hides a sprite pixel only under opaque high BG
(`frame.rs`). No mask exists.

Change: while the door walk runs, add an occluder rectangle (the shape above,
anchored at Ark's position when it starts) that hides every ordinary sprite
pixel under it where the BG pixel is opaque (low or high). Where BG colour is
0 the native shows palette-2 colour 3 (guess: never visible in these rooms).
Guess: the start trigger is `COP CB` at `$84:B988`; the 16-px offset is measured,
not decoded.

## 2. Yomi: priority 3 by COP BA

- Guide script `COP BA $30` at `$89:D2C1` (EU `$89:CBB1`) sets `+$08 = $3100`.
  OAM: Yomi's pieces are palette 5, **priority 3** in `$41`..`$44` on both ROMs
  (for the first ~20 frames after the fade in `$41` they are priority 2).
- `$41` BG1 high tiles: the bookcases' top halves (screen rows 80-103) and the
  desk top. A priority-2 Yomi is cut there; native Yomi is drawn over them
  (`local/depth/{jp,eu}-yomi`, Yomi at (72,120)).

Our rule: all residents are priority 2. `assets` already has
`PandoraActorPhase::priority_override = Some(3)`, but no runtime/app code reads it.

Change: carry an OBJ priority per resident (from `COP BA`, or the asset's
override). Priority 3 sprites ignore `Background::high`. Within sprites, keep the
depth sort.

## 3. The weapon: not a depth fault

- The display actor (`$83:957C`, script `$89:D9FE`):
  `COP B2 $FFF8` (y −8 → (72,360)), `COP D8 $A2C000`, loop `$89:DA07`:
  `COP 48 $8242`, `COP 80 08`, `COP 8E` (waits for the list to end), `BRA`.
- OAM: one 8×8 piece at (69,98) screen, palette 2, priority 2. Tile cycles
  `$60,$61,$62,$63,$62,$61`, 4 frames each (24-frame loop). Same on EU
  (actor resumes at `$89:D411`, also y 360).
- `$42` BG1 has **no** high-priority tiles. The pedestal is low BG; the
  sparkle is drawn on it (`local/depth/{jp,eu}-weapon`).

Ours: resident at (72,368) (no `COP B2`) and the pose is held on tile `$60`
(3×3 px), so it sits on the pedestal's bottom edge and looks covered. Our BG has
no high bits there; the depth rule is correct.

Change: apply `COP B2` to the resident's y, and let `COP 8E` wait for the list
end so that pose 8 cycles. Guess: our `COP 8E` fixed wait plus `COP 80`
restarting the list holds the first frame.
