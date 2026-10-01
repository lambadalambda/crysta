//! Pose records' lengths and their compositions' boxes (`docs/combat.md`):
//! what combat reads of a pose, without the art. A composition is a 4-byte
//! anchor, then three boxes (sprite, attack, body), each (dx, w, dy, h) as
//! signed bytes.

use super::SpriteError;

/// One pose record: frames, facing byte and boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// Shown for `duration + 1` frames.
    pub duration: u8,
    /// The record's facing byte.
    pub facing: u8,
    /// The sprite box.
    pub sprite: [i8; 4],
    /// The attack box, which hits other bodies.
    pub attack: [i8; 4],
    /// The body box, which attacks hit.
    pub body: [i8; 4],
}

/// Ark's resource table (`$80:A24F`), six bytes per resource.
const ARK_RESOURCES: usize = 0x00_A24F;
/// Records one list may hold.
const RECORDS: usize = 64;

/// List `selector` of Ark's resource `resource`.
///
/// # Errors
/// Refuses a table, list or composition outside the image.
pub fn ark_list(image: &[u8], resource: u8, selector: u8) -> Result<Vec<Record>, SpriteError> {
    let table = crate::layout::offset(image, ARK_RESOURCES)
        .ok_or(SpriteError::Invalid("unrecorded Ark resource table"))?;
    let at = table + usize::from(resource) * 6;
    let entry = image
        .get(at..at + 3)
        .ok_or(SpriteError::Invalid("truncated Ark resource table"))?;
    let base = (usize::from(entry[2] & 0x3F) << 16)
        | usize::from(u16::from_le_bytes([entry[0], entry[1]]));
    records(image, base, selector)
}

/// List `selector` of a decompressed packet's list table.
///
/// # Errors
/// Refuses a list or composition outside the packet.
pub fn packet_list(bytes: &[u8], selector: u8) -> Result<Vec<Record>, SpriteError> {
    records(bytes, 0, selector)
}

fn records(bytes: &[u8], base: usize, selector: u8) -> Result<Vec<Record>, SpriteError> {
    let truncated = || SpriteError::Invalid("truncated pose list");
    let word = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|pair| usize::from(u16::from_le_bytes([pair[0], pair[1]])))
            .ok_or_else(truncated)
    };
    let list = base + word(base + usize::from(selector) * 2)?;
    let mut records = Vec::new();
    for index in 0..RECORDS {
        let record = bytes
            .get(list + index * 4..list + index * 4 + 4)
            .ok_or_else(truncated)?;
        if record[0] >= 0x80 {
            return Ok(records);
        }
        let anchor = base + word(list + index * 4 + 2)?;
        let boxes = bytes.get(anchor + 4..anchor + 16).ok_or_else(truncated)?;
        let signed = |at: usize| std::array::from_fn(|i| boxes[at + i].cast_signed());
        records.push(Record {
            duration: record[0],
            facing: record[1],
            sprite: signed(0),
            attack: signed(4),
            body: signed(8),
        });
    }
    Err(SpriteError::Invalid("unbounded pose list"))
}
