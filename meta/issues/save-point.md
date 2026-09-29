# Save at the bedroom desk

## Summary

The desk in `$0F` (`$88:D63B`, `COP 00 $87:8590`) opens the native save
screen: "Records", three slots with name, level and time, A saves, B
cancels, the fades and jingle `$35` ([saves](../../docs/saves.md)).

## Dependencies

- [Read and write native SRAM](sram-codec.md)

## Acceptance Criteria

- Saving at the desk writes the slot natively: the native game, booted with
  that SRAM, loads it at the desk facing up. The screen matches native frames
  on both ROMs.
