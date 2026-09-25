# Accept the European ROM in the app and the web page

## Summary

The app and the web page refuse all but the Japanese ROM. Let them take
either and run the matching layout.

## Dependencies

- [Read every ROM address through a per-revision layout](revision-layout.md)
- [Decode the European text engine, font and windows](european-text.md)

## Requirements

- Both hosts authenticate the European ROM and start the matching slice.

## Acceptance Criteria

- Both hosts start the slice from the European ROM.

## Progress

- The app and the web page take either ROM and pace frames by its console
  (NTSC 60.1 Hz, PAL 50.007 Hz, `crysta_app::clock::frame_period`); the
  European opening renders in English (2026-09-24).

## Verification

- The native app accepted the European ROM and rendered the opening bedroom
  through its headless `--screenshot` mode (`wait:200`); its window startup
  also succeeded before headless-only testing was requested.
- The rebuilt browser Wasm site accepted the European ROM and started the
  bedroom without a browser error. The headless `smoke.mjs` run advanced
  600 frames, rendered lit pixels and non-silent audio, and reported no fault.
  These checks establish startup, **not** an end-to-end European story
  playthrough; that remains [European route](european-route.md).
