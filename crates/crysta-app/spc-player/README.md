# spc-player

Small, safe Rust owner for the existing MIT LakeSnes **audio-only** APU/SPC/DSP.
Part of [native Crysta music](../../../meta/issues/native-crysta-music.md).
No ROM, extracted music, capture, device, game CPU, or PPU is included.

## API and integration boundary

- `Apu::new() -> Result<Apu, Error>` resets to the physical IPL. Advance cycles
  until output ports 0/1 report `AA`/`BB`, then use the IPL host-upload protocol.
  The synthetic tests demonstrate upload and execution of a tiny loop.
- `write_port(index, value)` changes **only** CPU-to-APU `inPorts[0..4]`;
  `read_port(index)` reads APU-to-CPU `outPorts[0..4]`. Neither advances time.
- `run_cycles(u32) -> Result<u32, Error>` advances complete SPC opcodes and
  **discards** generated/pending audio. Limit: 1,024,000 requested cycles/call;
  return value is actual cycles (at most 31 extra). Use short calls and a caller
  timeout for host handshakes. A zero-cycle call advances nothing.
- `render(&mut [i16]) -> Result<usize, Error>` fills interleaved L/R samples at
  **32,000 stereo frames/sec**, returning frames, not individual samples.
  Accepts even lengths up to 64,000 samples, including zero; chunk larger work.
  The caller owns device-rate conversion, volume, pause, buffering, and output.
- `read_ram(start: usize, &mut [u8])` copies bounded **physical** 64 KiB RAM,
  without IPL overlay or I/O side effects, for source-transfer verification.

Construct/upload/render/drop the APU on one owning thread. It is deliberately
`!Send` and `!Sync`; construct it inside an audio worker rather than moving one
into it. The app can retain `forbid(unsafe_code)`: all FFI is private here.
No raw RAM loader, execution-PC setter, or capture loader is currently needed.
ROM extraction and source-specific upload/commands belong to the caller.

## Clock, allocation, and safety

The build compiles only `apu.c`, `spc.c`, `dsp.c`, `statehandler.c`, and our
`shim.c`. Header references to `Snes` do not pull in its implementation.
`apu.snes` is null; **never call `dsp_getSamples`**, which dereferences it and
uses SNES-frame resampling. The shim advances one `spc_runOpcode` at a time;
`apu_cycle` produces a native DSP stereo frame every 32 SPC cycles. Rendering
consumes the DSP ring via its 16-bit `sampleOffset`, with a separate persistent
consumer cursor (1024-frame ring and 16-bit counter wrap both covered).
No bulk frame stepping, SNES frame rates, or audio loss at call boundaries.

All caller port indices, RAM ranges, cycle requests, and output sizes are
validated in Rust before private C entry points. No C pointers escape. Calls
hold exclusive ownership for mutation; buffers are borrowed only for the call.
The shim uses one checked `calloc` containing the upstream APU/SPC/DSP structs,
then wires the same pointers as their constructors and invokes `apu_reset`.
This avoids the upstream constructors' unchecked `malloc` dereferences. Drop
frees that one allocation, not the embedded components individually. Rendering
and stepping allocate nothing. Pointer wiring must be re-audited on vendor
updates.

The pinned SPC opcode implementation has finite instruction paths (including
STOP/SLEEP idle cycles), at least one and no more than 32 cycles per call.
The shim checks this progress contract and gives rendering an explicit opcode
budget. Setup drains after every opcode, so arbitrarily long *sequences of
bounded calls* cannot replay old setup audio. These are per-call work bounds,
not a realtime scheduling guarantee. Validation failures leave state/output
untouched; a backend progress error may leave partial output and advanced state.

## Provenance and limitations

Uses the retained, old `vendor/lakesnes/` core pinned at upstream commit
**`9db90b8`**, documented in [ADR 0002](../../../docs/adr/0002-reference-emulator.md),
not the evaluated `048a0d72` fork or the currently used ares reference oracle.
Upstream LakeSnes is by `angelo_wf`/`elzo_d` and contributors; its existing
[MIT license](../../../vendor/lakesnes/LICENSE.txt) carries copyright
2021–2023 angelo_wf and contributors and must accompany redistribution.
The Rust boundary/build/shim are project-authored MIT code. The sole audio-core
patch is in `dsp_decodeBrr`: two signed left shifts are replaced with bounded
multiplications, preserving BRR scaling without C99 undefined behavior for
negative nibbles. Normal ranges multiply a nibble in `[-8,7]` by at most 4096;
reserved ranges multiply its sign-extended value (`-1` or `0`) by 4096. Existing
arithmetic right-shift assumptions and the rest of the core remain unchanged.

This is compatibility-grade opcode-stepped audio, **not a classic-fidelity or
cycle-exact hardware claim**. Synthetic tests establish the streaming and IPL
boundary, not any particular game's complete driver protocol, music accuracy,
or device playback. The C core retains its platform/compiler behavior
and upstream emulation limitations. Source upload can overwrite RAM/I/O just as
on the emulated APU; callers must supply a valid driver and bounded handshake.
No app integration or real-ROM/device smoke verification is claimed here.

## Tests / implementation record

From repository root (the app's isolated workspace, no root workspace/lock edits):

```sh
cargo test --manifest-path crates/crysta-app/spc-player/Cargo.toml --target-dir target/spc-player
cargo test --release --manifest-path crates/crysta-app/spc-player/Cargo.toml --target-dir target/spc-player
cargo clippy --manifest-path crates/crysta-app/spc-player/Cargo.toml --target-dir target/spc-player --all-targets -- -D warnings
```

Direct Cargo commands generate a crate-local `Cargo.lock`; it is not part of
this library change. Build products above use the repository's ignored target.

TDD: the first test command ran against no-op clock/render placeholders:
**6 failed, 1 passed** (IPL never reached `AA`, render left sentinel samples).
After incremental stepping/ring consumption: **7 unit tests + 1 compile-fail
thread-ownership doctest pass**. Tests cover physical IPL upload and RAM
verification, looping SPC input/output, host/APU port separation, reset silence,
frame counts, invalid bounds, stopped-SPC progress, setup discard beyond ring
capacity, and identical nonzero asymmetric stereo across arbitrary chunks over
70,013 frames (including producer-counter wrap). The setup-discard test was
corrected to finish the uploaded DSP initialization before assuming a fixed
4-cycle BRA loop; opcode-boundary overshoot during initialization is expected.

### BRR signed-arithmetic regression / UBSan

Two additional tests upload only synthetic SPC instructions, a sample directory,
and a looping filter-0 BRR block of negative (`$F`, signed -1) nibbles via IPL.
They exercise all normal ranges 0–12 and reserved ranges 13–15, checking exact
sustained stereo PCM after interpolation/gain/volume, not merely non-silence.
Before the arithmetic patch, each test separately aborted under UBSan at
`dsp.c:433` / `dsp.c:435`: **left shift of negative value -1** (Cargo exit 101).
After the patch, the complete suite passes under UBSan in **debug and release:
9 unit tests + 1 compile-fail doctest**, without sanitizer exclusions.

Reproduce on macOS/Apple Clang (instrument the C core and link its UBSan runtime
into the Rust test binary; Rust itself is not sanitizer-instrumented):

```sh
(
  rt="$(clang -print-resource-dir)/lib/darwin"
  export CFLAGS='-fsanitize=undefined -fno-sanitize-recover=undefined'
  export RUSTFLAGS="-C link-arg=$rt/libclang_rt.ubsan_osx_dynamic.dylib -C link-arg=-Wl,-rpath,$rt"
  export UBSAN_OPTIONS=halt_on_error=1
  cargo test --manifest-path crates/crysta-app/spc-player/Cargo.toml --target-dir target/spc-player-ubsan
  cargo test --release --manifest-path crates/crysta-app/spc-player/Cargo.toml --target-dir target/spc-player-ubsan
)
```

To reproduce the red phase on the original vendor arithmetic, append
`negative_brr_normal_ranges` or `negative_brr_reserved_ranges` to the debug test
command separately (UBSan deliberately stops at the first violation).
This regression does not establish that every upstream C path is UB-free.
