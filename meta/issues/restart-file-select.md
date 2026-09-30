# Choose a game on the Restart screen

## Summary

The native file select (`旅の再開`, European "Restart") that Start on the
title opens: slots 1–3 with name, level and time, New Game, Copy Data and
Erase Data, the cursor on the last slot saved (SRAM `$1FFE`).

## Dependencies

- [Keep the SRAM between sessions](persist-sram.md)

## Requirements

- Research the screen on both ROMs as the Records screen was
  (`docs/records-screen.md`): layers, art, text, cursor, timings.
- A on a valid slot resumes it (`World::resume`); on New Game or an empty
  slot it starts a new game. Copy and Erase write SRAM as natively.

## Acceptance Criteria

- The screen matches native frames on both ROMs; loading the 2008 save's
  first slot puts Ark at the desk, and Copy/Erase leave SRAM as the native
  game does.

## Notes

- The screen matches the seven native scenes of `local/restart/` on both
  ROMs (`crysta-app/tests/local_restart.rs`); Copy and Erase change the
  bytes the native game changes (`crysta-runtime/tests/local_saves.rs`).
  On the page, the imported 2008 save's slot 1 loads at the desk
  (`fresh_european_load_the_2008_save_at_the_desk`, replayed in Chromium).
- Open: the text engine's typing rule for the redraw (measured frames
  stand in); the load's dark frames follow the world's own table, not the
  native 80.
