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
  bedroom, exterior and weaver positions against the European ROM. Portable
  `crates/crysta-runtime/tests/local_european.rs` checks the same opening
  doorway and, separately, the Elder and weaver choices. All pass with the
  owned European ROM. This opening witness is not a continuous portable
  replay.
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
  European route now reaches the requested endpoint. Portable European tests
  separately cover the frozen Elder's `$21/$296` conversation and the town's
  `$3C` scene through the south gate. They seed the preceding flags at each
  checkpoint rather than replaying the journey continuously: the portable
  map `$03` currently needs an explicit 16-frame Down input to reach native
  settled `(536,544)` from raw `(536,528)`. The town controller also
  stops at unsupported `$88:855E` after granting `$3C`; it releases input
  and the exit remains playable, but its remaining behavior is unqualified.
  The full continuous portable replay and native-frame timing/text
  presentation remain open; English dialogue acknowledgements cannot be
  assumed from Japanese timings.
