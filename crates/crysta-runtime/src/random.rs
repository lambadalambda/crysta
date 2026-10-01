//! The game's one random generator (`$86:8236`, `COP 25`;
//! `docs/enemy-scripts.md`): 16 bytes at `$0408..$0417`, shared by every
//! script, the drop roll and the critical roll.

/// The generator's 16 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Random([u8; 16]);

impl Random {
    /// One step: each byte from the top down takes the sum of itself and
    /// the one above with the carry, then the last byte counts up.
    pub fn step(&mut self) {
        let bytes = &mut self.0;
        let mut carry = 0;
        for at in (1..16).rev() {
            let sum = u16::from(bytes[at]) + u16::from(bytes[at - 1]) + carry;
            bytes[at - 1] = sum.to_le_bytes()[0];
            carry = sum >> 8;
        }
        for at in (0..16).rev() {
            bytes[at] = bytes[at].wrapping_add(1);
            if bytes[at] != 0 {
                break;
            }
        }
    }

    /// `$0408` as a word, as scripts read it.
    #[must_use]
    pub const fn word(&self) -> u16 {
        u16::from_le_bytes([self.0[0], self.0[1]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_carry_down_and_count_up() {
        let mut random = Random::default();
        random.step();
        assert_eq!((random.word(), random.0[15]), (0, 1));
        random.step();
        // The 1 in the last byte runs down every byte above it.
        assert_eq!((random.word(), random.0[15]), (0x0101, 2));
        let mut wrap = Random([0xFF; 16]);
        wrap.step();
        // Each byte above takes 255 + 254 + 1; the count wraps into r[14].
        assert_eq!((wrap.0[0], wrap.0[14], wrap.0[15]), (0xFE, 0xFF, 0));
    }
}
