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

## Progress: it runs

`crates/crysta-app` opens a window, renders the slice at 256x224 scaled by the
largest integer factor that fits, and drives `crysta_runtime::world::World`
from a gamepad or the keyboard.

- Backgrounds come from the qualified renderer, `render_static_background`,
  rather than a second decode path; the app only decodes its 24-bit top-down
  BMP into a framebuffer.
- Input: D-pad or left stick past half deflection, arrows/WASD, and
  Space/Enter or South/East to interact. Interaction talks to a resident when
  one is faced and otherwise opens a doorway, so a resident standing in a
  doorway is spoken to rather than walked past.
- The interact button is edge-triggered, or holding it would re-talk every
  frame.

### A headless mode, so the renderer can be looked at

`--screenshot <path> <frames> [direction]` runs the world for N frames and
writes the composed view as a PPM without opening a window. That is how the
renderer was checked: 400 frames of Down from the opening house walks the
player out of map `$000B` and into `$000C`, and the frame shows the next room
with the camera following.

`frame.rs` is pure functions over pixels with 8 unit tests — camera clamping at
every edge, integer-only scaling, clipping, and a synthetic BMP round trip — so
none of it needs a window to test.

### Outside the workspace, and the gate is why

`winit` and `gilrs` enable `cc`'s parallel feature, and Cargo.lock records the
feature union across *all* members, so joining the workspace adds `jobserver`
and `libc` to map-inspector's recorded dependency closure and trips the capture
gate. The producer is built with `-p map-inspector` and would not enable them,
but the lockfile cannot express that, and weakening the gate a second time to
paper over it is the wrong trade. Run it with:

```sh
cargo run --release --manifest-path crates/crysta-app/Cargo.toml -- <rom>
```

## Progress: sprites and dialogue

The player is drawn from the ROM's own frames: `crysta_runtime::art::ArkAtlas`
rasterizes the three standing and eighteen walking compositions, with the
horizontal ones mirrored through the ROM's alternate anchors, and the world
advances a `room_core::AnimationState` from the walking state's active
direction exactly as the qualified slice does. Residents are drawn from their
spawn records through
[the record-driven loader](crysta-resident-art.md).

Compositing follows the browser viewer: background at the camera, then the
draw list sorted by world Y with later spawn records first and the player
last at equal Y, with every sprite pixel under an opaque high-priority
background tile left to the tile. The priority mask comes from the same
`render_static_background` call that supplies the bitmap.

Dialogue pages are drawn as the two-bit images the ROM holds, boxed along
the bottom of the view, and the interact button pages through them; the last
page's acknowledgement closes the box. While a conversation is open the
player stands still.

A resident with no graphics descriptor draws nothing, as in the game. One
whose descriptor was refused draws a coloured block, so that someone is
visibly there and visibly not right.

`--screenshot <path> <script>` runs a step script such as
`down:400,wait:12,up:24,wait:12,left:20,wait:12,talk` and lists every
resident with their art status and what talking to them yields.

## Remaining

- **One resident's line does not decode.** The map-`$1D` record's script
  hands the text service an address holding native code; see
  [the refused text controls](crysta-refused-text-controls.md). The eight
  others that showed nothing were window controls, now admitted and drawn.
- ~~The camera shows past the map's own region.~~ Fixed: the camera clamps
  to the source region; see
  [the camera region](clip-native-crysta-camera.md).
- **Walkers do not greet the player on approach.** `COP 2E` tests a bit of
  the held-button word `$0454`, and the runtime hands actors no buttons.
  Walkers do stop and face a player who faces them, as `COP 23` has them do;
  see [walkers stop and face the player](crysta-walker-stops-for-player.md).
- No `.app` bundle yet; it runs as a binary.
