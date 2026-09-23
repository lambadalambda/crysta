# Native Crysta app

```sh
cargo run --release --manifest-path crates/crysta-app/Cargo.toml -- \
  'local/Tenchi Souzou (Japan).sfc'
```

The authenticated Japanese ROM supplies all art and music locally. No asset
bundle or downloaded soundtrack is required. This crate stays outside the root
workspace so native platform dependencies do not change reference-producer pins.

## Controls

- Arrows/WASD or gamepad: movement.
- Space/Enter or gamepad South (SNES A): interact, acknowledge a page, confirm
  a choice.
- X/Backspace or gamepad East (SNES B): cancel a choice.
- Up/Down move a choice's cursor.
- **M:** pause/resume music (not gameplay).
- **− / +** (also numpad): change volume by 10%, clamped to 0–100%. Starts at50%.
- Escape: quit and stop audio.
- **V:** switch between the classic 256x224 view and a 400x224 (about 16:9) view.

Music pauses while the window is unfocused or suspended. Refocusing preserves
an explicit M-key pause. Zero volume is mute: the music timeline continues;
pause preserves the playback position. The title shows the music status.
Use `--no-music` after the ROM path to avoid opening an audio device entirely.
The existing `--screenshot <path> <script>` mode also does not initialize audio.

## Views

The classic view is the default. `--wide`, anywhere after the ROM path, starts
in the wide view; it also works with `--screenshot`, for example
`<rom> --wide --screenshot <path> <script>`. The camera stays inside
the map's source camera region in both views. The wide view follows the player
in regions wider than 400 pixels. A one-page room is centred, and everything
outside its region is black, so no neighbouring room shows. Dialogue boxes stay
in the classic area in the middle. The view is presentation only: the
simulation, residents and timing do not change.

## Outdoor refusals and diagnostic logs

Unsupported collision remains blocked, but no longer locks the input queue or
freezes residents: release the direction and turn away. This is a native-host
recovery policy, not admission of additional collision behavior. A checked world
loading failure instead stops simulation, names the error in the window title
and terminal, and leaves Escape available; restart the app after such an error.

Normal windowed runs automatically print a log path under
`local/crysta-app/logs/crysta-session-N/`. **After finding a bug, send both
`events-*.jsonl` files from that session folder** (or the only one, if it has not
rotated), along with what you saw. Copy them before launching several more runs.
Screenshot mode does not create session logs.

- JSONL records contain schema/build/ROM hashes, simulation ticks, semantic
  direction/interact inputs, before/after map coordinates, separate movement and
  interaction results, dialogue page numbers, refusals and checked world errors.
  Music-control state and normal quit are recorded too. No ROM bytes, dialogue
  text, extracted assets or secrets are logged.
- Four recent session slots, each with two 8 MiB segments, bound storage to about
  64 MiB. Within a session, sort by the header's `segment` number, not filename.
  Each segment repeats its session identity/header. Only run **one app at a time**
  from the same working directory; concurrent writers are not supported.
- Headers/refusals/world errors/quit flush immediately; normal records flush
  every 60 records or on the first write after one second. An abrupt kill can
  lose the buffered tail. Logging errors disable logging, not gameplay.
- These are **partial diagnostic histories**, not save states or guaranteed
  complete replays. Rotation may discard the setup for a bug; logs identify
  where and how a refusal happened but do not establish native-reference fidelity.

Owned-ROM recovery, trace ordering, logging failure and fatal-stop regressions:

```sh
CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --release --manifest-path crates/crysta-app/Cargo.toml \
  session_tests -- --ignored --nocapture
```

## Simulation speed

Windowed simulation uses the Japanese reference's nominal **60.098814 ticks/s**,
independent of display refresh or event-loop wakeups. Catch-up is capped at four
steps after a host stall; larger backlogs are discarded and logged. Headless
scripts remain exact logical steps. Audio pacing is independent.
See [timing measurements and NPC fidelity limits](../../docs/native-crysta-timing.md).

## Background presentation

Exterior tree transparency uses the ROM-derived scene backdrop, not the asset
inspector's gray checkerboard. This does not yet reproduce the exterior's
additive floating-leaf effects, secondary scrolling or full color math. Static
inspection exports deliberately keep their checkerboard. The river now runs its original palette cycle (seven phases, six host simulation
updates each), together with the source-backed exterior graphics animation.
Animation continues while standing still and during dialogue, resets on map
entry, and stops with a fatal world error. Redrawing does not advance its clock;
only affected map cells are refreshed. Logs include `background_tick` to help
diagnose phase-dependent rendering bugs.

This follows the fixed host simulation cadence; it does not claim native
video-frame phase synchronization. [Source and native-state comparison](../../tools/crysta-animation-qualification/README.md).

```sh
CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --release --manifest-path crates/crysta-app/Cargo.toml \
  background::tests -- --include-ignored
```

## Music scope and architecture

This first music slice plays **fresh Crysta selection3**, continuously across
map changes. It does not yet switch tracks with maps/events or play sound effects.
Browser playback, the full CPU/SPC command protocol, and bit-perfect hardware
fidelity remain separate work.

- [`music_data.rs`](src/music_data.rs) authenticates the ROM and extracts the
  original SPC driver, sequence, instrument metadata and BRR sample payloads.
  [Source recipe and transfer evidence](../../tools/native-music-qualification/README.md).
- [`music.rs`](src/music.rs) uploads through physical IPL/driver ports with
  bounded waits, then verifies uploaded sequence/sample RAM before playback.
  There is no captured-state initialization or game CPU running behind the app.
- [`spc-player`](spc-player/README.md) executes only SPC700/APU/DSP code. It renders
  stereo PCM at32000Hz; rodio/CPAL handle the output device and resampling.
- An independent worker owns the SPC and audio device. At most two16ms PCM blocks
  are queued plus one pending. A single continuous source avoids restarting the
  resampler at block boundaries. Its callback never waits for the producer;
  underruns produce stereo-aligned silence. Pause stops generation without
  dropping queued music; shutdown joins the worker and releases the device.
- No audio callback, device clock or host control enters `World`/`Session` or
  advances gameplay. This work does not change the existing simulation cadence.

If opening the device or initializing music fails, the app reports the error,
shows music as unavailable, and continues silently. SPC rendering errors after
startup stop only the music worker; restarting the app retries the device.

## Verification

ROM-free tests (including extraction, stereo buffering, controls and physical
IPL tests):

```sh
cargo test --manifest-path crates/crysta-app/Cargo.toml
cargo test --manifest-path crates/crysta-app/spc-player/Cargo.toml
cargo clippy --manifest-path crates/crysta-app/Cargo.toml --all-targets -- -D warnings
```

Owned-ROM extraction and ten-second deterministic, non-silent playback without
a device:

```sh
CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --release --manifest-path crates/crysta-app/Cargo.toml \
  authenticated_local_rom -- --ignored
CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --release --manifest-path crates/crysta-app/Cargo.toml \
  source_only_crysta -- --ignored
```

Real-device smoke test — **plays music**, verifies consumption beyond the bounded
prebuffer, pause/resume and worker shutdown. It is silent unless
`CRYSTA_PLAY_AUDIO=1` is set, so `--include-ignored` runs never play audio:

```sh
CRYSTA_PLAY_AUDIO=1 CRYSTA_JP_ROM="$PWD/local/Tenchi Souzou (Japan).sfc" \
  cargo test --release --manifest-path crates/crysta-app/Cargo.toml \
  native_device_play_pause_resume_and_shutdown -- --ignored --nocapture
```

The restricted development session currently reports **“No matching default
audio unit found”** at device creation. Source extraction, upload verification,
PCM rendering and unit tests pass, but this does **not** constitute audible
hardware verification by the agent. **The user subsequently confirmed audible
playback on the desktop**, completing the bounded
[native music issue](../../meta/issues/native-crysta-music.md). The automated
device control test above remains available for a normal desktop terminal; it
is not claimed to have passed in this restricted session.

Tests were red before implementation. A separate no-settling mutation removed
the required32-cycle payload-ACK delay and failed with an upload mismatch at
SPC `$0FE8`; restoring it passes. Backend negative-BRR tests also pass under
UBSan after a narrow signed-shift fix; see its README for reproduction.

## Licenses

- Project code and the retained LakeSnes audio core: **MIT**. LakeSnes copyright
  2021–2023 angelo_wf and contributors. Include
  [`vendor/lakesnes/LICENSE.txt`](../../vendor/lakesnes/LICENSE.txt) in distributions.
- `rodio`0.20.1: **MIT OR Apache-2.0**, file codecs disabled. Its device dependency
  `cpal`0.15.x is **Apache-2.0**, and `coreaudio-rs`0.11.x is **MIT OR Apache-2.0** on macOS.
  Retain dependency license notices in binary distributions.
- Existing window/input/render dependencies: `winit` (**Apache-2.0**),
  `softbuffer` (**MIT OR Apache-2.0**), `gilrs` (**MIT OR Apache-2.0**).
- ROM-derived music, driver, instruments and samples remain copyrighted game
  content supplied locally, not covered by those code licenses or distributed.
