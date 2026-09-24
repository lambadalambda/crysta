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
