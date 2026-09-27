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
page, rather than freezing the actor.

A fresh, empty-SRAM headless European boot checks the **first fully typed
bedroom page** at its native `$D5` wait. `crates/map-inspector/tests/local_european_text_native.rs`
reads the `$7F:D4C4` WRAM-staged 224×64 content tilemap and 2bpp VRAM, then
compares all 4,992 unique pixels in the first page's 26 glyph cells with the
ROM-decoded indexed page. It also checks all 76 staged frame tiles (4,864
indices) against the ROM-decoded `$10..$17` frame art, including flipped VRAM
sampling. Eight stable native CGRAM entries (1, 2, 4, 5, 6, 8, 9 and 10)
match decoded window/speaker colours; entry 0 is untested, and colour-3 entries
3/7/11 are omitted because HDMA rewrites them per scanline. The test maps PAL
logical rows to the uncropped ares framebuffer at `y+29` and observes 3,401
bounded non-shade candidates unchanged across two settled native frames,
omitting 4,512 shade-index candidates. That last result
is native framebuffer self-consistency only: composition may show other layers,
so it does not attribute those RGB values to BG3/CGRAM or claim portable RGB or
HDMA parity.

The first-choice test follows the first 53 European tour rows to the
**unanswered room-B Elder choice**: map `$0B`, `$26` set, `$0DC2=$FFFF`, anchor
`$04C4`. At both selection states the 224×64 page matches 14,208 indexed pixels,
excluding only the selected 8×16 cursor source cell. It additionally samples
both visible and hidden BG3 cursor phases at both positions, compares the
cursor/interior indexed art with ROM-decoded `WindowArt`, and verifies that all
224 uploaded BG3 tilemap words equal the WRAM staging words, including their
tile and attribute bits. This proves complete staged-word upload; it does not
independently derive every staged attribute or qualify exact blink timing or
composed video.

The same test binary has a separate fresh child for the **late doorway-Elder
mission**. It replays the retained fixtures without memory writes to map `$0D`
at `(120,704)`, with `$21` set and `$296` clear, and stops at decoded boundaries
rather than a fixed delay. It compares 13,568 indexed pixels on the 216×64 page
containing the x204 glyph, excluding only the source-positioned 16×16 D5 prompt,
and 13,696 pixels at each of two unanswered choice selections, excluding only
the selected 8×16 cursor source cell. Both option rows contain real ink outside
the cursor columns.

Finally, `local_european_title_native.rs` naturally reaches `$0A` and checks one
settled source-motion phase of the six-letter **Crysta** area title. Six current
16×16 OAM pieces, their uploaded OBJ glyph pixels, and palette-2 selection plus
its two used colours (entries 1 and 2) agree with the ROM-derived title. This is
current hardware configuration at one visible position, not framebuffer proof
or animation-cadence parity. English title and item-name decoding already
covers Crysta, Elder's, Merchant, Center, S.Bulb,
P. Cure and dictionary-expanded Crystal Thread. A native Crystal Thread frame
is not claimed: naturally selecting item `$32` requires a separate shop route
outside this slice.

## The first bedroom page

The script requests text by address in bank `$88`, as in Japanese: the
first page is `$88:9C15` (native European frame 3837): `C4 01`, `C1`,
`D2 02` ("Elle: " in palette 1, colour `$5E3F`), "...", a pause, "Are you
all right?", `D5`.
