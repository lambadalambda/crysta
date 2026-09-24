# Draw the text window as the game does

## Summary

The native window has a framed, shaded box, the speaker's name in colour
and a blinking prompt icon (user screenshot, bedroom). Ours is a plain box.

## Dependencies

- [Make the Crysta slice fully playable](playable-crysta-slice.md)

## Requirements

- Decode the window's frame, fill and prompt icon from the ROM and draw
  them as the text engine does.

## Acceptance Criteria

- A page in the bedroom matches a native frame of the same page.

## Resolution

- The window draws from ROM art (`assets::text::window`): the BG3 frame,
  colour 3 shaded per scanline (`$85:81E4`), each glyph in its palette with
  the speaker's colour (`CA 05`), and the prompt on `$D5` pages. A render of
  the bedroom page matches the native screenshot (2026-09-24). Not done:
  the options' green and red window colours, the choice cursor's sprite,
  and a `$C2` window of odd height (its frame is a tile short).
