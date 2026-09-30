# Continue a saved game in the app and on the page

## Summary

Keep SRAM between sessions (a `.srm` beside the app's ROM; the browser's own
storage on the page) and offer the native "Restart" file select at start.

## Dependencies

- [Save at the bedroom desk](save-point.md)
- [Keep the SRAM between sessions](persist-sram.md)
- [Choose a game on the Restart screen](restart-file-select.md)

## Acceptance Criteria

- A game saved in either host continues after a restart, from the last slot;
  a native `.srm` can be imported and exported.

## Sub-issues

1. [Keep the SRAM between sessions](persist-sram.md)
2. [Choose a game on the Restart screen](restart-file-select.md)
