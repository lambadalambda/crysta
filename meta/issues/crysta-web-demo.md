# Publish the Crysta slice as a web demo

## Summary

A static web build of the Crysta app for GitHub Pages: players select their
own Japanese ROM, which never leaves the browser, and play the slice.

## Requirements

- Build the runtime and the app's renderer to WebAssembly; share the
  renderer between the native and web frontends.
- Select the ROM locally (file input); authenticate it as the app does;
  never upload or cache it without consent.
- Canvas output, keyboard and Gamepad API input.
- Audio: build the SPC core for the web and play through an AudioWorklet
  after a user gesture, without threads.
- A GitHub Actions workflow that publishes the static files to Pages; the
  artifacts contain no ROM-derived bytes.

## Acceptance Criteria

- The slice plays in a current desktop browser from ROM selection to the
  world map, with music and sound.
- Network inspection shows no ROM bytes sent; the published files contain
  no ROM data.

## Notes

- Prior art: `tools/pandora-preview` runs `map_inspector::PandoraPreview`
  in a Web Worker from a local ROM.
- Needs a GitHub remote and the user's decision to publish.
- Parent of this demo's scope: [Ship the WebAssembly frontend](webassembly-frontend.md).
