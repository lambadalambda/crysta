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
  scene does not model the scripted player repositioning, and the town actor
  remains frozen at unsupported `$88:855E` after granting `$3C`. Input is
  released and the gate is playable; the actor's remainder is unqualified.
  PAL load/music tempo and native-frame text presentation still need their
  own evidence before this route and its dependencies can be archived.
