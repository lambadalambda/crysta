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

    /// Whether slot `slot`'s primary copy is valid (`$8D:A82E`).
    #[must_use]
    pub fn valid(&self, slot: usize) -> bool {
        slot < SLOTS && valid(self.stored(PRIMARY, slot)).is_some()
    }

    /// The slot list's pass (`$87:CB4B`): a valid primary is copied over
    /// its backup, a bad one is restored from the backup; `$1FFE` becomes 0
    /// when no slot is valid, else its low two bits. Returns which slots
    /// are valid.
    pub fn repair(&mut self) -> [bool; SLOTS] {
        let valid = std::array::from_fn(|slot| {
            let (primary, backup) = (offset(PRIMARY, slot), offset(BACKUP, slot));
            if !self.valid(slot) {
                self.bytes.copy_within(backup..backup + STORED, primary);
            }
            let valid = self.valid(slot);
            if valid {
                self.bytes.copy_within(primary..primary + STORED, backup);
            }
            valid
        });
        let last = if valid.contains(&true) {
            word(&self.bytes[..], LAST) & 3
        } else {
            0
        };
        set_word(&mut self.bytes[..], LAST, last);
        valid
    }

    /// The SRAM after [`Self::repair`].
    #[must_use]
    pub fn repaired(mut self) -> Self {
        self.repair();
        self
    }

    /// Where the Restart screen's cursor starts: `$1FFE`'s low two bits,
    /// which after [`Self::repair`] may point at New Game (3).
    #[must_use]
    pub fn cursor(&self) -> u8 {
        (word(&self.bytes[..], LAST) & 3) as u8
    }

    /// Copy Data (`$8D:A7A5`): a valid `source` over `destination`'s
    /// primary with its checksums, then over its backup; `$1FFE` stays.
    /// Returns whether it copied.
    pub fn copy_slot(&mut self, source: usize, destination: usize) -> bool {
        if !self.valid(source) || destination >= SLOTS {
            return false;
        }
        let (from, to) = (offset(PRIMARY, source), offset(PRIMARY, destination));
        self.bytes.copy_within(from..from + STORED, to);
        let backup = offset(BACKUP, destination);
        self.bytes.copy_within(to..to + STORED, backup);
        true
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
    fn the_slot_list_repairs_copies_and_masks_as_natively() {
        let mut sram = Sram::default();
        sram.write_slot(0, &slot(0x0F));
        sram.write_slot(2, &slot(0x10));
        // A bad primary is restored from its backup.
        sram.bytes[PRIMARY + 9] ^= 0xFF;
        // A good primary is copied over a bad backup.
        sram.bytes[BACKUP + 2 * STRIDE + 9] ^= 0xFF;
        set_word(&mut sram.bytes[..], LAST, 0x0106);
        assert_eq!(sram.repair(), [true, false, true]);
        assert_eq!(
            sram.bytes[PRIMARY..PRIMARY + STORED],
            sram.bytes[BACKUP..BACKUP + STORED]
        );
        let (third, backup) = (offset(PRIMARY, 2), offset(BACKUP, 2));
        assert_eq!(
            sram.bytes[third..third + STORED],
            sram.bytes[backup..backup + STORED]
        );
        assert_eq!(sram.cursor(), 2, "`$1FFE` masked to two bits");
        set_word(&mut sram.bytes[..], LAST, 0x0107);
        sram.repair();
        assert_eq!(sram.cursor(), 3, "New Game");
        // No slot: the cursor on slot 1.
        assert_eq!(
            Sram::from_bytes(&[0xFF; SRAM_BYTES])
                .unwrap()
                .repaired()
                .cursor(),
            0
        );
        // Copy: slot 1 over the empty slot 2, primary and backup; `$1FFE` stays.
        assert!(sram.copy_slot(0, 1));
        assert_eq!(sram.slot(1), Some(slot(0x0F)));
        let (second, backup) = (offset(PRIMARY, 1), offset(BACKUP, 1));
        assert_eq!(
            sram.bytes[second..second + STORED],
            sram.bytes[backup..backup + STORED]
        );
        assert_eq!(sram.cursor(), 3);
        // An erased slot repairs to both copies broken, starting `FF FE`.
        sram.erase(1);
        assert_eq!(sram.repair(), [true, false, true]);
        assert_eq!(&sram.bytes[second..second + 2], &[0xFF, 0xFE]);
        assert!(!sram.copy_slot(1, 2), "a bad source copies nothing");
        assert!(!sram.valid(1) && sram.valid(0));
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
