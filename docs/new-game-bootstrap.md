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

**Still opaque / bounded follow-up leads only:** exact bank-87 default/name
confirmation writers; event `$06DF` bit `$08` initialization between 800 and
1100; prologue transition from `$90:87A6/$87B2`; and the dialogue-release writer
of `$06C4` bit 0 near completed 5903. To investigate scheduling, compare the two
native call sites `$8D:8C1B/$8C2E → $80:C6F0` and player countdown decrement
`$80:C72C` across 5902–5904. Current evidence does **not** establish that all
actors pause, or decode the entire dialogue/intro scheduler. None of these
remaining leads blocks the qualified actual-startup replay.
