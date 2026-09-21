# Play the Crysta slice free-roam in a native app

## Summary

Run the 24-map Crysta slice as free exploration in a native macOS window with
gamepad input: walk anywhere the collision data allows, pass through every
doorway, and talk to the residents the spawn lists place. This is the parent
for that program; each capability lands as its own issue.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Why this is not the existing preview

`map_inspector::room_preview::Preview` already drives a browser viewer, but it
is a **qualified corridor, not a town simulation** — its own documentation says
so ([`docs/pandora-navigation.md`](../../docs/pandora-navigation.md): "It is not
a town simulation or VM"). Building a native shell over it would not deliver
free exploration:

| | `Preview` today | This issue |
| --- | --- | --- |
| Maps admitted | 15 | 24 (`$000A..=$0021`) |
| Walkable extent | hand-qualified halos; `$21` is a single 16px column, `$42`–`$44` one cell each | whatever the collision data allows |
| Leaving the admitted path | latches a permanent error; `step` becomes a no-op until reset | ordinary movement |
| Talkable residents | 2 | whatever the spawn lists place |

That corridor is the right shape for *qualification* — it fails closed, which
is what makes its parity claims meaningful. It is the wrong shape for playing.

## What already exists

The data side is largely done, and the portable core is already general:

- `room_core::Room` takes a cell grid and a material policy. **Without
  `with_sample_halo` it has no corridor restriction**, so free roam needs no
  change to the walking core.
- `assets::maps::visual::StaticBackground` decodes any map's layer and
  attribute table; `MapCell::qualified_passability` classifies 47,320 of 47,616
  cells across all 24 maps.
- `assets::maps::exits::ExitList` gives exit geometry, and the door-entry
  mechanism is measured.
- `assets::maps::actors::SpawnList` resolves all 24 spawn lists, and
  `assets::maps::actor_script` walks a resident's script from its spawn record
  to its dialogue and flag writes.
- `map_inspector::visual_export::render(rom, map_id)` renders any map to a BMP.

`crates/map-inspector/tests/local_crysta_rooms.rs` already builds a `Room` for
every one of the 24 maps and walks the player to an exit approach in each. That
test *is* the free-roam foundation; it is only trapped in a test file.

## Scope

In:

- Free walking across all 24 maps with the qualified collision policy.
- Map-to-map transitions through the real exit geometry.
- Residents placed from spawn lists, with collision, and dialogue on interact.
- A native macOS window, framebuffer rendering and gamepad input.

Out, and not implied by "playable":

- Combat, inventory, menus, saves, audio.
- Animation fidelity beyond what the existing art manifest supplies.
- Anything outside the 24-map slice.

## Acceptance Criteria

- The player can walk from the opening house through the town and into every
  building that has a bound doorway, on a gamepad, in a native window.
- Residents appear where the spawn lists place them and respond to interaction
  with their own dialogue.
- Nothing fails closed during ordinary play: leaving a qualified cell is either
  permitted or blocked as movement, never an error that freezes the simulation.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Make the Crysta slice fully playable](playable-crysta-slice.md)
- Windowing, framebuffer and gamepad use `winit`, `softbuffer` and `gilrs`,
  chosen deliberately over a hand-written Cocoa shim. All three build on the
  qualification machine. This roughly quintuples the lockfile, from 42 packages
  to about 222, which is the largest single change to this repository's
  dependency surface; it is confined to the app crate.
