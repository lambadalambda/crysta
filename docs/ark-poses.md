# Ark's scripted poses: stairs, item grant, Yomi's run-out

Research for [stair poses](../meta/issues/stair-poses.md),
[weapon overhead pose](../meta/issues/weapon-overhead-pose.md) and
[Yomi run-out](../meta/issues/yomi-run-out.md). Measured on both ROMs from
input-only native runs (Japanese: `tools/tower-approach-qualification/tower-route.jsonl`
with checkpoint save-state sync; European: from `local/pots/states/eu-pot-first-ready.state`
on). Both ROMs give the same sequences; only resource banks and the
frozen-return script move. Timings differ by at most one or two frames per
record (scheduler countdown, load length).

## How the native game picks Ark's frame

The player entity (`$1000`) runs a script (`+$0A/+$0C`). `COP 83`/`COP 84
pose, movement, resource` selects a list in one of Ark's resources: the
6-byte table at `$80:A24F` (same address in both ROMs).

| Resource | Japanese base | European base | Holds |
|---|---|---|---|
| 0 | `$A4:A1E4` | `$A6:A1E4` | standing 0..2, lift-stand 3..5, brake 9..11, recoil `$0C`.. |
| 1 | `$9A:D064` | `$9C:D064` | walk 0..2, stairs `$13..$16`, dash `$17..$19` |
| 3 | `$A6:9C70` | `$A8:9C70` | lift `$18..$1A` (`$24..$26` is the pot lift) |

List pointer: word at `base + 2*selector`. Record: duration byte (shown
duration+1 frames), facing byte, composition word (`base + word + 4`); a byte
`>= $80` ends it. Live state: `$7F:1008` selector, `$7F:100A` composition,
`$1014` facing, `$1008 & $4000` mirror.

Fixtures (CSV `frame,map,x,y,facing,script,resource,anim_base,selector,composition,hflip`,
one row a frame) are in `local/poses/{jp,eu}/`: `stairs-down-C-E`,
`stairs-down-E-20`, `stairs-down-20-21`, `stairs-up-21-20`, `stairs-up-20-E`,
`spear-get`, `spear-item-oam`, `yomi-run-out`. Probe and scripts:
`/Users/lainsoykaf/.claude/jobs/ef2e9592/tmp/poses/` (`poseprobe`, `jp.jsonl`,
`eu*.jsonl`, `fixtures.py`).

## 1. Stairs

Native: the exit swaps the player script for a stair script that plays one
resource-1 list. All four lists come from the ROM; none mirror.

| Walk | Script | List | Compositions (low words), facing byte |
|---|---|---|---|
| Down, leaving (selector 14) | `$84:BB36` | `$16` | `D64A D67E D6B9 D6FB D72F` (Up walk 1..5, f1), `EECC EEF2` (f4), `D76A` (horizontal walk 0, f3) |
| Down, arriving | `$84:BD7E` (then `BD82`) | `$13` | `EE42 EE61` (f5), `D549 D584 D5C6 D48A D4C5 D507` (Down walk 3,4,5,0,1,2, f0) |
| Up, leaving (selector 13) | `$84:BAF6` | `$14` | `D72F D608 D64A D67E` (Up walk, f1), `EF18 EF3E` (f7), `ECCB ED0D` (f2, unmirrored) |
| Up, arriving | `$84:BD3E` (then `BD45`) | `$15` | `ED48` (f2), `EE87 EEA6` (f6), `D48A D4C5 D507 D549 D584` (Down walk 0..4, f0) |

Timing: every record 8 frames (duration 7). Leaving starts on the exit frame
(Japanese 24987, our `STAIRS_*_LEAVING` frame 0); its first record is 7 frames,
the last is held until the load. Arriving: `$84:A12E` (arrival set-up, OAM
not yet refreshed, screen dark) for 8..9 frames, then the list; the first
record holds 9..11 frames. Japanese E to `$20`: list from 25607, our
`STAIRS_DOWN_ARRIVING` frame 9. After the list Ark stands facing **Down**
(`$84:A303`, `A54E`), whatever way he came in.

Ours: `transition.rs` `move_by` advances the ordinary walk in
`self.facing` (Up, as Ark pressed Up into the stairs) on moving frames and
selects standing on every pause frame, so Ark flickers between standing and
walking Up on the whole descent, and arrives standing Up.

Change:
- `assets`: decode resource 1 lists `$13..$16` too (`ark_art(loader, 1, ..)`,
  next to `run_art`); `CarryArt` serves them.
- `transition.rs`: give each stair motion its list (leave 14: `$16`,
  leave 13: `$14`, arrive 14: `$13`, arrive 13: `$15`) and a start frame
  (leaving 0, arriving 9). Expose the pose (resource 1, list, age) through a
  `World` accessor like `run_pose()`; draw it in `Session::carry_sprites`.
  Stop calling `animation.advance` for stair motions.
- End of a stair arrival: `facing = Down`, standing Down (both selectors).

## 2. The spear held up (COP 60)

Native (`$89:DA20 COP 60 81 A4 01 34`, handler `$80:9A04`, both ROMs): it
sets the player script to `$84:BEA2`, `$7F:1020` (on the player) to the frame
word (`$01A4` = 420), and parks the held-object entity (`$0DEE`) at `$84:B7E3`.
`$84:BEA2` uploads the item icon to VRAM (`JSL $84:D628`, European `D60E`),
starts the item entity (`$0DF4`) at `$84:C020` (`JSL $84:D6DB`, European
`D6C1`), then by facing (`COP 5F`): `COP 84 18/19/1A 00 03` (lift, resource 3;
Left mirrors with `COP B7`), then `COP 84 03/04/05 00 00` (lift-stand,
resource 0) and the item entity to `$84:C0B2`. It waits until `$7F:1020`
runs out, clears `$09C7`, plays the map's music (`COP 32 FF`) and returns to
ordinary standing (`$84:A2E9`).

Measured, facing Up (Japanese from 47427, European from 51350, same lengths):

| Frame | Ark | Item (16x16 OBJ, tile `$6A`, palette 7) |
|---|---|---|
| 0 | script `BEA2`, still standing Up | not drawn |
| 1..14 | lift `$19` record 0, `A9AA` | OAM at Ark + (-8, -41) |
| 15..22 | lift record 1, `A9EC` | Ark + (-8, -45) |
| 23..419 | lift-stand `$04`, `A65D` | Ark + (-8, -43) (entity at `$0952+8, $0954-10`) |
| 420..422 | `BF21`, still `A65D` | gone |
| 423 | standing Up `A597` | gone |

OAM offsets are OAM x/y minus Ark's screen position; the SNES draws a
sprite one line below its OAM y. The icon art and palette are what
`assets::shop_display::item_icon(image, 0x81)` decodes (`$A8` item table,
`$B1:DA31` palette index, OBJ palette 7 colours 8..15). Our lift and
lift-stand poses already exist: `PandoraSprites::carry_pose(Lifting, f)` and
`carry_pose(Standing, f)`.

Ours: `actors.rs` `GRANT_ITEM` adds the item and the fanfare only ("The
player's presentation pose is not drawn"). Ark keeps standing; no icon.

Change: `GRANT_ITEM` starts a world presentation `{ item, frames, age }` that
locks the player. `Session::carry_sprites` draws `carry_pose(Lifting, facing)`
once for ages 1..22, then `carry_pose(Standing, facing)` to age 422, and the
icon at the offsets above for ages 1..419. Then ordinary standing in the same
facing.

Guess, not measured: other facings. The item entity's lift follows the pot
lift lists `$2B/$2C/$2D` (`$84:C058`, Down also +8 in Y and priority), and
the hold position does not depend on facing.

## 3. Yomi sends Ark out (frozen return)

Native: map `$21` guide `COP DF` runs player script `$88:AF72` (European
`$88:B7F6`), 191 frames (Japanese from 52801, European from 53882):

| Frames | Script | Pose | Motion |
|---|---|---|---|
| 0 | `AF72` | standing Down `A54E` | none |
| 1..36 | `COP 84 17 0F 01` | dash Down: resource 1 list `$17`, `EF64 EFA6 EFE8 F01C F05E F0A0`, 6 frames each | +3,+2,+2 a frame, y 368 to 454 |
| 37..52 | `COP 84 09 1B 00` | brake Down: resource 0 list 9, `A7A7`, 16 frames | to y 464 |
| 53..114 | `COP 84 00 00 00`, `COP C1 3C` | standing Down `A54E` | |
| 115..176 | `COP 84 01 00 00`, `COP C1 3C` | standing Up `A597` | |
| 177.. | `COP 84 00 00 00` | standing Down, then ordinary (`$84:A303`) | |

Before it Ark faces Down: the 42 to 21 transfer lands him facing Down
(Japanese 48789), the guide turns him Left (`$88:AF50`: `COP B7`,
`COP 84 02 00 00`) and Down again (`$88:AF62`).

Ours: `actors.rs` admits exactly this stream (`cadence::FROZEN_RETURN_SELECTIONS`)
and moves Ark, but nothing reads its pose: `World::run_pose()` only knows the
pad dash, and the atlas draws `animation()`, standing in `self.facing`. That
facing is still Up from the spear, because script transfers keep the facing
(`fade.rs` `entered.face(self.facing)`) and `$88:AF50`/`AF62` are not admitted
player scripts. So Ark slides down standing Up.

Change:
- `Actor`: expose the admitted player pose (resource, list, mirror, age).
  `World::run_pose()` maps resource 1 list `$17` to `(Dashing, Down, age)` and
  resource 0 list 9 to `(Braking, Down, age)`; both lists are already in
  `CarryArt`. Resource 0 lists 0/1/2 set `facing` (Down/Up/horizontal with the
  mirror) and standing.
- Face Down when the mode-4 transfer from `$42` lands in `$21` (guess:
  every script transfer lands facing Down; only this one is measured).
- Admit `$88:AF50`/`AF62` as player streams of the same profile, so Ark
  faces Left, then Down, before the run.

## Open questions

- Do the stair compositions (`EE42`.., `EF18`.., `ECCB`..) pass the
  sprite decoder's shape checks? Not tried.
- Item presentation for facings other than Up, and when Ark is off the cell
  grid (`$0952/$0954`), is not measured.
- Arrival set-up length (8 or 9 frames) depends on the load; the fixtures
  hold both revisions' exact frames.
