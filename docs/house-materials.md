# House P16 and passive flagged-cell collision

Bounded qualification for [repeatable house movement](../meta/issues/qualify-repeatable-house-movement.md),
on top of the [O/S corner profile](house-movement.md) and
[ordinary input admission](input-admission.md). This changes **only collision
material classification and pair response**. Parent owns the combined profile
version, snapshots, slice integration, tracker and broader gameplay admission.

## Production policy

Unflagged materials now have three classes:

- **O**: stored types 0, 2, 22 (open).
- **S**: stored types 12, 14 (solid).
- **P**: stored type 16 (distinct partial-material pair response).

Rooms constructed with `Room::new` accept these unflagged classes during movement,
but reject sampled bit-15 cells. Construction itself validates only grid shape.
`Room::new_passive(width, height, raw_cells)` additionally admits **passive flagged
cells**. This is an explicit caller assertion, not automatic recognition of a
passive game mode. The caller must establish ordinary, action-free movement and
inactive collision action hooks: reference **`$0980 & $0050 == 0`**. Do not select
this constructor for attacks, interactions, pushing, action recovery or other
unqualified controller modes. It does **not** implement native action hooks.

Raw cells remain unchanged and observable through `cells()`. With the passive
policy, a **new-edge** raw bit-15 cell is class-3 solid, regardless of its stored
type; it is not classified as its masked low type. The **old-edge** stored-type
6/7 rejection happens first and remains in force even when bit 15 is set. Thus a
new flagged stored-6 sample can block like S, while an old flagged stored-6 sample
still fails closed. This preserves the ordering that a blanket flag mask loses.
Other unflagged stored types remain unsupported.

The policy is stored on the immutable Room, not in mutable walking history.
Parent integration must choose it deliberately and include the changed collision
semantics/policy in the combined profile identity. This work does not bump slice
`PROFILE_VERSION`, alter walking snapshots, or switch the slice's Room constructor.
A walking snapshot alone does not authenticate which Room policy the caller used.

## Pair rules (same in all four directions)

Keep existing old/new-edge sampling, positive-edge minus-one sampling, old-slope
rejection and bit-3 snap/rollback. The reference/Python integrator retains its
X-before-Y ordering; production admits only cardinal movement. No generic AABB
solver or new material geometry is introduced.

Let `q` be the perpendicular pixel remainder: `(y-16)&15` for X movement,
`(x-8)&15` for Y movement. At q=0 there is **one** new sample; O passes and S/P
blocks without a perpendicular nudge. Otherwise first/second mean upper/lower
for X, or left/right for Y:

| First/second | Primary axis | Perpendicular nudge |
|---|---|---|
| O/O | Pass | None |
| O/S, O/P | Block | −1 if q<8 |
| S/O, P/O | Block | +1 if q≥8 |
| S/S, P/P | Block | None |
| **S/P** | Block | **+1 if q≥8** |
| **P/S** | Block | **−1 if q<8** |

Nudges are retained even if the primary axis rolls back. They can align an edge,
changing the next frame to a single-sample block; blocking does not freeze the
ordinary cadence. Flagged new samples use the S row/column, never the P row/column.

## Source-backed dispatch

PCs/addresses are hexadecimal; types, positions and frame numbers are decimal.
ROM authentication and reproduction commands are below. The saved static sources
were inspected from the parent's ignored movement/collision disassembly using an
own copied decoder; none of those exports is committed.

Special-player first-sample tables are:

| Direction | First table | O-first branch | P-first branch | S-first branch |
|---|---|---|---|---|
| Up | `$80D542` | `$80D3AC` | `$80D3B6` | `$80D3C0` |
| Down | `$80D8E8` | `$80D7A0` | `$80D7AA` | `$80D7B4` |
| Left | `$80DC60` | `$80DB48` | `$80DB52` | `$80DB5C` |
| Right | `$80DFDC` | `$80DEBE` | `$80DEC8` | `$80DED2` |

The following three tables, each 64 bytes, are **O, P, S** second-sample tables,
**not O, S, P**. Confusing their order swaps the important S/P and P/S cases.
For example Up P-first uses `$80D5C2`; S-first uses `$80D602`:

- O/S or O/P → `$80D3F1` (negative test).
- P/O → `$80D3E6` (positive test); P/S → `$80D3F1`; P/P → `$80D3FD` (block).
- S/O or S/P → `$80D3E6`; S/S → `$80D3FD`.
- Corresponding positive/negative/block targets are Down
  `$80D7C8/$80D7D3/$80D7DE`, Left `$80DB70/$80DB7B/$80DB86`, Right
  `$80DEE6/$80DEF1/$80DEFC`.

`$80E7C2` tests horizontal-coordinate remainder against 8 for Y collision;
`$80E7CB` tests vertical-coordinate remainder for X collision. The positive and
negative paths increment/decrement the actual player's perpendicular coordinate,
then join the already qualified primary correction path.

New-sample helpers `$80E838`, `$80E750`, `$80E777` inspect bit `$0080` in the
high byte of the raw cell. If set, they substitute table index `$0006`—**class 3
multiplied by two**—before dispatch. All four first tables and all twelve
second-sample tables map class 3 to the **same targets as 12/14**.

Old-edge slope checks instead shift the raw high byte and mask `$1F` before
comparing 6/7, independently of the override flag. Examples: Up `$80D36C/$80D374`
(aligned) and `$80D386/$80D39A` (pair), Down `$80D760/$80D768`, Left
`$80DB08/$80DB10`, Right `$80DE7E/$80DE86`. The bounded implementation
conservatively rejects either old sample containing 6/7; it does not port slopes.

### Why flags require a passive policy

Blocking can call a player action hook. For Up, `$80D3FE → $80E1DF` checks mode
and facing, selects the relevant sample, and calls `$80E71B` to inspect its flag.
A flagged sample branches to `$80E2D0`. That handler reads `$0980`, tests `$0050`,
and returns via `$80E32C` when clear. Otherwise it can select a player/controller
script: it is **not universally inert**. Corresponding flagged handlers are Down
`$80E41C`, Left `$80E56E`, Right `$80E6BD`, with the same mask gate. P16's
unflagged hook classification is zero and falls into the existing generic
`$0050`-gated return path; it is not an invitation to implement actions.

**Important correction to endpoint-only observations:** completed idle snapshots
have `$0980=0`, but fresh per-frame logs show **`$0980=$00A0` while walking**.
The required condition is mask `$0050` clear, not the whole word zero. `$097C=0`
in all captured rows. The first fresh checker run intentionally failed its
overstrong whole-word-zero assertion; the source-masked check then passed.

At frame-counter **1603**, `trace-flag` reaches `$80E2D6` with A=`$00A0`, Z=1;
resumption records **only `$80E32C`** before the next stop. It returns without
native script writes/action dispatch, then reaches `$80D401` for wall correction.
This is integration visible in completed 1604, not an extra completed frame.

At frame-counter **1623**, `trace-partial` takes `$80D3A4 → $80D3B6 → $80D3F1`:
first sample raw **`$2044` (P16)**, second **`$9845` (flagged class3)**. At
`$80D3F7`, q=3, X=459; at `$80D401`, X=458. The actual P/S negative nudge is
visible in completed **1624**, followed by X=457 and X=456 at 1625/1626.
Collapsing P into S predicts X=459 at 1624 and is falsified immediately.

## Authenticated trajectories: no excluded frames

- **`wall-Up`**, completed 1601–1790: **189 transitions**, all stay `(472,176)`.
  The prior diagnostic admitted 11 setup/idle transitions and excluded 178
  flagged-contact transitions. This profile matches all 189.
- **`cadence`**, completed 1601–**1650**: **49 transitions**, with three measured
  negative P/S nudges. The prior nine P16 exclusions at 1624–1632 are now matched.
  Inputs are Right [1601,1611), Left [1611,1621), Up [1621,1631),
  Down [1631,1641), then neutral. **1651+ is deliberately not collision proof**:
  the complete original capture retains a later Right retap only for raw-hash
  reproduction. Its completed-1670 WRAM grid is authenticated and unchanged.

Python and Rust both match **238/238** selected position/stream transitions with
zero excluded frames. Rust additionally checks per-step walking snapshot restore,
actual P/S displacement and passive hook conditions. The raw full CSVs, final
WRAMs and per-frame hook CSVs are authenticated; both fresh copies must agree.
These are not 238 unique terrain situations, nor new native trajectories for
all four directions. All-direction pair coverage is source-table backed and
synthetic; the fresh native special-material examples are the two Up contacts.

## Reproduce and provenance

From the repository root, using existing Rust/C++ oracle prerequisites and private
ROM/SRAM symlinks:

```sh
# Four cases, two fresh processes each; regenerates ignored helpers/evidence.
PYTHONDONTWRITEBYTECODE=1 python3 tools/movement-qualification/verify.py --capture-house-materials
# Authenticate and recheck the captures without booting:
PYTHONDONTWRITEBYTECODE=1 python3 tools/movement-qualification/verify.py --house-materials
cargo test -p room-core --test materials
ROOM_CORE_MATERIAL_FIXTURES=/path/to/local/house-materials \
  cargo test -p room-core --test local_materials -- --nocapture
```

The verifier leaves its default legacy profile unchanged; house-material rules
are explicitly selected and **fail immediately**, never enumerate excluded frames.
It copies/extends `probe.rs` into its own ignored build directory to record
`hooks.csv`; the tracked probe and parent evidence are not modified.
Each case has exactly **one boot per process**, success `exit(0)`, read-only
CPU-register observations and no SRAM writes, injected state, patches or restores.
All CSV/WRAM/traces/register logs/build products stay under ignored
`local/house-materials/`. Both boots' expected capture artifact sets and bytes
are checked. Source-table/gate assertions execute during capture against the
hash-authenticated ROM; standalone verification uses the pinned private captures.

- Starting source revision: `d4a332ef55dc0738faab376100a639e15aa6421d`.
- ROM SHA-256: `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- SRAM SHA-256: `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
- Saved-slot-1 boot: Start [400,408), Up [900,908), Up [950,958),
  A [1100,1112), neutral to required completed **1601 map F `(472,176)`**.
- Raw `wall-Up/frames.csv` SHA-256:
  `12cdc28567f70e47fe5f76036a0658bd5700898a3f3ee5341a142a516e8bbb9b`.
- Raw `cadence/frames.csv` SHA-256:
  `f892cb3dd8c29412ad2d21f43707583121fbfc87c07c9cd16798ccbedb6d39d5`.
- These reproduce the parent's original raw CSVs/final WRAMs exactly. Hook CSVs
  and trace-case CSVs have additional pins in `verify.py`; no raw reference data
  is embedded in tracked files.

## Tests, integration and remaining limits

TDD red: the new all-pair synthetic test initially failed `UnsupportedType(16)`.
Green: four synthetic tests now cover all four directions, 16 remainders, all
O/S aliases and P pairs, every flagged stored type in both new-sample positions,
conservative default-policy rejection, atomic old 6/7 rejection with/without
flags, and nudge-to-alignment continuation at magnitude 2 with snapshot replay.
The separate P-as-S diagnostic mutation fails at completed 1624.

Scoped Clippy with `-D warnings`, the Wasm crate build, the five legacy Python
flat-profile tests, and the 42-case input-admission research/native replay pass.
Full crate tests remain blocked by parent-owned old admission API assertions.
Also update the admission atomic-collision synthetic test's sentinel from now
supported **16** to an actually unsupported type such as **17**; do not weaken
its rollback assertion. This work intentionally does not edit those files.
Independent read-only precommit review of implementation, evidence harness and
report found no blockers. Its small source-assertion/probe-drift hardening points
were addressed and the full two-boot capture command rerun successfully. Shared
private `first`/`second` captures were copied into the parent's ignored
`local/house-materials/`; the native fixture test also passes against that copy.

No action hook, push/interaction/attack behavior, slope 6/7, arbitrary unknown
unflagged material, mode transition, or flagged action-enabled history is
implemented. Passive mode is a documented external precondition, not a silently
ignored flag. Parent integration must enforce that precondition and combine its
collision/input profile version changes before enabling the policy in the slice.
