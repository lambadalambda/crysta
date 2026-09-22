# Map A animation qualification

Bounded source-derived free-roam animation; no captured production assets. The
existing static API/output is unchanged. The host owns its visit clock, backdrop,
and dirty-pixel updates. Caller authenticates the headerless Japanese ROM.

## Source contract

Map `$000A` selects scene `$83:89A7` through `$83:8014` (fallback `$82:8014` is
zero). This decoder qualifies the four fixed free-roam service instructions,
not every event-conditioned branch of that scene.

| Service | Selector / handler | Table | Transfers / hold | Period |
|---|---|---|---|---|
| `$83:8A73` | graphics 0 / `$87:98E8` | `$9B:807E` | 12 records, each 8 frames; delays 7,6,5,4,3,2,2,3,4,5,6,7 | 528 |
| `$83:8A78` | graphics 1 / `$87:98E8` | `$9B:84DF` | 64 single-frame records, delay 0 | 64 |
| `$83:8A7D` | palette `$23` / `$87:98BF` | `$9A:97A1` | 6 single-frame records, delay 7 | 48 |
| `$83:8A82` | palette `$06` / `$87:98BF` | `$9A:8788` | 7 single-frame records, delay 5 | 42 |

- Graphics tables are relative to `$9B:8000`. `$8D:940D` loads eight-byte
  records: count, source offset, VRAM word destination, byte size, delay.
  `$8D:9470` queues a transfer and advances its source by size.
- Palette pointers are absolute within `$9A`. `$8D:9331` loads six-byte
  records: count, source pointer, color destination, MVN count, delay.
  `$8D:93B2` copies **count + 1 bytes**, advancing source. Special alternate
  palette destinations used by other shapes are not admitted.
- `$80:A404` (COP95) / `$80:A3B5` (COP93) decrement repetitions, reload delay,
  advance records and return through the service loop at the zero sentinel.
  There is no extra sentinel tick. `$80:AAB3` (COPBD) saves the resume point;
  `$80:C70D/C72C` decrement the actor timer before testing negative: **hold d+1**.
- Selector 0 writes bytes `$0120..019F` (tiles 9..12). Selector 1 writes one
  128-byte span at `$3E00`, `$3E80`, `$3F00`, `$3F80` in order. Older destinations
  survive each partial transfer and wrap; age 0..2 still has uninitialized spans.
- Palette `$23` writes colors 96..111; `$06` writes 112..119. **Color 0 is not
  animated.** All payloads come from ROM; bank crossings and changed shapes fail.

The scheduler period LCM is 14784; steady states repeat from age 3. Age zero is
first service transfer, **not an asserted native map-entry/video-frame phase**.
No pauses, DMA queue saturation, brightness/fades, windows, color math, secondary-BG1
composition, alternate scene branches or native frame lock are implemented.

## Host integration

`CrystaAnimation::from_rom`, `apply(age, tiles, palette)`, `tile_updates(age)`,
`palette_updates(age)`, `affected_tiles`, `affected_colors`, `periods`, `period`,
`repeat_from` are immutable/bounded. Updates return allocation-free iterators of
`(first_destination_index, decoded_values)`. Sampling does not depend on whether
intermediate frames were rendered. For seeks back into startup ages 0..2, reset
to static base first; all ages >=3 fully define affected spans.

`phase_key(age)` has seven `Option<u64>` slots identifying latest source
transfers: tiles starting at **9,496,500,504,508**, then colors **96,112**. Equal
keys guarantee equal patches within the same decoded instance. Different keys
can still contain equal bytes. `None` denotes no startup transfer yet. Compare
only keys for destinations used by the host's pixel dependency mask.

In map A's first logical background (hardware BG2), source word occurrences are: graphics0 **8**, graphics1
**0**, palette bank6 **0**, palette bank7 **1286**. Graphics0's two 16x16 patches
start at `(496,640)` and `(768,640)`. The river uses palette `$06`, so **key slot
6**, six ticks per transfer, **42-tick period**, is the useful river invalidator.
Ages 1 and 4 intentionally agree; ages 11 and 32 differ. Do not fabricate extra
motion between native source transfers.

## Verification and results

TDD: synthetic tests initially failed to compile without the decoder; then green.
The phase-key test likewise ran red before its implementation. Five pure tests
cover d+1 holds, count repetition, partial-write retention, wrap, joint cycles,
u64-max seek, startup keys, unchanged outside spans, atomic short-buffer refusal,
changed source services/pointers/shapes, truncation and bank bounds.

```sh
cargo test -p assets
cargo clippy -p assets --tests -- -D warnings
CRYSTA_ANIMATION_ROM='/path/to/Tenchi Souzou (Japan).sfc' \
CRYSTA_ANIMATION_CAPTURES='/path/to/retained/journey' \
  cargo test -p assets owned_rom_river_cycle --lib -- --ignored --nocapture
```

All passed. The opt-in test authenticates ROM via `rom::Rom::load` and uses the
existing `local/pandora-tower-discovery/frozen-reference/journey` captures. It
reads each label's `.wram`, `.vram`, `.cgram`; it never saves/reuses native assets
as production inputs.

- River probe **x672, y352..599**: age1→11 and age1→32 each change **248/248 RGB
  pixels** while stationary; age1→4 changes zero. All 42 river phases repeat
  exactly at age+42. Selected full-source states repeat at age+14784, including
  an age ending at u64::MAX.
- Each retained snapshot matches a **complete joint source state**, including
  **all 768 BG graphics tiles** (base + partial writes) and **all 24 animated
  colors**. Matching age residues modulo 14784:

| Snapshot | Matching residues |
|---|---|
| town-west-rest | 497, 4529, 7089, 11121 |
| town-north-rest | 857, 4761, 8921 |
| town-gap-rest | 1049, 11673 |
| town-gap-up-rest | 1174, 2518, 3862, 9238, 14614 |
| town-door-align-rest | 202, 1290, 3978, 9610 |

These are membership witnesses, **not a fitted capture clock** or proof of
native video-frame synchronization. No whole-PPU RGB equality is claimed.
