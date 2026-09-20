//! Minimal 65C816 instruction lengths, for walking handlers instruction-aligned.
//!
//! Deriving a script service's operand length means reading its handler, and a
//! handler cannot be read by scanning bytes: `STA $40` contains `$40`, which is
//! `RTI`, and `LDA #$0036` contains `$36`. Every earlier attempt to find
//! `INC $36` runs by byte search mis-read operand data as opcodes.
//!
//! Immediate operands are one or two bytes depending on the `M` and `X` status
//! flags, so a walk has to track `SEP`/`REP`. This does not model `PLP` or
//! `XCE`, which leave the widths in a state it cannot know; a caller that walks
//! real code has to treat those as the end of what it understands.

/// Instruction length in bytes, indexed by opcode, with immediates counted as
/// their eight-bit form.
const BASE: [u8; 256] = [
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    3, 2, 4, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    1, 2, 2, 2, 3, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 4, 3, 3, 4,
    1, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    2, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, 2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4,
];
/// Opcodes whose immediate widens with a sixteen-bit accumulator.
const ACCUMULATOR_IMMEDIATE: [u8; 8] = [9, 41, 73, 105, 137, 169, 201, 233];
/// Opcodes whose immediate widens with sixteen-bit index registers.
const INDEX_IMMEDIATE: [u8; 4] = [160, 162, 192, 224];

/// Register widths a walk carries between instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Widths {
    /// Accumulator is sixteen bits wide (`M` clear).
    pub accumulator16: bool,
    /// Index registers are sixteen bits wide (`X` clear).
    pub index16: bool,
}

impl Widths {
    /// The state COP handlers are entered in: both registers sixteen bits.
    ///
    /// `$00:83B2`'s dispatcher reaches its handlers with `M` and `X` clear,
    /// which is why every handler opens `TYX : LDA [$36]` and reads a word.
    #[must_use]
    pub const fn native() -> Self {
        Self {
            accumulator16: true,
            index16: true,
        }
    }
}

/// Length of the instruction at `opcode` under `widths`.
#[must_use]
pub fn instruction_length(opcode: u8, widths: Widths) -> usize {
    let mut length = usize::from(BASE[opcode as usize]);
    if widths.accumulator16 && ACCUMULATOR_IMMEDIATE.contains(&opcode) {
        length += 1;
    }
    if widths.index16 && INDEX_IMMEDIATE.contains(&opcode) {
        length += 1;
    }
    length
}

/// Advances one instruction, applying `SEP`/`REP` to `widths`.
///
/// Returns the offset of the next instruction, or `None` past the end of the
/// image.
pub fn step(image: &[u8], at: usize, widths: &mut Widths) -> Option<usize> {
    let opcode = *image.get(at)?;
    let length = instruction_length(opcode, *widths);
    // Bounds first: a refused step must not leave the caller's widths changed.
    image.get(at + length - 1)?;
    // SEP sets the flag bits it names, narrowing; REP clears them, widening.
    // The length above is unaffected, both being two bytes either way.
    if matches!(opcode, 0xE2 | 0xC2) {
        let operand = *image.get(at + 1)?;
        let wide = opcode == 0xC2;
        if operand & 0x20 != 0 {
            widths.accumulator16 = wide;
        }
        if operand & 0x10 != 0 {
            widths.index16 = wide;
        }
    }
    Some(at + length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implied_instructions_are_one_byte() {
        for opcode in [0xEA, 0x60, 0x6B, 0x40, 0xBB, 0x18, 0x1A, 0x3A, 0x48, 0x68] {
            assert_eq!(instruction_length(opcode, Widths::native()), 1);
        }
    }

    #[test]
    fn immediates_follow_the_register_widths() {
        let narrow = Widths {
            accumulator16: false,
            index16: false,
        };
        // LDA #, AND #, BIT #: accumulator width.
        for opcode in [0xA9, 0x29, 0x89, 0x69] {
            assert_eq!(instruction_length(opcode, Widths::native()), 3);
            assert_eq!(instruction_length(opcode, narrow), 2);
        }
        // LDX #, LDY #, CPX #, CPY #: index width, unaffected by the accumulator.
        for opcode in [0xA2, 0xA0, 0xE0, 0xC0] {
            assert_eq!(instruction_length(opcode, Widths::native()), 3);
            assert_eq!(instruction_length(opcode, narrow), 2);
            assert_eq!(
                instruction_length(
                    opcode,
                    Widths {
                        accumulator16: false,
                        index16: true
                    }
                ),
                3
            );
        }
        // SEP/REP take an eight-bit operand whatever the widths.
        for opcode in [0xE2, 0xC2, 0x02, 0x00] {
            assert_eq!(instruction_length(opcode, Widths::native()), 2);
            assert_eq!(instruction_length(opcode, narrow), 2);
        }
    }

    #[test]
    fn addressing_modes_have_their_documented_lengths() {
        let w = Widths::native();
        assert_eq!(instruction_length(0xA5, w), 2, "LDA dp");
        assert_eq!(instruction_length(0xA7, w), 2, "LDA [dp]");
        assert_eq!(instruction_length(0xAD, w), 3, "LDA abs");
        assert_eq!(instruction_length(0xBD, w), 3, "LDA abs,X");
        assert_eq!(instruction_length(0xAF, w), 4, "LDA long");
        assert_eq!(instruction_length(0xBF, w), 4, "LDA long,X");
        assert_eq!(instruction_length(0x22, w), 4, "JSL long");
        assert_eq!(instruction_length(0x5C, w), 4, "JML long");
        assert_eq!(instruction_length(0x20, w), 3, "JSR abs");
        assert_eq!(instruction_length(0x4C, w), 3, "JMP abs");
        assert_eq!(instruction_length(0x82, w), 3, "BRL rel16");
        assert_eq!(instruction_length(0x62, w), 3, "PER rel16");
        assert_eq!(instruction_length(0x44, w), 3, "MVP");
        assert_eq!(instruction_length(0x54, w), 3, "MVN");
        assert_eq!(instruction_length(0xF4, w), 3, "PEA");
        assert_eq!(instruction_length(0xD4, w), 2, "PEI");
        assert_eq!(instruction_length(0x83, w), 2, "STA sr,S");
    }

    #[test]
    fn sep_and_rep_change_the_widths_they_name() {
        let mut widths = Widths::native();
        // SEP #$20 narrows the accumulator only.
        assert_eq!(step(&[0xE2, 0x20], 0, &mut widths), Some(2));
        assert!(!widths.accumulator16 && widths.index16);
        // SEP #$10 then narrows the index registers too.
        assert_eq!(step(&[0xE2, 0x10], 0, &mut widths), Some(2));
        assert!(!widths.accumulator16 && !widths.index16);
        // REP #$30 widens both back.
        assert_eq!(step(&[0xC2, 0x30], 0, &mut widths), Some(2));
        assert_eq!(widths, Widths::native());
    }

    #[test]
    fn stepping_refuses_to_run_past_the_image() {
        let mut widths = Widths::native();
        // LDA long needs four bytes and only three are present.
        assert_eq!(step(&[0xAF, 0x00, 0x00], 0, &mut widths), None);
        assert_eq!(step(&[], 0, &mut widths), None);
    }

    #[test]
    fn a_hand_decoded_handler_walks_to_its_instruction_boundaries() {
        // `$80:963A`, the COP $47 handler, whose operand bytes include `$40`
        // (RTI) and `$36`. A byte scan stops inside `STA $40`; an
        // instruction-aligned walk reaches the branch that follows it.
        //   TYX / LDA [$36] / INC $36 / INC $36 / STA $3E / JSR $BBA6
        //   ROL A / AND #$0001 / STA $40 / LDA $3E / BMI
        let handler = [
            0xBB, 0xA7, 0x36, 0xE6, 0x36, 0xE6, 0x36, 0x85, 0x3E, 0x20, 0xA6, 0xBB, 0x2A, 0x29,
            0x01, 0x00, 0x85, 0x40, 0xA5, 0x3E, 0x30, 0x70,
        ];
        let mut widths = Widths::native();
        let mut at = 0;
        let mut starts = vec![];
        while at < handler.len() {
            starts.push(at);
            at = step(&handler, at, &mut widths).expect("handler must decode");
        }
        assert_eq!(starts, vec![0, 1, 3, 5, 7, 9, 12, 13, 16, 18, 20]);
        assert_eq!(at, handler.len(), "the walk must land exactly on the end");
    }
}
