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
`$0104`, four lines; `DB` is `$044A`, 22 tiles, four lines. The choice
records keep their absolute tiles, so relative to the European window they
sit at y = 24 and 40 (not checked on screen).

## The first bedroom page

The script requests text by address in bank `$88`, as in Japanese: the
first page is `$88:9C15` (native European frame 3837): `C4 01`, `C1`,
`D2 02` ("Elle: " in palette 1, colour `$5E3F`), "...", a pause, "Are you
all right?", `D5`.
