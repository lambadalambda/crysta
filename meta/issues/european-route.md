# Check the European slice against a native route

## Summary

Record a native European route through the slice in the reference emulator
and run the story tests against it.

## Dependencies

- [Accept the European ROM in the app and the web page](european-hosts.md)
- [Run the European version at its own timing and sound](european-timing.md)

## Acceptance Criteria

- The European story tests pass against the recorded route.

## Progress

- An input-only native European New Game starts from empty SRAM, accepts
  the default name, plays Elle's wake-up dialogue to `$20`, walks through
  `$0F → $10 → $0C → $0B → $0C → $0D → $0A → $13`, completes the Elder's
  conversation (which grants `$26` before its choice is answered), and
  answers the weaver (`$28`). The fresh child process in
  `crates/oracle/tests/local_roms_eu.rs` verifies those milestones and the
  bedroom, exterior and weaver positions against the European ROM. Separate
  portable opening tests also cover the doorway and conversations; the
  single-world replay described below now connects these checkpoints.
- The headless native exploration continued, using only real buttons from the
  same empty-SRAM European boot: returning home grants `$27/$2E`, a thrown pot
  breaks the blue door (`$292`), the second Down approach opens Pandora's Box
  (`$22`), and the forced tour passes `$41 → $44 → $42 → $43 → $41`, sets
  `$243/$244`, and releases Ark to move on both axes. A reproducible fresh-child
  input fixture and native test check the opening and tour checkpoints.
- A second fresh child replays that prefix plus the European spear/return
  continuation without save-state restoration or memory writes. It checks
  `$240/$241/$242` and the actual spear inventory entry, the frozen return
  `$FE/$23`, Elder `$21` followed by mission `$296`, town `$3C`, and arrival
  on world-map `$03` at `(536,544)` at frame 73937. The headless native
  European route now reaches the requested endpoint. In
  `crates/crysta-runtime/tests/local_european.rs`, a single `World` from
  `fresh_game_flags()` now plays the bedroom, Elder, weaver, friend's choice,
  three real pot throws, Box and tour, spear pickup, frozen return, Elder's
  mission, town scene and south gate without re-entry, flag injection, debug
  placement or direct door strikes. It checks the native milestones and
  `(536,544)` on `$03`, then walks on the plane. English pages and choices
  are acknowledged when ready rather than at the native/Japanese frame counts.
  The town descent needs a few extra Left inputs to reach the native lane
  (`x≤356`); the untouched portable route otherwise stops at the wall.
  `Plane::tick` performs the native 16-pixel entrance walk on neutral frames
  from raw `(536,528)` to `(536,544)`. `World::in_transition()` ends after
  the dark load, before this plane arrival; Down input is not required.
- The continuous functional replay is green with the owned European ROM, but
  it does not prove full native-frame equivalence: the portable frozen-return
  scene does not model the scripted player repositioning. The town controller's
  `$88:855E` `COP BF` now redirects to `$88:8519` on the next tick rather than
  freezing or falling into the player's script; the replay observes no frozen
  actor after 48 neutral frames, with `$3C` set and input released. The target's
  animation cadence and native controller PC have not been compared. PAL load
  and movement timing beyond the bounded bedroom witnesses, route-wide sound,
  and native-frame text presentation still need their own evidence before this
  route and its dependencies can be archived. The timing issue now records a
  native-vs-direct track-4 tempo match for the steady bedroom window only.
- A fresh `crysta-web` `Game::new` host test also reaches and walks on `$03`
  from the bedroom without seeding flags or maps. Unlike the native and
  original continuous `World` route, it first **refuses, then accepts** the
  friends' retry (`$2F`, not `$2E`), opens the blue door with separate pot
  inputs, and detours through a frozen `$10` resident and the empty `$0B`
  Elder room before speaking to the `$0D` doorway Elder. The resulting
  28,020-frame input trace replayed through a headless browser's real
  keyboard handlers, PAL frame loop and European Wasm game without input-edge
  mismatch or game fault; a screenshot shows Ark on the world map. This
  qualifies a current browser-host path, **not** native frame-by-frame
  equivalence of that alternate detour. The original reporter's build/state
  remains unknown.
