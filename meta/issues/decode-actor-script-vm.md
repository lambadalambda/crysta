# Decode the actor script VM

## Summary

Dialogue and progression are the last two Crysta capabilities, and they are the
same dependency: the scripts each actor spawn record points at are not
executed. Decode that VM so a resident can say something and a flag can be set.

## Dependencies

- [Decode the actor spawn stream's opcode lengths](decode-actor-spawn-stream.md)
- [Reverse the event script bytecode](reverse-event-bytecode.md)

## Why the two are one issue

A spawn record carries two three-byte pointers. They are **scripts, not text**.
Decoding all 189 pointer fields across the slice's records as dialogue yields
five results, each a single page, which is what a coincidentally valid byte
sequence looks like rather than a resident's lines. This is asserted by
`spawn_record_pointers_are_scripts_rather_than_text`.

The known chain for the one fully documented resident shows the shape:
record `$83:8B96` → script `$88:8E4B` → callback `$88:8EDE` → text `$88:8FF0`,
with the progression flag `$0026` written at `$88:8F08`. Text and flag are both
reached *through* the script, so neither dialogue nor progression can be
generalised past the hand-qualified house without running it.

## What is already available

- `assets::text::HouseDialogue::decode_at` decodes the pages at any source
  address, validated against all seven qualified sources. Only the discovery of
  which address to use is missing.
- `assets::maps::actors::SpawnList::resolve` gives the records that apply for a
  given event-flag state, so the caller knows which scripts would run.
- `assets::maps::scripts::EventFlags` already models the `$7E:06C0` bitmap that
  `$80:BBC7` tests, and both the loading scripts and the spawn stream use it.

## Requirements

- Decode the script VM's opcode dispatch from the interpreter, with lengths
  read from the code rather than fitted. The spawn stream showed why: a wrong
  length emits data that looks exactly like valid records.
- Distinguish the callback structure from straight-line script, since the
  documented chain reaches text through a callback rather than inline.
- Evaluate flag writes, at minimum `$0026`, so progression gates can be
  modelled. `$88:8F08` is the known write site.
- Keep refusals fatal, as the spawn decoder does.

## Acceptance Criteria

- A resident's dialogue source is reached from its spawn record without a
  hand-written table, for at least the nine documented residents.
- Flag writes are identified, and the `$0026` gate is reproduced.
- Unaccounted opcodes are refused with their address, not resynchronised.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Make the Crysta slice fully playable](playable-crysta-slice.md)
- This is the last dependency for that slice. Map loading, collision, exits,
  the door mechanism, the played traversal, spawn lists and spawn conditions
  are all complete; what remains is everything that needs a script to run.
- It is comparable in size to the event bytecode work rather than to the spawn
  stream, because a script has control flow and side effects, not just records.

## Progress: the chain runs from the spawn record

Actor scripts are **native 65C816 code**, and their commands are `COP`
signatures dispatched through the table at `$00:83B2`. That is the same
dispatch [the COP table bound](correct-cop-table-bound.md) is about. Each
handler advances the stream pointer `$36` by its own operand length, so lengths
are read from the handlers rather than assumed.

### Reading a handler needs an instruction decoder

Lengths cannot be found by scanning a handler for `INC $36`. `STA $40` contains
`$40`, which is `RTI`; `AND #$0001` contains `$01`; `LDA #$0036` contains `$36`.
A byte scan desynchronises inside operand data and under-reports. `$80:963A` is
the case that exposed it: the scan stopped inside `STA $40` and returned two
where the stream needs four.

`assets::cpu` is a minimal 65C816 length table for exactly this, tracking `M`
and `X` through `SEP`/`REP` because immediate widths depend on them.

### Lengths come from how a handler leaves `$36`

Nor is it enough to walk to the first branch: handlers open with guards.
`$80:8BEB` tests three busy flags before fetching anything. So every path is
explored, and each terminal is classified by how it leaves the stream pointer:

| Terminal | Meaning | Length? |
| --- | --- | --- |
| `LDA $36 (+d) : STA $02,S : RTI` | proceeds; overwrites the dispatcher's resume address | yes, `INC` run + `d` |
| `LDA $36 (+d) : STA $000A,X` | yields into the actor slot's own script pointer | yes |
| `PLA : PLA : RTL` | stalls; `$80:8BEB` rewinds two so the `COP` re-runs | no |
| `LDA [$36] : STA $02,S` | branches; the resume address comes from the stream | no |

The yield target is the actor slot field at offset ten — the same word the
emulator probe reads back out of WRAM to recover a running actor's script.

Handlers loop, so the exploration tolerates back edges: `$80:8C4A` spins on the
text renderer until it reports done, and only then restores `$36`.

### Corrected: `$08` and `$48` are not the same length

| Service | Handler | Operands | Effect |
| --- | --- | ---: | --- |
| `$07` | `$80:8669` | 2 | writes an event flag through `$80:BB77` |
| `$1B` | `$80:8BEB` | 2 | stores a dialogue address to `$0DC2` |
| `$21` | `$80:8CC8` | 2 | registers an interaction callback |
| `$24` | `$80:8D0B` | 3 | |
| `$08` | `$80:8678` | 4 | branches on an event flag: condition + target |
| `$48` | `$80:96CB` | **2** | tests a condition and aborts; **no target** |
| `$1F` | `$80:8C4A` | 0 | runs the text renderer to completion |
| `$3B` | `$80:9301` | 0 | |
| `$C1` | `$80:AB17` | 2 | yields, resuming from the actor slot |
| `$09`, `$47` | `$80:8695`, `$80:963A` | *stream* | chained conditions |

An earlier table recorded `$48` as four operands, on a shape match: it opens
with the same eight bytes of flag test as `$08`. The opening does not carry the
length. The arms do — `$08` reaches `$80:8397`, which adds four, while `$48`
reaches `$80:8390`, which adds two. The stream settles it: `02 48 74 80` is
followed by `02 47`.

### Chained conditions are sized from the stream

`$09` and `$47` test a word, then examine its high nibble: a bit in `$F000`
selects a boolean combinator and pulls in a further condition word. Their length
is a property of the stream, not of the handler, so the handler derivation
refuses them and `chained_condition_length` reads them. This is the same shape
as the spawn stream's `$FA`, where the mask is `$F800`.

### The path from a record to its script

**`pointer + 5`.** The record's pointer field is followed by a five-byte header.
Measured, not fitted: map `$000F`'s `(8,16)` record points at `$88:8038`, and
the running game's actor at that position carries script `$88:803D`, read out of
WRAM while the reference emulator ran.

Across the slice 90 of 115 records land on a `COP` that way and **none** land on
one at the pointer itself, so the offset is not an artefact of where `COP` bytes
happen to fall. The other 25 resolve to neither and are reported, not assumed.

### The documented chain now runs end to end

From the spawn record alone, with no hand-written table:

```text
record $83:8B96 -> script $88:8E50 -> callback $88:8EDE -> text $88:8FF0 (2 pages)
                                                        -> flag $8026 at $88:8F08
```

Those are exactly the addresses `docs/house-dialogue.md` records by hand. The
callback turns out to be a six-way `$08` dispatch over progression state, which
is why the flag was only reachable once branches could be followed.

`walk_with_events` follows those branches against an `EventFlags` bitmap, taking
the same-sense decision the spawn stream uses, and refuses a condition the
supplied flags cannot answer rather than guessing.

## Remaining

- **12 of 115 record scripts** stop at a service whose advance is unaccounted
  for, across six services (`$B0` four times, `$22` three, `$0D`, `$4B`, `$8F`).
  The other 103 walk to the end of their command stream. Effects collected
  before a refusal are complete.

  None of the three commonest are *unknown*; they are stream-dependent in ways
  the handler derivation deliberately refuses to guess at:

  - **`$B0`** (`$80:A975`) reads one byte. If it is `$FF` the handler takes a
    second path that reads two more, so the service is one byte or three
    depending on its own first operand — the same situation as `$09`/`$47`, and
    the cheapest of the three to close.
  - **`$22`** (`$80:8CD8`) is a **jump table**: it reads two bytes, compares the
    second against `$0026,X`, and adds a scaled entry to `$36`. It does not have
    a length so much as a computed destination.
  - **`$0D`** (`$80:87C2`) reads a byte, then branches on both `$0956` and
    `$0966` with arithmetic in between.
- Scripts are native code, so a stream does not have to end at a non-`COP`
  byte — `02 8e 80 f2` is a command followed by a real `BRA`. The walk reports
  that as the end of commands, which is a floor on what a script does, not a
  description of it.
- `operand_length` does not follow `JSR`/`JSL` into a subroutine that fetches on
  the handler's behalf. No service in the slice needs it, but nothing rules it
  out either.
