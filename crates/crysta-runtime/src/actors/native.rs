//! Short native runs between script commands that only use script scratch
//! words and the display: the tour's guide and controller take turns through
//! `$04BC` with `INC`, `LDA`/`CMP` and a branch to `RTL` (`$89:D3E6`,
//! `$89:D2EC`); the freezing's whitening narrows the accumulator, writes PPU
//! registers and calls palette routines in a loop (`$88:B507`).
//!
//! A run starts 16-bit, as the scheduler enters scripts, and stops before the
//! first opcode the script loop owns (`COP`, `RTL`, `JMP`, `BRA`) or another
//! recogniser may take, once its stack is balanced, its accumulator wide
//! and X the actor's again. Anything else -- another address, another call,
//! a register or flag the run has not set -- is refused, so the script freezes
//! as before rather than guessing. A scratch word never written reads 0, as
//! the words start cleared.

use std::collections::BTreeMap;

/// Scratch words by address.
pub type Scratch = BTreeMap<u16, u16>;

/// Words runs may use: scripts' own variables, and one engine word the
/// runtime does not read. With the evidence.
const SCRATCH: [(u16, u16); 3] = [
    // `$89:D2B2` clears `$0440`, `$04BC`, `$04BE`, `$04C0`, `$04C2`.
    (0x0440, 0x0441),
    (0x04BC, 0x04C3),
    // An engine word the runtime does not read: the spear's grant sets and
    // clears its bit 8 (`$89:DA96`, `$89:DA31`); bit 15 places windows.
    (0x048A, 0x048B),
];

/// Instructions one run may take: the freezing's whitening loops 37 times.
const STEPS: usize = 512;

/// Engine routines a run may call, with the evidence: `$8D:A8EA` / `$8D:A8FD`
/// save and restore the palette buffer (`MVN $7F,$7F` between `$0600` and
/// `$0400`), `$8D:AA96` steps it toward white. `$80:80DF` runs a nested
/// frame, in which `$80:C85E` runs the actors with `+$04` bit 12 -- in the
/// whitening, the figure and the particles; not modelled, so their timers
/// do not advance those frames. The calls clobber A and the flags, and
/// `$8D:AA96` also X.
const CALLS: [usize; 4] = [0x0D_A8EA, 0x0D_AA96, 0x0D_A8FD, 0x00_80DF];
/// The call among [`CALLS`] that also clobbers X.
const CLOBBERS_X: usize = 0x0D_AA96;

/// A pushed value: A with its width, or whether X was still the actor's.
#[derive(Clone, Copy)]
enum Pushed {
    A(Option<u16>, bool),
    X(bool),
}

/// Whether an address is a scratch word's: even, so that no two words
/// overlap.
fn scratch(address: u16) -> bool {
    address.is_multiple_of(2)
        && SCRATCH
            .iter()
            .any(|&(first, last)| (first..=last).contains(&address))
}

/// Runs native code at `at` and returns where the script loop takes over, or
/// `None` when the code is not a run this module admits.
pub(super) fn run(image: &[u8], at: usize, words: &mut Scratch) -> Option<usize> {
    let mut machine = Machine {
        image,
        pc: at,
        a: None,
        zero: None,
        negative: None,
        carry: None,
        narrow: false,
        x: true,
        stack: Vec::new(),
    };
    for step in 0..STEPS {
        if !machine.step(words)? {
            let settled = step > 0 && !machine.narrow && machine.x && machine.stack.is_empty();
            return settled.then_some(machine.pc);
        }
    }
    None
}

/// The registers a run uses. A value or flag is `None` until the run sets it:
/// a use before that is refused rather than guessed.
struct Machine<'a> {
    image: &'a [u8],
    pc: usize,
    a: Option<u16>,
    zero: Option<bool>,
    negative: Option<bool>,
    carry: Option<bool>,
    /// `SEP #$20` narrowed the accumulator.
    narrow: bool,
    /// X still holds the actor, as the script loop and the other
    /// recognisers need.
    x: bool,
    /// Values pushed and not yet pulled.
    stack: Vec<Pushed>,
}

impl Machine<'_> {
    /// Executes one instruction. `Some(false)` stops before it, for the
    /// script loop or another recogniser; `None` refuses the run.
    fn step(&mut self, words: &mut Scratch) -> Option<bool> {
        let opcode = *self.image.get(self.pc)?;
        self.pc = match opcode {
            0xE2 | 0xC2 => self.width(opcode)?,
            0x8D if self
                .operand()
                .is_some_and(|address| (0x2100..=0x213F).contains(&address)) =>
            {
                self.a?;
                self.pc + 3
            }
            // Scratch words are words: a narrow accumulator refuses them.
            0x9C | 0x8D | 0xAD | 0xEE | 0xCE | 0xC9 | 0xCD | 0x0C | 0x1C if self.narrow => {
                return None;
            }
            0x9C | 0x8D | 0xAD | 0xEE | 0xCE | 0x0C | 0x1C => self.memory(opcode, words)?,
            0xA9 | 0xC9 | 0xCD | 0x1A | 0x3A => self.accumulator(opcode, words)?,
            0x48 | 0x68 | 0xDA | 0xFA => self.stack_op(opcode)?,
            0xF0 | 0xD0 | 0x90 | 0xB0 | 0x10 | 0x30 => self.branch(opcode)?,
            0x22 => self.call()?,
            // The loop's own (`COP`, `RTL`, `JMP`, `BRA`) or another
            // recogniser's: stop before it.
            _ => return Some(false),
        };
        Some(true)
    }

    fn operand(&self) -> Option<u16> {
        let bytes = self.image.get(self.pc + 1..self.pc + 3)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn address(&self) -> Option<u16> {
        self.operand().filter(|&address| scratch(address))
    }

    fn set(&mut self, value: u16) {
        self.a = Some(value);
        self.zero = Some(value == 0);
        self.negative = Some(value & if self.narrow { 0x80 } else { 0x8000 } != 0);
    }

    /// `SEP` / `REP #$20`: the accumulator's width, and nothing else.
    fn width(&mut self, opcode: u8) -> Option<usize> {
        if *self.image.get(self.pc + 1)? != 0x20 {
            return None;
        }
        self.narrow = opcode == 0xE2;
        if self.narrow {
            self.a = self.a.map(|value| value & 0xFF);
        }
        Some(self.pc + 2)
    }

    /// `STZ`, `STA`, `LDA`, `INC`, `DEC`, `TSB`, `TRB` on a scratch word.
    fn memory(&mut self, opcode: u8, words: &mut Scratch) -> Option<usize> {
        let address = self.address()?;
        match opcode {
            0x9C => {
                words.insert(address, 0);
            }
            0x8D => {
                words.insert(address, self.a?);
            }
            0xAD => {
                let value = words.get(&address).copied().unwrap_or(0);
                self.set(value);
            }
            0xEE | 0xCE => {
                let word = words.entry(address).or_insert(0);
                *word = if opcode == 0xEE {
                    word.wrapping_add(1)
                } else {
                    word.wrapping_sub(1)
                };
                let value = *word;
                self.zero = Some(value == 0);
                self.negative = Some(value & 0x8000 != 0);
            }
            _ => {
                // TSB / TRB: Z from `A & word`, before the write.
                let a = self.a?;
                let word = words.entry(address).or_insert(0);
                self.zero = Some(*word & a == 0);
                *word = if opcode == 0x0C {
                    *word | a
                } else {
                    *word & !a
                };
            }
        }
        Some(self.pc + 3)
    }

    /// `LDA #`, `CMP #`, `CMP` a scratch word, `INC A`, `DEC A`.
    fn accumulator(&mut self, opcode: u8, words: &Scratch) -> Option<usize> {
        match opcode {
            0xA9 => {
                let (value, next) = if self.narrow {
                    (u16::from(*self.image.get(self.pc + 1)?), self.pc + 2)
                } else {
                    (self.operand()?, self.pc + 3)
                };
                self.set(value);
                Some(next)
            }
            0xC9 | 0xCD => {
                let value = if opcode == 0xC9 {
                    self.operand()?
                } else {
                    words.get(&self.address()?).copied().unwrap_or(0)
                };
                let a = self.a?;
                self.carry = Some(a >= value);
                let difference = a.wrapping_sub(value);
                self.zero = Some(difference == 0);
                self.negative = Some(difference & 0x8000 != 0);
                Some(self.pc + 3)
            }
            _ => {
                let mask = if self.narrow { 0xFF } else { 0xFFFF };
                let a = self.a?;
                let value = if opcode == 0x1A {
                    a.wrapping_add(1)
                } else {
                    a.wrapping_sub(1)
                } & mask;
                self.set(value);
                Some(self.pc + 1)
            }
        }
    }

    /// `PHA`, `PLA`, `PHX`, `PLX`, each pulled as it was pushed.
    fn stack_op(&mut self, opcode: u8) -> Option<usize> {
        match opcode {
            0x48 => self.stack.push(Pushed::A(self.a, self.narrow)),
            0x68 => match self.stack.pop()? {
                Pushed::A(value, narrow) if narrow == self.narrow => self.set(value?),
                _ => return None,
            },
            0xDA => self.stack.push(Pushed::X(self.x)),
            _ => match self.stack.pop()? {
                Pushed::X(valid) => self.x = valid,
                Pushed::A(..) => return None,
            },
        }
        Some(self.pc + 1)
    }

    /// `BEQ`, `BNE`, `BCC`, `BCS`, `BPL`, `BMI`, within the bank.
    fn branch(&self, opcode: u8) -> Option<usize> {
        let taken = match opcode {
            0xF0 => self.zero?,
            0xD0 => !self.zero?,
            0x90 => !self.carry?,
            0xB0 => self.carry?,
            0x10 => !self.negative?,
            _ => self.negative?,
        };
        let next = self.pc + 2;
        if !taken {
            return Some(next);
        }
        let displacement = i8::from_ne_bytes([*self.image.get(self.pc + 1)?]);
        Some((self.pc & 0xFF_0000) | (next.wrapping_add_signed(isize::from(displacement)) & 0xFFFF))
    }

    /// `JSL` to one of [`CALLS`]: A and the flags are unknown after it.
    fn call(&mut self) -> Option<usize> {
        let bytes = self.image.get(self.pc + 1..self.pc + 4)?;
        let target =
            usize::from(bytes[0]) | usize::from(bytes[1]) << 8 | usize::from(bytes[2] & 0x7F) << 16;
        let target = target & 0x3F_FFFF;
        if !CALLS.contains(&target) {
            return None;
        }
        self.a = None;
        (self.zero, self.negative, self.carry) = (None, None, None);
        if target == CLOBBERS_X {
            self.x = false;
        }
        Some(self.pc + 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: usize = 0x09_8000;

    fn image(code: &[u8]) -> Vec<u8> {
        let mut image = vec![0; AT];
        image.extend_from_slice(code);
        image
    }

    #[test]
    fn the_controller_steps_the_shared_counter_and_waits_for_its_turn() {
        // INC $04BC; COP BC (the loop's); LDA $04BC; CMP #2; BEQ +1; RTL.
        let code = [
            0xEE, 0xBC, 0x04, 0x02, 0xBC, 0xAD, 0xBC, 0x04, 0xC9, 0x02, 0x00, 0xF0, 0x01, 0x6B,
            0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::new();
        assert_eq!(
            run(&image, AT, &mut words),
            Some(AT + 3),
            "stops at the COP"
        );
        assert_eq!(words.get(&0x04BC), Some(&1));
        assert_eq!(
            run(&image, AT + 5, &mut words),
            Some(AT + 13),
            "not yet: RTL"
        );
        words.insert(0x04BC, 2);
        assert_eq!(run(&image, AT + 5, &mut words), Some(AT + 14), "its turn");
    }

    #[test]
    fn the_guide_clears_its_words_and_loops_back_until_the_count_is_reached() {
        // STZ $0440; STZ $04BC; LDA $04BC; CMP #1; BNE -7 (to the LDA); COP.
        let code = [
            0x9C, 0x40, 0x04, 0x9C, 0xBC, 0x04, 0xAD, 0xBC, 0x04, 0xC9, 0x01, 0x00, 0xD0, 0xF8,
            0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::from([(0x04BC, 7), (0x0440, 3)]);
        assert_eq!(
            run(&image, AT, &mut words),
            None,
            "spins: no COP within budget"
        );
        assert_eq!(words.get(&0x0440), Some(&0));
        words.insert(0x04BC, 1);
        assert_eq!(run(&image, AT + 6, &mut words), Some(AT + 14));
    }

    #[test]
    fn tsb_and_trb_set_and_clear_bits_of_048a() {
        // LDA #$0100; TSB $048A; COP; then LDA #$0100; TRB $048A; COP.
        let code = [
            0xA9, 0x00, 0x01, 0x0C, 0x8A, 0x04, 0x02, 0xA9, 0x00, 0x01, 0x1C, 0x8A, 0x04, 0x02,
        ];
        let image = image(&code);
        let mut words = BTreeMap::from([(0x048A, 0x8000)]);
        assert_eq!(run(&image, AT, &mut words), Some(AT + 6));
        assert_eq!(words.get(&0x048A), Some(&0x8100));
        assert_eq!(run(&image, AT + 7, &mut words), Some(AT + 13));
        assert_eq!(words.get(&0x048A), Some(&0x8000));
    }

    #[test]
    fn a_whitening_loop_runs_through_display_calls_and_hands_back() {
        // PHX; SEP #$20; LDA #$A3; STA $2131; LDA #$02; PHA; JSL $8D:AA96;
        // JSL $80:80DF; PLA; DEC; BPL -13; REP #$20; PLX; LDA $0004,X.
        let code = [
            0xDA, 0xE2, 0x20, 0xA9, 0xA3, 0x8D, 0x31, 0x21, 0xA9, 0x02, 0x48, 0x22, 0x96, 0xAA,
            0x8D, 0x22, 0xDF, 0x80, 0x80, 0x68, 0x3A, 0x10, 0xF3, 0xC2, 0x20, 0xFA, 0xBD, 0x04,
            0x00,
        ];
        let whitening = image(&code);
        let mut words = BTreeMap::new();
        assert_eq!(
            run(&whitening, AT, &mut words),
            Some(AT + 26),
            "at the LDA $0004,X"
        );
        // A call outside the display routines is refused.
        let code = [0x22, 0x00, 0x80, 0x80, 0x02];
        assert_eq!(run(&image(&code), AT, &mut words), None);
    }

    #[test]
    fn a_call_clobbers_a_and_the_flags_and_aa96_x_until_pulled() {
        // LDA #1; JSL $8D:A8EA; BNE: the flags are gone.
        let code = [0xA9, 0x01, 0x00, 0x22, 0xEA, 0xA8, 0x8D, 0xD0, 0x00, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
        // JSL $8D:AA96 without PHX/PLX around it: X is not the actor's.
        let code = [0x22, 0x96, 0xAA, 0x8D, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
        // PHX; JSL $8D:AA96; PLX: hands back.
        let code = [0xDA, 0x22, 0x96, 0xAA, 0x8D, 0xFA, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), Some(AT + 6));
        // A narrow PHA pulled wide is refused.
        let code = [0xE2, 0x20, 0xA9, 0x05, 0x48, 0xC2, 0x20, 0x68, 0x02];
        assert_eq!(run(&image(&code), AT, &mut BTreeMap::new()), None);
    }

    #[test]
    fn other_addresses_widths_and_opcodes_are_refused() {
        let mut words = BTreeMap::new();
        for code in [
            &[0x9C, 0x00, 0x05, 0x02][..], // STZ $0500
            &[0xAD, 0x54, 0x04, 0x02][..], // LDA $0454 (the pad)
            &[0xE2, 0x20, 0x02][..],       // SEP #$20
            &[0x02, 0xBD][..],             // nothing native to run
            &[0xAD, 0xBD, 0x04, 0x02][..], // LDA $04BD: an odd address
            &[0x8D, 0xBC, 0x04, 0x02][..], // STA before A is known
            &[0xD0, 0x00, 0x02][..],       // BNE before the flags are
        ] {
            assert_eq!(run(&image(code), AT, &mut words), None, "{code:02X?}");
        }
    }
}
