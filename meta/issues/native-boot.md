# Boot as the native game does

## Summary

Start the hosts as the cartridge starts: the title, the Restart screen, the
name entry and the opening text before the bedroom, and later the logos and
the intro film.

## Dependencies

- [Continue a saved game in the app and on the page](continue-saved-game.md)

## Requirements

- Each screen is researched on both ROMs and drawn from the ROM alone.

## Acceptance Criteria

- From power-on, the same presses reach the same screens and the bedroom
  as on the native game, on both ROMs.

## Sub-issues

1. [Show the title screen](title-screen.md)
2. [Enter a name and read the opening text](name-entry-opening.md)
3. [Play the logos and the intro film](intro-film.md)

## Notes

- Decided 2026-09-30: saves and the file select first, then the title and
  the name entry; the intro film last.
