//! Pure, bounded Japanese exit-list decoding and doorway geometry.
//!
//! `$81:8000` holds little-endian bank-$81 pointers. Only the map-loading
//! table's qualified ID prefix is exposed here, not a claimed full exit-table
//! bound: the earliest observed exit pointer, `$88AC`, suggests more entries.
//! Conditional destinations and transition/effect selectors remain opaque;
//! this module neither evaluates flags nor executes departure/arrival effects.
use rom::RuntimeRomAddress;
use std::{fmt, ops::Range};

/// Conservative lookup prefix: map IDs `$0000..=$044F`, not a full table count.
pub const SUPPORTED_MAP_COUNT: u16 = super::scripts::MAP_COUNT;
/// Hard decoding budget, excluding the terminating `$FF` byte.
pub const MAX_RECORDS: usize = 256;
const TABLE: usize = 0x01_8000;
const FIRST_DATA_POINTER: u16 = 0x88AC;
const BANK_END: usize = 0x02_0000;
const RECORD_SIZE: usize = 12;

/// Invalid input or behavior outside the qualified static exit projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitError {
    /// Map ID is outside the conservative lookup prefix.
    MapIndex {
        /// Rejected map ID.
        index: u16,
    },
    /// Nonzero pointer targets non-ROM memory or the table/reserved prefix.
    InvalidPointer {
        /// Original bank-$81 pointer word.
        pointer: u16,
    },
    /// Input does not contain the required bytes.
    Truncated {
        /// Normalized starting offset.
        offset: usize,
        /// Required byte count.
        needed: usize,
    },
    /// A record or sentinel would cross out of the bank-$81 ROM window.
    BankCrossing,
    /// More than 256 records precede the sentinel.
    RecordLimit,
    /// Bit 15 selects a conditional table, not a direct map ID.
    ConditionalDestination {
        /// Unmodified destination operand, including bit 15.
        raw: u16,
    },
}
impl fmt::Display for ExitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "map exits: {self:?}")
    }
}
impl std::error::Error for ExitError {}

/// One 12-byte exit record, preserving unknown operand bits without interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitRecord {
    offset: usize,
    bytes: [u8; RECORD_SIZE],
}
impl ExitRecord {
    /// Exact record bytes, excluding the list sentinel.
    #[must_use]
    pub const fn bytes(&self) -> &[u8; RECORD_SIZE] {
        &self.bytes
    }
    /// Exact normalized, headerless ROM extent of this record.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.offset..self.offset + RECORD_SIZE
    }
    /// Doorway X in 16-pixel tiles.
    #[must_use]
    pub const fn x(&self) -> u8 {
        self.bytes[0]
    }
    /// Doorway Y in 16-pixel tiles.
    #[must_use]
    pub const fn y(&self) -> u8 {
        self.bytes[1]
    }
    /// Doorway width in tiles; zero never matches.
    #[must_use]
    pub const fn width(&self) -> u8 {
        self.bytes[2]
    }
    /// Doorway height in tiles; zero never matches.
    #[must_use]
    pub const fn height(&self) -> u8 {
        self.bytes[3]
    }
    /// Destination operand, retaining the conditional-table bit (bit 15).
    #[must_use]
    pub fn raw_destination(&self) -> u16 {
        self.word(4)
    }
    /// Returns an unflagged destination map ID, without applying lookup bounds.
    ///
    /// # Errors
    /// Bit 15 selects an unimplemented conditional table, not a direct map ID.
    pub fn direct_destination(&self) -> Result<u16, ExitError> {
        let raw = self.raw_destination();
        if raw & 0x8000 != 0 {
            Err(ExitError::ConditionalDestination { raw })
        } else {
            Ok(raw)
        }
    }
    /// Raw transition mode (byte 6); unknown values are preserved.
    #[must_use]
    pub const fn transition_mode(&self) -> u8 {
        self.bytes[6]
    }
    /// Raw departure/arrival selector (byte 7), not an effect script pointer.
    #[must_use]
    pub const fn selector(&self) -> u8 {
        self.bytes[7]
    }
    /// Raw destination X/Y words, before any arrival adjustment.
    #[must_use]
    pub fn destination_position(&self) -> (u16, u16) {
        (self.word(8), self.word(10))
    }
    fn word(&self, index: usize) -> u16 {
        u16::from_le_bytes([self.bytes[index], self.bytes[index + 1]])
    }
    fn coarse_matches(&self, x: u16, y: u16) -> bool {
        let tile_byte = |origin: u16| (origin >> 4).to_le_bytes()[0];
        tile_byte(x).wrapping_sub(self.x()) < self.width()
            && tile_byte(y).wrapping_sub(self.y()) < self.height()
    }
    fn fine_matches(&self, x: u16, y: u16) -> bool {
        // Only called after coarse matching, which rules out zero dimensions.
        x.wrapping_sub(u16::from(self.x()) * 16) < u16::from(self.width()) * 16 - 15
            && y.wrapping_sub(u16::from(self.y()) * 16) < u16::from(self.height()) * 16 - 15
    }
}

/// A map's ordered exit records and exact source encoding, including `$FF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitList {
    entry: Option<RuntimeRomAddress>,
    records: Vec<ExitRecord>,
    source: Vec<u8>,
}
impl ExitList {
    /// Reads a map's exit list from a normalized, headerless ROM image.
    ///
    /// The caller authenticates the Japanese ROM revision. Nonzero pointers
    /// below the earliest observed data pointer `$88AC` are rejected, including
    /// the unqualified tail after the supported lookup prefix. No full exit
    /// table size is inferred. Null pointers produce a sourceless empty list.
    ///
    /// # Errors
    /// Rejects IDs outside the supported prefix, invalid/table pointers,
    /// truncation, bank crossing, and lists exceeding 256 records.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, ExitError> {
        if map_id >= SUPPORTED_MAP_COUNT {
            return Err(ExitError::MapIndex { index: map_id });
        }
        let bytes = slice(image, TABLE + usize::from(map_id) * 2, 2)?;
        let pointer = u16::from_le_bytes([bytes[0], bytes[1]]);
        if pointer == 0 {
            return Ok(Self {
                entry: None,
                records: Vec::new(),
                source: Vec::new(),
            });
        }
        if pointer < FIRST_DATA_POINTER {
            return Err(ExitError::InvalidPointer { pointer });
        }
        let entry = RuntimeRomAddress::from_parts(0x81, pointer)
            .map_err(|_| ExitError::InvalidPointer { pointer })?;
        let start = entry.normalized().value() as usize;
        let mut offset = start;
        let mut records = Vec::new();
        loop {
            if slice(image, offset, 1)?[0] == 0xFF {
                return Ok(Self {
                    entry: Some(entry),
                    records,
                    source: image[start..=offset].to_vec(),
                });
            }
            if records.len() == MAX_RECORDS {
                return Err(ExitError::RecordLimit);
            }
            let mut bytes = [0; RECORD_SIZE];
            bytes.copy_from_slice(slice(image, offset, RECORD_SIZE)?);
            records.push(ExitRecord { offset, bytes });
            offset += RECORD_SIZE;
        }
    }
    /// Original pointer in CPU bank `$81`; `None` represents a null table entry.
    #[must_use]
    pub const fn entry(&self) -> Option<RuntimeRomAddress> {
        self.entry
    }
    /// Records in original scan order.
    #[must_use]
    pub fn records(&self) -> &[ExitRecord] {
        &self.records
    }
    /// Exact source bytes through `$FF`, or no bytes for a null table entry.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    /// Normalized source extent including `$FF`; null entries have no extent.
    #[must_use]
    pub fn source_range(&self) -> Option<Range<usize>> {
        self.entry.map(|entry| {
            let start = entry.normalized().value() as usize;
            start..start + self.source.len()
        })
    }
    /// Reproduces the geometry scan at `$8D:8797..8838` for a player bounds origin.
    ///
    /// Inputs are the bounding origin, **not** the player position. The first
    /// byte-wrapping coarse tile match is chosen, then tested with word-wrapping
    /// pixel subtraction against `dimension * 16 - 15` (exclusive). A failed
    /// fine test returns `None`; later records are not considered. This does not
    /// model runtime movement gates, conditional destinations, or effects.
    #[must_use]
    pub fn select(&self, origin_x: u16, origin_y: u16) -> Option<&ExitRecord> {
        self.records
            .iter()
            .find(|record| record.coarse_matches(origin_x, origin_y))
            .filter(|record| record.fine_matches(origin_x, origin_y))
    }
}

fn slice(image: &[u8], offset: usize, needed: usize) -> Result<&[u8], ExitError> {
    if offset + needed > BANK_END {
        return Err(ExitError::BankCrossing);
    }
    image
        .get(offset..offset + needed)
        .ok_or(ExitError::Truncated { offset, needed })
}
