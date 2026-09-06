# Bounded Pandora navigation qualification

Owned issue: [qualify navigation/contact](../meta/issues/qualify-pandora-navigation.md).
This additive contract does not enable movement in the old house profiles. ROM
assets are inputs; neither a captured grid nor the observer is an initializer.
Scope is the direct A → 13 → A → D → C/pots → E → 20 → 21 route,
forced first-time tour, and final41 control. It is not a town simulation or VM.

## Source discovery (before implementation tests)

The headerless JP ROM is authenticated by SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
The existing background decoder supplies full attributed sheets, not admission.
Source words remain `tile | ((attribute[tile] & 127) << 9)` including bit15.

### Actual samples and materials

Ark's collision origin is `(x-8,y-16)`. Left/Up sample that origin;
Right samples `(x+7,y-16)`, Down `(x-8,y-1)`. The second sample is the
next perpendicular cell boundary only when the first is not aligned. Validate
both old and tentative edges; do not introduce bounding-circle probes. Old-edge
stored slopes6/7 are tested before the new-edge occupancy override. Under the
bounded passive policy `$0980 & $0050 == 0`, a bit15 sample is solid. Carrying
`$0020` is separately admitted by the pot component, not ordinary action walking.

Four first/O/P/S tables per direction start at `$80D542/$80D8E8/$80DC60/$80DFDC`,
with subtable strides64. All sixteen give type25 **the identical handler to12**,
and type5 the identical handler to16. The route needs town25 as solid and C's
closed lower door type5 as partial. The latter is confined to C `(11,21)` Up.
Type29 equals open0 in fifteen tables, **not** Right S-first `$80E09C`. Only Up
at C `(11,21)`, E `(6,53)` and20 `(22,53)` is stair admission. No global29 alias.
Other unknown types and samples outside the bounded profile fail atomically.

Source-frozen town occupancy is a portable policy, not native wandering timing.
The six scene instances `$8389BF/$8389C9/$838A05/$838A19/$838A23/$838A2D` use
geometry bytes `F8 10 F0 10` from their decoded source frames, interpreted by
`$86BAEF..BB49` as `(-8,16,-16,16)`. COP3B `$80BE8E` uses the small-footprint
stamp `(actor.x-8,actor.y-16)`, **not inclusive rectangle intersection**.
Birds freeze at source spawns; their native wandering occupancy is not production
initialization. In particular the recorded return at Y401 can sample a frozen
bird's row25. Y400 is a **candidate** horizontal corridor (X356..552), pending
continuous sample qualification; it avoids that row but is not by itself a
reachability proof. Report a blocker rather than erase that bird. The native
frame recipe is evidence, not an exact portable input/pacing recipe.

Direct C residents retain source-relocated `(152,368),(56,384),(184,416),(216,368)`.
The pot carry route must go around them. The departed phase is distinct from the
direct phase. D's source-origin wanderer remains frozen; `$26` removes only the
separate hidden exterior gate. E/20 have no admitted resident stamps.

### Map13 exact interaction

Spawn `$838EBE` gives `(360,128)`; `$88B63B COP21` installs callback `$88B653`.
Descriptor `$83ECC6` points to compressed packet `$D65EA2`; list0 geometry offset
`$72` and list2 offset `$120` both contain `F8 10 F0 10` (anchor+4).
Thus the actor interaction bounds are inclusive X352..368/Y112..128.
`$87C783` walks root `$0DFA`, links `+$2E`, eligible `+$04 & $0100`. It tries
**far16 then near8 within each actor**, and stops at the first geometric hit.
`$8793B9` performs callback policy afterward; failure does not resume the search.
At `(360,144)` Up, far `(360,128)` hits; at Y145, far129 and near137 both miss.
This does not authorize remote actors or later-story callback branches.

### Exits and transfer ownership

Retain complete ordered 12-byte source exit lists, including unsupported targets.
Selection is first coarse match, then that record's fine test; failed fine must
not fall through. Use the existing source exit decoder, not destination lookup.
The required outgoing records are A `$818D6B/$818D8F`,13 `$818EAC`,
D `$818DFE/$818E0A`, C `$818DF1`, E `$818E2F`,20 `$818FC1`.
These are source-list addresses, not destination lookup keys.

Ordinary doors retain the existing **17 departure / load / 17 arrival** portable
policy. Selector14 (decimal) is not a duration. `$8D89BD` supplies signed initial
adjustment `(-14,-23)`, and `$8D8982` selects departure `$84BAFB`. Its adjusted
loaded anchors precede the settled destination by `(14,23)`:

| Source exit | Raw destination | Settled destination |
| --- | --- | --- |
| C `$818DF1`, `(11,21,1,1)` | E `(144,864)` | `(152,880)` |
| E `$818E2F`, `(6,53,1,1)` |20 `(400,864)` | `(408,880)` |
|20 `$818FC1`, `(22,53,1,1)` |21 `(128,112)` | `(136,128)` |

Keep stairs a separate forced-ownership transfer; intermediate map-ID writes
are not settled arrivals. Finite portable interpolation/durations are explicitly
pacing policy, not inferred native constants from capture labels. Discard input
while forced and begin a fresh walking-admission epoch on actual release.
Box reload/tour retain forced ownership even when the player script looks idle.
The final release is the last request return and `$244`, not reaching map41.

### Box opening is a polling predicate, not an input edge

`$88AD35 COP0D FF FF FF 01 01` uses `$8087C2` and signed-byte-times16
`$80BC2F`. At box origin `(136,384)` its **inclusive** spatial gate is
X120..152/Y360..392, any facing. `$88AD2F` requires local1; `$88AD3E COP09`
requires local1 AND local2. There is no Down, A or second-callback condition.
Neutral after warning stays closed because the witnessed Y359 is outside this
rectangle. If both flags are set inside it, the polling script can proceed.

The warning controller `$88ADA5` tests local1 XOR local2. It masks input, waits
COPC1 `$20`, requests `$88ADF2`, waits for return, waits COPC1 `$20` again,
clears mask `$FF50` at `$88ADC1`, then sets local2 at `$88ADC5`.
COPC1 `$80AB17` stores **32 actor-delay units**, not a proved rendered-frame count.
After proximity/flags pass, `$88AD46` masks input, `$88AD4A COPDF $888EA6` must
succeed before `$88AD4F` grants22 and `$88AD53` reloads21. COPDF `$80B827`
yields/retries while `$097C & $0810 != 0`; do not grant/reload ahead of it.

First-contact callback registration is `$88AD03 STA $7F1010,X`, distinct from
COP21. Callback `$88AD69` sets local1, clears actor flag0200, stamps via COP3B,
then returns to the animation/proximity loop. Its first-contact geometry must
be decoded through `$85F835`; the sprite's ordinary rectangle is not a substitute.

### Shared-sheet lifecycle

Layer handler `$868AAD..AB8` calls `$869145` using cache `$043F`. The helper
compares the incoming pointer's low word and bank at `$869148..9156`: equal
returns carry-clear, clearing the first-layer load bit; unequal stores the new
pointer and returns carry-set. `$868AD0..AD7` returns when no layer bits remain.
Consequently B/C/D/E/20 sharing `$AFCBB3` retain live **tile/attribute patches**,
including consumed FA/FB cells and opened doors. A/13/21 replace that source.
A→D therefore rebuilds the house base, not the old opened C→B door. A→13→A
likewise reconstructs A's closed wooden doors; no persistent door-open event is
invented. Room-local reset `$8D8735 → $8D8AED` clears `$06C0/$06C2/$0640` via X=0;
this does not clear global `$292`. Occupancy is rebuilt for the new scene rather
than carrying the old scene's actors across the shared resource cache.

Native `replay-WKFf0d/a/journey/door-passable.wram`, first layer `$A000`,
retains the departing C actor's final `(7,31)` stamp (`$1CE8 → $9CE8`);
settled E/20 do not. The five persistent E/20 grid deltas are precisely the two
opened C door cells and three removed pots. Box21 has no initial stamp, then
contact stamps `(8,23)`, and opening reload removes it. Captures validate these
facts; they do not supply the compiler's cells or actor positions.

## Evidence and limits

Read-only evidence roots: original `ilar-task-pandora-source/local/pandora-qualification/{journey,discovery}`
and repaired-observer twins `terranigma/local/oracle-video-qualification/replay-WKFf0d/{a,b}/journey`,
with adjacent JSONL logs. No new emulator session or capture is needed here.
No source pixels, extracted grids or frame logs belong in Git.

Native contact operand completion, tested compiler/sample checker and parent
reproduction remain pending. Keep the issue open. Source discovery necessarily
preceded red → green implementation; source-only transitions, frozen occupancy,
carry timing, reaction/tour pacing and frame-perfect movement are not silently
claimed as native trajectory equivalence.
