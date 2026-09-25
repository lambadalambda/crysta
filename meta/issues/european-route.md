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
  owned European ROM. This is **not** the requested bedroom-to-world-map
  native route or a continuous portable replay.
- The headless native exploration continued, using only real buttons from the
  same empty-SRAM European boot: returning home grants `$27/$2E`, a thrown pot
  breaks the blue door (`$292`), the second Down approach opens Pandora's Box
  (`$22`), and the forced tour passes `$41 → $44 → $42 → $43 → $41`, sets
  `$243/$244`, and releases Ark to move on both axes. A reproducible fresh-child
  input fixture and native test check the opening and tour checkpoints. This
  is still **not** a continuous portable replay or the world-map route.
- Next: collect the spear, return to frozen Crysta, receive the Elder's mission,
  and reach world-map `$03`; check the portable story against those European
  inputs. The European dialogue acknowledgement schedules must be observed
  rather than copied from the Japanese route.
