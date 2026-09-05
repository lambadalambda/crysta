# Ordinary input reactivation versus dash admission (bounded research)

Research for [repeatable house movement](../meta/issues/qualify-repeatable-house-movement.md).
This supersedes **only the proposed permanent used-directions scope restriction**
in [movement qualification](movement-qualification.md), not its collision limits.
No `room-core`, collider, snapshot format, transition policy, or tracker changes
are made here. Parent owns implementation and terrain qualification.

## Finding and practical port state

**Previously used directions are not permanently forbidden.** The game remembers
only the **most recent activation direction**, not a set of all previously used directions. An ordinary
activation arms an **11-tick window measured from activation, not release**.
A new activation of that same direction while the window is nonzero selects dash.
A different direction replaces the remembered direction and rearms the window.
Neutral does not erase history or rearm the window; a continuous hold does not
rearm it either. Thus:

- `Left → neutral → Left` can dash, but only inside the onset window.
- `Left → Right → Left` is ordinary even inside that window.
- Hold Left for 19 ticks, neutral for one, then Left is ordinary.
- One-frame Left tap, then another onset **10 submitted-input ticks later**:
  dash. Onset **11 ticks later**: ordinary. This is **not** 11 neutral frames.

Smallest proposed replacement guard, in the parent's existing submitted-input
clock (one call per tick), is the pure `Admission` hypothesis in
`tools/movement-qualification/input_admission.py`:

1. Start at the authenticated completed-1601 checkpoint with remembered
   direction=None and remaining=0; previous submitted direction=None.
2. Before examining this submitted input, decrement remaining, saturating at 0.
3. An activation means a non-neutral direction different from the **previous
   submitted input**. On activation:
   - if direction equals remembered direction and remaining>0, reject as
     **unsupported accelerated action**, transactionally;
   - otherwise remember direction and set remaining=11.
4. Save current submitted input, including neutral. Do not reset walking cadence
   merely because this history timer expires.

Additional portable fields: **last activation direction** (None or four
cardinals) and **remaining `0..11`**. Reuse the existing delayed-input field as
previous submitted input, as the current core already does. Keep existing active
direction, cadence phase, position, facing/ownership state. Remove—not retain
alongside these—the permanent used mask. A standalone hypothesis stores
`previous` explicitly only because it is independent of the walking model.
Snapshots must retain the two new fields; a fresh admission epoch at a doorway
is **not qualified by this research**. Do not reset history on arbitrary room
changes, loads, or action recovery on this evidence alone.

This is a bounded **input-clock guard**, not a WRAM scheduler emulator. It matches
the measured regular-frame bootstrap, phase shifts, reversals, and route below.
Exact oracle instruction-stop state instead uses `$096A/$096C` and execution
ordering described next. No extra global video phase is needed by the tested
ordinary guard; event pauses and other controller modes remain outside its claim.

## Actual dispatch and source path

Addresses are hexadecimal, counters/positions decimal. ROM/SRAM hashes and the
starting source revision are recorded below; capture authenticates the ROM/SRAM.
Static interpretation was checked against actual
instruction traces, not a linear disassembly of COP operands as CPU instructions.

### COP `$61` is stream selection, not the double-tap detector

The COP entry `$808378` fetches the opcode, doubles it, and dispatches through
`JMP ($83B2,X)` at `$80838D`. Table entry `$808474` (opcode `$61`) is `$9C03`.
The `trace-cop61` run actually stops at `$809C03` with A=X=`$00C2`,
Y=`$11C0` (controller entity). Its COP caller is `$848EF1` for Left:
selector=2, stream choices=5/3/4, loop target=`$848EB9`.

`$809C03` restores controller X from Y, reads the selector, and checks held
`$0454`: selector 0=Down (`$0400`), 1=Up (`$0800`), 2=Left (`$0200`),
3=Right (`$0100`). For cardinal-only input it chooses operand index Y=1.
`$809C67` temporarily loads **actual player** X from `$0DEA` (`$1000`),
compares the chosen stream index with `$7F2014+X` (actual `$7F3014`), and
retains it if it matches and either X/Y stream pointer is live. Otherwise
`$809C88 → $80BBDB` reloads the streams. `$809C8B` restores controller X and
returns to the COP's loop target. The retained Left trace reaches `$809C8B`
without visiting `$80BBDB`.

Loss of the selected direction branches through `$809C64 → $8083A0`, skipping
the remaining COP operands so controller script can select another direction
or idle. Crucially, the live-walk loop returns **after** the window load;
a held direction does not continually refresh the timer. This is separate from
the existing 54-tick horizontal animation restart/cadence gap.

### History check and timer

Ordinary direction-entry code passes the corresponding held mask to `$84AE12`:
Down `$848D37`, Up `$848DB2`, Right `$848E2D`, Left `$848EA8`.
The helper's exact tests are:

- `$84AE13–18`: `held $0454 & caller direction mask`; zero fails.
- `$84AE1A–1D`: that result intersects **remembered mask `$096C`**; zero fails.
- `$84AE1F–22`: **window `$096A` != 0**; zero fails.
- Success `$84AE24–29`: clear `$096C`, restore caller mask, return carry set.
- Failure `$84AE2A–2F`: restore caller mask, store it in `$096C`, return carry clear.

This is **not a generic joypad rising-edge test using `$0456`**: it is executed
on controller direction-entry. That script structure supplies the activation
semantics. The ordinary failure path loads `$000B` into `$096A` at Down
`$848D42–45`, Up `$848DBD–C0`, Right `$848E38–3B`, Left `$848EB3–B6`.
The regularly scheduled helper at `$8480EE–F4` decrements `$096A` if nonzero.
It saturates at zero and does not clear `$096C` when it expires.

The decisive instruction-stop comparison (both fresh boots) is:

| Trace | Input onsets | At `$84AE22` | Next branch | Result |
|---|---|---|---|---|
| `trace-dash` | Left 1601, 1611 | A=1, Z=0; `$096C=$0200` | `$84AE24` clears history; carry set | `$849144 → $84A464` |
| `trace-walk` | Left 1601, 1612 | A=0, Z=1; `$096C=$0200` | `$84AE2B` stores Left; carry clear | `$848EB3`; A=11 at `$848EB6` |

Do not assert that completed-frame CSV `$096A` decrements by exactly one per row.
The oracle can return on either side of this helper's execution: observed rows
repeat a value or drop by two (e.g. initial Left window 11 at completed 1603,
10 at 1604, 8 at 1605). Input F is set **before** `run_frame`, CSV row F+1 is the
completed frame; direction-entry/setup is completed F+2. The instruction stop
counter reports the *current*, not newly completed, frame. The 10/11 input
boundary was tested at four onset phases 1700, 1701, 1702, 1703 for **all four
cardinals**, after the same selected-floor approach and long idle.

### Accelerated branch: trigger qualified, action deliberately not ported

Carry set redirects Down to `$849051`, Up `$8490A2`, Right `$8490F3`, Left
`$849144`. Each resets selected stream through `$84AE0A` and uses COP `$CB`
with mode operand `$10` to install the actual player's accelerated script:

| Direction | Script entry | Observed setup resume |
|---|---|---|
| Down | `$84A43E` | `$84A44B` |
| Up | `$84A44F` | `$84A45C` |
| Right | `$84A460` | `$84A471` |
| Left | `$84A464` | `$84A471` |

The script also clears the selected-stream index, then selects its accelerated
animation/stream. Observed onset outputs are zero on setup, then signed
**3,2,2**; the short fixtures confirm continuing output after release.
`quick` reproduces Left setup at completed **1607**, output **−3 at 1608**.
The checker verifies this onset signature and cleared history, but **does not
validate accelerated collision, sustained trajectory, termination, or recovery**.
No whole action system is necessary to reject this boundary correctly. If dash
is wanted next, investigate a small mode/phase state plus cancellation and
recovery rules, retaining the helper's consumed-history semantics; do not assume
walking release behavior or extrapolate three samples to a complete dash model.

## Revisit route and measurements

All route steps reuse the **unchanged** `verify.collide(..., flat_only=True)`
selected O={0,2,22}, S={12,14} profile, rejecting flags, unknown types, mixed
pairs, and changed movement-mode flags. No new terrain rule is inferred. This
route stays in map F and does not enter an exit; no doorway ownership assertion
is added. There is no permanent used mask in this checker.

From completed 1601 `(472,176)`, submit these half-open intervals. All gaps are
neutral. Endpoint column is after the last residual delayed-direction step:

| Input interval | Direction | Completed endpoint | Position |
|---|---|---:|---|
| [1601,1634) | Left | 1635 | (424,176) |
| [1634,1646) | Down | 1647 | (424,192) |
| [1646,1658) | Right | 1659 | (440,192) |
| [1658,1670) | Up | 1671 | (440,176) |
| [1670,1682) | Left | 1683 | **(424,176), revisited** |
| [1682,1694) | Right | 1695 | **(440,176), revisited** |
| [1710,1722) | Left | 1723 | **(424,176), revisited** |
| [1735,1747) | Left | 1748 | (408,176) |
| [1760,1793) | Right | 1794 | (456,176) |

Idle through completed 1810. **209/209** ordinary steps match stream output,
mode/map, and strict resolved coordinates. The entire suite has **42 cases,
4,013 ordinary steps** (including duplicated approach/idle prefixes, not 4,013
unique terrain challenges). It includes the quick-retap falsifier, 10/11
boundary, one-frame idle after a long hold, three-tick and one-tick immediate
reversals, the revisit route, three instruction-stop cases, and 32 cardinal ×
onset-phase × boundary cases. After predicted dash setup, only onset/mode
observations are checked; those frames are not counted as ordinary steps.

## Reproduce, provenance, red/green

Run from repository root with the project's existing Rust/C++ oracle toolchain:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/movement-qualification/input-admission-check.py --synthetic
PYTHONDONTWRITEBYTECODE=1 python3 tools/movement-qualification/input-admission-check.py --capture
# Recheck already captured files without booting:
PYTHONDONTWRITEBYTECODE=1 python3 tools/movement-qualification/input-admission-check.py
```

`--capture` builds only `local/input-admission/probe`, then runs each case in
**two separate fresh processes**. There is exactly one boot/session per process,
`std::process::exit(0)` on success, no destructor/reboot reuse, SRAM write,
patch, state restore, register mutation, or injected state. The ROM/SRAM are
read-only inputs through the pre-existing local symlinks; their hashes are
checked before and after capture. CPU registers are read-only observations.
All CSV, WRAM, traces, stop registers, and build products stay ignored beneath
`local/input-admission/`. The static decoder used during research was an own
copy of the parent's ignored helper, not a parent-workspace edit.

- Starting source revision: `cff410ed222715d9ad75a345411c5a424502ecc9`.
- ROM SHA-256: `f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
- SRAM SHA-256: `709c1cb67b8aff8db49cba05959f128b1c0a1ca32184c9bb62c415d537658055`.
- Saved-slot-1 bootstrap: Start [400,408), Up [900,908), Up [950,958),
  A [1100,1112), neutral to completed **1601, map F, `(472,176)`**; probe asserts it.
- All **42 raw CSV hashes** are pinned in checker `CSV_SHA256`, not just hashes
  of a filtered projection. In addition, both boots' CSV, WRAM, instruction
  traces and register logs compared byte-for-byte. The initial foreground
  capture was interrupted by the tool timeout; remaining cases were run in new
  processes (no emulator state resumed), with all 42 complete pairs compared.
- Key raw CSV SHA-256 values:
  - `route`: `75bd1b5a63366d45e938cc03e0c562116bd22c00bba0b922116c10f127827bdd`
  - `boundary-dash`: `db4bf3741449c53f7a5cdbec54b71948cd2a4cff3d3f02b4c4931d10eaba148d`
  - `boundary-walk`: `3a3f79a02a29f659700ba229894bd384a2db85ccee37d32c29537bfe4da5080b`

Six ROM-free tests were written first (red: missing hypothesis module), then
passed with the pure guard. Additional ignored mutation runs failed as expected
for unconditional walk admission, release-based timer reset, and retaining the
first direction instead of replacing it. The tests explicitly distinguish the wrong hypotheses
“all retaps walk,” “cooldown starts on release,” and “history is per direction”
from the measured rule, and cover the inclusive/exclusive boundary, long idle,
actions/diagonals, and transactional rejection. The native corpus independently
falsifies applying ordinary cadence to dash and validates accepted reactivations.
This is research evidence, **not native/Wasm production implementation coverage**.
Independent read-only precommit review of the harness, sampled private evidence,
and report found no blockers. Its artifact-completeness finding was addressed
with an expected filename set for both boots; minor report wording was corrected.

## Remaining unsupported behavior

Reject diagonals, opposing simultaneous cardinals, and all non-direction actions
(attack, jump, interaction, menus, etc.) under this profile. Dash onset is now
recognized precisely for the bounded ordinary histories, but dash movement,
braking, direction change while dashing, stop/recovery, and chained actions remain
unsupported. Other player/controller modes, actors/events, pause histories,
transitions and post-transition history, arbitrary loaded snapshots, room/map
boundaries, mixed corners, unknown/flagged terrain and other materials are not
newly admitted. The parent owns those separate qualifications. No claim that
an input history alone can make an unknown collision route supported.
