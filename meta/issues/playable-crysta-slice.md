# Make the Crysta slice fully playable

## Summary

Bring the opening area — the first house, the town of Crysta, and every
building in it — to a state where a player can walk all of it, enter every
door, and talk to everyone, driven by the portable core and ROM-derived data.
This is the parent for that program; each capability lands as its own issue.

## Dependencies

- [Complete the Crysta and Pandora vertical slice](opening-vertical-slice.md)

## Scope

The static exit graph bounds the slice at **24 maps**, `$000A..=$0021`:

| Group | Maps | State |
| --- | --- | --- |
| Town exterior | `$0A` | background decodes; not admitted for walking |
| First house | `$0B`–`$11`, `$20`, `$21` | 6 of 9 rooms walkable; `$0E`, `$20`, `$21` unentered |
| Other buildings | `$12`–`$1F` | untouched |

`$0F` also exits to `$0122`, which belongs to a different area and is out of
scope here. `$0A` exits to `$0003`, the wider world, likewise.

The other buildings form four clusters plus three single rooms: `{$12,$13,$14,$15}`,
`{$16,$17,$18,$19}`, `{$1A,$1B,$1C}`, and `$1D`, `$1E`, `$1F` entered directly
from the exterior.

## Approach

The six walkable rooms were reached by hand-qualifying each one. That does not
scale to 24, and would not converge. The remaining work is instead to make the
**generic ROM-derived path** carry the whole slice:

1. **Map loading for every map.** The script projection is the general
   mechanism; `visual.rs` still carries a per-map allowlist and fixed resource
   offsets that do not generalise across subscript groups.
2. **Collision for every map.** The attribute decode covers 96% of cells in the
   maps that currently load; the remainder and the exterior's own attributes
   are open.
3. **Exits and transitions.** `ExitList` decodes statically; nothing wires it
   into the portable core.
4. **Actors and dialogue.** Residents are censused for the house only, and
   dialogue is per-room qualified.
5. **Events and progression.** The gates that open doors, starting with
   `$0026` for the house exterior.

## Progress

- **Map loading: done.** All 24 maps decode a background and a collision grid
  from ROM. Seventeen had no qualified profile before. The generic path selects
  loads by their VRAM destination operands, and reproduces the seven
  independently qualified fixed-offset profiles exactly.
- **Collision: 99.4% of cells.** `0`, `2` and `22` walkable; `12`, `14`, `16`
  and `25` solid, measured from movement in two maps. What remains is `5`, `6`,
  `7`, `8`, `21` and `29`, 296 cells, and none of them affects connectivity.
- **Portable core: every map walks.** Each of the 24 becomes a `room-core`
  room with no hand-written profile, and the core refuses to cross the town's
  impassable band.
- **Connectivity: 6 of 24 maps**, blocked by one mechanism worth 17 of the
  remaining 18. See
  [the door-entry trigger](decode-door-entry-trigger.md).
- **Connectivity, with doorways: 24 of 24.** Doors are opened by facing and
  interacting, measured live on the town's own front door; modelling an exit
  rectangle as enterable from an adjacent walkable cell reaches the whole
  slice. Not yet implemented in the portable core.
- **Actors: every map's spawn list decodes.** All 24 lists, 115 records, with
  every element length read from the interpreter at `$80:F4EA` rather than
  fitted. All nine documented residents decode at their source addresses, and
  origins are cross-checked against actor positions measured in the running
  game.
- **Spawn conditions are evaluated.** `$FA` is a conditional branch through
  `$80:BBC7`, the same flag routine the loading scripts use, with the sense on
  bit 15 and chains accumulating as parity. Following those branches against
  the measured new-game flags resolves all 24 lists, reproduces every
  documented resident, and matches the census's counts in four of six rooms —
  the two that exceed it each hold a documented hidden actor. The `$4000` and
  `$2000` condition variants are refused rather than guessed.
- **A played traversal walks all 24 maps.** The collision reference is settled
  from source: `$80:940D` computes the sample as `(x - 8, y - 16)`, the same
  corner the exit probe uses, so the player occupies a 16x16 box and
  cell-indexed pathfinding was simply the wrong model. Expressed in pixels
  instead, with every edge produced by stepping `room-core` until the player
  actually moved a cell, the player reaches an onward exit approach from its
  arrival position in every map. That is the graph connectivity result
  confirmed by simulation rather than asserted.
- **`room-core`'s qualified material aliases are installed.** `TownSolid25`,
  `ClosedDoorPartial5` and `StairOpen29` were already qualified but had to be
  opted into per room; the Crysta builder now does so. The core consequently
  agrees with the measured town band instead of refusing to classify it.
- **Dialogue and progression are one dependency, not two.** A spawn record's
  pointers are scripts, not text: decoding all 189 pointer fields as dialogue
  yields five single-page results, which is coincidence rather than lines. The
  documented chain runs record → script → callback → text, with the `$0026`
  flag written inside it, so both capabilities need
  [the actor script VM](decode-actor-script-vm.md). **That VM now decodes the
  documented chain**: scripts are native code whose commands are `COP`
  signatures, and walking the callback `$88:8EDE` yields its dialogue source
  and its progression flag at the hand-documented sites. What remains is
  following branches, accounting for the remaining services, and establishing
  the path from a spawn record to its script.

## Requirements

- Each capability is qualified and landed separately, with its own evidence.
- The generic path replaces per-room qualification rather than sitting beside
  it; a room that loads generically must not need a hand-written profile.
- Every map in the table above is reachable in the portable core, through real
  exits, without warps or state restoration.
- Fidelity gaps are written down per map rather than implied by silence.

## Acceptance Criteria

- A deterministic replay walks from the fresh-game start to every one of the 24
  maps through ordinary movement and exits.
- Collision, backgrounds and exits for those maps come from decoded ROM data,
  not per-room tables.
- Residents are present and interactable in every room that has them.
- Documented remaining gaps are specific and per-map.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- This is the user-visible goal the M3 decoders and M4 ports serve; it does not
  replace them, it sequences them against one concrete playable target.
- It is deliberately independent of
  [the tower-approach route](qualify-tower-approach-route.md), which is stuck on
  a separate research question and gates the Pandora continuation rather than
  Crysta.
