//! A map's display profile (`$86:8C61`, `docs/map-colour-math.md`,
//! `docs/tower-second-layer.md`): the 9-byte record at `$96:BB64 + 2p`
//! (European `$99:C2AE`) its spawn list's second header byte `p` names,
//! and how it shows the second layer.

use super::{relocated, VisualMapError};

/// How a map shows its second layer (`10 02`), from the profile's screen
/// designation, colour math and layer swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// On the main screen in front of the first layer (the layers swapped,
    /// no colour math): the tower floors' torches, pillars and frames.
    Front,
    /// Added onto the view: on the main screen with colour math (the
    /// Crysta rooms' rays, tower 3), or the subscreen added (the town's
    /// clouds, the light room).
    Added,
    /// On the subscreen, subtracted (`$11B`, `$123`).
    Subtracted,
    /// Behind the first layer, not scrolling (the towers' night sky).
    Sky,
    /// Not on either screen.
    Hidden,
}

/// The profile record of `map_id`.
///
/// # Errors
/// Refuses a map without a spawn list and truncated input.
pub fn display_profile(image: &[u8], map_id: u16) -> Result<[u8; 9], VisualMapError> {
    let unsupported = VisualMapError::Unsupported;
    let header = crate::maps::actors::list_header(image, map_id)
        .map_err(|_| unsupported("no spawn list header"))?;
    let table = relocated(image, 0x16_bb64, "unrecorded display profile table")?;
    let at = table + usize::from(header[1] & 0x3f) * 2;
    let pointer = image
        .get(at..at + 2)
        .ok_or(unsupported("truncated display profile table"))?;
    let record = (table & 0x3f_0000) | usize::from(u16::from_le_bytes([pointer[0], pointer[1]]));
    image
        .get(record..record + 9)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(unsupported("truncated display profile"))
}

/// How the profile shows the second layer: hardware BG1 when byte +5 bit 7
/// swaps the layers, else BG2; TM, TS and CGADSUB (+0, +1, +3) for it; a
/// layer that does not scroll (+7 and +8 zero) is a sky.
#[must_use]
pub const fn presentation(record: &[u8; 9]) -> Presentation {
    let bit = if record[5] & 0x80 != 0 { 0x01 } else { 0x02 };
    let (main, sub) = (record[0] & bit != 0, record[1] & bit != 0);
    let math = record[3] & bit != 0;
    let fixed = record[7] == 0 && record[8] == 0;
    match (main, sub) {
        (true, _) if fixed => Presentation::Sky,
        (true, _) if math => Presentation::Added,
        (true, _) => Presentation::Front,
        (false, true) if record[3] & 0x80 != 0 => Presentation::Subtracted,
        (false, true) => Presentation::Added,
        (false, false) => Presentation::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profiles_show_the_second_layer_their_way() {
        // Records from `$96:BB64` (`docs/tower-second-layer.md` §2).
        let cases: [([u8; 9], Presentation); 8] = [
            (
                [0x17, 0x00, 0x80, 0x20, 0xE4, 0x80, 0x09, 0x11, 0x11],
                Presentation::Front,
            ),
            (
                [0x17, 0x12, 0x82, 0x21, 0xA4, 0x80, 0x09, 0x11, 0x11],
                Presentation::Added,
            ),
            (
                [0x17, 0x12, 0x82, 0x21, 0x64, 0x80, 0x09, 0x11, 0x11],
                Presentation::Added,
            ),
            (
                [0x16, 0x01, 0x82, 0x33, 0x64, 0xC0, 0x09, 0xED, 0x13],
                Presentation::Added,
            ),
            (
                [0x16, 0x01, 0x82, 0x36, 0xC0, 0x80, 0x01, 0x11, 0x11],
                Presentation::Added,
            ),
            (
                [0x15, 0x02, 0x82, 0xB1, 0x40, 0x00, 0x09, 0x11, 0x11],
                Presentation::Subtracted,
            ),
            (
                [0x17, 0x00, 0x80, 0x02, 0xA4, 0x00, 0x09, 0x00, 0x00],
                Presentation::Sky,
            ),
            (
                [0x15, 0x00, 0x80, 0x20, 0x64, 0x00, 0x09, 0x00, 0x00],
                Presentation::Hidden,
            ),
        ];
        for (record, expected) in cases {
            assert_eq!(presentation(&record), expected, "{record:02x?}");
        }
    }
}
