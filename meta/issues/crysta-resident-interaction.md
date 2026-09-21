# Place Crysta residents and let the player talk to them

## Summary

Spawn lists resolve for all 24 maps and a resident's script walks from its
spawn record to its dialogue. Put those together at runtime: residents stand
where the spawn lists place them, block movement, and speak when the player
interacts.

## Dependencies

- [Promote the Crysta room builder into a library](crysta-room-library.md)
- [Decode the actor script VM](decode-actor-script-vm.md)

## What is already known

- `SpawnList::resolve(image, map_id, events)` gives the records that apply for
  a given event-flag state; lists routinely repeat a position under different
  conditions, so the flags decide which resident is present.
- `SpawnRecord::origin()` is `(tile_x * 16 + 8, tile_y * 16)`, cross-checked
  against actor positions read out of WRAM on the reference emulator.
- `SpawnRecord::script()` is the record's pointer plus five, and
  `actor_script::walk_with_events` follows that to the resident's dialogue
  source and flag writes. The documented resident's whole chain reproduces:
  record `$83:8B96` → script `$88:8E50` → callback `$88:8EDE` → text `$88:8FF0`
  and flag `$8026`.
- `assets::text::HouseDialogue::decode_at` renders the pages at any source.

## Requirements

- Residents come from the spawn lists under the live event flags, not a fixed
  roster. A resident whose condition does not hold is absent.
- Interaction reaches dialogue through the script, since the documented chain
  reaches text through a callback rather than inline.
- A script the walker cannot account for leaves the resident silent and says
  so. 12 of 115 record scripts currently stop at an unaccounted service; those
  residents must not appear to work by falling back to a guess.
- Flag writes the script performs are applied, so progression gates move.

## Acceptance Criteria

- The documented resident can be walked up to and talked to, showing the pages
  the ROM holds, with flag `$0026` set afterwards.
- Residents occupy their cells for collision, so the player cannot walk through
  them.
- Every map's resident set matches what `SpawnList::resolve` reports for the
  current flags.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Play the Crysta slice free-roam in a native app](free-roam-crysta-app.md)
- Dialogue decodes to rendered pixel pages, not text, so presenting it is a
  rendering concern rather than a string concern.

## Progress: residents stand and speak, from their records

`crysta_runtime::residents` joins the spawn stream to the script walker, and
`World` carries the roster and the event-flag bitmap.

### Talking runs the callback, not the entry script

The entry script is what an actor does when it spawns; `$21` registers the
**interaction** callback, and that is what talking runs. Collecting both gives
the documented resident three pages where the chain has two — their ambient
line and their conversation are different text.

### The flags choose what is said, and the documented pages are not the first

`$88:8EDE` is a six-way `$08` dispatch over progression. Each condition has bit
15 set, so each branches when its flag is **clear**, and the arm
`docs/house-dialogue.md` records is the fall-through — reached only with all six
of `$109`, `$03B`, `$296`, `$021`, `$028`, `$026` set.

On a new game the first branch fires instead and the resident gives their
first-visit line at `$88:95B3`, which is a **choice prompt** the text decoder
does not render. That is reported as `Unsupported`, distinct from `Silent`,
because the script did reach text.

Census over the slice's records under new-game flags, before the chained
conditions were sized correctly: **4 speak, 6 unsupported, 16 unaccounted, 89
silent.**

### Chained conditions were sized wrongly, and `$47` is a despawn

`COP 09` and `COP 47` test a chain of flag words. Reading their handlers,
`$80:8695` and `$80:963A`: a word continues the chain when bit 15 is clear
and any of `$7000` is set, where `$4000` ors the next flag in, `$2000` ands
it and `$1000` exclusive-ors it; bit 15 ends the chain and inverts the
result. The old rule took bit 15 as continuation. `$09` is followed by a
two-byte branch target, which the old length did not include, so every walk
past one stopped at the target bytes. `$47` has no target: when its chain
holds the handler calls `$80:BD57`, which unlinks the actor from the scene
list, and abandons the stream. The walker now evaluates both, and the roster
leaves out a resident whose entry script despawns them.

The documented resident's entry script opens with `$47` on the exclusive-or
of `$027` and `$021`, so reaching their documented pages needs `$027` set
alongside the six callback flags, or they are not in the room to talk to.

Census after the fix: **16 speak, 14 unsupported, 17 unaccounted, 41
silent.**

### And the `$08` branch was inverted

It had been evaluated with the spawn stream's `$FA` rule. `$80:8678` is the
opposite sense: a word without bit 15 branches when the flag is clear, one
with it branches when the flag is set. So the six-way dispatch above falls
through to the documented pages on a **new game**, as `docs/house-dialogue.md`
recorded, and the choice prompt at `$88:95B3` is a later line. Census with
the sense corrected: **22 speak, 9 unsupported, 15 unaccounted, 42 silent.**

### Occupancy is available but off, and the reason is measured

A resident's collision cell is one row above the cell they visually stand in,
because movement samples at `(x - 8, y - 16)`. `world::occupy` blocks it
correctly, and is **not applied by default**: a spawn list is not a cast list,
and making all 115 records solid takes reachability from 19 maps to 2, because
records sit on the cells doorway approaches need.

## Remaining

- The `Unsupported` residents were not choice prompts but window controls;
  [the refused text controls](crysta-refused-text-controls.md) admits them.
  One remains, whose text address holds native code.
- 17 scripts stop at an unaccounted `COP` service; `$06`, a call through a
  long pointer that sits on the not-taken arm of most house scripts, is one.
- 89 records register no interaction callback at all. Whether those are scenery
  or residents whose callback the walker misses is not established.
- Telling records that are bodies from records that are not would let occupancy
  be turned on.
