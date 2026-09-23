//! Bounded per-map actor spawn lists, not an actor runtime.
//!
//! `$80:F3FD` reads `$0480` and indexes `$83:8000` by map ID times two to reach
//! a list. The list is a stream: positioned spawn records interleaved with
//! control opcodes, ending at `$FF`. Every element length here is read from
//! the interpreter at `$80:F4EA` and its handlers, never fitted by requiring a
//! walk to land on a plausible record — a wrong length emits positions taken
//! from operand bytes, and those look exactly like real spawns. An opcode
//! outside the decoded set is refused rather than resynchronised.
//!
//! It neither evaluates record conditions nor executes their scripts, so a
//! decoded list is every record the stream contains, not the set that applies
//! on a given save. Lists routinely repeat a position under different
//! conditions; see `docs/house-scene.md`.
use std::fmt;

const TABLE: usize = 0x03_8000;
const BANK: usize = 0x03_0000;
const BANK_END: usize = 0x04_0000;
/// Map IDs addressable in the table, matching the loading-script projection.
pub const SUPPORTED_MAP_COUNT: u16 = super::scripts::MAP_COUNT;
/// Hard decoding budget for one list.
pub const MAX_RECORDS: usize = 256;

/// Invalid input or a list outside the qualified spawn-stream subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorError {
    /// Map ID outside the conservative lookup prefix.
    MapIndex {
        /// Rejected map ID.
        index: u16,
    },
    /// The map has no spawn list.
    Absent {
        /// Requested map ID.
        index: u16,
    },
    /// Missing bytes at a normalized offset.
    Truncated {
        /// Offset of the requested slice.
        offset: usize,
    },
    /// An opcode this decoder does not account for.
    ///
    /// Deliberately fatal: resynchronising past an unknown length would emit
    /// positions read from operand bytes, which look like plausible spawns.
    Unqualified {
        /// Byte offset within the list.
        offset: usize,
        /// The opcode.
        opcode: u8,
        /// Its following byte, which selects a form for some opcodes.
        selector: u8,
    },
    /// The stream ran past its decoding budget.
    Budget,
}
impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor spawn list: {self:?}")
    }
}
impl std::error::Error for ActorError {}

/// One positioned spawn record, with its exact bytes retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRecord {
    opcode: u8,
    tile_x: u8,
    tile_y: u8,
    bytes: Vec<u8>,
    offset: usize,
}
impl SpawnRecord {
    /// Record opcode: `$00`, `$01` or `$FD`. All three carry a position.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        self.opcode
    }
    /// Source origin in pixels, `(tile_x * 16 + 8, tile_y * 16)`.
    ///
    /// The half-cell horizontal bias is measured, not assumed: it is the only
    /// reading that reproduces every documented resident origin.
    #[must_use]
    pub const fn origin(&self) -> (u16, u16) {
        (self.tile_x as u16 * 16 + 8, self.tile_y as u16 * 16)
    }
    /// Exact record bytes, including fields this decoder does not interpret.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Normalized ROM offset of the record.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
    /// Normalized ROM offset of the record's own resource descriptor, when
    /// its pointer names ROM (`$80..$BF:8000..FFFF` or `$C0..$FF`). A reusing
    /// record has none; see [`descriptor_owner`].
    #[must_use]
    pub fn descriptor_offset(&self) -> Option<usize> {
        rom_offset(self.bytes.get(7..10).filter(|_| self.opcode <= 1)?)
    }
    /// Runtime address of the actor script this record installs.
    ///
    /// The record's pointer field is followed by a five-byte header, so the
    /// command stream begins at `pointer + 5`. Measured rather than fitted:
    /// map `$000F`'s `(8,16)` record points at `$88:8038`, and the running
    /// game's actor at that position carries script `$88:803D` in its slot,
    /// read out of WRAM while the reference emulator ran.
    ///
    /// Across the slice 90 of 115 records land on a `COP` this way and none
    /// land on one at the pointer itself, so the offset is not an artefact of
    /// where `COP` bytes happen to fall. The remaining 25 resolve to neither
    /// and are returned regardless: this reports the field, and a caller that
    /// walks it will be refused at the first byte that is not a command.
    ///
    /// Returns `None` when the field is not a ROM-backed address.
    #[must_use]
    pub fn script(&self) -> Option<u32> {
        // An `FE` controller has no position bytes: its pointer is bytes 2..4.
        let field = if matches!(self.opcode, 0xFB | 0xFE) {
            self.bytes.get(2..5)?
        } else {
            self.bytes.get(4..7)?
        };
        let pointer =
            u32::from(field[0]) | (u32::from(field[1]) << 8) | (u32::from(field[2]) << 16);
        // A linear add, where the CPU would wrap within the bank. The two
        // cannot disagree on an accepted address: a pointer whose low word is
        // at least `$FFFB` carries into the next bank and lands below `$8000`,
        // which the predicate below rejects either way.
        // A compact actor's header is three bytes (`docs/house-scene.md`).
        let start = pointer.checked_add(if self.opcode == 0xFB { 3 } else { 5 })?;
        ((0x80..=0xBF).contains(&(start >> 16)) && start & 0xFFFF >= 0x8000).then_some(start)
    }
}

/// Every positioned record in one map's spawn stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnList {
    map_id: u16,
    entry: u16,
    records: Vec<SpawnRecord>,
}
impl SpawnList {
    /// Decodes one map's spawn stream.
    ///
    /// # Errors
    /// Rejects out-of-range IDs, absent lists, truncation, an unaccounted
    /// opcode, and budget exhaustion.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, ActorError> {
        if map_id >= SUPPORTED_MAP_COUNT {
            return Err(ActorError::MapIndex { index: map_id });
        }
        let at = TABLE + usize::from(map_id) * 2;
        let head = image
            .get(at..at + 2)
            .ok_or(ActorError::Truncated { offset: at })?;
        let entry = u16::from_le_bytes([head[0], head[1]]);
        if entry == 0 {
            return Err(ActorError::Absent { index: map_id });
        }
        let base = BANK | usize::from(entry);
        // Two-byte list header, skipped by the loader's own INC A / INC A.
        let mut cursor = base + 2;
        let mut records = Vec::new();
        loop {
            if records.len() > MAX_RECORDS {
                return Err(ActorError::Budget);
            }
            if cursor + 2 > BANK_END {
                return Err(ActorError::Truncated { offset: cursor });
            }
            let window = image
                .get(cursor..cursor + 2)
                .ok_or(ActorError::Truncated { offset: cursor })?;
            let (opcode, selector) = (window[0], window[1]);
            let length =
                record_length(image, cursor, opcode, selector).ok_or(ActorError::Unqualified {
                    offset: cursor - base,
                    opcode,
                    selector,
                })?;
            let bytes = image
                .get(cursor..cursor + length)
                .ok_or(ActorError::Truncated { offset: cursor })?;
            if matches!(opcode, 0x00 | 0x01 | 0xFD) {
                records.push(SpawnRecord {
                    opcode,
                    tile_x: bytes[1],
                    tile_y: bytes[2],
                    bytes: bytes.to_vec(),
                    offset: cursor,
                });
            }
            cursor += length;
            // $80:F4A4 ends the list. $FF is a terminator, and the bytes after
            // it are $FA branch targets rather than fall-through, so walking
            // past it reads operands as opcodes.
            if opcode == 0xFF {
                return Ok(Self {
                    map_id,
                    entry,
                    records,
                });
            }
        }
    }
    /// Records that apply for a given event-flag state.
    ///
    /// Follows `$FA` conditional branches instead of reading the stream
    /// linearly, so the result is the set the game would install rather than
    /// every record the bytes contain.
    ///
    /// # Errors
    /// Rejects an undecodable stream, a chained condition, and a branch that
    /// leaves the bank or loops past the budget.
    pub fn resolve(
        image: &[u8],
        map_id: u16,
        events: super::scripts::EventFlags<'_>,
    ) -> Result<Vec<SpawnRecord>, ResolveError> {
        let list = Self::from_rom(image, map_id).map_err(ResolveError::Decode)?;
        let base = BANK | usize::from(list.entry);
        let mut cursor = base + 2;
        let mut records = Vec::new();
        for _ in 0..MAX_RECORDS {
            if cursor + 2 > BANK_END {
                return Err(ResolveError::Runaway {
                    offset: cursor.saturating_sub(base),
                });
            }
            let window = image.get(cursor..cursor + 2).ok_or(ResolveError::Runaway {
                offset: cursor - base,
            })?;
            let (opcode, selector) = (window[0], window[1]);
            let length = record_length(image, cursor, opcode, selector).ok_or(
                ResolveError::Decode(ActorError::Unqualified {
                    offset: cursor - base,
                    opcode,
                    selector,
                }),
            )?;
            let bytes = image
                .get(cursor..cursor + length)
                .ok_or(ResolveError::Runaway {
                    offset: cursor - base,
                })?;
            match opcode {
                0xFF => return Ok(records),
                0x00 | 0x01 | 0xFD => records.push(SpawnRecord {
                    opcode,
                    tile_x: bytes[1],
                    tile_y: bytes[2],
                    bytes: bytes.to_vec(),
                    offset: cursor,
                }),
                // A script-only controller: implicit descriptor, spawned at
                // (8,0) (`docs/house-scene.md`).
                0xFB | 0xFE => records.push(SpawnRecord {
                    opcode,
                    tile_x: 0,
                    tile_y: 0,
                    bytes: bytes.to_vec(),
                    offset: cursor,
                }),
                0xFA => {
                    // $80:F787 accumulates each word's flag result with ADC and
                    // $80:F765 re-normalises with AND #$0001, so a chain is the
                    // parity of its results. The sense comes from the last word
                    // read, because $80:F76C reloads $3E each time round.
                    let mut parity = false;
                    let mut at = 1usize;
                    let mut sense;
                    loop {
                        let word = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
                        let set = events.get(word & 0x0FFF).ok_or(ResolveError::Runaway {
                            offset: cursor - base,
                        })?;
                        parity ^= set;
                        sense = word;
                        // $80:F773 and $80:F778 take other paths for these bits;
                        // only the plain chain is decoded.
                        if word & 0x8000 != 0 || word & 0xF800 == 0 {
                            break;
                        }
                        if word & 0x6000 != 0 {
                            return Err(ResolveError::ChainedCondition {
                                offset: cursor - base,
                                word,
                            });
                        }
                        at += 2;
                        if at + 1 >= bytes.len() - 2 {
                            break;
                        }
                    }
                    if condition_takes_branch(sense, parity) {
                        let target =
                            u16::from_le_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]]);
                        cursor = BANK | usize::from(target);
                        continue;
                    }
                }
                _ => {}
            }
            cursor += length;
        }
        Err(ResolveError::Runaway {
            offset: cursor.saturating_sub(base),
        })
    }

    /// Caller-selected map ID.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        self.map_id
    }
    /// Bank-`$83` address of the list.
    #[must_use]
    pub const fn entry(&self) -> u16 {
        self.entry
    }
    /// Every positioned record, in stream order.
    #[must_use]
    pub fn records(&self) -> &[SpawnRecord] {
        &self.records
    }
}

/// Why a stream could not be resolved against event flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The stream itself could not be decoded.
    Decode(ActorError),
    /// A condition chains further words, which is not decoded.
    ///
    /// `$80:F773` inspects bits `$4000` and `$2000` and `$80:F77D` reads a
    /// second word, accumulating results. Only the single-word form is
    /// evaluated here.
    ChainedCondition {
        /// Byte offset within the list.
        offset: usize,
        /// The condition word.
        word: u16,
    },
    /// A branch left the bank, or the walk exceeded its budget.
    Runaway {
        /// Byte offset within the list.
        offset: usize,
    },
}
impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor spawn resolve: {self:?}")
    }
}
impl std::error::Error for ResolveError {}

/// Whether a `$FA` condition takes its branch.
///
/// `$80:F760` tests the flag through `$80:BBC7`, the same routine the loading
/// scripts use, so the index is `word & $0FFF`. The sense is decided by bit 15
/// **before** the chain mask: a non-negative word reaches `$80:F7DB` and jumps
/// when the flag is set, while a negative word reaches `$80:F7E1` and jumps
/// when it is clear. Note this is the opposite convention to the loading
/// script's `$08 FD`.
#[must_use]
pub fn condition_takes_branch(word: u16, flag_set: bool) -> bool {
    if word & 0x8000 == 0 {
        flag_set
    } else {
        !flag_set
    }
}

/// Byte length of one stream element, as `$80:F4EA` and its handlers read it.
///
/// Every length here comes from the interpreter, not from requiring a walk to
/// land on a plausible record. A wrong length emits positions read from operand
/// bytes, and those look exactly like real spawns.
fn record_length(image: &[u8], at: usize, opcode: u8, selector: u8) -> Option<usize> {
    match opcode {
        // Ordinary records. `$80:F564` tests byte 3's top two bits and reads
        // four further fields when both are set, so the record widens.
        0x00 | 0x01 => Some(if image.get(at + 3)? & 0xC0 == 0xC0 {
            16
        } else {
            10
        }),
        0xFD => Some(7),
        0xFB | 0xFE => Some(5),
        0xFF => Some(2),
        // `$80:F759` reads a condition word and tests it through `$80:BBC7`,
        // the same event-flag routine the loading scripts use. `$80:F76C`
        // branches on bit 15 *before* masking `$F800`, so a negative word takes
        // the short path; otherwise bits in `$F800` chain a second word.
        0xFA => {
            let word = u16::from_le_bytes([selector, *image.get(at + 2)?]);
            Some(if word & 0x8000 != 0 || word & 0xF800 == 0 {
                5
            } else {
                7
            })
        }
        _ => None,
    }
}

/// Index of the record whose resource descriptor the record at `index` is
/// built from: its own, or the one a zero pointer reuses.
///
/// `records` must be in the order the stream executes them, as
/// [`SpawnList::resolve`] returns them: `$FA` branches decide which records
/// run. A `$00` or `$01` record parses its descriptor at `$80:FA65`; a
/// pointer whose address word is zero instead reuses the last parsed class,
/// movement base, palette and packet (`$80:FAF9..FB1A`). `$FD` and `$FE`
/// records never touch that state. A descriptor in a bank at or above `$90`
/// skips the parse, which is not followed here.
#[must_use]
pub fn descriptor_owner(records: &[SpawnRecord], index: usize) -> Option<usize> {
    match parse(records.get(index)?) {
        Parse::Own => Some(index),
        Parse::Reuse => parsed_before(records, index),
        Parse::None | Parse::Unknown => None,
    }
}

/// Index of the last record before `index` that parsed a descriptor of its
/// own, which a reuse at `index` would take. See [`descriptor_owner`].
#[must_use]
pub fn parsed_before(records: &[SpawnRecord], index: usize) -> Option<usize> {
    for at in (0..index.min(records.len())).rev() {
        match parse(&records[at]) {
            Parse::Own => return Some(at),
            Parse::Unknown => return None,
            Parse::Reuse | Parse::None => {}
        }
    }
    None
}

/// What a record does to the reuse state.
enum Parse {
    /// Parses a descriptor of its own.
    Own,
    /// Reuses the last parsed one: an address word of zero.
    Reuse,
    /// Leaves it alone: `$FD`/`$FE`.
    None,
    /// Not followed here: a bank at or above `$90`, which skips the parse,
    /// or a record whose length is not the ten bytes the layout is known for.
    Unknown,
}

fn parse(record: &SpawnRecord) -> Parse {
    if record.opcode() > 1 {
        return Parse::None;
    }
    match record.bytes() {
        bytes if bytes.len() != 10 || bytes[9] >= 0x90 => Parse::Unknown,
        [.., 0, 0, _] => Parse::Reuse,
        _ => Parse::Own,
    }
}

/// Normalized offset of a long ROM pointer: `$80..$BF:8000..FFFF` or
/// `$C0..$FF`; `None` for anything else, such as RAM.
#[must_use]
pub fn rom_offset(pointer: &[u8]) -> Option<usize> {
    let [low, high, bank] = *pointer else {
        return None;
    };
    let address = u16::from_le_bytes([low, high]);
    (bank >= 0xC0 || (bank >= 0x80 && address >= 0x8000))
        .then_some(usize::from(bank & 0x3F) << 16 | usize::from(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(opcode: u8, offset: usize, descriptor: [u8; 3]) -> SpawnRecord {
        let mut bytes = vec![opcode, 0, 0, 0, 0, 0x80, 0x88];
        bytes.extend_from_slice(&descriptor);
        SpawnRecord {
            opcode,
            tile_x: 0,
            tile_y: 0,
            bytes,
            offset,
        }
    }

    #[test]
    fn a_zero_descriptor_reuses_the_last_parsed_one() {
        let own = [0x37, 0xED, 0x83];
        let other = [0x7E, 0xEC, 0x83];
        let none = [0, 0, 0];
        let fd = SpawnRecord {
            bytes: vec![0xFD, 0, 0],
            ..record(0xFD, 0, none)
        };
        let records = [
            record(1, 0, own),
            record(1, 1, none),
            fd,
            record(1, 3, none),
            record(0, 4, other),
            record(1, 5, none),
        ];
        // A record with its own descriptor owns it; reuse carries back
        // through reusing records and over `$FD`, which never parses one.
        assert_eq!(descriptor_owner(&records, 0), Some(0));
        assert_eq!(descriptor_owner(&records, 1), Some(0));
        assert_eq!(descriptor_owner(&records, 3), Some(0));
        // A `$00` record parses its descriptor exactly as `$01` does.
        assert_eq!(descriptor_owner(&records, 4), Some(4));
        assert_eq!(descriptor_owner(&records, 5), Some(4));
        // An `$FD` has no descriptor of its own.
        assert_eq!(descriptor_owner(&records, 2), None);
        assert_eq!(descriptor_owner(&records, 9), None);
        // Nothing parsed before the first reuse.
        assert_eq!(descriptor_owner(&records[1..], 0), None);
        // A bank at or above `$90` skips the parse; not followed here.
        let far = [
            record(1, 0, own),
            record(1, 1, [0, 0x80, 0x90]),
            record(1, 2, none),
        ];
        assert_eq!(descriptor_owner(&far, 2), None);
        assert_eq!(descriptor_owner(&far, 1), None);
        // What a record at `index` would reuse, whatever it carries itself.
        assert_eq!(parsed_before(&records, 0), None);
        assert_eq!(parsed_before(&records, 3), Some(0));
        assert_eq!(parsed_before(&records, 4), Some(0));
        assert_eq!(parsed_before(&records, 5), Some(4));
        assert_eq!(parsed_before(&records, 99), Some(4));
        assert_eq!(records[0].descriptor_offset(), Some(0x03_ED37));
        assert_eq!(records[1].descriptor_offset(), None);
        assert_eq!(record(1, 0, [0, 0x70, 0x83]).descriptor_offset(), None);
        assert_eq!(
            record(1, 0, [0x22, 0x10, 0xD8]).descriptor_offset(),
            Some(0x18_1022)
        );
        assert_eq!(rom_offset(&[0, 0x80, 0x7E]), None);
        assert_eq!(rom_offset(&[0, 0x80]), None);
        // A record longer than ten bytes stops the chain like a far bank.
        let long = SpawnRecord {
            bytes: [record(1, 0, own).bytes, vec![0; 6]].concat(),
            ..record(1, 1, own)
        };
        let chain = [record(1, 0, own), long, record(1, 2, none)];
        assert_eq!(descriptor_owner(&chain, 2), None);
        assert_eq!(descriptor_owner(&chain, 1), None);
    }
}
