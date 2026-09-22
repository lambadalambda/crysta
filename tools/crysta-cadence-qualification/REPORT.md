# Bounded map-D wanderer cadence investigation

**Result:** this ordinary class-0 resident moves **16 pixels in 32 native frames**, using integer deltas **1, 0, 1, 0, …** on the selected axis (negated for up/left). It is **0.5 pixel/native frame**, not 2 pixels/frame. The class-0 idle/refusal choice lasts **16 frames**, not 8. A display duration byte `d` lasts **d+1 frames**; a zero-duration display record lasts **one frame**, not infinitely.

No production/runtime/tracker changes. Only reusable tooling and source annotations are retained here; raw evidence remains private under ignored `local/`. See [README.md](README.md) for build, verification and replay commands.

## Identity and source chain

- Map `$000D`, spawn record `$83:8CB4`: `01 04 2A 00 37 A8 88 EB ED 83`.
- Header `$88:A837`, executable script `$88:A83C`, descriptor `$83:EDEB`, initial position `(72,672)`.
- Slot `$1040` in the native replay; `$7F:2018+slot = 0` (class 0). Post-setup script remains `$88:A86E`, the COP8F site following COP26 at `$88:A868` with rectangle operands `04 0A 29 29`.
- Descriptor begins `22 10 D8 20`: composition packet `$D8:1022`, mode `$20`, no private movement pointer. Common movement table is used.
- Descriptor packet: normalized ROM `$181022..18118A`, decoded size `$028B`. Ordinary display selectors 3/4/5 point to decoded offsets `$0026/$0038/$004A`; each list has four duration-7 records and then `$FFFF`. Idle selectors 0/1/2 at `$0014/$001A/$0020` each have one duration-0 record then `$FFFF`.

### Common velocity initializer — now source-bound

The table is **already absolute-addressed in the compressed ROM resource**. No relocation is necessary on this common-resource path; do not confuse it with private movement relocation at `$80:FB5A`.

1. Source load instructions `$98:817D` and `$98:8272` are `01 01 02 00 37 F0 09`.
2. Opcode `$01` dispatch: `$86:872D -> $86:8FAA`; nonzero submode takes `$86:8FD1`.
3. Packed `37 F0 09`, script base bank `$98`, resolves through `$86:90E7` to **`$AB:F037`**, normalized **`$2BF037`**.
4. Destination operand `$02` is shifted left four, XBA, then added to `$4000` at `$86:8FE7..8FF7`, producing **`$6000`**. `$86:8FF9..8FFC` sets destination bank `$7F`; `$86:8FFE` calls decompressor `$86:83BE`. Cache helper `$86:9145` can skip a repeated source.
5. The decoder links the production `assets::compression` API and decodes **`$2BF037..2BF8F7` to `$1A0C` bytes**. **All 6668 bytes exactly equal native `$7F:6000..7A0C`**, not merely three coincidentally matching streams. The native cached source at WRAM `$044B..044D` is also `37 F0 AB`.

The initializer addresses were source-decoded; no separate native breakpoint trace of initialization is claimed. The cached pointer and complete decoded-resource/native comparison independently bind this source to the observed resident table.

### Ordinary table and stream pointers

`$80:8F32` indexes `$80:8F6D` using `(class & $FFFC)*2 + direction*2`. For class 0 its four `(pose,movement)` byte pairs are `(03,68), (04,69), (05,60), (05,60)`. Horizontal left sets flip bit `$4000` in entity `+$08` at `$80:8EB0`; the X stepper negates the stream value for that bit.

`$80:8F5D` selects `$7F:6000 + movement*4`; `$80:BBE4` initializes current/base X/Y stream pointers at `$7F:0010/12/14/16+slot`, and zeros entity `+$28/2A` stream counters. `$80:8F65` sets entity `+$22` to **1 list repetition**, not one display record or a one-tick step.

| Movement selector | Decoded table offset | X pointer | Y pointer | Actual duration/value records | Loop target |
|---|---:|---:|---:|---|---:|
| `$60` | `$0180` | `$6D18` | 0 | `$6D1A`: `(0,+1)`, `$6D1E`: `(0,0)` | `$6D1A` |
| `$68` | `$01A0` | 0 | `$6FC8` | `$6FCA`: `(0,+1)`, `$6FCE`: `(0,0)` | `$6FCA` |
| `$69` | `$01A4` | 0 | `$6FD4` | `$6FD6`: `(0,-1)`, `$6FDA`: `(0,0)` | `$6FD6` |

Pointers in this table are **two bytes before the first duration word**. E.g. `$6FC8` is not itself a duration/value record. Native counter starts at zero, decrements to negative, advances pointer two bytes, then loads duration and advances another two to the value. This explains an otherwise misleading leading word belonging to the preceding stream's loop pointer.

- `$80:F251..F312`: counter pre-decrement; on underflow, fetch next duration; negative duration follows the next word as an absolute loop target. A negative **value** is signed motion, not a sentinel. Each duration-0 record applies once.
- X flip at `$80:F297..F2A7`, Y flip at `$80:F2F3..F303`; signed integer values go into last-delta fields `$7F:000C/0E+slot`, and accumulate in `$7F:0018/1A+slot`.
- Actual actor update `$80:C75D` calls the stream stepper, then `$80:C760` calls `$D0CF`. This resident's entity flags are `$1300` (bit `$0004` clear), so **`$80:D0D7..D0EA` directly adds those accumulated integer deltas to entity X/Y**, then `$80:D0ED..D0F4` clears them. There is no hidden fixed-point scaling on this path.

## Native every-frame witness

`probe.rs` is adapted from `tools/house-scene-qualification/probe.rs` and directly references the shared `tools/new-game-qualification/bootstrap.rs`. One fresh empty-SRAM Session, real buttons only, accepted itinerary through `settledD`, then **400 neutral native frames**, sampling only slot `$1040` and its auxiliary state every frame. No save/load, memory writes, warp, debugger interventions, or fabricated state. Accessors are passive. The original investigation compared `settledD` byte-for-byte with its parent baseline. The retained verifier does not require that separate baseline; fresh replay stdout and settledD WRAM were independently compared with the original investigation capture during retention.

The private probe stdout JSONL has 401 post-frame snapshots, completed native frames **8175..8575**. `verify.py` asserts exact deltas against ROM-decoded streams, list/counter timing and stable map/class.

### First complete down step

- Frame **8194** (before step): `(72,672)`, idle selector 1, display countdown 0, next-record index 1, repeat count 1, no velocity pointers.
- Frames **8195..8226 inclusive**: pose selector 3, repeat count **1 throughout**, 32 movement applications. Y deltas are **`(+1,0) * 16`**; X deltas are all zero. Final position **`(72,688)`**.
- First record frames **8195..8202**: display countdown **7,6,5,4,3,2,1,0**, next-record index 1.
- Second record **8203..8210**, third **8211..8218**, fourth **8219..8226** have the same countdown, next-record indices 2/3/4 respectively.
- Active Y pointer alternates **`$6FCC/$6FD0`**, last applied Y value alternates **1/0**, entity stream-duration counter `+$2A` is zero every frame (each duration is zero). Accumulators are zero after position application, not unapplied/subpixel residues.
- Frame **8227**: movement list exhausted, next idle action already selected; pointers cleared, selector 0, repeat count 16. No extra movement or gap frame is appended to the 32-frame step.

Same capture also independently verifies **8371..8402: `(72,688)->(72,672)`**, deltas `(0,-1),(0,0)` repeated 16 times, and **8403..8434: `(72,672)->(88,672)`**, deltas `(1,0),(0,0)` repeated 16 times. Thus consecutive movement lists can join with no idle tick between them. A later down list at 8467..8498 reproduces the first result but is not needed by the assertion scope.

### Waits and zero-duration records

`$80:8F85` selects idle pose/count using `class & 3`. Class 0 table `$80:8FB5` is `(00,10),(01,10),(02,10),(02,10)` in hex: **16 repetitions** of a single idle display record. COP26's random-idle and refused-movement paths both go through this helper, then clear current/base velocities at `$80:8F10..8F1F`.

Native **8227..8242 inclusive** stays `(72,688)`, selector 0, display duration 0 and next-record index 1. Repeat count is exactly **16,15,…,1**, one decrement each frame. Frame **8243** selects another idle/refusal and reloads 16. This directly contradicts “zero-duration record waits forever”; its apparent held raster is a repeated single-record list.

`$80:C72C` pre-decrements display counter and skips script dispatch while nonnegative. `$80:EDA0` loads the raw duration byte into that counter; `$80:EDFB` increments next-record index. `$80:A33F` (COP8F) calls `$80:ED75`; exhaustion returns carry via `$80:ED51..ED74`, decrements `+$22`, and either restarts the list immediately or resumes the script. Therefore raw **7 means eight frames**, and raw **0 means one frame**, including native-visible list/repeat progression, not only raster selection.

This establishes COP26 class-0 idle/refusal, **not every VM wait opcode**. There is no claim that arbitrary script delays/COPC1 should be globally changed by this factor.
