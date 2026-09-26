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
  not native-frame presentation parity. The frozen-return player walk is not
  fully modelled; the town controller's `COP BF` redirect now runs without
  freezing, but its animation cadence and native PC have not been compared.
  Text raster and PAL load/music tempo acceptance remain open.
- A fresh European `Game::new` host replay now takes the alternate friend's
  refusal→retry route, opens the door, plays through the Box and frozen-town
  detour to the doorway Elder, and walks on world-map `$03`. Its input trace
  reaches the same rendered endpoint on the rebuilt headless browser page
  with no mismatched action edges or game fault. This adds a current
  browser-host path, not native-frame text/sound parity or a reproduction of
  the original reporter's older build/save state.
