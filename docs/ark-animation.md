# Ark ordinary standing and walking animation

Qualified Japanese reference ROM: SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
This is animation **selection**, not graphics/palette/composition decoding or a
replacement for [qualified movement cadence](input-admission.md). The asset
qualification owns art; the parent owns renderer/host/UI integration. Core
module/state/snapshot integration is implemented below. No oracle changes,
patches, native restores, SRAM bootstrap, save-state observations, or native
animation VM are required.

## Integration contract (usable subset)

`AnimationState`, `AnimationFrame`, and `AnimationSet` are exported at the crate
root. `GameState` owns the animation component and exposes its derived key as
**`FrameOutput.animation`**, both from `output()` and successful `step()` calls.
Both constructors start in ordinary Down standing. Internally, walking and
animation advance on a cloned candidate state; late exit failures discard that
candidate along with all its movement, input history, tick, and animation changes.

For other core callers the same ownership order applies:

```rust,ignore
// New-game semantic pose; native initial facing is Down.
let mut animation = AnimationState::standing(Direction::Down);
// After admission/collision succeeded on the candidate whole state:
let movement = walking.step(room, input)?;
let frame = animation.advance(walking.active_direction());
```

Do not feed current raw input, movement phase, attempted speed, or actual dx/dy
into animation. `WalkingState::active_direction()` already owns the one-frame
input delay and setup/turn decisions. Never advance animation on rejection;
commit walking/animation together only after the whole tick succeeds. The pure
component cannot admit dash/combat or otherwise bypass walking's rejection.

`frame()` returns the ROM-independent asset key:

| Field | Meaning |
|---|---|
| `set` | `AnimationSet::Standing` or `Walking` |
| `sequence` | Down=0, Up=1, horizontal=2 |
| `record` | **zero-based**: standing=0, walking=0..5 |
| `mirror_x` | true only for Left, sharing the Right sequence |

Walking selects **six records, nine ticks each**, repeating every 54 animation
ticks for *all* four directions. Turn, reversal, or standing→walking resets phase
to zero on the active-direction setup tick, even though displacement is zero.
Held wall-blocked input continues the same animation clock. Neutral immediately
selects ordinary standing in the last facing. A subsequent same-direction walk
restarts at record zero (only when the walking admission gate accepts it).

### Integrated snapshot profile 7

`GameState` retains the top-level `RSLC` format version 1 and bumps the semantic
profile byte at offset 5 to **7**. Snapshots are now **103 bytes**; profiles ≤6 and
other lengths are rejected, not implicitly upgraded. Existing layout is stable:

| Byte offset | Encoding |
|---|---|
| 83..99 | unchanged 16-byte walking v3 component, or all zero during a doorway |
| 99 | unchanged fresh-bedroom flag |
| 100 | animation facing: Down=0, Up=1, Left=2, Right=3 |
| 101 | animation walking: false=0, true=1, other values rejected |
| 102 | animation phase 0..53; standing requires zero |

`frame` remains derived, not separately serialized. `GameState::restore` checks
canonical animation parts plus ownership coherence: ordinary active walking must
have walking animation facing the **active**, not delayed, direction; ordinary
neutral must be standing, with any retained facing permitted. Horizontal phases
must agree. Vertical movement phase must match animation parity, allowing phase
zero both at setup and at later 54-tick animation wraps. Doorway animation must
be standing in `transition.direction()`.

On the handoff tick, during every departure/arrival update, and on completion,
`GameState` explicitly selects standing in the transition direction (Down F→10,
Up 10→F). Transition inputs remain discarded. Completion resets walking admission
history but preserves that pose; neutral after completion retains its facing.
This is semantic rendering policy, **not native doorway animation timing**.

### Asset stream selector mapping (metadata, not decoded art)

| Set | Native base | Sequence table / selected records |
|---|---|---|
| Standing | `$A4:A1E4` | sequence 0 at `$A4:A23C`, 1 at `$A4:A242`, 2 at `$A4:A248` |
| Walking | `$9A:D064` | sequence 0 at `$9A:D0C4`, 1 at `$9A:D0DE`, 2 at `$9A:D0F8` |

Each sequence pointer is a little-endian base-relative word. Each record is four
bytes: duration byte, facing byte, base-relative composition-header word. A
negative first word terminates the sequence. The native composition-list pointer
is **base + header offset + 4** (the header itself belongs to the assets agent).

Standing composition-list keys, in sequence order: `$A4:A54E`, `$A4:A597`,
`$A4:A5E0`. Walking composition-list keys, in record order:

- Down: `$9A:D48A`, `D4C5`, `D507`, `D549`, `D584`, `D5C6`.
- Up: `$9A:D608`, `D64A`, `D67E`, `D6B9`, `D6FB`, `D72F`.
- Horizontal: `$9A:D76A`, `D7AC`, `D7E7`, `D822`, `D86B`, `D8A6`.

Do **not** call `$7F3014` a sprite frame ID: it is the movement stream selector.
Its values (Down=1, Up=2, horizontal=3 after setup) can lag setup and remain after
release. Animation state instead lives in:

| WRAM address (player entity `$1000`) | Meaning |
|---|---|
| `$7E1010/$7E1012` | animation base low word/bank |
| `$7F1008` | sequence selector |
| `$7E1020` | cursor; incremented after loading, thus displayed walk record + 1 |
| `$7F100A` | selected composition-list low pointer |
| `$7E100E` | scheduler countdown, not a universal elapsed-time field |
| `$7E1014` | facing: Down=0, Up=1, Left=2, Right=3 |
| `$7E1008 & $4000` | horizontal mirroring; unrelated other flag bits not modeled here |

### Initial facing and deliberate idle policy

Fresh completed frame **6800** is map F, `(304,112)`, facing **Down**, but already
in an idle/fidget script: base `$A5:DCA6`, selector `$21`, cursor 0, composition
`$A5:F839`, resume `$84:A2A3`, timer 141. This is **not** the ordinary standing key.
The semantic new-game pose is explicitly Down standing (`$A4:A54E`), not a claim
of pixel equality to the native fidget at frame 6800.

After a walk release, native ordinary standing remains visually selected while
its sentinel clears the cursor and a 480-count idle wait starts. Longer idle
enters separate fidget scripts with RNG-dependent waits/variants. The fresh idle
capture witnesses selector `$23` at frame 6942 and `$22` at 7484; the actor does not
walk and remains Down-facing. Our neutral policy holds the ordinary standing
pose indefinitely. It neither freezes native time nor purports to port these
fidgets. Likewise, doorway departure/arrival sprite timing is **semantic policy**,
not ordinary animation qualification. Native arrival frame holds include
scheduler stalls (e.g. first map-10 arrival record 6998→7010 takes 12 frames), so
feeding wall-clock ages across transitions into this component is incorrect.
`GameState` selects a directional standing pose at each semantic handoff; exact
native doorway timing and fidgets remain outside the usable subset.

## Source and stopped-native witnesses

Bounded source hashes and stopped register/WRAM/trace hashes are in
[`pins.json`](../tools/player-animation-qualification/pins.json). No raw ROM,
WRAM, CPU traces, OAM or art is committed.

- `$84:A2E9..A33E`: facing-directed standing scripts; COP `$84` sequences 0/1/2,
  null movement, table 0. Left/right differ through COP `$B7/$B6` mirror control.
- `$84:A33E..A389`: Down/Up use COP `$83` sequences 0/1, table 1;
  horizontal uses COP `$84` sequence 2, null movement, table 1. All wait with
  COP `$8E` and loop. The latter reloads movement on every sequence wrap; this is
  why movement's horizontal 54-tick gap does **not** imply a different animation
  clock for vertical directions.
- `$80:A200..A2D8`: COP table/sequence selection, cursor reset, table base load;
  COP84 also calls `$80:BBDB` for movement-stream setup.
- `$80:A32F..A33D`: COP8E calls `$80:ED75`; carry indicates sequence termination.
- `$80:ED75..EDA6`: base + sequence pointer + cursor×4 selects a record; negative
  duration branches to sentinel handling; duration and facing bytes load into
  `$100E/$1014`. `$80:EDAD..EDB5` adds base and 4 for composition-list pointer.
- `$80:EDFB` increments cursor. `$80:EE05..EE0E` converts facing 3 to 2 when
  horizontal mirroring is active.
- `$80:C72C..C731`: countdown decrements and resumes only when negative, hence
  a stored duration **8 lasts 9 completed frames**, not eight.
- `$80:ED51` clears cursor; the ordinary walking player retains live movement
  streams under its flags. Horizontal COP84 restart separately reloads null
  streams. Standing's zero-duration record is selected once; its composition
  pointer survives the next sentinel and idle wait.

Read-only native PC stops prove player ownership (`X=$1000`, except the COP
trampoline dispatch index `$0108`, with player in Y):

| Plan / completed-frame interval | Stops and result |
|---|---|
| setup / 6801 | `84A380 → 80A295 → 80ED99 → 80EDFB → 80EE11`; Y=`D0F8` identifies horizontal record 0; timer 8, facing 3, composition `D76A`, cursor 0→1 |
| release / 6863 | `84A335 → 80A295 → 80ED99 → 80EDFB → 80EE11`; Y=`A248` identifies standing horizontal record; timer 0, composition `A5E0`, cursor 0→1 |
| wrap / 6855 | `84A385 → 80ED51 → 84A387 → 84A380 → 80EDFB → 80EE11`; sentinel Y=`D110`, cursor 6→0→1, final timer 8 and composition `D76A` |

Every stop stays inside its stated completed-frame interval. These captures use
`Session::new`, input only, `wram_image`, `cpu_registers`, and `trace_until_pc`.
One boot per process and explicit `process::exit(0)` avoid session teardown
ambiguity. `save_state` is not used as a passive observation.

## Reproduction and red/green tests

```sh
cargo test -p room-core                         # pure component + slice integration
./tools/player-animation-qualification/replay.sh # local JP ROM symlink by default
# To recheck existing private captures without rebooting:
python3 tools/player-animation-qualification/verify.py ROM local/player-animation-qualification/replay-XXXXXX
```

[`plans.json`](../tools/player-animation-qualification/plans.json) fixes half-open
input intervals: input label F precedes completed frame F+1. Shared
`tools/new-game-qualification/bootstrap.rs` reaches 6800 without saves/patches.
The f6800 full-WRAM pin is
`49ab74b6af5283a4fb8151e56cdafc27339276213eaa7f1e997fe65e55b0ea82`.
Each of six plans runs in **two fresh processes**; CSV, reports, final WRAM and
all stopped-PC evidence match pinned hashes. All raw captures stay under ignored
`local/player-animation-qualification/`.

The comparator uses the real `WalkingState` and captured room grid, checking
positions plus the pure animation's table/sequence/record-selected native
composition, facing/mirroring, walking cursor and countdown on every step:

| Qualified inclusive span | Steps | Active zero-displacement ticks (includes setup) |
|---|---:|---:|
| F 6800→6967 | 167 | 3 |
| F after return 7250→7550 | 300 | 30 |
| 10 after arrival 7050→7410 | 360 | 190 |
| **Total per fresh replay set** | **827** | **223** |

Only completed6801 in the span starting6800 is exempt from ordinary-pose
comparison: fresh mapF at304,112, delayed Right still inactive, semantic Down
standing, native unmirrored Down fidget `$A5:DCA6/$21`, cursor0, composition
`$F839`, countdown140. Movement is still checked. Any changed initial pose or
later fidget must fail ordinary selection, not bypass it. Nine mutation controls
in `test_compare.py` cover that boundary and run after replay authentication.
Transition intervals are not compared.
Captured initial/final room grids are observational data for the local comparator,
not snapshot restore or a native patch.

Concrete timing checks:

- Right input `[6800,6862)`: 6801 retains initial pose, 6802 walk record 0 and
  zero movement, 6803 first displacement. Records change at 6811, 6820, 6829,
  6838, 6847, 6856. The latter wraps animation while horizontal movement has a gap.
- Release: 6863 still walking/moving; **6864 standing**, 6865 idle resume/cursor 0
  with the **same standing composition**. Do not wait for idle resume to render.
- Right→Left at 7340: 7341 retains Right; 7342 Left-mirrored record 0 with zero
  setup displacement. Up and Down turns similarly reset at 7422 and 7462.
- Map-10 Up wall hold: y=352 throughout 7090..7201 while all six records cycle.
  Release selects Up standing at 7202, not a Down default.
- One-frame Left tap `[7380,7381)`: 7382 Left walking record 0, 7383 Left standing;
  no displacement. **Do not substitute polled `$0454` for submitted input**:
  the sampled joy word still reads Left at 7382; the probe records both fields.

TDD: five synthetic tests first failed because the module did not exist, then
passed after implementation. A deliberately wrong two-frame comparator fails at
**6820** (`D76A` predicted vs native `D7E7`); the real six-record component passes
both 827-step runs, including timestamp/cursor checks. Restore-part tests cover
canonical validation and deterministic continuation.

Integration TDD: `tests/slice_animation.rs` first failed on the missing root
exports and `FrameOutput.animation`. It now covers initial pose, delayed setup,
turn/release, blocked six-record cycles in all directions with snapshot replay,
profile/layout and malformed/coherence rejection, early admission and late exit
rollback, and both complete doorway routes. The original component tests now
exercise the root exports rather than a path-imported duplicate module. Existing
slice snapshot-replay tests also compare the added animation field. Host/UI
adapters are intentionally outside this change and must consume `output.animation`.
