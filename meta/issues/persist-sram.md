# Keep the SRAM between sessions

## Summary

The desk's save lives only as long as the session. Keep the 8 KiB SRAM as
the native cartridge does: a `.srm` beside the app's ROM, the browser's own
storage on the page, with import and export of native `.srm` files.

## Dependencies

- [Save at the bedroom desk](save-point.md)

## Requirements

- The app reads `<rom>.srm` at start and writes it after each save; a file
  that is not 8 KiB is refused and left untouched.
- The page keeps the SRAM per ROM revision in its own storage; it never
  leaves the browser. It imports and exports `.srm` files.
- Writes are whole-file and atomic (temporary file, then rename).

## Acceptance Criteria

- A save in either host is still in its SRAM after a restart; the 2008
  `.srm` imports on the page and exports byte-identical.

## Notes

- The app's `.srm` module round-trips and refuses foreign files
  (`crysta-app/src/sram_file.rs`); the page's storage, restore and guards
  are tested in `crates/crysta-web/input.test.mjs`; in headless Chromium the
  2008 `.srm` imports and is kept byte for byte, and a desk save writes the
  kept SRAM (`tools/web-replay --sram`).
- The page keeps the SRAM in `localStorage`, which the whole
  `lambadalambda.github.io` origin shares.
