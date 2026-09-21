# Derive resident art from spawn records across the Crysta slice

## Summary

`HouseScenes` decodes sprite art for nine frozen residents in six house maps,
each named by a hand-written profile. The free-roam runtime places residents
from spawn lists across all 24 maps, so their art has to come from the record
itself: the resource descriptor the record points at, and the pose the entry
script selects.

## Dependencies

- [Place Crysta residents and let the player talk to them](crysta-resident-interaction.md)

## What is already known

- A `$01` spawn record is ten bytes: opcode, tile X, tile Y, flags, a script
  pointer and a resource descriptor pointer. `HouseScenes` follows the
  descriptor to a compressed composition packet, a palette descriptor and a
  graphics packet, and validates each shape.
- The entry script's header byte is the initial pose selector. The ordinary
  interaction loop sets the pose again: clear or set H-flip (`COP B6`/`B7`),
  select animation (`COP 80 n`), then wait (`COP 8E`).
- A descriptor of `$000000` reuses the previous record's resource, and graphics
  `$FFFF` reuses the previous record's graphics.

## Requirements

- One loader, driven by the record and the walked script rather than by a
  profile table, produces the same art as `HouseScenes` for every frozen
  resident.
- A record whose descriptor has a shape the loader does not qualify is refused,
  and the runtime draws a placeholder rather than a guess.
- Depth at equal Y follows the spawn list: later records draw first, and the
  player draws last. This matches every frozen tie rank.

## Acceptance Criteria

- For each of the nine frozen residents, the record-derived actor has the same
  composition bytes, palette, palette base and position as the frozen one.
- A census over the slice's records reports how many decode, and the number is
  recorded here.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Native macOS window, renderer and gamepad for the Crysta slice](crysta-native-shell.md)

## Progress: the loader is the record, and the frozen nine reproduce

`HouseActor::from_records` decodes a map's spawn records in list order,
following each `$01` record's descriptor with the shape checks the frozen
loader already had, and `ResidentPose::from_script` derives the pose by
walking the record's entry script to its first `COP 8E`. For all nine frozen
residents the result has the same composition bytes, palette, palette base,
position, selector and mirror as `HouseScenes` -- with the pose coming from
the walked script, not from the profile.

### Two residents take a fallback pose

The map-`$0011` resident runs an intro on a new game, gated on flag 1 being
clear, that ends in native code the walker does not execute. The map-`$000D`
wanderer loops on `$026` until the command budget runs out. Neither walk
reaches a wait under new-game flags, so the pose is taken from a walk with
every flag set: one-time sequences are gated on a clear flag they set when
they finish, and that walk reaches the ordinary loop, or stops short of it and
keeps the header's setup pose. Both match the frozen roster.

### The reuse chain is owned by the loader

A record may reuse the resource or graphics of the record before it. The
native loader always has that predecessor; this one may have refused it. In
map `$000A` the record at `$83:8A19` has a descriptor mode outside the
qualified set and the three records after it reuse its resource; handing them
the last body that *did* decode drew the wrong sprite and reported success.
The list-level constructor refuses such a reuse instead.

### Census, new-game flags

| Outcome | Records |
| --- | --- |
| Decode | 39 |
| Carry no descriptor (`$00`/`$FD`, a script with a position) | 46 |
| Reuse a refused predecessor | 5 |
| Refused | 25 |

Of the 25 refused, 23 are one record present in every map, descriptor
`$83:ED4F`: palette offset 6 and base 176, outside the values the frozen
loader qualified, with graphics reused rather than loaded. What it is has not
been established. The other two are a movement-resource mode of `$22` in map
`$000A` and a palette table selector of `$90` in map `$0021`.

Neither the record's flag byte nor the header's tail names art. Byte 3 is
`$00` on 58 of 61 descriptor-bearing records and `$01`/`$02` on the rest; the
header is `[selector, n, flags, 0, n]` with flags `$41`/`$50`/`$51`/`$D1`
seen. Refusing on them would drop a third of the drawable residents for bytes
that seed the actor VM, not the graphics, so they are read and left alone.
