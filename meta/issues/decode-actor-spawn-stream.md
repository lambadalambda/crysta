# Decode the actor spawn stream's opcode lengths

## Summary

`assets::maps::actors::SpawnList` decodes spawn lists for **14 of the 24**
Crysta maps. The other ten hit a control opcode whose length this decoder
cannot account for, and are refused. Decode the stream's opcode dispatch so
every map yields its records.

## Dependencies

- [Decode text and gameplay data tables](decode-text-gameplay-data.md)

## What is already decoded

The loader at `$80:F3F1` zeroes `$0DFA`/`$0DFC`/`$0498`, reads `$0480`, and
selects between the tables at `$82:8000` and `$83:8000` indexed by map ID times
two. The list opens with a two-byte header, which the loader's own `INC A` pair
skips. Its interpreter loop reads through `[$6E]`, compares the opcode against
`$FF` and `$FC`, and calls `JSL $86:832E` per record.

Positioned records are decoded and verified: `$00` and `$01` are ten bytes,
`$FD` is seven, and the origin is `(tile_x * 16 + 8, tile_y * 16)`. Six of the
nine documented residents are checked at their exact source addresses.

## The specific gap

Two control opcodes have **selector-dependent lengths**, which is what breaks a
fixed-stride walk:

```text
$FF d4                          2 bytes   (map $0014 +0x2c, parses)
$FF 80 a2 82 56 53 80 47 d4     9 bytes   (map $000D +0x38, refused)

$FA ac 01 28 ea                 5 bytes   (map $000B +0x09, parses)
$FA 96 01 df 8b                 5 bytes   (map $000B +0x0e, parses)
$FA ba 10 bb 00 ba 8b           7 bytes   (map $000B +0x13, parses)
$FA 2a 10 2c 80 a6 8d           7 bytes   (map $0010 +0x36, refused)
```

The `$FA` lengths were derived by requiring the walk to land on a known record,
which is inference from alignment, not from the interpreter. `$AC` and `$96`
take five while `$BA` and `$2A` take seven, and no bit pattern over those four
selectors explains it, so the lengths must come from the dispatch itself.

The ten refused maps and their stopping opcodes: `$000A` (`FA/99`), `$000D`
(after `FF/80`), `$0010` (`FA/2A`), `$0013`, `$0017`, `$001B`, `$001D`,
`$001E`, `$001F`, `$0021` (`FA/44`). Most report an opcode that is plainly
operand data, which is the decoder correctly reporting where it lost
synchronisation rather than the true opcode.

## Requirements

- Decode the opcode dispatch from the interpreter, not by fitting lengths until
  a walk lands on a plausible record. A wrong length emits positions read from
  operand bytes, and those look exactly like real spawns.
- Keep the refusal fatal. Resynchronising is what this decoder deliberately
  does not do.
- Record which opcodes carry conditions, since a decoded list is currently
  every record the stream holds rather than the set that applies on a save.
  Lists routinely repeat a position under different conditions.

## Acceptance Criteria

- All 24 Crysta lists decode end to end, or each refusal names an opcode read
  from the dispatch rather than from a desynchronised walk.
- All nine documented residents are checked at their source addresses, not six.
- Opcode lengths are justified from the interpreter in the doc, with the
  selector-dependent cases called out.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Make the Crysta slice fully playable](playable-crysta-slice.md)
- This is the remaining blocker for that slice's "residents are present and
  interactable" criterion. It was briefly and wrongly recorded as blocked on
  the event bytecode, on a sixteen-entry table-base error; the table base is
  `$83:8000`.

## Qualification

Done. All 24 Crysta lists decode end to end, 115 records, with every length
read from the interpreter rather than fitted.

Three findings closed it, and the first two were the reason a fixed-stride walk
could never work:

- **`$FF` terminates the list.** `$80:F4A4` ends the walk, so the bytes after
  it are `$FA` branch targets rather than fall-through. Walking past the
  terminator was reading operands as opcodes, which is what produced the
  "unaccounted opcodes" that were plainly data.
- **Ordinary records are variable length.** `$80:F564` tests byte 3's top two
  bits and reads four further fields when both are set, widening the record
  from ten bytes to sixteen.
- **`$FA` is a chained event-flag condition**, evaluated through `$80:BBC7` —
  the same routine the loading scripts use. `$80:F76C` branches on bit 15
  *before* masking `$F800`, so a negative condition word takes the short
  five-byte form; otherwise bits in `$F800` chain a second word into seven.
  Getting that order wrong left exactly one map, `$0021`, misparsed.

Verification: all nine documented residents decode at their exact source
addresses, and decoded origins are cross-checked against **actor positions read
out of WRAM while walking the reference emulator** — map `$000B` at `(120,112)`
and `$000C` at all four of its residents. That is stronger than matching the
scene census, which is itself a derived document.

One nuance worth keeping: the installer at `$80:F52B` stores `tile * 16` with
no bias, yet the running game reports the origin eight pixels right. The `+8`
is applied after installation and is not decoded; `origin()` reports what the
game shows.

## Remaining

Conditions are still not evaluated, so a decoded list is every record the
stream holds rather than the set that applies on a given save. Lists routinely
repeat a position under different conditions.
