# Reference Emulator Selection

Status: decided 2026-08-25. ADR for
[the selection issue](../../meta/issues/select-reference-emulator.md).

## Context

M1 needs a deterministic, headless reference oracle: boot a verified ROM,
step frame by frame, apply recorded input, and export comparable state. No
SNES emulator is installed on the development machine, and windowed
emulators are explicitly unsuitable.

## Decision

Vendor the **LakeSnes core** (MIT, `elzo_d`/`angelo_wf`) at upstream commit
`9db90b8` under `vendor/lakesnes/`, compiled via `cc` in the `oracle`
crate. Only the headless `snes/` core is used — no SDL, no windowing, no
audio device; framebuffers and samples go to caller-provided buffers.

## Why LakeSnes over alternatives

| Candidate | Verdict |
| --- | --- |
| **LakeSnes core** | MIT; ~11 K-LOC headless C core; frame-stepped by design; `snes_runFrame` + explicit `snes_setPixels` flush; save/load state built in; compact enough to audit |
| Mesen / Mesen2 | Excellent debugging, but heavy UI-oriented application; embedding headless requires its Lua/IPC surface or significant surgery; license (GPL-3) conflicts with dependency rules |
| bsnes / ares | Most accurate; both are permissively licensed (ISC) today, so licensing is *not* the blocker — but the library API and accuracy-tier cost are far larger than this boundary needs. Revisit if LakeSnes accuracy becomes limiting |
| snes9x | Snes9x license is non-commercial; conflicts with MIT distribution; rejected |
| Custom harness | Maximum control, months of work before the first useful trace; rejected for now, revisit if LakeSnes accuracy becomes the limiting factor |

## Consequences

- Deterministic frame stepping and full WRAM access are available now; the
  smoke test proves boot to a stable boundary and input divergence on the
  real Japanese dump.
- `snes_setPixels` must be re-invoked after each `snes_runFrame` — the copy
  to the caller buffer is *not* automatic (upstream interactive frontend
  calls it once per frame; we do the same).
- LakeSnes is compatibility-grade, not cycle-exact. If oracle/reference
  traces later disagree with hardware findings, the boundary (Session API)
  stays; the core can be swapped.
- Upstream is archived (dormant since 2023; an active fork exists with CX4
  work). We carry the vendored copy and patch it in place with upstreamable
  diffs where practical. `vendor/lakesnes/shims.c` is **project-authored**,
  not upstream.

## Rejected-but-recorded

Swapping in a more accurate core later is intentionally cheap: the `Session`
surface (`new/run_frame/set_button/frame_state`) hides the core entirely.

## Update 2026-08-26: fork swap evaluated, not adopted

The M1 boot sequence stalls in an SPC driver-upload handshake (see
[oracle-boot-probes.md](../oracle-boot-probes.md)). Because upstream
LakeSnes was archived, the active fork (`dinkc64/LakeSnes`, with a
single-cycle SPC rework) was tested as a drop-in: vendored at its latest
commit (`048a0d72`), `cx4.c`/`cx4.h` added to the build, and a compat no-op
`shims.c` `snes_setPixelFormat` (the fork hardcodes the same XRGB8888
layout). The fork compiled unmodified against our shims, but the boot
replays **identically** — same map history, same state `$170` / pending map
41 wedge, same frame counts. Conclusion: both LakeSnes generations share
the CPU↔SPC interleaving gap (upstream README itself warns "communication
between the CPU and SPC is also not cycle-accurate"), so a core swap within
the LakeSnes family does not unblock the post-name-entry scenarios. The
vendored core stays at the ADR-pinned commit `9db90b8`. Revisiting accuracy
would mean a different core family (bsnes/ares), per the decision above.
