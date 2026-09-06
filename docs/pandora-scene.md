# Bounded Pandora actor presentation

[Owned issue](../meta/issues/decode-pandora-scene-art.md). This is an **additive
source-art/finite-presentation compiler**, not a renderer, NPC scheduler, event
interpreter or a claim of whole-scene RGB equality. It preserves `ArkSprites`,
`HouseScenes` and `HouseNpc` unchanged.

The caller authenticates the headerless Japanese image (SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`).
The export tool performs that authentication itself. Production data comes only
from ROM pointers, compressed packets, source frame lists, spawn records and
position operands. No capture supplies a raster, palette, resource pointer or
production position initializer.

## What is available

`assets::sprites::PandoraSprites::from_rom(image)` returns **16 immutable art
resources, 241 source list records (482 normal/mirrored raster exports), 33
finite phase choices and nine C departure segments**. Repeated records remain
repeated: this is not an atlas inferred from whichever frames were captured.

| Art / instance scope | Source origin and selection |
|---|---|
| Wider A corridor | Six source instances `$8389BF/$8389C9/$838A05/$838A19/$838A23/$838A2D`; descriptors `$83EBE4/$83EC7E/$83ED37`. Northern pair, northern resident and corridor birds. The `town-source` phase is explicitly **spawn-frozen**, not the native wandering schedule. |
| 13 resident | `$838EBE`, descriptor `$83ECC6`, `(360,128)`, ordinary list 2 and request-facing list 0. |
| Changed direct C | `$838C0A/$838C14/$838C1E/$838C28`, descriptors `$83ED5A/$83ED74/$83EDF8`, source relocation COPs `$88A387/$889AD0/$88A20A`. Direct control positions are `(152,368), (56,384), (184,416), (216,368)`. |
| E / 20 | Explicit empty resident phases. Shared resident programs reject these maps; retained settled native lists independently have no visible compressed resident. Backgrounds, stair art and scene services are not empty merely because this roster is empty. |
| 21 contact box | `$83928F`, `$83F984`, `(136,384)`, **all of selector 3**, not a frozen guessed frame. |
| 21 opening guide | `$83927B`, `$83F8C0`; lists 3/4 are finite parent-selected cues. Lists 1/11 are also retained source-only for the opening transition. |
| Forced tour guide | `$839530/$839572/$8395C0/$8395FD`, `$83F8A8`, list 3. COPED `$89D2F4..D390` yields the seven source-relative movements in map41. COPBA `$89D2C1` supplies the separate OBJ priority override 3. |
| Tour room object | Map42 `$83957C`, header/art ID `$89D9F9`; source COPB2 changes Y by -8 to `(72,360)`, COPD8 chooses `$A2C000`, list 8. Uses the already-qualified common object sheet/palette (the same source load used by F's prop), not the guide's descriptor-reuse raster. |
| Ark lift/carry/throw | Six-byte resource entries `$80A24F/$80A255/$80A261`; direct source lists and planar graphics; palette from bank-first COP5A `$80F941`. |
| FA / FB pots | C's source held records `$96E1A6/$96E1AB`, reached through the map table `$96DDBD`; separate source art with shared holding/throw/flight compositions. |

The FD objects at the tour rooms' corners (`$89DC78/$89DC9D/$89DCC2/$89DCE7`)
are hidden room-transition controllers, not missing statue sprites. Their
visible room imagery belongs to the shared background owner. Only map42's
source-switched animated object needs this additional OBJ art.

This is not all-town NPC admission. A's eastern workers and remote performers
are outside this corridor profile. Map13's additional non-interacting dynamic
object at `(424,128)` is **not admitted by this resident-only phase**; the remote
copy at `(680,128)` is likewise outside this art contract. A consumer that shows
those areas/objects must qualify them rather than treating this as a complete
native scene. `town-source` birds deliberately use source spawn positions,
whereas the native art witness may find a bird at a different wandering position.

## Consumer contract

```rust,ignore
use assets::sprites::{PandoraSprites, SpritePixel};

let sprites = PandoraSprites::from_rom(rom.image())?;
let phase = sprites.phase("c-direct").expect("admitted parent phase");
for actor in phase.actors() {
    let art = sprites.get(actor.art_id).expect("validated art reference");
    let list = art.list(actor.selector).expect("validated list reference");
    // Parent owns the finite record choice/clock. Do not silently clamp bad indices.
    let frame = &list.frames()[record_index];
    let facing = list.effective_facing(record_index, actor.hflip);
    let composition = frame.composition();
    // x/y are signed coordinates relative to actor.position. No additional OAM Y+1.
    if let SpritePixel::Opaque { palette_index, priority, .. } =
        composition.sample(art.graphics(), actor.hflip, false, x, y)?
    {
        let rgb = art.palette()[usize::from(palette_index - art.palette_base())].rgb8();
        let priority = actor.priority_override.unwrap_or(priority);
        // Parent compositor retains priority/alpha and applies its BG/effect policy.
    }
}
```

- Art, lists, tiles and palettes are immutable behind shared/slice accessors.
  `HouseFrame` is reused as a record container; in this API its `facing()` is
  the **unmirrored** source byte. `effective_facing` applies the phase's independent
  H-flip. Legacy house frames retain their old, already-adjusted behavior.
- `source_composition()` retains exact original bytes; `composition()` changes
  only the palette field. Source tile indexing is never native dynamic VRAM
  slot indexing. `HousePoseKey::Compressed` offsets belong to decompressed
  packets, not ROM address arithmetic.
- Positions and departure endpoints are read/calculated from source operands.
  Phase membership, selected script boundaries, mirrors and ordinary tie ranks
  are **bounded RE metadata**, not an interpreter evaluating story bits.
- `tie_rank` is an ordinary equal-world-Y painter rank; larger ranks paint later.
  `ark_tie_rank()` follows the roster. Priority override is separate. Do not
  sort actors by resource key, runtime slot or source component priority.
- Phase art references and motion actor/list references are validated at
  construction. Unknown resource/list/phase/facing lookups do not fall back.
- `source_ranges()` records consumed sprite/resource/position dependencies,
  including the relevant common palette/definition loading operands. Ranges
  overlap and repeat. The committed metadata digest binds the complete export;
  reading a range alone is not authentication without the ROM/hash check.

### C request and departure boundaries

The parent owns flags, input locks, request acknowledgements and phase completion
as specified by [Pandora progression](pandora-progression.md). It should not
advance these choices on a guessed text page count or merely on two pots consumed.

- `c-entry` / `c-choice` / `c-direct` retain the direct-story roster. The choice
  approach X=120 comes from COP39 `$889B01`; the source Y stays 416.
- `c-first-hit` retains all four. `c-second-hit` is the onset choice; the fourth
  resident's short source 0/2/1/2 pose cues are separately available.
- The **fourth resident leaves first**, via `$88A56D/$88A578/$88A583` to
  `(56,448) → (120,448) → (120,512)`. It is already absent at the retained
  `cellar-sequence-000` boundary. It must not remain frozen through the reaction.
- `c-color-math`, `c-reaction-speaker`, `c-reaction-right` have three residents.
  The right resident leaves via `$88A247/$88A256`; `c-reaction-left` has two.
  The left leaves via `$88A3DF/$88A3EA`; `c-reaction-final` has only the speaker.
- The speaker leaves via `$889C16/$889C25`; **`c-departed` is empty**.
  `motions()` exposes all nine segments' source IDs, from/to endpoints, list
  selectors, raw motion operands and final removal markers. These are not an
  elapsed-time scheduler; parent/source-profile code owns interpolation and
  interruption/completion boundaries.

The tour is likewise forced parent presentation: map41 source endpoints are
`(136,96) → (136,120) → (64,120) → (88,152) → (136,152) → (184,152) →
(208,120) → (136,96)`, followed by **44 → 42 → 43 → 41**. `tour-control` is
not selected until the final request returns and the parent grants `$244`.

## Carry contract for core/parent communication

`PandoraSprites::carry_pose(motion, facing)` is pure selection, with native-facing
bytes Down=0, Up=1, Left=2, Right=3. It returns Ark resource/list, pot list and
independent mirrors. Invalid facing returns `None`.

| Choice | Ark resource / selectors Down, Up, horizontal | Pot selectors Down, Up, horizontal |
|---|---|---|
| `Lifting` | resource3 (`$80A261`), **24,25,26**; source `$84BE7C/BE88/BE98` | **43,44,45** |
| `Standing` | resource0 (`$80A24F`), **3,4,5**; `$84B4C8/B4DA/B4F7` | **25,26,26** |
| `Walking` | resource1 (`$80A255`), **9,10,11**; `$84B509/B51A/B52F` | **29,29,30** |
| `Throwing` | resource3, **15,16,17**; `$84B540/B553/B56A` | **46,47,48** |
| Free flight | Ark leaves the throw choice under parent control | **60 (`$3C`)**, source assignment `$84C701` |

Left mirrors the horizontal lists. Full source list records and duration bytes
are retained, including repeats; this is not a promise that a duration byte is
an elapsed host frame independent of the source script. Holding poses already
contain the visual elevation in their anchors. **Do not add another hardcoded
“pot above head” offset.** Parent physics supplies the actor/world trajectory;
this library neither removes FA/FB cells nor implements collision, hits, gravity,
carry masks or release timing. Native lift/control transients can also override
OBJ priority; they are not baked into otherwise immutable priority-2 frames.

For FA and FB respectively, look up art IDs **`$96E1A6` and `$96E1AB`**. The
loader follows record byte2 through `$B08000` to the first source component tile
(6 / 8), then the lift consumer's queued transfer operands select two 64-byte
strips at **`$A4C338/$A4C538`** or **`$A4C378/$A4C578`**. These are relocated to
source-indexed OBJ tiles `$6A/$6B/$7A/$7B` for the carrying compositions. The
source tile selected through `$B08000` is **not** the destination `$6A`.

The pot palette is a **partial** load: the upper eight colors of the source C
metatile palette are copied to CGRAM **248..255**. Returned pot palette entries
0..7 are deliberately unprovided/zero and unused by the admitted pixels; this
is not a qualification of those native lower colors. Both pot resources share
all composition lists but retain distinct source planar strips.

## Native evidence and explicit fidelity limits

The checker validates every exported raster independently: exact indexed,
priority and natural-RGBA bytes, both mirrors, nonzero opaque counts, source
composition and palette-only relocation. **18 named native art witnesses** then
check selected source tiles against planar VRAM, anchors, complete contiguous
OAM component records, size/ninth-X bits, used palette colors and OBJSEL/first
sprite. **24 named phase witnesses** additionally check membership, positions,
selectors, mirrors, tour priority and available equal-Y native ordering. Every
phase checkpoint, including empty E/20, retains all five hardware-surface hashes;
changes outside the selected OBJ pieces still fail strict metadata equality.

Not every one of the 241 source records has a retained native OAM witness.
Lift/throw/free-flight lists and unused ordinary variants are explicitly
**source-only** where the checkpoint schedule did not retain the transient.
No new journey was run merely to obtain rendering evidence.

Required limitations are exposed as `PandoraSceneLimit`, not silently removed:

- **`CellarColorMath`:** required source `$889CD8` changes PPU window/color-math
  registers and fixed color during C's second-hit reaction. Actor composition is
  qualified; this effect is not implemented by this art library.
- **`OpeningPalette`:** at original `21-opening`, the opening guide's palette is
  all-white. The checker retains a strict, labeled native-white effect witness
  rather than substituting that palette into ROM art. `tutorial-002` separately
  proves the natural source palette. The effect program is not compiled here.
- **`FrozenTown` / `ScriptedMotion`:** no invented wandering schedule or timing.
- **`SceneComposition`:** Ark's shadow, door/stair transition and reaction poses
  outside the selected carry lists, BG occlusion/layers, brightness, color math,
  text, clipping and hardware per-scanline OBJ overflow limits are not solved by
  an actor raster. No whole-native-RGB promise follows from selected OAM equality.

### Capture roots and the observer race

Evidence roots are kept distinct:

1. Original source journey:
   `/Users/lainsoykaf/repos/ilar-task-pandora-source/local/pandora-qualification/journey`.
   Discovery is retained separately and is not used to initialize the direct C
   branch or replace its roster.
2. Parent fresh journey:
   `/Users/lainsoykaf/repos/terranigma/local/pandora-qualification/replay-JmCgU8/journey`.
   The same strict sprite/phase checker passes this root's selected hardware
   evidence too. **This is not a claim of parent route acceptance.**

The parent/source diagnosis reports an unsynchronized asynchronous publication
race in `vendor/ares/shims.cpp` (`lastFrame` versus `snes_setPixels`), documented
by source commit `5b7b88b` / parent `42fe162`. Intermittent zero regions affect
some `.pixels` files, while source, frame logs and non-pixel captures agree.
**This checker never reads `.pixels`.** There are no black-pixel exemptions or
blind golden updates. Repaired-observer capture roots and any renewed strict
whole-output acceptance remain a separate parent/source gate.

## Reproduce (no native Session)

```sh
ROM="$(pwd)/local/Tenchi Souzou (Japan).sfc" # absolute: Cargo changes test CWD
CAPTURE='/path/to/original-or-parent/journey'
OUT='local/pandora-scene-qualification/new-export' # must not already exist
sh tools/pandora-scene-qualification/export.sh "$ROM" "$OUT"
PANDORA_ROM="$ROM" cargo test -p assets --lib pandora -- --include-ignored
cargo test -p assets
cargo clippy -p assets --all-targets -- -D warnings
for flags in '-B' '-O -B'; do
  python3 $flags tools/pandora-scene-qualification/test_check.py
  python3 $flags tools/pandora-scene-qualification/test_local.py "$ROM" "$OUT" "$CAPTURE"
  python3 $flags tools/pandora-scene-qualification/check.py "$ROM" "$OUT" "$CAPTURE"
done
```

`reference.json` contains only metadata/hashes, including a canonical digest of
the complete export metadata. `--record` is an explicit maintainer operation,
not part of verification. All raw exports remain ignored under `local/`.
The original house10 source export was also compared exactly with the existing
`art-reference.json`; old Ark28's authenticated local pixel test still passes.

RE of unknown source layouts necessarily preceded tests. Executable list/raster,
carry-choice and review-driven malformed-operand tests went red then green;
Python native OAM/VRAM/CGRAM/anchor/size mutations and six phase mutations are
nonvacuous. Replacing the native checker with a no-op makes its test fail.
Independent source/API and evidence-tool reviews are recorded in the owned issue.
