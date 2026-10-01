//! Treasure chests (`docs/chests.md`): map cells, not sprites, and their
//! contents table (`$96:D10F`, European `$99:D859`), a word per map
//! pointing to a list in the same bank ended by `$FF`.

use crate::layout::per_revision;

/// A chest's closed cell (low nine bits).
pub const CLOSED: u16 = 0x0F0;
/// An opened chest's cell.
pub const OPEN: u16 = 0x0F1;

/// What a chest holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contents {
    /// Nothing ("Darn! Empty!!!").
    Empty,
    /// Gems, the amount in decimal (stored BCD).
    Gems(u16),
    /// An item.
    Item(u8),
}

impl Contents {
    fn decode(word: u16) -> Self {
        match word {
            0 => Self::Empty,
            word if word & 0x8000 != 0 => Self::Gems(bcd(word & 0x7FFF)),
            word => Self::Item(word.to_le_bytes()[0]),
        }
    }
}

/// When a chest reacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// Always.
    None,
    /// Only with this flag set.
    Flag(u16),
    /// Always, holding the other contents with this flag set.
    Swap(u16, Contents),
}

/// One chest of a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chest {
    /// Its cell: column and row, 16-pixel cells.
    pub cell: (u8, u8),
    /// The flags byte: bits 3-4 nonzero play the fanfare.
    pub flags: u8,
    /// When it reacts.
    pub condition: Condition,
    /// What it holds.
    pub contents: Contents,
    /// The flag set once opened, `$500 + n`.
    pub opened: u16,
}

impl Chest {
    /// Whether opening it plays the fanfare.
    #[must_use]
    pub const fn fanfare(&self) -> bool {
        self.flags & 0x18 != 0
    }

    /// What it holds with the event flags `set` answers.
    #[must_use]
    pub fn holds(&self, set: impl Fn(u16) -> bool) -> Option<Contents> {
        match self.condition {
            Condition::None => Some(self.contents),
            Condition::Flag(flag) => set(flag).then_some(self.contents),
            Condition::Swap(flag, other) => Some(if set(flag) { other } else { self.contents }),
        }
    }
}

/// The chests of `map`, in table order; none for a map outside the table.
#[must_use]
pub fn chests(image: &[u8], map: u16) -> Vec<Chest> {
    let offset: usize = per_revision(image, 0x16_D10F, 0x19_D859);
    let word = |at: usize| {
        image
            .get(at..at + 2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
    };
    let Some(list) = word(offset + usize::from(map) * 2) else {
        return Vec::new();
    };
    let mut at = (offset & 0xFF_0000) | usize::from(list);
    let mut chests = Vec::new();
    while let Some(&[column, row, flags, ..]) = image.get(at..at + 3) {
        if column == 0xFF {
            break;
        }
        let (condition, length) = match flags & 0xC0 {
            0 => (Condition::None, 7),
            0x40 => {
                let (Some(flag), Some(other)) = (word(at + 3), word(at + 7)) else {
                    break;
                };
                (Condition::Swap(flag, Contents::decode(other)), 11)
            }
            _ => match word(at + 3) {
                Some(flag) => (Condition::Flag(flag), 9),
                None => break,
            },
        };
        let first = if length == 7 { at + 3 } else { at + 5 };
        let (Some(contents), Some(opened)) = (word(first), word(at + length - 2)) else {
            break;
        };
        chests.push(Chest {
            cell: (column, row),
            flags,
            condition,
            contents: Contents::decode(contents),
            opened: 0x500 + opened,
        });
        at += length;
    }
    chests
}

/// A BCD word's value.
fn bcd(word: u16) -> u16 {
    (0..4)
        .rev()
        .fold(0, |value, digit| value * 10 + ((word >> (digit * 4)) & 0xF))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_words_read_as_decimal() {
        assert_eq!((bcd(0x0030), bcd(0x0044), bcd(0x1234)), (30, 44, 1234));
    }

    #[test]
    fn the_contents_word_holds_gems_items_or_nothing() {
        assert_eq!(Contents::decode(0x8030), Contents::Gems(30));
        assert_eq!(Contents::decode(0x0010), Contents::Item(0x10));
        assert_eq!(Contents::decode(0), Contents::Empty);
    }
}
