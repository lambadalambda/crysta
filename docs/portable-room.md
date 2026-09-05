# Portable room preview

The `room-core` crate is a deterministic, device-free **bounded walking core**
and an explicitly opted-in **semantic doorway preview**. It is not classic-mode
frame fidelity or a complete game engine. Original CPU execution is not part of
its simulation loop. Native and WebAssembly compile the same Rust source.

## Run locally

```sh
cargo run -p map-inspector -- serve-room \
  'local/Tenchi Souzou (Japan).sfc' 8765
```

Open the printed `http://127.0.0.1:8765/` address. Use port `0` for an OS-assigned
loopback port. No SRAM is needed. Choose **Doorway demonstration** to reset and
run the complete bounded route, or Resume and use Arrow/WASD/touch controls.
Reset starts a fresh preview; Ctrl-C stops the native host.

```sh
cargo run -p map-inspector -- verify-room \
  'local/Tenchi Souzou (Japan).sfc' semantic-preview
cargo test -p room-core
cargo build -p room-core --target wasm32-unknown-unknown
```

This browser frontend currently uses the **native Rust host**, not Wasm glue.
The complete core builds for Wasm; direct in-browser Wasm hosting is future work.
The host also writes an ignored static export using the shared renderer.

## Walking versus doorway policy

Walking implements measured input delay, setup frames, vertical 1/2 cadence,
the horizontal 54-frame restart gap, and four-direction flat-wall resolution.
It is not constant-speed generic AABB movement. The initial checkpoint is actual
save slot 1, room `$000F`, player `(472,176)`, corresponding to completed
reference frame 1601. Initialization is a deliberate preview lifecycle operation,
not an implementation of the save menu or new game.

[Movement qualification](movement-qualification.md) describes the exact
supported reference trajectories and source witnesses. Unflagged types 0/2/22
are open and 12/14 are solid. Unknown materials, bit-15 cells, mixed open/solid
corner pairs, map-edge samples and coordinate overflow return explicit errors.
A failed update leaves state unchanged. Blocking does not stop movement cadence.

Dash activation is not qualified. Therefore each direction may have only **one
contiguous activation interval per reset/input epoch**. Releasing and retapping,
or returning to a previously used direction, stops the preview rather than
quietly treating a dash as walking. Diagonals/actions are not accepted. This is
an intentionally conservative admission rule, not the game's actual cooldown.
Use Reset after a scope error; the automatic doorway demonstration gives a
reproducible route without needing frame-perfect key release.

### Doorway: endpoint-qualified, not frame-qualified

The exact supported handoff is `(392,209)` after 56 Left and 24 Down updates.
The selected exit is direct map `$0010`, mode 0, selector 5. Other exits or
handoff positions fail explicitly. The asset adapter validates the departure
pointer, signed arrival adjustment, destination FD/player header and arrival
selector pointer against the authenticated ROM.

The chosen **SemanticPreview** policy is:

1. 17 logical `(0,+1)` departure updates: `(392,209)` → `(392,226)`.
2. One atomic load/spawn update: raw `(384,336)` + decoded `(0,−16)` adjustment
   gives queue `(384,320)`; the verified `(8,16)` anchor gives `(392,336)` in
   map `$0010`.
3. 17 logical `(0,+1)` arrival updates: `(392,336)` → `(392,353)`.
4. Restore walking ownership with a fresh input-admission epoch. Inputs during
   the doorway are discarded, not buffered.

The **endpoints and source path are reference-qualified**. Distributing movement
into 17/load/17 logical updates is an explicit preview policy. The native path
has loader/display stalls and actor-scheduler gates not reproduced here. In
particular, COP C1's counter 16 is **not** proof of 17 video frames or 17 player
updates. Do not compare logical preview tick 115 with a reference video-frame
number. [Doorway research](opening-doorway.md) retains the real-frame evidence.

This distinction is why the API requires a named policy opt-in and the frontend
labels itself a semantic preview. No classic-fidelity claim is made for its
transition pacing, animation, flags VM or actor scheduler.

## Data and state boundary

- `Room` holds immutable dimensions and raw attributed cells. `WalkingState`
  holds integer position, delayed/active input, phase and conservative history.
- `GameData` owns the two compiled rooms, ordered exit metadata and caller-
  authenticated source/content SHA-256 identities. No device or file handles.
- `GameState::step` consumes `FrameInput` and produces a semantic `FrameOutput`.
  Rendering and the host's tick clock do not affect the update implementation.
- Snapshot encoding is fixed little-endian and versioned. It identifies source
  ROM, asset/profile schema and compiled data; it does not embed immutable assets.
  The RNG policy is version 0 (**no RNG used in this subset**), not a fabricated
  replacement for the game's RNG. Unknown versions/fields or incompatible data
  are rejected. No deterministic host-service responses are needed yet; reset
  is an explicit lifecycle operation, not a hidden input.

The adapter initializes collision attributes directly from decoded ROM data.
Five known runtime-modified cells are excluded from this static simulation:
map F index 317; map 10 indices 731, 732, 826, 827. They differ from the static
initialization by bit 15 at the authenticated checkpoints. The adapter marks
these cells out of scope so the core **rejects** them; this does not implement
the dynamic events that changed them. All other initial attributed cells match
the corresponding checkpoint. Ongoing actor/event grid changes are unsupported.

## Rendering and host limits

The local frontend uses the shared ROM-decoded first-background sheet and a
plain player bounds marker. It does not show an original player sprite, NPCs,
BG2, animation, sunlight/windows, color math, audio, combat or dialogue. The
[static room graphics limits](room-graphics.md) still apply.

The browser sends bounded input requests to a **127.0.0.1-only native host**.
Only the host owns simulation state; JavaScript does not reimplement movement.
Requests are serialized, paused on errors/hidden pages, and mutation requires
the host's exact origin. There are no external libraries, uploads, arbitrary
file-serving routes or public listener. Browser timing is a frontend concern;
one accepted step always performs one deterministic core update.

## Verification

- Ten ROM-free walking tests, eight semantic/snapshot tests and a portable-boundary
  architecture guard pass. Repeated synthetic snapshots resume identically at
  every logical update, including transition ownership.
- Seven authenticated private trajectories match **all 1,204 walking steps**,
  attempted stream outputs, positions, blocking and restored continuations.
  Compiled ROM grids (with explicit exclusions above) match those fixture grids:
  F SHA-256 `c5d86aec915b09ec3481d48e903bd1d94a824303f4b4eee24da19acfbf8028e1`;
  10 SHA-256 `261e3b4637587b69465178667f70cddb5eb6d996650f7008c37aefbec5325eed`.
- `verify-room` repeats byte-identical JSON results across fresh CPU-free processes.
  Handoff is logical tick 80; departure/map switch/spawn/arrival endpoints match
  reference evidence. Arrival is preview tick 115, **not a reference-frame pin**.
- HTTP tests cover exact routes, same-origin mutations, fragmented bodies and an
  absolute request deadline. Synthetic browser tests cover serialized requests,
  keyboard/touch release, blur/hidden pause, reset, demonstration and error handling.
- Real browser QA exercised the complete doorway demonstration, manual west-wall
  walking, explicit reactivation error and pausing, desktop/mobile rendering,
  and no horizontal document overflow at 390px. No console errors were observed.
  Screenshots stay in `local/map-research/portable-room-{desktop,mobile}.png`.
- Native/`wasm32-unknown-unknown` builds, workspace tests, strict Clippy/rustdoc,
  formatting, safety and tracker checks passed; independent core/host/UI reviews
  found and cleared snapshot-ownership and request-framing/deadline issues.

No ROM, SRAM, graphics or raw reference trajectories are distributed. The local
trajectory harness can reproduce fixtures; absent optional inputs skip clearly,
while explicitly required or hash-mismatched fixtures fail closed.
