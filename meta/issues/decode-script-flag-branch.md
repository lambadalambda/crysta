# Decode the loading script's event-flag branch

## Summary

The map-loading script projection rejects sub-opcode `$08 FD`, and that single
gap stops **21 of the 24 Crysta maps** from resolving. Decode the instruction
from the interpreter and support it, so map loading stops being per-map
allowlisted work.

## Dependencies

- [Resolve map loading scripts and qualify additional layers](resolve-map-loading-scripts.md)

## Evidence

The interpreter's sub-opcode dispatch is a compare chain at `$86:8C17`. `$FD`
routes to `$86:907D`:

```text
$86:907D  REP #$20
          LDA [$62],Y          ; condition word
          BMI  set_path
          AND #$7FFF
          JSL  $80BBC7         ; carry = event flag set
          BCS  skip            ; clear-sense: taken when the flag is clear
          BRA  take
set_path: AND #$7FFF
          JSL  $80BBC7
          BCS  take            ; set-sense: taken when the flag is set
skip:     SEP #$20 : INY x4 : RTS        ; skip condition and target
take:     SEP #$20 : INY x2 : JSR $902C  ; jump through the subscript table
```

`$80:BBC7` wraps `$80:BBA6`, which is the event-flag test:

```text
$80:BBA6  PHX : STA $3E : AND #$0007 : TAX
          LDA $3E : AND #$0FFF : LSR x3 : TAY
          SEP #$20
          LDA $80BBD3,X        ; bit mask table $01,$02,$04,$08,$10,$20,$40,$80
          AND $06C0,Y          ; the event flag bitmap
          SEC : BNE +1 : CLC
          PLX : REP #$20 : RTS
```

So the instruction is six bytes, `08 FD <condition:u16> <subscript:u16>`:

- flag index is `condition & $0FFF`, addressing `$7E:06C0` bit-wise, the same
  bitmap the existing probes already read;
- bit 15 of `condition` selects the sense — set means branch-if-set, clear
  means branch-if-clear;
- taken jumps through the subscript table, exactly as `$08 FF` does; not taken
  skips both operand words.

A live instance is `$98:85B1`, `08 fd ac 81 cf 00`: flag 428, branch-if-set,
target subscript `$00CF`.

## Requirements

- Support `$08 FD` in the projection as a typed branch, retaining its exact
  operand bytes like every other instruction.
- Evaluate it against caller-supplied event flags rather than guessing, and
  refuse a flag the supplied bitmap does not cover rather than reading it as
  clear. An all-clear default must be named as such: the measured new-game
  state sets flags 32 and 251, so "fresh" would be a false label.
- Do not change the resolved path of any script that contains no `$FD`.
- Keep the existing bounds and refusals; a branch must not enable unbounded
  looping.
- Reject a target subscript outside the qualified table prefix.

## Acceptance Criteria

- `resolve_map` resolves the Crysta maps that previously failed on opcode 253,
  and the count is recorded.
- Scripts without `$FD` resolve to byte-identical programs before and after.
- Both branch senses and both flag states are covered by tests, including
  operand-byte retention and an out-of-range target.
- `docs/map-scripts.md` records the instruction and the flag encoding.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- Parent: [Make the Crysta slice fully playable](playable-crysta-slice.md)
- Found while sizing that parent: 21 of 24 maps failed on this one opcode.
- The flag bitmap at `$7E:06C0` is already read by the qualification probes,
  so the portable core can supply real flags rather than a fresh-game stub once
  progression is wired.

## Qualification

Done. The instruction is implemented as `Command::BranchOnEventFlag`, evaluated
against `EventFlags`.

**Result.** The Crysta slice goes from 3 of 24 maps resolving to **24 of 24**,
and the whole map table from **557 of 1,104 to 900**. The 30 flags branched on
across the table span 35..=663.

**Additivity, proven rather than argued.** An independent review built the
pre-change and post-change resolvers side by side over all 1,104 map IDs: 557
programs byte-identical, 0 changed, 0 regressions, and every one of the 343
newly resolving maps had previously failed with exactly
`Unsupported { opcode: 0xFD }`. The `length >= 4` change to operand extraction
cannot alter anything, since lengths are only 2, 4 and 6 and both 4 and 6 read
the same operand slice.

**Decode confirmed independently.** The reviewer re-disassembled `$86:907D`,
`$80:BBC7` and `$80:BBA6` from the ROM and confirmed the branch sense, the
six-byte length and the flag arithmetic. Two corroborations worth keeping:
`$902C` reads the target word itself, which is why the instruction is six bytes
and not four; and `crates/map-inspector/src/new_game.rs` already derives the
identical flag encoding from a different routine, COP 07 via `$80:BB77`.

Sense is also checked against real data rather than only the disassembly: on an
all-clear bitmap the taken branch is flag 35 branch-if-clear, while flag 428
branch-if-set is not taken. Under an inverted sense every Crysta map would jump
to the late-game subscript from a standing start.

**Fail-closed on coverage.** A flag past the end of a supplied bitmap is
refused rather than read as clear. Map `$0176` is the only table entry
branching above 511 (663), beyond the documented 64-byte event block, so a
caller supplying a faithful 64-byte bitmap is told rather than silently given
the fall-through.

**Naming corrected during review.** The all-clear default was initially called
`fresh` and described as the new-game projection. The repository's own measured
evidence contradicts that: a new game sets flags 32 and 251. No branch in the
table references either, so behaviour was unaffected, but the claim was wrong
and is now `EventFlags::AllClear` with the difference documented.

**Verification.** 19 `map_scripts` tests covering both senses against both flag
states, six-byte operand retention, index masking, an out-of-range target, an
uncovered flag, and flag-free scripts resolving identically for any bitmap.
`cargo test --workspace` 561 pass, fmt, clippy and tracker clean.
