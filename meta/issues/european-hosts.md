# Accept the European ROM in the app and the web page

## Summary

The app and the web page refuse all but the Japanese ROM. Let them take
either and run the matching layout.

## Dependencies

- [Read every ROM address through a per-revision layout](revision-layout.md)
- [Decode the European text engine, font and windows](european-text.md)

## Acceptance Criteria

- Both hosts start the slice from the European ROM.

## Progress

- The app and the web page take either ROM and pace frames by its console
  (NTSC 60.1 Hz, PAL 50.007 Hz, `crysta_app::clock::frame_period`); the
  European opening renders in English (2026-09-24). Open: a playtest.
