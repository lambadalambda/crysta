# Native macOS window, renderer and gamepad for the Crysta slice

## Summary

A macOS application that opens a window, renders the slice at the native
256×224 resolution scaled up, and takes input from a gamepad, driving the
free-roam runtime.

## Dependencies

- [Promote the Crysta room builder into a library](crysta-room-library.md)

## Requirements

- `winit` for the window and event loop, `softbuffer` for the framebuffer, and
  `gilrs` for gamepad input. Keyboard input stays available so the app is
  usable without a controller.
- Integer-scaled nearest-neighbour presentation of a 256×224 buffer. No
  filtering, no non-integer scale.
- Rendering composites the same way the browser viewer does, since that path is
  already qualified: background sheet at the camera, then a depth-sorted draw
  list of sprites, then the BG-high occlusion mask.
- Dialogue pages are drawn as the images the ROM holds, not as re-typeset text.
- The ROM is chosen by the user at runtime and is never copied into the
  repository, logged, or persisted. The existing local-only conventions apply
  unchanged.
- The simulation runs at a fixed step decoupled from the present rate, so the
  app does not become the definition of the game's timing.

## Acceptance Criteria

- The app launches, takes a ROM path, and presents a frame.
- A gamepad moves the player, and an interaction button talks to a resident.
- The renderer's output for a given state matches the browser viewer's for the
  same state, so the native path is not a second, divergent renderer.
- No ROM bytes or extracted assets are written anywhere outside ignored local
  paths.

## Notes

- Milestone: [M4 — Portable vertical slice](../milestones.md#m4-portable-vertical-slice)
- Parent: [Play the Crysta slice free-roam in a native app](free-roam-crysta-app.md)
- This is the one place the dependency surface grows. The lockfile goes from 42
  packages to roughly 222; keeping that inside the app crate means the decoding
  and simulation crates stay as lean as they are.
- The compositing to port is about 440 lines of the browser viewer, and it has
  no framework dependency — the art manifest hands over fully decoded RGBA
  frames with pre-composed mirrors and a pre-sorted draw list.
