//! The native 8 KiB SRAM ([saves](../../../docs/saves.md)): three slots from
//! `$0100` every `$500`, each a [`SaveSlot`] and its checksum words, a backup
//! of each from `$1100`, and the last slot saved at `$1FFE`.
//!
//! The checksum (`$8D:A867`) runs over the slot's `$27D` words: sum and xor
//! from `$5236`, the sum wrapping. A slot reads from its primary copy, else
//! from its backup, as the file select (`$87:CB50`) restores it; a slot with
//! neither valid holds no data. Reading changes nothing.

use crate::save::{SaveSlot, SLOT_BYTES};
use std::fmt;

/// Bytes of the SRAM.
pub const SRAM_BYTES: usize = 0x2000;
/// The slots.
pub const SLOTS: usize = 3;
const PRIMARY: usize = 0x100;
const BACKUP: usize = 0x1100;
const STRIDE: usize = 0x500;
/// A slot with its checksum words.
const STORED: usize = SLOT_BYTES + 4;
const SEED: u16 = 0x5236;
const LAST: usize = 0x1FFE;
/// What the erase (`$87:83F7`) writes over the primary's and the backup's
/// map word.
const ERASED: (u16, u16) = (0xFFFE, 0xFEFF);

/// SRAM that is not 8 KiB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SramSize(pub usize);

impl fmt::Display for SramSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native SRAM is {SRAM_BYTES} bytes, not {}", self.0)
    }
}

impl std::error::Error for SramSize {}

/// The native SRAM.
#[derive(Clone, PartialEq, Eq)]
pub struct Sram {
    bytes: Box<[u8; SRAM_BYTES]>,
}

impl fmt::Debug for Sram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sram")
            .field("last", &self.last_slot())
            .finish_non_exhaustive()
    }
}

impl Default for Sram {
    /// Blank SRAM: every slot empty, as a new cartridge's zeros read.
    fn default() -> Self {
        Self {
            bytes: Box::new([0; SRAM_BYTES]),
        }
    }
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn set_word(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

/// Where a copy of a slot starts.
fn offset(base: usize, slot: usize) -> usize {
    base + slot * STRIDE
}

/// The checksum words of a slot's data.
#[must_use]
pub fn checksum(data: &[u8; SLOT_BYTES]) -> (u16, u16) {
    data.chunks_exact(2)
        .map(|pair| word(pair, 0))
        .fold((SEED, SEED), |(sum, xor), word| {
            (sum.wrapping_add(word), xor ^ word)
        })
}

/// A stored copy's data, when its checksum holds.
fn valid(stored: &[u8]) -> Option<SaveSlot> {
    let data: &[u8; SLOT_BYTES] = stored[..SLOT_BYTES].try_into().ok()?;
    (checksum(data) == (word(stored, SLOT_BYTES), word(stored, SLOT_BYTES + 2)))
        .then(|| SaveSlot::from_bytes(data))
}

impl Sram {
    /// SRAM from its bytes, as a `.srm` file holds them.
    ///
    /// # Errors
    /// Refuses anything but 8 KiB.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SramSize> {
        let bytes: [u8; SRAM_BYTES] = bytes.try_into().map_err(|_| SramSize(bytes.len()))?;
        Ok(Self {
            bytes: Box::new(bytes),
        })
    }

    /// The SRAM's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8; SRAM_BYTES] {
        &self.bytes
    }

    fn stored(&self, base: usize, slot: usize) -> &[u8] {
        let at = offset(base, slot);
        &self.bytes[at..at + STORED]
    }

    /// Slot `slot` (0..3): its primary copy, else its backup; `None` when
    /// neither is valid.
    #[must_use]
    pub fn slot(&self, slot: usize) -> Option<SaveSlot> {
        if slot >= SLOTS {
            return None;
        }
        valid(self.stored(PRIMARY, slot)).or_else(|| valid(self.stored(BACKUP, slot)))
    }

    /// Writes a slot as `$8D:A6FB` does: the data and its checksum to the
    /// primary copy and the backup, and the slot as the last saved.
    ///
    /// # Panics
    /// If `slot` is not 0, 1 or 2.
    pub fn write_slot(&mut self, slot: usize, data: &SaveSlot) {
        assert!(slot < SLOTS, "no slot {slot}");
        let (sum, xor) = checksum(data.bytes());
        for base in [PRIMARY, BACKUP] {
            let at = offset(base, slot);
            self.bytes[at..at + SLOT_BYTES].copy_from_slice(data.bytes());
            set_word(&mut self.bytes[..], at + SLOT_BYTES, sum);
            set_word(&mut self.bytes[..], at + SLOT_BYTES + 2, xor);
        }
        set_word(&mut self.bytes[..], LAST, u16::try_from(slot).unwrap_or(0));
    }

    /// Erases a slot as `$87:83F7` does, breaking both copies' checksums.
    ///
    /// # Panics
    /// If `slot` is not 0, 1 or 2.
    pub fn erase(&mut self, slot: usize) {
        assert!(slot < SLOTS, "no slot {slot}");
        for (base, word) in [(PRIMARY, ERASED.0), (BACKUP, ERASED.1)] {
            set_word(&mut self.bytes[..], offset(base, slot), word);
        }
    }

    /// The last slot saved (`$1FFE`), where the file select's cursor
    /// starts; the first slot when the word names none.
    #[must_use]
    pub fn last_slot(&self) -> usize {
        let last = usize::from(word(&self.bytes[..], LAST));
        if last < SLOTS {
            last
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(map: u16) -> SaveSlot {
        let mut slot = SaveSlot::default();
        slot.set_place(map, 1, (472, 176));
        slot
    }

    #[test]
    fn a_written_slot_reads_back_from_either_copy() {
        let mut sram = Sram::default();
        assert!(
            (0..SLOTS).all(|n| sram.slot(n).is_none()),
            "zeros hold no data"
        );
        sram.write_slot(1, &slot(0x0F));
        assert_eq!(sram.slot(1), Some(slot(0x0F)));
        assert_eq!(sram.last_slot(), 1);
        // A valid primary wins over a different valid backup.
        let mut backup = Sram::default();
        backup.write_slot(1, &slot(0x10));
        let mut mixed = sram.clone();
        let at = offset(BACKUP, 1);
        mixed.bytes[at..at + STORED].copy_from_slice(&backup.bytes[at..at + STORED]);
        assert_eq!(mixed.slot(1), Some(slot(0x0F)));
        // A corrupt primary reads from its backup; both corrupt, nothing.
        let mut corrupt = sram.clone();
        corrupt.bytes[PRIMARY + STRIDE + 7] ^= 0xFF;
        assert_eq!(corrupt.slot(1), Some(slot(0x0F)));
        corrupt.bytes[BACKUP + STRIDE + 7] ^= 0xFF;
        let before = corrupt.clone();
        assert_eq!(corrupt.slot(1), None);
        assert_eq!(corrupt, before, "reading changes nothing");
    }

    #[test]
    fn the_checksum_seeds_both_words_and_an_erase_breaks_both_copies() {
        let data = SaveSlot::default();
        assert_eq!(checksum(data.bytes()), (SEED, SEED));
        let mut sram = Sram::default();
        sram.write_slot(0, &slot(0x0A));
        sram.write_slot(2, &slot(0x0F));
        sram.erase(0);
        assert_eq!(sram.slot(0), None);
        assert_eq!(word(sram.bytes(), PRIMARY), 0xFFFE);
        assert_eq!(word(sram.bytes(), BACKUP), 0xFEFF);
        assert_eq!(sram.last_slot(), 2, "an erase keeps the last slot");
        assert_eq!(sram.slot(3), None);
        assert_eq!(Sram::from_bytes(&[0; 16]), Err(SramSize(16)));
        // A blank cartridge's dump, `$FF` over the slots: no data, the
        // cursor on the first slot.
        let blank = Sram::from_bytes(&[0xFF; SRAM_BYTES]).unwrap();
        assert!((0..SLOTS).all(|n| blank.slot(n).is_none()));
        assert_eq!(blank.last_slot(), 0);
    }
}
