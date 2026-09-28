# Port the slice to the European English ROM

## Summary

Run the Crysta slice from the European English ROM, its own code, data,
text and timing ([ADR 0004](../../docs/adr/0004-european-executable.md)).

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- The app and the web page run the slice from either ROM.

## Acceptance Criteria

- The European slice plays from the bedroom to the world map, checked
  against a native European route.

## Sub-issues

1. [Map the European ROM to the Japanese one](european-address-map.md)
2. [Read every ROM address through a per-revision layout](revision-layout.md)
3. [Decode the European text engine, font and windows](european-text.md)
4. [Run the European version at its own timing and sound](european-timing.md)
5. [Accept the European ROM in the app and the web page](european-hosts.md)
6. [Check the European slice against a native route](european-route.md)

## Progress

- A headless empty-SRAM European native replay reaches map `$03` at `(536,544)`;
  a single portable `World` now reaches and walks the same map from fresh
  bedroom flags without seeded checkpoints or warps. See [the route
  issue](european-route.md). This establishes the connected functional path,
  not native-frame presentation parity. The frozen-return boundary now agrees:
  the exact source-pinned native and portable player stream releases at
  `(136,464)`, before later manual movement reaches `(120,448)`. This is bounded
  support for that map-`$21` profile, not general scripted-player movement. The
  town controller redirect runs but its animation/native PC are not compared.
  Bounded evidence now covers two PAL movement legs, the frozen-return stream,
  bedroom tempo, route audio-port patterns, three native dialogue/choice
  rasters, first-choice cursor/BG3 upload, eight stable bedroom BG3 CGRAM
  entries and bounded two-frame native framebuffer self-consistency, plus one
  Crysta title OBJ phase. Full text/RGB/HDMA, remaining loads/movement, native
  item-name presentation, host wall-clock delivery and PCM/DSP parity remain
  open; known door/spear timing mismatches are documented rather than hidden.
- A fresh European `Game::new` host replay now takes the alternate friend's
  refusal→retry route, opens the door, plays through the Box and frozen-town
  detour to the doorway Elder, and walks on world-map `$03`. Its regenerated
  28,121-frame input trace includes the corrected `(136,464)` release plus
  manual Left/Up boundaries and reaches the same rendered endpoint on the
  rebuilt headless browser page with no mismatched action edges or game fault.
  This adds a current browser-host path, not native-frame text/sound parity or a
  reproduction of the original reporter's older build/save state.
