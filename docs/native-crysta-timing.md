# Native Crysta timing

Track [movement/animation cadence](../meta/issues/verify-native-crysta-cadence.md).
There are two different clocks to verify: real-time host updates and the number
of game ticks used by a movement/animation script. A visual slowdown must not
substitute for either investigation.

## Host rate: corrected

The native app previously called `Session::advance` on every `about_to_wait`
callback under `ControlFlow::Poll`. Neither input events nor display redraws
are game ticks. A local window measured **134.6 simulation updates/second** over
8 seconds, about **2.24 times** the intended NTSC rate. This initial measurement
read buffered log tails, so its endpoints have up to60 ticks of flush quantization.

For the Japanese non-interlaced reference, the nominal rate is **60.098814 Hz**:

- NTSC master clock: `(315 MHz / 88) * 6` (vendored ares constants/system).
- 262 scanlines ×1364 clocks, with one four-clock-short scanline on alternating
  frames (`vendor/ares/ares/sfc/ppu/counter/inline.hpp`).
- Mean tick period: **16,639,263.492 ns**, rounded to16,639,263 ns for host pacing.

The app now uses monotonic fixed deadlines and `WaitUntil`, independent of
monitor refresh. Late wakeups may catch up at most four steps; longer host stalls
are logged and rebased rather than fast-forwarding minutes of held input.
Suspend/resume discards suspended time. Focus continues to affect music only,
as before. Audio runs on its independent worker; no playback-speed change.
Headless scripts still advance exactly the requested logical ticks, without sleeps.

A second native window measured **60.0945 updates/second**, using480 updates and
7.9874 seconds between per-frame monotonic timestamps. Logs now include
`host_elapsed_ns`, header `tick_period_ns`/`timing_policy` and backlog-drop events.
Private before/after measurements: `local/crysta-cadence/host-{before,after}.json`.

Keyboard interaction presses are latched until a simulation tick, so a quick
press/release between deadlines is not lost or repeated during catch-up. Gamepad
interaction remains level-polled: taps wholly between polls can still be missed.

## Per-tick behavior

- Ark's qualified ordinary walking art: **six records × nine ticks =54 ticks**
  per cycle, approximately0.899 seconds. This is already implemented in room-core;
  see [Ark animation](ark-animation.md). Host pacing does not alter those records.
- The source-qualified map-D wanderer `$83:8CB4` now moves **16 pixels in32
  ticks**, alternating **1,0 pixels** along its selected axis: approximately
  **30.05 pixels/second** while walking, or **0.532 seconds per tile**. Its
  ordinary idle/refusal action lasts **16 ticks**, approximately **0.266 seconds**.
  This replaces two pixels per moving tick and an appended eight-tick wait.
  Other residents/classes retain the existing explicitly approximate projection.
- Resident raster durations are raw native countdown bytes. The source scheduler
  decrements before testing negative, so a duration7 record lasts **eight
  ticks**, not seven. The resident raster player now uses that rule, including
  zero→one tick and255→256 ticks. Four duration7 records take32 ticks (~0.532s).
  Looping a selected list remains a host presentation policy, not a full VM.

## Ordinary NPC source/native qualification

The map-D spawn points to header `$88:A837`, script `$88:A83C`, descriptor
`$83:EDEB` and composition packet `$D8:1022`. Descriptor mode `$0020` initializes
**class0 and the common movement base**, not a private movement resource.
Class alone, a shared sprite, or the initial selector does not qualify another
actor's speed.

- Source load instructions `$98:817D`/`$98:8272` resolve packed `$09F037` to
  `$AB:F037`, destination `$7F:6000`. The production codec decodes6668 bytes;
  **every byte matches native `$7F:6000..7A0C`**. This path does not relocate.
- `$80:8F32` chooses common movement selectors `$68/$69/$60` for down/up/X.
  Their signed integer streams contain duration/value pairs `(0,±1),(0,0)`
  and a loop. The stored pointer precedes the first duration by two bytes;
  `$80:F251..F312` pre-decrements its counter. Zero duration therefore means
  one application. Left negates the X stream through horizontal flip.
- `$80:D0D7..D0F4` directly applies and clears the integer accumulators for
  this actor. There is no hidden fixed-point divisor.
- `$80:8F65` sets **one entire list repetition**, not one record. Walking
  selectors3/4/5 each contain four duration7 records; idle selectors0/1/2
  contain one duration0 record. `$80:8F85` selects16 idle repetitions.
- A fresh input-only native replay yielded401 consecutive snapshots at frames
  8175–8575. Complete down8195–8226, up8371–8402 and right8403–8434 actions
  each apply16 pixels in32 frames, with exact alternating deltas. Idle8227–8242
  lasts16 frames; frame8243 begins the next action. Consecutive movement actions
  also join without an extra gap. The settledD WRAM matches the retained
  qualification baseline byte-for-byte.

Runtime admission checks this source record/header/descriptor, COP26 site and
following COP8F, class tables, loader bindings, decoded velocity records and
finite display-list durations. Only exact audited skipped-service sites preserve
admission; other skipped services revoke it, including the legacy unmodelled
COP06 long-jump path. The qualified action consumes its own COP8F, restarts
movement and raster phase even for consecutive equal poses, and reserves a fixed
destination until completion. It does **not** alter arbitrary COP8E/COP8F/COPC1
waits, other classes/private resources, native RNG or callback execution.

Reusable evidence tools and reproduction commands are in
[`tools/crysta-cadence-qualification`](../tools/crysta-cadence-qualification/README.md).
Raw ROM-derived resources, disassemblies and native snapshots remain ignored.

Do not claim a fixed wall clock reproduces native lag frames, transition timing,
NPC RNG, or the complete actor scheduler.

## Verification

Clock tests cover extra wakeups, 60/120/144/240/1000-Hz event schedules, bounded
catch-up and suspend/resume. Input tests cover short keyboard taps, held gamepad
buttons, and one-shot consumption. Native Session/headless regressions remain
unchanged by real-time scheduling. Independent correctness/architecture review
approved the host timing boundary.

NPC tests cover exact per-action deltas/ages and consecutive boundaries, four
signed directions, idle/refusal, stable reservations, source admission/revocation,
and the real resident's bounded wandering/solidity/stop-facing behavior. Full
release workspace tests and strict workspace/native Clippy pass, as do49 native
ROM-free tests and five owned-ROM Session regressions. Independent reviews
approved the runtime timing change and retained source/native verifier.
