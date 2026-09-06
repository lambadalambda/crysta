# Bounded Japanese house dialogue

This asset-only compiler handles room-B entry, the first and repeat prompts,
and their two alternative follow-ups. It does **not** run event scripts,
interpret callbacks, set flags, schedule dialogue, or implement browser input.
The [event qualification](house-conversation.md) owns that contract:
resident `$838B96`, callback **`$888EDE`**, flag-write site `$888F08`.
Entry `$888FDA` is not the progression conversation and never awards `$0026`.

## Interface and request-return contract

```rust,ignore
use assets::text::{Acknowledgement, HouseDialogue, FIRST_TEXT, TEXT_SOURCES};
let dialogue = HouseDialogue::from_rom(normalized_japanese_rom)?;
for page in dialogue.pages(FIRST_TEXT).unwrap() {
    match page.acknowledgement() {
        Acknowledgement::Next => { /* display; wait; clear/continue request */ }
        Acknowledgement::End => { /* display; wait; close/return request */ }
        Acknowledgement::None => { /* display retained tail; return WITHOUT waiting */ }
    }
}
let catalog = dialogue.choice(0).unwrap();
// Its option labels are already native pixels on the retained prompt page.
```

**Do not turn every raster into an acknowledged ShowPage.** First request
`$888FF0` has one acknowledged page followed by an **auto-completing retained
tail** containing the real two-option Japanese prompt. On that request's return,
the event writes `$0026` **before** entering catalog 0. The choice and subsequent
follow-up are not extra prerequisites for that native write. Repeat `$889156`
consists entirely of a retained tail and has **no text acknowledgement** before
catalog 1. The event owner, not this asset layer, decides request-return effects.

`HouseDialogue::pages(source)` returns immutable page resources in source order.
`TEXT_SOURCES` lists the seven admitted requests. `FIRST_TEXT` is `$888FF0`;
`RESIDENT_TEXT` is the historical API name for **alternate follow-up `$88905A`**,
not the first interaction prompt. Unknown sources return `None`. A stable host
key is `(text source, zero-based page index)`; a host may assign sequential
`u32` keys scoped to the authenticated asset identity.

Each page owns a **224×48** row-major, one-byte-per-pixel buffer of native
2bpp font indices. Index **3 is background**; the original font uses 1 for light
foreground and 2 for its dark edge. The host may map these to a high-contrast
panel palette instead of reproducing native color/window effects. This is
native glyph presentation, not whole-screen RGBA equivalence. `glyphs()` exposes
each encoded character source, 64-byte font source and pixel position;
`boundary_source()` is the actual `$D5`, `$D3` or top-level `$D4` source address.

`choice(0|1)` exposes two `DialogueOption` records in native result order:
result **1/2**, source address, cursor position **`[0,16]` / `[0,32]`**, and
Up/Down/Left/Right neighbors. Up/Down toggle results; Left/Right have no link.
Native initial result is 1, A/L confirms and B cancels with result 0. The real
option labels already occupy the retained page's last two rows, after a
12-pixel reserved cursor/space cell. A host can use each original bitmap crop
**`[12,y,212,16]`** as the option button, and preserve the whole page as context.
There is no Unicode transcription or fabricated option label in this API.

## Qualified text boundaries

| Request | Page | Glyph records (spaces included) | Boundary | Required action |
| --- | ---: | ---: | --- | --- |
| `$888FDA` entry | 0 | 19 | `$888FEF D3` | acknowledge, close |
| `$888FF0` first prompt | 0 | 37 | `$889027 D5` | acknowledge, clear/continue |
| | 1 | 36 | `$889059 D4` | **return, retain choice context; no ack** |
| `$88905A` first result 2/cancel | 0 | 39 | `$889083 D5` | acknowledge, continue |
| | 1 | 33 | `$8890B0 D5` | acknowledge, continue |
| | 2 | 36 | `$8890D8 D3` | acknowledge, close |
| `$8890D9` first result 1 | 0 | 29 | `$8890F8 D5` | acknowledge, continue |
| | 1 | 39 | `$889126 D5` | acknowledge, continue |
| | 2 | 39 | `$889155 D3` | acknowledge, close |
| `$889156` repeat prompt | 0 | 35 | `$88918B D4` | **return, retain choice context; no ack** |
| `$88918C` repeat result 1 | 0 | 27 | `$8891AA D5` | acknowledge, continue |
| | 1 | 36 | `$8891D5 D3` | acknowledge, close |
| `$8891D6` repeat result 2/cancel | 0 | 20 | `$8891EB D5` | acknowledge, continue |
| | 1 | 38 | `$889217 D3` | acknowledge, close |

The live `$0DC0` text cursor need not equal the executed D4 terminator after an
auto-completing tail. Fresh choice contexts retain `$9047` / `$917E`; source
execution ends at **`$889059` / `$88918B`**. In contrast, acknowledged pages'
native banked cursor matches the exact D5/D3 boundary in the table.

## ROM/source evidence

The loader authenticates the **normalized 4 MiB Japanese image** against SHA-256
`f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548`.
Normalize copier headers with `rom::Rom` first. Production decoding has no file
I/O, oracle, original CPU, Unicode mapping, translated dialogue, extracted font,
or captured runtime-state dependency.

Two related engines must not be confused. Room labels use `$85869B` and dispatch
`$858763`; ordinary dialogue uses `$85910E`, byte dispatch `$85913E` and control
jump table **`$859198`**. Their command meanings are not interchangeable.

- `$85913E..9169`: bytes below `$80` are one-byte glyph indices; `$80..BF`
  starts a big-endian two-byte index with the top two bits removed. This is a
  game-specific encoding, **not Shift-JIS**.
- `$859358..939F`: single-byte glyph `c` addresses `$B48000 + 64*c`, plus
  `$2000` while `$D0` alternate-kana mode is set. `$D1` clears that mode.
- `$85930D..9357`: two-byte index `n` addresses bank `$B5 + (n >> 9)`, address
  `$8000 + 64*(n & 511)`.
- `$8593A5..93D5`: raw 64-byte glyph records contain four SNES **2bpp** tiles
  in TL/TR/BL/BR order, each 16 bytes. The copier visits offsets `0..16,32..48`,
  then `16..32,48..64` for the two tile columns.
- `$85928D..930C` packs glyphs at **12-pixel advance**. `$85960D..964C` opens
  the 28-tile-wide, three-row content area. `$CF` at `$859BB3` advances a
  16-pixel row and clears alternate-kana mode. Native scrolling/wrapping outside
  this admitted geometry is rejected.
- `$C6` / `$DC` at `$8598FE..992C` / `$85969F` change/reset text palette.
  Their **half-tile flush matters even without color effects**: pending odd
  glyphs align the next glyph to 8 pixels (+4). This affects speaker prefixes
  and the highlighted words in the first prompt, proven in native font cells.
- `$D2` at `$859C7A..9C9B` calls the `$92C447` word-pointer table with a return
  stack. Only indices 0, 1 and 7 are needed. Index 0 points to WRAM `$0610`;
  indices 1 and 7 point to ROM speaker-prefix subroutines.
- Default-name branch **`$878C97..8CB6`** writes six immediate bytes to
  `$0610..0615`. The compiler reads those operands and checks their
  LDA-immediate/STA-absolute shape and destinations. It does not copy observed
  name RAM, transcribe the name or support custom player names.
- `$D4` at `$859D13` returns to a saved text caller; with an empty stack it
  ends the request **without acknowledgement and without clearing the page**.
- `$D5` at `$859D94..9E03` waits on input mask `$00A0`, then clears the page
  through `$8595BD`, resetting position/kana. Wait-arrow animation is omitted.
- `$D3` at `$859C9E..9D7E` waits on `$CFE0`, closes the window, clears the
  return stack and finishes. The host's explicit Continue edge is intentionally
  a subset of native accepted inputs.
- Choice entry `$859F28` resolves catalog words at **`$92C259`**. Catalog 0
  entries are `$92C271/C27B`; catalog 1 entries `$92C285/C28F`. Each is ten
  bytes: cursor coordinate word, four neighbor pointers in U/D/L/R order.
  `$85A051..A077` converts the low-byte row and high-byte column to a tilemap
  byte offset; low-byte bit 7 adds the content base `$0504`. Absolute positions
  subtract that same standard base for page-relative coordinates.
  `$859FC3..9FE6` returns selected index+1 or cancellation 0. No event branch
  table is decoded or executed here.

Supported text controls: `$C0/$C1` initial clear/window setup, `$C4 01` raw font,
`$C5` delay, `$C6 00/04` palette, `$C7` sound, `$C8` speed, `$CA 05 word`
speaker-color storage at `$7F060A`, `$CF` newline, `$D0/$D1` kana,
`$D2 00/01/07` calls, `$D3/$D4/$D5` as above and `$DC` palette reset.
Delay/sound/speed/color operands are consumed without native timing/audio/color
effects; `$CA` cannot write arbitrary memory or progression flags. Unsupported
text commands (including in-text selection commands distinct from these
**event-level catalogs**) error rather than flattening into Continue. Rejects
unacknowledged clears, transformed-font modes, malformed catalog neighbors,
recursion depth >8, >16 pages, >4096 tokens, bank crossing, truncation, empty page
boundaries and out-of-panel glyphs. The public loader admits only the exact ROM.

## Reproduction and independent checks

```sh
cargo test -p assets --lib text::
cargo clippy -p assets --all-targets -- -D warnings
python3 -B tools/house-dialogue-qualification/test_check.py
sh tools/house-dialogue-qualification/export.sh 'local/Tenchi Souzou (Japan).sfc'
# Use the printed .../pages export and the event owner's existing fresh journey:
python3 -B tools/house-dialogue-qualification/native_check.py \
  'local/Tenchi Souzou (Japan).sfc' local/house-dialogue-qualification/export-XXXXXX/pages \
  /path/to/house-conversation-qualification/journey
# Repeat the last check with python3 -O -B.
```

`export.rs` exercises the production API; all resources stay under ignored
`local/house-dialogue-qualification/`. The independent Python walker resolves
subroutines/default-name operands, reconstructs every glyph with its own planar
implementation and compares **all 14 pages**, source positions/boundaries,
choice catalogs and exact bitmap bytes. It checks actual conversation dispatch
targets too. `reference.json` contains addresses/counts/hashes only.

`native_check.py` consumes the event owner's already-produced **fresh input-only
journey**, never implements another route or boots/steps/patches a CPU. Eleven
selected page/tail samples match **67,904 native font-cell pixels exactly**,
using WRAM text tilemap `$7FD504` and native 2bpp VRAM character base `$E000`.
It covers entry, both first-prompt pages, all three first-result-1 follow-ups,
repeat prompt and both repeat-result follow-ups. All nine acknowledged samples
also have the expected live banked D5/D3 cursor. Both retained choice contexts
are compared without inventing a text wait.

The only excluded pixels *inside* compared glyph cells are the native blinking
8×16 choice cursor in a source/catalog-selected, expected-blank reserved space;
the checker explicitly rejects masking any non-background label pixel. Wait
arrows outside glyph cells, window frames and colors are not claimed as page
art. The three first-result-2/cancel follow-up pages are independently ROM/source
checked but have **no selected fresh native raster sample** in that journey.
`native-reference.json` records the exact input capture hashes and sample counts.
The parent independently replayed the event route into
`local/house-conversation-qualification/replay-OYXAH7/journey`; both normal and
optimized native text checks reproduced the **same capture hashes and 67,904
matching pixels**. No capture contents became production input.
This is selected content/placement evidence, not whole-screen/timing equivalence.

TDD: initial unresolved API red → green; synthetic malformed/truncated controls,
missing ends, geometry overflow, recursion, two-byte and alternate-kana glyphs,
page boundaries and four planar quadrants. Additional red/green cycles fixed
half-tile palette alignment and a reviewer-found newline overflow, then added
D4/no-ack returns and source-only choice catalogs. All asset tests and strict
assets clippy pass. The Python source checker/tamper tests run normally and under
`-O`. Raw ROM/assets/source dumps/native captures/screenshots remain local and
ignored; neither text nor font data is committed.
