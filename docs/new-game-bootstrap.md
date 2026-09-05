# Fresh Japanese new-game bedroom bootstrap (experimental)

**Qualified from actual startup, not a saved checkpoint.** Two independent
`Session::new` processes with default zeroed SRAM reproduce the same input-only
route through the Japanese menu, name entry, prologue and opening conversation,
then demonstrate controllable Ark in bedroom `$000F`. No supplied SRAM, memory
patches, forced warps, snapshot restores, or second session in a process.

This is an optional owned-ROM oracle experiment, **not** a new portable intro VM
or a change to the main CLI/core. The existing saved-slot-1 baseline `(472,176)`
is a different starting state. Fresh startup places Ark at **`(304,112)`**;
the final movement-tested checkpoint is **`(332,140)`** in the same room.

## Reproduce

From repository root, with the authenticated Japanese ROM present:

```sh
sh tools/new-game-qualification/replay.sh 'local/Tenchi Souzou (Japan).sfc'
# ROM-free checker tests only:
python3 -B tools/new-game-qualification/test_verify.py
# Optional local screenshots; Pillow required only for this command:
python3 tools/new-game-qualification/screenshots.py local/new-game-qualification/replay-XXXXXX/a
```

`replay.sh` builds a standalone ignored crate; no workspace membership changes.
It runs two normal fresh processes, verifies both against committed metadata,
and compares their complete checkpoint reports and frame streams. It also runs
two fresh negative controls: omitting Start at name confirmation, and omitting
both movement holds. Both must fail the same checker on actual captured state;
there is no diagnostic-mode field that trivially causes rejection. Every probe
explicitly exits 0 after flushing output, avoiding ares singleton teardown.
The runner, not successful capture alone, establishes qualification.

ROM SHA-256:
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
The experiment used a read-only-opened ROM symlink in its own worktree. There is
no SRAM-file argument or read in the harness. `Session::new` itself supplies the
oracle's default zeroed SRAM, as defined in `crates/oracle/src/lib.rs`.

Captures go into a new ignored `local/new-game-qualification/replay-XXXXXX/`
directory. Each boot retains 13 raw WRAM/framebuffer checkpoints, a 7,100-row
CSV, and a JSON report. Only original source, selected numeric metadata and
hashes are committed. No ROM bytes, screenshots, raw WRAM or emulator snapshots
are committed. `checkpoints.json` pins full WRAM, little-endian VRAM, full raw
framebuffer, 64-byte event-block and 64-byte player-entity hashes.

**Frame convention:** input label F is applied before `run_frame`; completed
checkpoint N follows N calls and input label N−1. Ranges below are half-open,
matching [opening-doorway.md](opening-doorway.md) and
[movement-qualification.md](movement-qualification.md). All unlisted buttons
are released. Timing is a qualified conservative replay, not a shortest path.

## Visually confirmed menu and dialogue

The existing `oracle/tests/local_roms.rs` and boot/name-entry fixture supplied
Start timing and the three Down taps. Their checkpoint names alone are **not**
visual identification of the UI. At completed 800, this experiment visibly
shows **`旅の再開`** (resume journey), three `NO DATA` slots, and these options:

- `はじめから` — from the beginning / New Game;
- `きろくをうつす` — copy a record;
- `きろくをけす` — erase a record.

Three Down taps select **`はじめから`**, not the third kana row. A at 900 opens
the real **`NAME ENTRY`** kana grid, visually confirmed at 1100, with default
name **`アーク`**. Start at 1200 confirms that default name; no character-grid
editing is needed. At 1800 the screen reads **`序章 / 旅立ち`** (Prologue /
Departure). Neither a confirmation stall nor an emulator defect is established
by the older menu-wait experiments.

The local screenshot converter decodes 512×480 little-endian XRGB as B,G,R,X,
selects **even columns in the first 240 rows**, then scales 256×240 by 3 using
nearest-neighbor. The retained 800, 1100 and 7100 images were inspected using
that exact conversion. Earlier exploratory captures also guided each dialogue
advance; inputs were not chosen by indiscriminate button flooding.

| Button | Input labels | Visible purpose |
| --- | --- | --- |
| Start | `[400,410)` | Leave title for load menu |
| Down | `[700,712)`, `[730,742)`, `[760,772)` | Move past three empty slots to New Game |
| A | `[900,912)` | Select New Game |
| Start | `[1200,1210)` | Confirm default `アーク` on actual name-entry screen |
| A | `[3500,3512)` | Elle asks what happened; Ark was having a nightmare |
| A | `[4100,4112)` | Ark describes the same strange dream every day |
| A | `[4700,4712)` | Elle says that is unusual for him |
| A | `[5300,5312)` | Elle suggests going outside to see Crystal Blue |
| A | `[5900,5912)` | Dismiss final page: he will forget the unpleasant dream |
| Right | `[6800,6820)` | Deliberate horizontal control probe |
| Down | `[6900,6920)` | Deliberate vertical control probe, after neutral interval |

By 6800 Elle has left and dialogue is gone. After the movement probes, the 7100
image shows Ark standing below/right of the empty bed. This is an actual house
scene, not the prologue's temporary map field or the reused menu actor slot.

## Named state and control evidence

Full metadata and hashes are in
[`tools/new-game-qualification/checkpoints.json`](../tools/new-game-qualification/checkpoints.json).
Menu/prologue coordinates below are slot `$1000` values, **not player spawns**.

| Completed frame | Map | Slot/player position | Player resume | Meaning |
| ---: | --- | --- | --- | --- |
| 800 | `$0004` | 8,16 | `$87:81C0` | New Game selected in empty load menu |
| 1100 | `$0004` | 8,16 | `$87:8A03` | Actual kana name entry |
| 1800 | `$0129` | 8,0 | `$90:87A6` | Prologue title |
| 2500 | `$000F` | 304,112 | `$84:A303` | Bedroom intro, not yet admitted as control |
| 3500–5900 (five sampled pages) | `$000F` | 304,112 | `$84:A258` | Dialogue; player idle countdown frozen at 441 |
| 6800 | `$000F` | 304,112 | `$84:A2A3` | Intro done, neutral before deliberate input |
| 6900 | `$000F` | 332,112 | `$84:A258` | Right released and position stable |
| 7000 / 7100 | `$000F` | 332,140 | `$84:A258` | Down released and position stable |

The early transient map `$000F` at completed 1202 is **not** a playable house.
It becomes `$0129` at 1283; the eventual house request appears at 2346, while
loading still occupies the old actor slot. The qualified startup replay retains
the prologue and does not shortcut it.

During the 6800–7100 control witness:

- Program state `$0450=$00AA`, current map `$047E=$000F`.
- Player index `$0DEA=$1000`; control actor index `$0DEE=$11C0`.
- Player flags `$1004=$0414` through completed 6801, then `$0415`;
  offset-8 flags `$1008=0`. Transition bit `$1000` is absent.
- Global control gates `$097C/$097E=0/0`; player auxiliary `$7F:301E=0`.
- Event block `$06C0..0700` has only offsets 4=`$01`, 31=`$08` nonzero.
  Offset 31 was already set at name entry, before Start confirmation.
- Right appears in sampled held input `$0454=$0100` at 6801; walking resume
  `$84:A385` at 6802. Position changes while held, with the final delayed step
  at 6821 after release, then is fixed at `(332,112)` from 6823 through 6900.
- Down similarly appears as `$0400` at 6901, resume `$84:A351` at 6902;
  final delayed step at 6921, then `(332,140)` is fixed from 6923 through 7100.

The checker asserts these input/resume observations, global/player ownership
fields, two axes of response and neutral stability—not just coordinates or
elapsed time. It also checks the whole per-frame stream hash. The ROM-free tests
reject frozen position, drift after release, missing input, unfinished intro,
wrong map/actor index, transition ownership, and blocked control gates. This is
an exact witness, **not** a general speed/collision/retap model.

The event marker `$06C4` changes from 0 to 1 at completed **5903**; the player
countdown resumes at **5904**. It is a measured intro-release marker, not proof
that this bit alone grants control. Player flags and idle resume already looked
similar during dialogue, so those alone would give a false control claim.

Two release-build normal runs matched every report field and CSV byte:

- Full 7,100-row CSV SHA-256:
  `0be0605689766805a364c02f5f8aacb3d39b9e089cdc2f7c642dab1eb3c2dd32`.
- Completed 6800 full WRAM:
  `49ab74b6af5283a4fb8151e56cdafc27339276213eaa7f1e997fe65e55b0ea82`.
- Completed 7100 full WRAM:
  `1e64faa91c8d975524dc63494b44ea68053a400378c1a5fd017fa665e32e6de2`.

Experimental TDD: the pure control-witness tests first failed against an
unimplemented checker, then passed. Full startup behavior was investigated
visually before pinning it; it was not implemented through synthetic game tests.
Fresh negative boots without confirmation or movement both fail qualification.

## Bounded native/COP initialization evidence

These are native 65C816 actors interleaved with COP services whose inline operands
must not be disassembled as ordinary instructions. No general VM is inferred.
[`sources.json`](../tools/new-game-qualification/sources.json) pins the small
ROM ranges, including the **selector-0** table word; it contains no ROM bytes.

A separate fresh input-only diagnostic replay stopped after completed **2430**,
then called `trace_until_pc(target, 2_000_000, 10)` in this order. All six stops
were `TargetReached` at frame count **2431**, before their target instructions:

| Target | Fresh-start observation |
| --- | --- |
| `$84:A12E` | Player initialization, X=`$1000`, flags `$4414`, startup selector 0 |
| `$84:A155` | After two `COP 9B` creations; Y=`$11C0`, `$0DEE` still zero before its store |
| `$84:87D8` | `$0DEE=$11C0`; controller about to read player startup selector |
| `$84:87FA` | Selector 0 indexed `$84:87FE`, A=`$88AC`; `$097C=$8000` |
| `$84:88AC` | Direct common-restoration entry, not selector-5 doorway movement |
| `$84:88E2` | Gates `$097C/$097E=0/0`; immediately before `COP CB` schedules `$84:A2F3` |

Decoded bounded semantics:

1. `$84:A12E..A173` initializes player auxiliary/camera values, creates actors
   `$84:80E8` and `$84:87CE`, and stores the latter index at `$0DEE`.
2. Fresh execution takes `$A1A3 → A1BE → A21A`; `$A21A..A222` sets gate bit
   `$097C & $8000`, then saves continuation with `COP BC`.
3. `$84:87CE..87FD` reads the temporary selector at `$7F:0008,X`; selector 0
   dispatches through `$84:87FE` to `$84:88AC`. This is not the saved doorway's
   selector 5. Later animation reuses that auxiliary field.
4. `$84:88AC..88E8` adjusts controller/player flags, clears control masks and
   related runtime fields, and schedules player recovery through `COP CB`.
   The pre-COP stop still has player flags `$4414`; the later `$4000` clear is
   not attributed to this instruction. `$84:A2F3..A307` is the static recovery
   continuation using `COP B6/84/8E` before the common idle path.

A second fresh diagnostic replay to completed **6800**, with all buttons then
neutral, reached `$84:80F7` (actor X=`$1180`, `COP 71`) and success path
`$80:A016 → A027 → A037 → A042 → A047`. The last stop has frame count 6801.
The handler checks player auxiliary bit `$8000`, player flags `$0080/$0004`,
and inline masks against `$097E/$097C`. This observed path passes with player
flags `$0414`, auxiliary zero, both gates zero. It complements, rather than
replaces, the normal run's movement/release proof.

## Source-backed room-only semantic NewGame

The follow-up resolves the initial map/spawn and two event writers. The CPU-free
`startup.py` authenticates the owned ROM and every annotated `sources.json`
extent, then derives the room projection **without reading SRAM, WRAM captures,
or checkpoint coordinates**:

```sh
python3 -B tools/new-game-qualification/startup.py 'local/Tenchi Souzou (Japan).sfc'
python3 -B tools/new-game-qualification/test_startup.py
```

This projection explicitly treats opening-dialogue completion as a semantic
operation: **omit intro timing/rendering, run its known event assignment, admit
room walking**. It is not a byte-identical SNES state initializer. Inventory,
statistics, NPC simulation and emulator animation/scheduler state are outside
this room-only profile. In particular, it reports the source header's initial
`$4414` flags; it does not relabel those as the later `$0414` control flags.
The earlier live movement witness establishes that the projected room state is
usable after the actual intro. A production caller can honestly expose a
**semantic New Game (opening presentation omitted)**, but not claim that this
module simulates the whole intro or initializes all game subsystems.

### Exact spawn producer → consumer

All CPU/source ranges below are half-open and separately hash-pinned in
`sources.json`. Headerless normalization is `CPU & $3FFFFF` for these ROM banks.

| Source | Decoded bounded operation |
| --- | --- |
| `$90:87A6..87B0` | `COP $14`: map `$000F`, mode 1, selector 0, queued X `$0128` (296), Y `$0060` (96) |
| `$80:8A23..8A58` | COP handler stores map to `$047C`, mode to `$0484`, selector to `$0490`, X/Y to `$0492/$0494`; advances over eight operand bytes |
| `$82:801E..8020` | Map-F first actor-table slot is zero; select the second bank |
| `$83:801E..8020` | Map-F bank-$83 table slot points to `$83:8D1E` |
| `$83:8D1E..8D20` | Two-byte list prefix, followed by first record |
| `$83:8D20..8D27` | Seven-byte FD record: tile `(19,7)`, byte-3 operand 0, header pointer `$84:A129` |
| `$84:A129..A12E` | Five-byte player header: auxiliary selector 0, flags `$4414/$8441`, executable entry header+5 = `$84:A12E` |
| `$80:F5F9..F689` | FD actor creation yields default position `(19*16+8, 7*16)` = **(312,112)** |
| `$80:F7F3..F815` | If queued X OR Y is nonzero, replace position with `(X+8,Y+16)` (u16 arithmetic), then clear both queue words |
| `$80:F8C6..F8E1` | Establish player index `$0DEA`; transfer `$0490` to temporary `$7F:0008,X` and clear it |

Thus fresh `(304,112)` comes from the **explicit prologue request `(296,96)`**,
not a hard-coded checkpoint and not the FD record's default `(312,112)`.
The player loader chooses `$1000` in this run; CPU-free walking does not need
that emulator slot number. Selector 0 is a separately consumed startup field,
not inferred from zeroed memory or the later animation reuse of that address.

Live instruction stops confirm the producer and consumer independently:

- At frame count **2264**, `$90:87A6 → $80:8A23 → $80:8A53 → $90:87B0`.
  Before the handler, queue `(0,0)`; after its stores, pending map F and queue
  `(296,96)`, selector 0. This precedes the current-map switch at completed 2346.
- At frame count **2422**, `$80:F42F/$F5F9` identify first-record pointer
  `$83:8D20`. At `$F7F3`, player default `(312,112)`, queued `(296,96)`;
  at `$F80F`, player **(304,112)**; at `$F815`, queue `(0,0)`;
  at `$F8E1`, player ownership `$1000` and selector zero are established.
  Completed **2423** already contains the spawned player; its own native init
  script first runs at frame count 2431. These are distinct stages.

### Exact default and intro-release event writers

1. Selecting `はじめから` runs the branch at `$87:820E..8232`, calling
   `$87:CCA7..CCCE`. That clears **`$000600..000800`** (including the 64-byte
   event projection `$06C0..0700`) and `$7F:8000..8300`, then invokes the default
   table loader `$86:B900..B93F`. The table `$86:B93F..B9C9` consists of 34
   `(address,value)` word pairs and a negative-address terminator; none writes
   the event projection. The second table at `$86:B9C9` is immediately terminated.
   This is an actual New Game reset routine, not adoption of SRAM contents.
2. **`$87:80EC..80F0`: `COP $07 $80FB`** sets event index `$00FB`.
   Fresh stops at frame count **964** show `$80:BB77` receiving A=`$80FB`,
   `$80:BB9F` about to write A=`$08`, Y=31, then `$06DF=$08` afterward.
   It is set during name-entry setup, **before** Start confirmation at 1200.
3. **`$88:9791..9795`: `COP $07 $8020`** sets event index `$0020` after the final
   opening dialogue. Fresh stops at frame count **5902** show A=`$8020` at
   `$80:BB77`, A=1/Y=4 at `$80:BB9F`, then `$06C4=1` at `$88:9795`.
   Completed frame 5903 exposes the change. The default `$06DF=$08` is preserved.
4. Both calls use handler `$80:8669..8678` and shared writer `$80:BB77..BBA6`:
   byte offset `(operand & $0FFF) >> 3`, bit `operand & 7`; operand bit 15
   selects **set**, otherwise **clear**, preserving other bits. The eight masks
   at `$80:BBD3..BBDB` are pinned too. This is a decoded bit operation, not a VM.

The semantic event block starts zero, sets `$00FB`, then explicitly completes
the intro by setting `$0020`. Result: offsets 4=`$01`, 31=`$08`, SHA-256
`6c9f094ecf92d1e5c904aa8d4c2e301ba7c0b195adb86158722a62d530a6797b`,
matching the live control boundary exactly. `test_startup.py` covers set/clear,
preservation, idempotence, bounds, queue override/default and wrapping arithmetic;
its first run failed against unimplemented operations before implementation.

**Why dialogue completion is not merely flipping a control flag:** the final
continuation `$88:9775..9795` contains dialogue request/waits (`COP $1B/$1F`),
then `COP $19` resume-record metadata and the event assignment. The `$1F` handler
`$80:8C4A..8C9A` temporarily saves/clears `$045E`, calls the dialogue engine in a
wait loop, and restores the mask and actor flags on return. Only then does the
actor continue to the event write. The `$19` handler `$80:8B35..8B85` writes
`$0600..060F` resume metadata; it is **not** the player-spawn writer and does not
move Ark. The semantic shortcut models completion of these presentation waits,
not an unsupported claim that event `$0020` alone unblocks a live emulator.

### Automatic source and native evidence checks

`replay.sh` now also executes five **separate fresh boots**, named `trace-reset`,
`trace-default`, `trace-map`, `trace-spawn`, `trace-release`. Their 22 target stops
are pre-instruction, bounded to 2,000,000 instructions/two frames per call, and
all must remain within their initial completed-frame interval. No input edge or
release boundary is crossed while tracing. Raw traces and stopped WRAM stay
local; `native-reference.json` pins the five complete selected-metadata reports.
`semantic_check.py` authenticates those reports and raw WRAM, derives startup
from ROM sources, and compares the derived map, spawn and events against the
independently authenticated normal-run control checkpoint. There is no path
from captured coordinates back into the semantic initializer.

The exact name character encoding, inventory/stat effects of the reset table,
rendering, dialogue text processing and later actor lifecycle remain outside
this projection. No full intro VM, save reconstruction, core or frontend change
is part of this qualification.
