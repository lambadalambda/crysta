//! Collision probe table used by `COP CA` and the motion resolver's helpers.
//!
//! At `$84:8E6C`, `COP CA` samples the player's right edge. A nonzero table
//! byte makes it bypass a conditional branch to the accelerated-action entry
//! `$84:90F3`, continuing ordinary `COP 61` stream selection at `$84:8E76`.
//! **It does not refuse ordinary walking.** Both paths are traced in
//! `docs/collision.md`.
//!
//! This module decodes the 32-byte table at `$80:E85C` and the `COP CA`
//! attribute lookup, not sampling geometry or the motion resolver at `$80:D107`.
//! That resolver has directional/pair dispatch and slope handlers for 6/7;
//! its `$80:E849` helper also reads this table, without CA's dynamic-bit override.

use std::fmt;

/// Normalized offset of the table, `$80:E85C`.
pub const TABLE_OFFSET: usize = 0x00_E85C;
/// Entries in the table: one per five-bit attribute.
pub const TABLE_LEN: usize = 32;

/// The collision probe's table, as the ROM holds it; not a passability map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeTable {
    entries: [u8; TABLE_LEN],
}

/// The image is too short to hold the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableUnavailable {
    /// Bytes the image holds.
    pub image_len: usize,
}

impl fmt::Display for TableUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "image of {} bytes does not reach the admission table at {TABLE_OFFSET:#x}",
            self.image_len
        )
    }
}

impl std::error::Error for TableUnavailable {}

impl ProbeTable {
    /// Reads the table from a normalized ROM image.
    ///
    /// # Errors
    /// Refuses an image that does not reach the table.
    pub fn from_rom(image: &[u8]) -> Result<Self, TableUnavailable> {
        let bytes = image
            .get(TABLE_OFFSET..TABLE_OFFSET + TABLE_LEN)
            .ok_or(TableUnavailable {
                image_len: image.len(),
            })?;
        let mut entries = [0u8; TABLE_LEN];
        entries.copy_from_slice(bytes);
        Ok(Self { entries })
    }

    /// The table's byte for a probe attribute. Zero clears the probe test.
    ///
    /// `$80:ADA6`: `LDA $80:E85C,X` with `X` the attribute, then
    /// `AND #$00FF; BNE refuse`. The attribute is masked to five bits by the
    /// handler, so values above 31 alias into the table.
    #[must_use]
    pub const fn entry(&self, attribute: u8) -> u8 {
        self.entries[(attribute & 0x1F) as usize]
    }

    /// Whether the table entry is zero; not whether ordinary walking is allowed.
    #[must_use]
    pub const fn is_clear(&self, attribute: u8) -> bool {
        self.entry(attribute) == 0
    }

    /// Whether CA sees a zero entry for this cell, including its bit-15 override.
    #[must_use]
    pub const fn cell_is_clear(&self, word: u16) -> bool {
        self.is_clear(probe_attribute(word))
    }

    /// Attributes with zero table entries, ascending.
    #[must_use]
    pub fn clear_attributes(&self) -> Vec<u8> {
        (0..=31)
            .filter(|attribute| self.is_clear(*attribute))
            .collect()
    }
}

/// The attribute COP CA derives from a runtime cell word.
///
/// `$80:AD95`..`$80:ADA5`: the high byte is read; when its bit 7, word bit
/// 15, is set the value 6 replaces the high byte *before* the right shift,
/// selecting entry 3. Otherwise word bits 9 to 13 select the entry. Word bit
/// 14 never reaches the table.
#[must_use]
pub const fn probe_attribute(word: u16) -> u8 {
    if word & 0x8000 != 0 {
        3
    } else {
        ((word >> 9) & 0x1F) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_with(entries: [u8; TABLE_LEN]) -> Vec<u8> {
        let mut image = vec![0u8; TABLE_OFFSET + TABLE_LEN + 16];
        image[TABLE_OFFSET..TABLE_OFFSET + TABLE_LEN].copy_from_slice(&entries);
        image
    }

    fn table_with(entries: [u8; TABLE_LEN]) -> ProbeTable {
        ProbeTable::from_rom(&image_with(entries)).unwrap()
    }

    #[test]
    fn a_zero_entry_clears_and_any_nonzero_entry_fails_the_probe() {
        let mut entries = [0u8; TABLE_LEN];
        entries[3] = 0x0F;
        entries[6] = 0x06;
        entries[31] = 0x01;
        let table = table_with(entries);
        assert!(table.is_clear(0));
        assert!(table.is_clear(2));
        assert!(!table.is_clear(3));
        assert!(!table.is_clear(6), "a partial mask still refuses the probe");
        assert!(!table.is_clear(31));
        assert_eq!(table.entry(6), 0x06);
        assert_eq!(table.clear_attributes().len(), TABLE_LEN - 3);
    }

    #[test]
    fn the_attribute_is_word_bits_nine_to_thirteen_and_bit_fifteen_means_three() {
        assert_eq!(probe_attribute(0x0000), 0);
        assert_eq!(probe_attribute(0x0100), 0, "bit 8 belongs to the index");
        assert_eq!(probe_attribute(22 << 9), 22);
        assert_eq!(probe_attribute((22 << 9) | 0x01FF), 22);
        assert_eq!(probe_attribute(31 << 9), 31);
        assert_eq!(
            probe_attribute(32 << 9),
            0,
            "bit 14 never reaches the table"
        );
        assert_eq!(probe_attribute(0x8000), 3);
        assert_eq!(probe_attribute(0x8000 | (12 << 9)), 3);
    }

    #[test]
    fn mutation_controls_flip_the_verdict() {
        let mut entries = [0u8; TABLE_LEN];
        entries[12] = 0x0F;
        let table = table_with(entries);
        assert!(!table.cell_is_clear(12 << 9));
        assert!(table.cell_is_clear(14 << 9));
        // Clearing the byte is_clear; setting the neighbour refuses.
        entries[12] = 0;
        entries[14] = 0x0F;
        let table = table_with(entries);
        assert!(table.cell_is_clear(12 << 9));
        assert!(!table.cell_is_clear(14 << 9));
        // The dynamic bit selects entry 3 whatever the stored attribute says.
        let mut entries = [0u8; TABLE_LEN];
        entries[3] = 0x0F;
        let table = table_with(entries);
        assert!(table.cell_is_clear(0));
        assert!(!table.cell_is_clear(0x8000));
        entries[3] = 0;
        assert!(table_with(entries).cell_is_clear(0x8000));
    }

    #[test]
    fn attributes_above_thirty_one_alias_into_the_table() {
        let mut entries = [0u8; TABLE_LEN];
        entries[1] = 0x0F;
        let table = table_with(entries);
        assert!(!table.is_clear(33));
        assert!(table.is_clear(32));
    }

    #[test]
    fn a_dynamic_cell_selects_class_three_after_the_handlers_shift() {
        // $80:AD9E loads 6, then $80:ADA1 shifts it before indexing.
        let mut entries = [0u8; TABLE_LEN];
        entries[3] = 0x0F;
        let table = table_with(entries);
        assert_eq!(probe_attribute(0x8000 | (22 << 9)), 3);
        assert!(!table.cell_is_clear(0x8000));
        assert!(table.is_clear(6), "the override is not table entry six");
    }

    #[test]
    fn a_short_image_is_refused() {
        let error = ProbeTable::from_rom(&[0u8; 0x100]).unwrap_err();
        assert_eq!(error.image_len, 0x100);
        assert!(error.to_string().contains("admission table"));
    }
}
