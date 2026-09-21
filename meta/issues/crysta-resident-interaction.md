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
