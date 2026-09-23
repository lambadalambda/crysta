//! Short native runs between script commands that only use script scratch
//! words: the tour's guide and controller take turns through `$04BC` with
//! `INC`, `LDA`/`CMP` and a branch to `RTL` (`$89:D3E6`, `$89:D2EC`).
//!
//! The run is 16-bit throughout, as the scheduler enters scripts, and stops
//! before the first opcode the script loop owns (`COP`, `RTL`, `JMP`, `BRA`).
//! Anything else -- another address, another width, another opcode -- is
//! refused, so the script freezes as before rather than guessing.

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

/// Instructions one run may take.
const STEPS: usize = 64;

/// Whether an address is a scratch word's: even, so that no two words
/// overlap.
fn scratch(address: u16) -> bool {
    address.is_multiple_of(2)
        && SCRATCH
            .iter()
            .any(|&(first, last)| (first..=last).contains(&address))
}

/// Runs native code at `at` and returns where the script loop takes over, or
/// `None` when the code is not a scratch-word run.
pub(super) fn run(image: &[u8], at: usize, words: &mut Scratch) -> Option<usize> {
    let bank = at & 0xFF_0000;
    let mut pc = at;
    // A and the flags are unknown until the run sets them: a store or a
    // branch before that is refused rather than guessed.
    let mut a: Option<u16> = None;
    let (mut zero, mut negative, mut carry): (Option<bool>, Option<bool>, Option<bool>) =
        (None, None, None);
    for step in 0..STEPS {
        let opcode = *image.get(pc)?;
        let operand = || {
            image
                .get(pc + 1..pc + 3)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        };
        let address = || operand().filter(|&address| scratch(address));
        let mut set = |value: u16| {
            zero = Some(value == 0);
            negative = Some(value & 0x8000 != 0);
        };
        pc = match opcode {
            0x02 | 0x6B | 0x4C | 0x80 => return (step > 0).then_some(pc),
            // STZ / STA / LDA / INC / DEC absolute.
            0x9C => {
                words.insert(address()?, 0);
                pc + 3
            }
            0x8D => {
                words.insert(address()?, a?);
                pc + 3
            }
            0xAD => {
                let value = words.get(&address()?).copied().unwrap_or(0);
                a = Some(value);
                set(value);
                pc + 3
            }
            0xEE | 0xCE => {
                let address = address()?;
                let word = words.entry(address).or_insert(0);
                *word = if opcode == 0xEE {
                    word.wrapping_add(1)
                } else {
                    word.wrapping_sub(1)
                };
                set(*word);
                pc + 3
            }
            // TSB / TRB absolute: Z from `A & word`.
            0x0C | 0x1C => {
                let (address, a) = (address()?, a?);
                let word = words.entry(address).or_insert(0);
                zero = Some(*word & a == 0);
                *word = if opcode == 0x0C {
                    *word | a
                } else {
                    *word & !a
                };
                pc + 3
            }
            // LDA / CMP immediate, CMP absolute.
            0xA9 => {
                let value = operand()?;
                a = Some(value);
                set(value);
                pc + 3
            }
            0xC9 | 0xCD => {
                let value = if opcode == 0xC9 {
                    operand()?
                } else {
                    words.get(&address()?).copied().unwrap_or(0)
                };
                let a = a?;
                carry = Some(a >= value);
                set(a.wrapping_sub(value));
                pc + 3
            }
            // BEQ / BNE / BCC / BCS / BPL / BMI.
            0xF0 | 0xD0 | 0x90 | 0xB0 | 0x10 | 0x30 => {
                let taken = match opcode {
                    0xF0 => zero?,
                    0xD0 => !zero?,
                    0x90 => !carry?,
                    0xB0 => carry?,
                    0x10 => !negative?,
                    _ => negative?,
                };
                let displacement = i8::from_ne_bytes([*image.get(pc + 1)?]);
                let next = pc + 2;
                if taken {
                    bank | (next.wrapping_add_signed(displacement as isize) & 0xFFFF)
                } else {
                    next
                }
            }
            _ => return None,
        };
    }
    None
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
