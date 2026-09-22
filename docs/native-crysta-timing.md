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
- NPC translation currently assumes **two pixels per moving tick**, eight
  increments per16-pixel tile. This is an explicit approximation, not yet a
  source-qualified speed. Loop waits/setup are additional, so eight ticks is not
  the entire interval between random choices.
- Resident raster durations are raw native countdown bytes. The source scheduler
  decrements before testing negative, so a duration7 record should last **eight
  ticks**, not seven. Exact movement/script scheduling remains a separate question.
- The map-D wanderer `$83:8CB4` is the bounded native speed investigation target;
  its four duration7 records alone do not prove a tile-walk duration.

Do not claim a fixed wall clock reproduces native lag frames, transition timing,
NPC RNG, or the complete actor scheduler.

## Verification

Clock tests cover extra wakeups, 60/120/144/240/1000-Hz event schedules, bounded
catch-up and suspend/resume. Input tests cover short keyboard taps, held gamepad
buttons, and one-shot consumption. Native Session/headless regressions remain
unchanged by real-time scheduling. Independent correctness/architecture review
approved the host timing boundary.
