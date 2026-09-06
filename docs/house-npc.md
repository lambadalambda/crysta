# One frozen, ROM-backed house resident

`assets::sprites::HouseNpc::from_rom(authenticated_rom.image())` supplies **one**
ordinary right-facing resident of adjoining house room `$0010`. The persistent
identity is source spawn `$83:8D7C`, not runtime entity `$1040`; a narrative name
has not been established. This resident is closer to the entrance than the
adjacent resident at `(440,416)` and is visible in the existing `(256,256)` camera.

This is explicitly a **frozen ordinary-pose presentation**. It does not implement
NPC AI, interaction, dialogue, collision, event conditions, animation scheduling,
or a generic sprite/script VM. It is not a claim that the resident remains in
this pose through arbitrary native interactions or later story events. The host
adds it only in room 16 and removes it on returning to room 15. No NPC state needs
to enter the portable player state or snapshots.

## Host contract

```rust,ignore
let npc = assets::sprites::HouseNpc::from_rom(rom.image())?;
assert_eq!(npc.map_id(), 16);
let position = npc.position(); // [424,416], derived from ROM spawn coordinates
let frame = npc.composition();
let bounds = frame.bounds(npc.hflip(), false); // [-8,-33,8,0], half-open
let pixel = frame.sample(npc.graphics(), npc.hflip(), false, relative_x, relative_y)?;
// Transparent: leave destination alone.
// Opaque { palette_index, priority, component }:
//   RGB = npc.palette()[usize::from(palette_index - npc.palette_base())].rgb8()
//   place at position + relative_xy - camera.
```

Palette base is **208**, not Ark's 128. All four components use **OBJ priority 2**.
The shared compositor preserves first-component opaque precedence, alternate
anchors and whole-component flips. Effective pixel Y already accounts for native
**OAM Y+1**: do not add another pixel in the host. `source_composition()` retains
original decompressed bytes (palette 1); `composition()` applies the native
palette-1 → palette-5 relocation but leaves ROM tile indices intact. The later
VRAM tile-slot +256 adjustment must **not** be baked into source sampling.

`pose_key()` is **`(0xD64B77, 0x0168)`**: compressed packet CPU address plus decoded
composition offset. These are two coordinates, **not an addition producing a ROM
address**. The relocated native composition `$7E:7168` is a witness, not an asset
key. `graphics_packet()` is `$BF:96F9`. `facing()` is native 3 (Right), `hflip()` is
false. `source_ranges()` lists exact headerless input extents, including consumed
compressed packets. Caller authenticates the Japanese ROM, as for Ark/backgrounds.
The bounded decoder follows pointers and rejects changed supported shapes rather
than executing the spawn condition or interaction code.

### Scene ordering: bounded source-derived rule

For this ordinary pair, paint back-to-front in increasing **entity world Y**.
At equal Y, **NPC first, Ark last**. Thus a two-entry host list is:

```text
Ark.y < NPC.y: [Ark, NPC]
otherwise:    [NPC, Ark]
```

This is not sorting by sprite bounds/bottom, nor a guessed entity-ID tie-break.
Source `$80:EAE6..EC1C` enumerates the linked list rooted at `$0DFC` through entity
`+$2C`. With `$7F1020+entity = 0`, `+$06 & $7800 = 0`, and admitted screen Y in
`[0,255]`, `$80:EB4E..EB96` computes bucket `2*(255-screen_y)` **before subtracting
the Y anchor**. Buckets are consumed ascending; equal-bucket insertions prepend,
reversing enumeration order. `$80:EC60..EC6D` emits that list forward. Earlier OAM
wins opaque OBJ overlap regardless of OBJ priority (`ares/ppu/object.cpp`).

All four fresh settled samples give this native enumeration:

```text
1240,1100,1080,1040,1280,1200,12C0,1300,1380,1340,1000,11C0,1180
```

NPC is encountered before Ark; Ark therefore wins the equal-Y tie. Both actors
pass the ordinary branch checks. At sampled Ark Y353 and NPC Y416, NPC components
are OAM 0..3 and Ark components 6..13, exactly as the depth buckets predict.
**No actual equal-Y/overlap capture is claimed**: the tie is source-derived from
the witnessed list relationship, with synthetic tie tests. This does not qualify
special ordering flags, changed interaction-time lists, off-domain screen Y,
auxiliary effects, first-sprite rotation or OAM scanline limits. The immutable
host policy keeps the qualified ordinary pair rather than inventing those modes.

Against the portable first background, reuse the qualified house rule from
[ark-sprites.md](ark-sprites.md): actual house BGMODE `$09`, portable first layer
is hardware **BG2**, and **low BG < OBJ2 < opaque high BG**. Background color zero
never occludes. Do not reinterpret it as hardware BG1 or use runtime map-cell
high bits instead of tile-word priority. Whole-scene effects remain unsupported.

## ROM source chain

Addresses below are CPU addresses; normalized packet extents appear separately.
No art, palettes, decompressed packets or capture bytes are distributed.

| Source | Role |
|---|---|
| `$82:8020 = 0`, `$83:8020 → $8D69` | Room 16 actor table fallback, loaded by `$80:F3F1..F42D` with room ID × 2 |
| `$83:8D69..8D7C` | Two-byte prefix, player spawn and two `FA` conditional records; selected fresh ordinary path falls through |
| `$83:8D7C..8D86` | Flags 1; tile coordinates `(26,26)`; parameter 0; header `$88:98A9`; descriptor `$83:ED5A` |
| `$80:F599..F5C3` | Header consumer; position conversion is `(tile_x*16+8, tile_y*16)` in actor loading |
| `$88:98A9..98AE` | Initial animation selector 2, flags `$5100/$0000` |
| `$88:98D3..98E0` | Ordinary interaction loop: clear H-flip (`COP B6`), select animation 2 (`COP 80 02`), resolve/wait (`COP 8E`), loop |
| `$83:ED5A..ED67` | Composition pointer, movement/collision selector bytes, palette operands, graphics transfer operands |
| `$D6:4B77` | Compressed composition/animation packet → `$7E:7000` for this allocation |
| `$80:FA97 → $86:83BE` | Native composition decompression path |
| decoded `+$0004 → $0022` | Animation 2 list pointer |
| decoded `+$0022..0028` | One duration-0, facing-3 record → anchor `+$0164`, then `$FFFF` |
| decoded `+$0164..0191` | Anchor `(8,8,33,0)`, prefix, four components; composition starts `+$0168` |
| `$80:FC75 → $CC:2B0C` | Descriptor palette table selection 1; source offset 2 means +32 bytes |
| `$CC:2B2C..2B4C` | Sixteen colors → staging `$7F:07A0` → CGRAM 208..223 |
| `$80:FE8F..FF0D` | Component palette relocation 1 → 5 |
| `$80:FDA7 → $BF:96F9` | Descriptor graphics index 3 selects compressed 256-tile packet |
| `$80:FE0E..FE5B` | Graphics staging DMA; qualified transfer yields VRAM words `$5000..6000` |

Source components use tiles `$A2,$BF,$AF,$C0`, sizes `16,8,8,16`, palette 1,
priority 2, no component flips. Post-palette words are `$2AA2,$2ABF,$2AAF,$2AC0`;
hardware OAM adds tile offset `$100`, producing `$2BA2,...`, without changing
palette/priority. The graphics packet decodes to 8192 planar bytes (256 tiles).

Normalized compressed ranges are **`$164B77..164D4C` → 858 composition bytes** and
**`$3F96F9..3FADF1` → 8192 graphics bytes**. The production loader is ROM-only;
it does not copy these bytes from native memory or use captured numeric spawn
initialization.

## Qualification and reproduction

```sh
cargo test -p assets --lib --test sprites --test local_sprites
cargo clippy -p assets --all-targets -- -D warnings
python3 -B tools/house-npc-qualification/test_checks.py
python3 -O -B tools/house-npc-qualification/test_checks.py
sh tools/house-npc-qualification/replay.sh 'local/Tenchi Souzou (Japan).sfc'
```

The replay builds local probe/export binaries and starts **two separate fresh
`Session::new` processes with empty SRAM**. Both reuse
`tools/new-game-qualification/bootstrap.rs`, take real Right `[6800,6862)` then
Down `[6900,6967)` input, and release input. They never patch RAM, warp, restore,
or call `save_state` (which is not a passive observation). Read-only
`Session::sprite_state` provides OAM/OBJSEL. Completed 6800 verifies the initial
room-15 checkpoint; 7050/7100/7200/7300 verify the stationary room-16 resident.

Each sample checks ROM-derived position/facing/ordinary script resume, relocated
composition, both anchors, all four hardware OAM component coordinates/attributes/
sizes/order, all **256 source tiles** against VRAM, all **16 natural palette
words** against CGRAM, and **313/313 opaque output pixels**. The output comparison
uses ares's display gamma and its fixed +8 output-row placement, not a production
palette alteration or a searched translation. ROM-only RGBA/indexed/priority
exports are independently recomposed and checked. Native linked-list eligibility
and NPC/Ark OAM order are also checked. Both processes' complete selected raw
surface hashes agree. These are isolated actor/selected-order comparisons, not
whole-scene equality.

Synthetic TDD was red for the missing adapter and then green for pointer-driven
position/pose/palette, relocated resources, exact source extents, transparency,
shared composition and malformed/truncated source rejection. Checker tests were
red before implementation, then green for tampered OAM/size, equal-depth native
list ties and fail-closed exceptions (also under Python optimization). Native RE
used fresh experimental observations instead of synthetic CPU execution tests.
`reference.json` contains metadata and hashes only; `check.py --record` is an
explicit maintainer reference-update operation, never part of normal replay.
Generated raw assets and evidence stay under ignored `local/`.

Final normal replay: `local/house-npc-qualification/replay-nderf3`; its checker also
passes under Python `-O`. Independent correctness/architecture review found no
blockers. A shared-compositor large-tile column-15 wrap edge remains outside this
qualified frame (its large tiles start at `$A2` and `$C0`); broadening sprite
families would require qualifying that case rather than assuming linear wrap.
