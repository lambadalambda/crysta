# Bounded Japanese Pandora dialogue

[Issue](../meta/issues/decode-pandora-dialogue.md). This is an asset compiler for
[the direct Pandora route](pandora-progression.md), not an event interpreter or
presentation scheduler. It preserves original Japanese font pixels, not invented
Unicode dialogue. `HouseDialogue`, its seven requests, fourteen pages, both choice
catalogs and existing export/native checks remain compatible.

## Parent compiler contract

```rust,ignore
use assets::text::{pandora::PandoraDialogue, Acknowledgement};
let text = PandoraDialogue::from_rom(normalized_japanese_rom)?;
for request in text.requests() {
    // This enumerates resources for compilation, NOT event playback.
    for (index, page) in request.pages().iter().enumerate() {
        let logical_id = request.page_id(index).unwrap(); // source << 4 | index
        // Copy indexed(), width(), height(), background_index() into the pack.
        match page.acknowledgement() {
            Acknowledgement::Next => { /* D5: wait, clear, continue request */ }
            Acknowledgement::End => { /* D3: wait, close, return request */ }
            Acknowledgement::None => { /* D4: retain, return WITHOUT ack */ }
        }
    }
    if let Some(catalog) = request.choice_catalog() {
        let choice = text.choice(catalog).unwrap();
        // Native option text is already pixels on the retained last page.
        // Result 1 is initially selected, 1/2 confirm, 0 cancels.
    }
}
// Unknown requests/catalogs return None, not a fallback or fabricated resource.
let grant_pages = text.pages(0x88_b758).unwrap();
```

Requests/pages are immutable and have source-order identities independent of
which native captures happen to exist. `requests()` enumerates **33 distinct
resources / 76 pages**: first direct-use order followed by the separately admitted
map13 retry choice context `$88B722`, then refusal `$88B7E3` (appended to preserve
all previous resource/page identities). It is **not playback order**.
`DIRECT_INVOCATIONS` contains **34 ordered event sites/text sources**, including
four separate invocations of the same `$89D720` resource, and puts `$89D735` last.
The parent event compiler owns playback, not this resource enumeration. Index zero starts each source request;
`page_id` returns `None` outside that request. Requests longer than fifteen pages
are explicitly rejected rather than silently overflowing/repacking the host key.
The manifest exported by `tools/pandora-dialogue-qualification/export.rs` includes
ordered `invocations` (`site`, `source`), distinct `requests` (`source`, `page_ids`, `choice_catalog`) and ordered `pages`
(`text_source`, `index`, `page_id`, `boundary_source`, `acknowledgement`, dimensions,
background index, bitmap hash and glyph provenance). Raw bitmaps and glyph exports stay ignored in
`local/`; committed qualification references contain metadata and hashes only.

The API authenticates the normalized 4 MiB Japanese image SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
Use `rom::Rom` to normalize a copier header first. No native captures, SRAM,
original CPU, mutable event flags or filesystem are production decoder inputs.
Default player-name bytes come from the source initialization immediates, not a
copied WRAM name or a transcription. Custom names remain outside admission.

The event owner must retain request completion boundaries. In particular map13's
four-page grant request returns **before** `$28`, C's entry sets `$27` **before**
its entry request returns, C's direct answer returns **before** `$2E`, and the
four-page final tutorial request returns **before** `$244`. Text pages cannot
substitute for those event operations. The forced tutorial remains a forced
sequence, not free inventory-room navigation.

## Fidelity and acceptance boundary

Page pixels are row-major native two-bit indices. Use the returned dimensions
and `background_index()`: most pages are **224×48/background 3**, `$889DC3` is
**192×48/background 0**, and `$88ADCB/$88ADF2` are **224×48/background 0**. The
latter use the source transparent-font plane transformation, not an inferred
alpha channel or a promise of palette/RGBA compositing. The API
exposes glyph source/placement and the actual D3/D5/top-level D4 boundary address.
Colors, window/frame placement, wait arrows, sound, typewriter/delay timing and
scripted actor/camera animation are not full-screen equivalence claims. Catalog
cursor/navigation semantics remain explicit; option labels are never fabricated.

The original retained journey and the parent's independent fresh journey are
separate evidence inputs. The source owner diagnosed the exact-checker mismatch
as an unsynchronized async `.pixels` publication race in the native shim
(source `5b7b88b`, parent `42fe162`); frame logs, native states and non-pixel
captures match across 2,682 other files. **`.pixels` is not uniformly trustworthy
RGB evidence.** This checker uses WRAM/VRAM font cells instead. It does not exempt
black pixels, substitute golden captures or loosen reference equality. The
parent's separately authorized observer fix and renewed strict replay remain a
source acceptance gate, not something this dialogue task claims to have passed.
This issue stays open for parent-owned integration/acceptance; no shared index
or milestone is changed here.

## Source-qualified additions

The ordinary dialogue dispatch remains `$859198`, not the overlay engine.
All previous house controls keep their existing admission. `$C4 00`, `$E3`
and `$E4` are Pandora-profile-only; `$DA`, `$C2` and `$CC` were later admitted
in every profile, and `$C2` accepts any on-screen layout, because Crysta
residents use them. Unknown controls, geometry and calls fail closed.

| Addition | Source behavior and finite compiler interpretation |
| --- | --- |
| `$DA → $85964D` | Chooses bottom `$0DB6=$0504/$0DB4=$6A80` if signed `$048A` is negative or `$0954-$0822-$70` is negative; otherwise top `$0104/$6880`. Both use 28×6 tiles. Content is compiled page-relative; the page's `Placement::AwayFromPlayer` tells the host which anchor applies. |
| `$C2 → $85982D` | Four operands are tile column, row, width, height. The Pandora tuple `(6,6,24,6)` produces 192×48 content at native `$0DB6=$018C/$0DB4=$68C0`. Any on-screen tuple with a glyph row is admitted since the Crysta slice needed `(3,3,25,6)`; the page records `Placement::Tile`. |
| `$C4 00 → $8598B7` | Sets `$0DA4` bit4; `$C4 01` clears it. `$85947F..95BC` transforms each planar pair `a,b` to `a^(a&b), b^(a&b)`: index3 becomes0, indices0/1/2 remain unchanged. This is **not scaling**. Odd-half-tile packing fills with zero; `$8595BD` clears to tile0. `$859EB1` preserves bit4 through page resets. |
| `$CC → $859A13` | Little-endian long text call; `$859ECA` stores the banked return after three operand bytes. Nested `$D4` returns to that address, not a page boundary. Depth remains bounded at eight. |
| `$D2 03` | Existing word-pointer call table `$92C447`; new required speaker subroutine `$92C49F`. Default player name still reads source initialization, not Unicode or capture RAM. |
| `$E4 → $859725` | Calls `$92C5E7 + 2*index` in bank92. Only required indices `$06/$25` are admitted; their original item-label glyphs return via D4, with no fabricated ack. |
| `$E3 → $8596BE` | Looks up an action mask in controller assignments `$0634..063E`, then calls the corresponding label via `$85970D`. Read/check six immediate16/STA-absolute pairs from the native default-reset handler `$85BEF7..BF1B`, reordering the last two stores by destination. This is a **default controller configuration** profile, not an inventory lookup. Remapped controller labels are outside admission. Native default assignments are checked independently. |

As before, palette changes/reset flush a pending half tile even though colors
are omitted. The default assignments are source-derived `[0080,8000,0040,4000,
0020,0010]` in address order; the captures corroborate them but are not decoder
inputs. E3 resolves the native font labels rather than invented Unicode button
names. All font/label bytes stay local.

### Choice and warning boundaries

Map13 `$88B6C7` has a D5 page then retained D4 context; catalog1 is entered at
`$88B679`, result table `$88B68B` (0/2→`$88B691`, 1→`$88B69C`). The admitted
retry context `$88B722` is D4-only; choice site `$88B685` uses that same table.
C `$889EFC` has three D5 pages then a retained D4 context; choice sites
`$889B17/$889B40` use catalog1/table `$889D80` (0/2→`$889D86`, 1→`$889DA9`).
Map13's result0/2 request **`$88B691 → $88B7E3`** is also admitted: page0 ends at
**`$88B807 D5`**, page1 at **`$88B82D D3`**, logical IDs `$088B7E30/$088B7E31`.
The event owner may route cancel/result2 → refusal → local1 → retry `$88B722`,
then result1 → four-page grant `$88B758` → `$28`. Refusal has no choice catalog
and does not grant `$28`; this compiler does not perform the local1 write or
any branching. `DIRECT_INVOCATIONS` remains the original 34 direct invocations,
not this optional retry path. The two added native samples are discovery's
button-free `resident13-refusal` and `resident13-refusal-next` checkpoints,
matching 12,288 font-cell pixels including 2,959 foreground pixels.

C's second refusal choice / `$2F` branch is not admitted. C's alternative staging sites
`$889B24/$889B3A` request the same `$889EE3/$889EFC` resources; they do not get
new resource/page identities.

**Correction to a cursor-only reading of the progression notes:** warning
`$88ADF2` has exactly **two** logical pages, ending at `$88AE29 D5` and
`$88AE5E D3`. Observed cursor `$88AE50` points to glyph byte `$56`, **not** an
acknowledgement. It is not a third page. Live cursors during typewriter output,
delays, or retained choice processing are not equivalent to source boundaries.

The longest admitted request is `$89D7C1` at eight pages; none exceeds fifteen.

## Reproduce without native navigation

```sh
ROM='/path/to/Tenchi Souzou (Japan).sfc'
sh tools/pandora-dialogue-qualification/export.sh "$ROM"
# Use the printed local export directory; this compiles resources, not a route.
EXPORT=local/pandora-dialogue-qualification/export-XXXXXX/pages
PANDORA_ROM="$ROM" cargo test -p assets --lib text::pandora -- --ignored
cargo test -p assets --lib
cargo clippy -p assets --all-targets -- -D warnings
for flags in '-B' '-O -B'; do
  python3 $flags tools/pandora-dialogue-qualification/test_check.py "$ROM" "$EXPORT"
  python3 $flags tools/pandora-dialogue-qualification/check.py "$ROM" "$EXPORT"
  python3 $flags tools/pandora-dialogue-qualification/native_check.py \
    "$ROM" "$EXPORT" "$ORIGINAL_JOURNEY" original
  python3 $flags tools/pandora-dialogue-qualification/native_check.py \
    "$ROM" "$EXPORT" "$PARENT_FRESH_JOURNEY" parentfresh
  python3 $flags tools/pandora-dialogue-qualification/native_check.py \
    "$ROM" "$EXPORT" "$DISCOVERY_JOURNEY" discovery
done
```

The selected retained roots for this qualification were:

- `ORIGINAL_JOURNEY=/Users/lainsoykaf/repos/ilar-task-pandora-source/local/pandora-qualification/journey`
- `PARENT_FRESH_JOURNEY=/Users/lainsoykaf/repos/terranigma/local/pandora-qualification/replay-JmCgU8/journey`
- `DISCOVERY_JOURNEY=/Users/lainsoykaf/repos/ilar-task-pandora-source/local/pandora-qualification/discovery`

`reference.json` is the machine-readable ordered source request/page/ack manifest,
including repeated invocations and bitmap hashes. The independent Python walker
resolves requests from the actual COP1B operands and choices from COP1A operands,
checks dispatch targets and initializer instruction shapes, reconstructs all
76 complete bitmaps from original font planes, and compares dimensions, mode,
placements, boundaries, choices, identities, and exact bytes against the Rust
export. Reference equality is an additional check, not a replacement for decoding.
No Unicode strings, font cells, text bodies or raw bitmaps are committed.

`native-reference.json` records separately selected capture labels, WRAM/VRAM
hashes, exact pixel/foreground counts, and each set's source-only page IDs:

| Capture set | Selected distinct pages | Compared font-cell pixels | Foreground pixels |
| --- | ---: | ---: | ---: |
| Original direct | 73 | 380,672 | 97,371 |
| Parent fresh direct | 73 | 380,672 | 97,371 |
| Discovery (supplemental) | 67 | 355,968 | 92,434 |

Both direct sets leave **retry `$88B722` page0 and refusal `$88B7E3` pages0–1
source-only**. Discovery supplies that retained context and both refusal pages; it leaves direct answer `$889FA9` pages0–1
and the seven first/second-hit reaction resources source-only (`$88A15F`,
`$88A17B`, `$889DC3`, `$889DEB`, `$88A295`, `$88A420`, `$889E0A`, each page0).
The union covers all **76 pages across 33 resources**, but discovery is never a substitute
for a failing direct sample. A source-only page still has independent full ROM
reconstruction; “source-only” here means no selected native raster **in that set**.
Repeated `$89D720` invocations reuse a qualified page; this is not a pixel witness
for every invocation or for every typewriter frame.

Native comparison resolves the source-proven WRAM window tilemap and 2bpp VRAM
font tiles. It compares the **12×16 advanced cell of every placed glyph**, with
positive pixel and foreground counts for every sample. The font record is 16×16;
its final four columns are not an extra native character advance. Native wait
arrows outside the glyph cells and frame/RGBA compositing are outside this claim.
Only the selected source/catalog-defined 8×16 cursor region may be excluded,
and every excluded expected pixel must be background, so a cursor mask cannot
hide label ink. Acknowledged samples must have the actual banked D3/D5 pointer;
D4 choice tails do not acquire fabricated waits. Native controller assignment
words are checked against the source defaults as well. Every selected capture
hash must match the **explicitly named** reference set; missing or mismatching
captures fail rather than triggering alternate selection.

Verification was run normally and under optimized Python, using only existing
captures. The 20 Python tests include synthetic planar/layout/control tests,
22 ROM-export mutations, native foreground/blank/cursor/ack mutations, and native
orchestration tests for uncompared-byte hash tampering, missing/unknown sets,
changed controller defaults, and incorrect coverage inventories. Deliberately
removing the native reference-equality guard caused two orchestration failures;
restoring it returned green. Replacing source/native comparison functions with
no-ops caused failures under both normal and optimized Python. A Rust transparent
font no-op was also killed by its synthetic test. The owned-ROM Rust test pins
33 resources, 76 pages, repeated invocation order, the two-page warning and
the two-page refusal. A frozen metadata/bitmap-hash digest protects all previous
74 page identities/pixels against changes.
The original house tests and independent 14-page/two-catalog source check remain
unchanged; their 67,904 selected native font-cell pixels still match.

Independent static reviews covered decoder correctness/architecture and
qualification correctness/architecture/pixel nonvacuity. They did not execute
native navigation or independently reproduce these totals. Review found and
closed the dynamic-geometry/resource-order documentation issues and the native
orchestration coverage gap. The parent has since corrected the AE50 wording in the source owner's
progression document; this task did not edit outside its ownership.

### Component acceptance status

The parent has independently reproduced and accepted this component and the
strict direct Pandora source route under `headless-sync-video-v1`. This
supersedes earlier pending observer/parent handoff statements, **not** the
source-only coverage or fidelity/admission limits in this contract. Portable
Pandora integration remains open and the live host stays on the house profile.
