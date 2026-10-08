# Runtime review edge cases

Source reads of the JP ROM (runtime addresses) for
`meta/issues/runtime-review-edges.md`. "Guess" marks a claim without a
native measurement or a complete read. "Chapter 1" means Crysta, the world
map `$03`, the towers `$100`..`$123`, `$127`, `$12A` and `$12B`
(`docs/underworld-inventory.md`).

## 0. The entity list and the spawns

- `$0DFA` is the list head, `$0DFC` the tail. `+$2E` is the next entity,
  `+$2C` the previous one.
- `$80:BC54` inserts the child before the spawner. `$80:BC7C` inserts it
  after the spawner: child `+$2C` = spawner, child `+$2E` = the spawner's
  old next, whose `+$2C` becomes the child.
- `$80:BCA4` (all spawns) copies `+$00..$17` from the spawner, then sets
  child `+$04` = (copy | `$8000`) & `~$1000` (`$80:BCD2`..`BCDB`). The
  child starts hidden, with bit 12 clear. It keeps only bit 1 of `+$06`.
- The spawns with a flags word write `+$04` after that: `9B`, `9D`, `9F`,
  `A0`, `A2`, `A4`, `A6`. The others keep the inherited value: `9A`, `9C`,
  `9E` (before the spawner), `A1`, `A3`, `A5` (after it). `E6`/`E7`/`E8`
  are `A1`/`A2`/`A4` with a group.
- Placed actors take `+$04` and `+$06` from the script header (bytes 1-4,
  `$80:F5A9`, `$80:F63E`).

## 1. `COP 9C` children and the interaction test

Native: `$87:C783` walks the list from `$0DFA` and takes the first actor
whose `+$04` has bit 8 and whose box (`$7F:0028..002E`) holds the probe.
`$87:93B9` then checks only that actor: a callback (`$7F:0020`), and
`+$06` `$0200`, or `$0100` with the facing opposite. When the check fails,
nothing happens: no tile fallback and no lift (`$87:9282`).

Runtime (`world.rs` `faced_resident`): the first actor on the faced cell
whose callback and `+$06` qualify. `+$04` bit 8 is not kept.

Chapter 1:
- `COP 9C` is used only by the save desks (`$88:D618`, `$88:D62F`). The
  book inherits `$D100` (bit 8, hidden) and goes before the desk in the
  list, but its script clears `+$04` on its first frame (`$88:D641`). The
  difference lasts one frame. The runtime gives it flags 0.
- Bit 8 is set on all residents, gates, guardians, doors, chests and
  blocks; no enemy has it. A bit-8 actor without a callback blocks the
  test natively; the runtime looks past it. No chapter-1 layout puts one in
  front of an interactable actor (guess, from the spawn lists).
- Scripts that clear bit 8: the guide in `$21` (`$88:AF1A`) and a Crysta
  resident (`$88:99D8`), both before they leave or hide; the pad is held
  then (guess).

**Found while tracing:** the inherited spawns start hidden natively. The
runtime gives `9C` children flags 0 and `A1`/`E6` children the spawner's
`+$04` without `$8000` (`actors.rs` `spawn`, `SPAWN_AFTER`, `SPAWN_AT`).
Most `A1` children in chapter 1 are invisible controllers that never show
themselves (`$97:BCAD`, `$97:B3FB`, `$97:CD11`, `$90:89D4`, ...). Probe:
on `$10F` the flyer's burst controller `$97:BCAD` stays `hidden = false`
for its 40 frames, with the flyer's descriptor, so `art.rs` (the
"child draws as its parent" fallback) draws it as a second flyer.

Fix: for spawns without a flags word, child flags = spawner `+$04` |
`$8000` (bit 12 clear). Keep `9C` with that rule too.

## 2. `LDA $002C,X`

`+$2C` is the previous entity in the list (section 0). It is the parent
only for an after-spawn whose parent has not spawned again and while no
entity between them has gone.

- Flyer burst (`$97:BCD4`, `BCF6`, `BD18`): `COP E6` puts the controller
  after the flyer. The flyer spawns again only at the next `$97:BC48`,
  some poses later (guess: more than the burst's 40 frames). The guess
  holds.
- `$11D` orb trail (`$90:A2C9`): the orb spawns the trail (`$90:A1ED`).
  On the cape branch (`$90:A21D`, `$064C` = `$BF`) the orb then spawns the
  text actor `$90:A26E` after itself (`$90:A240`), which becomes the
  trail's `+$2C`. The text is cooperative (`COP 1B`, `COP 20`), so the
  trail stays where the orb was while the orb flies up. When the orb sets
  local flag 1 (`$90:A267`), the trail deletes itself (`COP 48 01 80`).
  The runtime's trail follows the orb. This branch is the normal path.

Fix: keep a `prev` id per actor. An after-spawn sets child `prev` =
spawner and moves the spawner's previous after-child's `prev` to the new
child. A deletion gives its `prev` to the actor that pointed at it. Read
`LDA $002C,X` from `prev`.

## 3. Mode-4 objects as foes

Native: the hit scans read `+$04` only. A target needs `$0200` and none of
`$04E2` (`$85:D2C3`); an attacker needs `$0200` and none of `$01D0`
(`$85:D30C`). The profile is `$7F:1022`, from descriptor byte 4
(`$80:FACF`), or the previous record's when the descriptor is reused
(`$80:FAF9`). `COP 65` sets `$0200` and clears the profile (`$80:9D25`).
The descriptor's mode (byte 3) plays no part.

Runtime (`actors.rs`, `foe`): header `$0200` and a profile. This is the
native rule. Nearly every chapter-1 actor is mode 4. Those with a profile
but without `$0200` (tower 2 statues `$90:9585`.., tower 3 blocks
`$90:9A29`.., the guardians, the `$12A` boulders) are not foes natively
either; no chapter-1 script sets `$0200` on them except through `COP 65`.
Leave.

## 4. The pending map `$047C`

Native: the main loop calls `$8D:86F8` each frame. When `$047C` is not 0
and `$04B8` is 0, it fades out by `$0484`, sets `$0482` = `$047E`,
`$047E` = `$047C`, clears `$047C`, loads, fades in, and clears `$0484`.
The load places Ark at (`$0492` + 8, `$0494` + 16) when either is not 0,
else he keeps his position; then it clears `$0490`, `$0492`, `$0494`
(`$80:F7F3`, `$80:F8DE`). A bare `$047C` write therefore reloads with
mode 0, selector 0, Ark where he stands, as the runtime does
(`world.rs` `run_actors`, `PENDING_MAP`). The runtime does not set the
previous map to the current one; no chapter-1 script reads `$0482` after
that reload (guess).

Bare `$047C` writes in chapter 1: `$90:8B0F` (`$07`, handled) and
`$90:A4B7` (`$047E`, handled). The others (`$90:AE8D`, `B46D`, `B56D`,
`BBC1`, `$93:D60D`, `D758`, `$97:AA17`) are in no chapter-1 spawn list.
Leave.

## 5. Ark's own script

Pokes and group deletions from the `COP DF` script apply on the next
frame. The chapter-1 `COP DF` scripts (`$90:FB0B` family, `$90:8B74`,
`$93:D84E`, `$97:C5A9`, `$88:AF72`, ...) write no other entity and delete
no group. They write Ark's own `+$04` (`$97:C5B8`: bits `$0030`, not kept)
and engine words. Leave.

## 6. `World::place()`

Only the tests call it (13 calls); the app does not. Native has no
counterpart. A drop (`jump`), a fall, the rope, a thrust and a recoil
carry on from the new position. Cheap fix: clear them in `place()`.

## 7. `+$04` bit 12

Bit 12 does not gate the main scheduler. `$80:C8E1` (main loop,
`$80:819E`) skips an actor on `+$06` `$0400` or `+$04` `$0040` only.
`$80:C85E` tests bit 12; `$80:80DF` calls it. `$80:80DF` is the nested
frame used while code waits inside one frame: the transfer fades
(`$8D:89D4`..`8ADE`, from `$8D:86F8`), the guide's whitening
(`$88:B523`), `$8D:DA27`, `$92:CF32`. With `$04A0` bit 7 it takes
`$80:C76E` instead (guess: the world map).

Who sets bit 12: script headers (`$5100` residents, gates, guardians,
doors; enemies do not have it), flags words (`$1010`, the freeze
crystals), `COP D5`, `COP 77`, `COP DC` (`$80:B416`, `B468`, `B712`).
Inherited spawns clear it. Who clears it: the guide on itself
(`$88:B50A`, so it does not run again inside its own wait) and Ark's
scripts on Ark (`$84:87E5`, `$84:A2F6`.., `$85:A3E6`, `$92:CEA7`).

So, natively, during a transfer's fade only actors with bit 12 run:
enemies, chests and children without it stand still. The runtime runs
every actor on the fade's game frames (`world/fade.rs` `fade_frame`). The
fade module's comment is wrong on this point.

Fix: keep bit 12 per actor (header, flags word, the rule in section 1),
accept its writes, and run only bit-12 actors on the fades' frames (and
in the whitening, once it yields).
