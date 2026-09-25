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
- Next: return home through the blue-door and pot scenes, Pandora's Box,
  frozen return, the Elder's mission, and world-map `$03`. The European
  dialogue acknowledgement schedules must be observed rather than copied
  from the Japanese route.
