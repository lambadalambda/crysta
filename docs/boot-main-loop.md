# Boot, interrupts, and main loop

This document records the first labeled 65C816 control-flow islands in the
Japanese behavior reference. Source annotations and placement assertions live
in [`crates/disasm/asm/boot.s`](../crates/disasm/asm/boot.s). All canonical
addresses use the contiguous HiROM mapping from
[ADR 0003](adr/0003-matching-disassembly-toolchain.md); traces preserve the
actual runtime mirror.

## Address map

| Role | Normalized offset | Canonical source address | Runtime address |
| --- | ---: | ---: | ---: |
| Reset | `$008000` | `$C0:8000` | `$00:8000` |
| Native NMI trampoline | `$008007` | `$C0:8007` | `$00:8007` |
| Native IRQ trampoline | `$00800B` | `$C0:800B` | `$00:800B` |
| Native COP trampoline | `$00800F` | `$C0:800F` | `$00:800F` |
| Native BRK trampoline | `$008013` | `$C0:8013` | `$00:8013` |
| Native reset setup | `$008017` | `$C0:8017` | `$80:8017` |
| Main-loop frame gate | `$008043` | `$C0:8043` | `$80:8043` |
| Top-level state dispatch | `$00805A` | `$C0:805A` | `$80:805A` |
| Native COP handler | `$008378` | `$C0:8378` | `$80:8378` |
| Native NMI handler | `$05F98F` | `$C5:F98F` | `$85:F98F` |
| Native IRQ handler | `$05FB00` | `$C5:FB00` | `$85:FB00` |
| Native BRK handler | `$05FB01` | `$C5:FB01` | `$85:FB01` |
| Wait for NMI latch | `$068009` | `$C6:8009` | `$86:8009` |

## Reset and initialization

The emulation reset vector at `$00:FFFC` contains `$8000`. Hardware enters with
`E=1`, `M=1`, `X=1`, `PBR=$00`, `DBR=$00`, direct page `$0000`, and maskable
interrupts disabled. Reset executes `SEI; CLC; XCE`, entering native mode while
leaving 8-bit accumulator and index widths, then long-jumps to `$80:8017`.

Native setup:

1. clears decimal mode;
2. uses `REP #$30` to establish 16-bit accumulator and indexes;
3. sets direct page to `$0000` and native stack to `$01FF`;
4. returns the accumulator to 8-bit width while leaving indexes 16-bit;
5. sets `DBR=$81`, whose low addresses mirror WRAM;
6. calls the table-driven hardware-register initializer at `$86:B9CB`, which
   consumes address/value entries beginning at `$86:B9E8`;
7. clears WRAM and initializes low-WRAM fields at `$86:B8D6`;
8. uploads the initial SPC driver/data at `$86:AA9C`;
9. initializes opening state before entering the main loop.

The `$86:B9E8` startup table disables DMA/HDMA (`$420B/$420C`), keeps the PPU
force-blanked (`$2100=$80`), initializes display registers `$2101..$2133`,
disables NMI/IRQ/auto-joypad (`$4200=$00`), clears arithmetic/timer registers,
and enables fast ROM timing (`$420D=$01`). The subsequent WRAM routine clears
all of banks `$7E/$7F` before applying its low-WRAM initialization table.

The state entering the loop is native mode, `M=1`, `X=0`, `D=$0000`,
`DBR=$81`, `PBR=$80`, and `S=$01FF`. Reset keeps IRQ masked; the native IRQ
handler is intentionally a no-op in any case.

## Interrupt behavior

The native vector table at `$00:FFE4` points into four bank-$00 trampolines,
which use `JML` to establish each handler's program bank. Native interrupt M/X
widths are inherited from the interrupted code until a handler explicitly
changes them.

- **NMI** → `$00:8007` → `$85:F98F`. The handler saves P, DBR, A, X, Y, and D;
  establishes 16-bit A/X, `D=$0000`, and `DBR=$81`; disables HDMA; performs
  queued display transfers, beginning with a 512-byte `$7F:0600` → CGRAM DMA;
  restores HDMA; waits for HBlank to end; captures both auto-joypad ports from
  `$4218/$421A`; services an optional APU-port update; increments the low-WRAM
  frame counters at direct-page `$42/$44`; restores the complete context; and
  executes `RTI` at `$85:FAFF`.
- **IRQ** → `$00:800B` → `$85:FB00`. It immediately executes `RTI`; maskable IRQ
  has no game handler.
- **BRK** → `$00:8013` → `$85:FB01`. It executes two `NOP`s, stores A to
  `$FF:8000` as the original debug/emulator marker, then executes `RTI`.
- **COP** → `$00:800F` → `$80:8378`. Its ABI requires native mode, 16-bit
  indexes (`X=0`), and `D=$0000`; M and DBR are inherited, then M is forced to
  16-bit. The dispatcher clobbers A/X/Y and direct-page scratch `$36/$38`, reads
  the COP signature byte through the stacked return address, doubles it into a
  16-bit index, and jumps through the service table at `$80:83B2`. Shared return
  helpers advance the stacked address by two, four, five, or eight bytes before
  `RTI`.
- **ABORT** and the native reserved slot contain `$0000`; they are unsupported.

Only the emulation RESET vector is used. Other emulation-vector words are
retained exactly but remain unqualified: reset switches to native mode before
interrupts are enabled.

## Authoritative frame boundary and dispatch

Every main-loop iteration starts at `$80:8043` by calling
`$86:8000`. That routine preserves P/A/Y, forces an 8-bit accumulator, clears a
possibly stale NMI latch with one read of `RDNMI` (`$4210`), then loops at
`$86:8009` until a new NMI period sets bit 7. It reads `RDNMI` once more,
normalizes controller input in its remaining body, restores the caller's
register widths, and returns.

The return from this routine is the authoritative main-thread frame boundary:
one consumed NMI period permits one common-update and state-dispatch pass. The
main loop then calls common subsystems and reaches `$80:805A`, where
`JMP ($049E)` dispatches through a mutable 16-bit low-WRAM handler pointer.
State handlers return to `$80:8043`, directly or through a shared tail. The
program-state values observed elsewhere at WRAM `$0450` describe state, while
`$049E` is the proved executable dispatch pointer.

## Reference trace qualification

The ares shim exposes bounded structured pre-instruction traces through
`Session::trace_until_pc`. Each record contains actual 24-bit `PBR:PC`, P,
emulation mode, direct page, and DBR. The target record is included before that
instruction executes. Tracing disables ares's recent-address suppression so
polling loops retain duplicate PCs; instruction and frame limits prevent an
unbounded run, and the public API caps one trace at 2,000,000 records.

The ROM-backed integration test in
[`crates/oracle/tests/local_roms.rs`](../crates/oracle/tests/local_roms.rs)
traces hard reset to the first `$80:8043` instruction. Two clean child-process
runs agreed on:

- stop reason: target reached;
- instruction records: `965,059`;
- elapsed frame events: `64`;
- first PCs: `$00:8000`, `$00:8001`, `$00:8002`, `$00:8003`, `$80:8017`;
- initial state: `P=$34`, `E=1`, `D=$0000`, `DBR=$00`;
- state before reset's `JML`: `P=$35`, `E=0`;
- final pre-instruction PC: `$80:8043`;
- trace digest: `8a6db5e98dac5f7b6e085a7509f4259dafa283ab3df4b929fda76d21d7b09df9`.

The version-1 digest stream starts with the domain bytes
`terranigma.cpu-trace\0`, `version:u32-le`, and `record_count:u64-le`, followed
by each fixed-width record: `address:u32-le`, `status:u8`, `emulation:u8`,
`direct_page:u16-le`, and `data_bank:u8`. Only the count, format, checkpoints,
and digest are committed; raw trace output remains under ignored `local/`
according to the repository's [oracle artifact policy](../CONTRIBUTING.md).
