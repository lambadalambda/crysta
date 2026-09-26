# The European text engine

Research for [the European text issue](../meta/issues/european-text.md),
checked against dumps of the European ROM in the reference emulator. The
engine is the Japanese one (`crates/assets/src/text.rs`) with new constants;
glyphs keep their 16×16 cells, 2bpp, and the fixed 12-pixel pitch.

## Encoding

Single bytes `$00-$7F` are glyphs, not ASCII: `$20` space, `$21-$3A` A-Z,
`$40` "ed", `$41-$5A` a-z, `$60` ?, `$61` (, `$62` ), `$63-$6C` 0-9, `$6D` !,
`$6E` ,, `$6F` :, `$70-$7F` → ← ↑ ↓ “ ” ' = … % * + - / & . The kana switch
`D0` and the two-byte codes `$80-$BF` do not occur; they point past the font.

`E5 nn` and `E6 nn` call word `nn` of the dictionaries at `$92:C793` and
`$92:D2BB` (256 each, `D4`-terminated, no controls), as `CC`/`D2` calls
(`$85:9F62`); `E4` is `E5`. The other controls keep their Japanese meaning.

## Addresses

| What | Japanese | European |
| --- | --- | --- |
| Control dispatch | `$85:9198` | `$85:91E4` (`JMP ($91E4,X)` at `$85:91E1`) |
| Font | `$B4:8000` | `$B6:8000` |
| `D2` table (names, speakers) | `$92:C447` | `$92:C5CD` (25 entries) |
| Default name | `$87:8C99` | `$87:8C8E`, 5-byte stride |
| Choice records | `$92:C259` | `$92:C407` |
| Window characters | `$A9:9000` | `$AB:9000` |
| Window colours | `$B2:8B78` | `$B4:90DB` |
| Window shade, HDMA | `$85:81E4`, `$85:8160` | the same |
| Prompt | `$CB:7A98` | `$CD:7A98` |
| Label palette (OBJ 2) | `$B2:8B58` | `$B4:90BB` |
| Item names, `CE` names | `$92:8179`, `$92:8379` | `$92:81A4`, `$92:83A4` |
| Title effect scripts | `$B0:DE49` | `$B2:E26C` |

## Windows

`C1` opens at base `$04C4`, a row higher than the Japanese window, with four
lines: its content is 224×64 (Japanese 224×48). `DA` at the top is base
`$0104`, four lines; `DB` is `$044A`, 22 tiles, four lines. The standard
absolute choice records sit at y = 24 and 40 relative to this window
(not checked here). The Elder's first-choice catalog 2 instead has relative
records `$84 00` and `$86 00`: its decoded cursor-source cells start at y32
and y48.

The frozen-return Elder's `COP 1B` at `$88:8D36` requests `$88:8D6C`.
Its 216-pixel custom window contains a glyph at x204: the 12-pixel pitch
fits, though its 16-pixel cell extends four pixels past the portable content
bitmap. The decoder clips that edge when composing and partially typing the
page, rather than freezing the actor. This is a bounded portable bitmap rule,
not a native raster comparison of that window's right edge.

A fresh, empty-SRAM headless European boot now checks the **first fully typed
bedroom page** at its native `$D5` wait. `crates/map-inspector/tests/local_european_text_native.rs`
reads the `$7F:D4C4` **WRAM-staged** 224×64 content tilemap and 2bpp VRAM,
then compares all 4,992 unique pixels in the first page's 26 glyph cells with
the ROM-decoded indexed page. In the same snapshot it checks all 76 staged
frame tiles around the content (4,864 2bpp indices) against the ROM-decoded
`$10..$17` frame art, including flipped VRAM sampling. It asserts real
foreground and frame ink and the European `$04C4` anchor without inspecting
the uploaded BG3 tilemap/attributes, prompt or framebuffer colour. The page
does not exercise every content row. This optional owned-ROM check skips if the
dump is absent; it is one native page's indexed content/staged frame layout
evidence, not a proof that all dialogue, choices, labels, item names, or their
RGB/HDMA presentation match native frames.

A separate fresh-child test, `crates/map-inspector/tests/local_european_choice_native.rs`,
boots once with empty SRAM and follows the first 53 input rows of the European
Pandora tour after the 1960-frame boot. It stops at the **first room-B Elder
choice**, before answering or approaching the Box: map `$0B`, event `$26` set,
`$0DC2=$FFFF`, window anchor `$0DB6=$04C4`. The ROM-decoded request
`$88:91C6` ends in a retained `$D4` at `$88:925B`; catalog 2 initially
selects result 1 (`$0DCE=0`), then one Down edge selects result 2 (`$0DCE=1`)
without answering. At both selections, the entire 224×64 page matches the
WRAM-staged `$7F:D4C4` tilemap and 2bpp VRAM (14,208 pixels each), except
**only** the selected cursor's 8×16 blank source cell. Both option labels
have foreground ink in their own glyph rows y32/48, within the requested
y24/40 option-row bands. This is indexed staged content evidence, not native
cursor animation, uploaded BG3 attributes, RGB/HDMA, or other pages/choices.

## The first bedroom page

The script requests text by address in bank `$88`, as in Japanese: the
first page is `$88:9C15` (native European frame 3837): `C4 01`, `C1`,
`D2 02` ("Elle: " in palette 1, colour `$5E3F`), "...", a pause, "Are you
all right?", `D5`.
