//! The Records screen ([notes](../../../docs/records-screen.md)): the save
//! screen the bedroom desk opens, its text requests and its art.

use crate::graphics::{decode_tiles_4bpp, Bgr555, Tile4bpp};
use crate::labels::{label_glyphs, Glyph};
use crate::layout::Address;
use crate::shop_display::{colours, packet};
use crate::shops::{read, ShopError};
use crate::sprites::SpriteFrame;
use crate::text::{DialoguePage, HouseDialogue, Request, TextError};

/// The page: ` 1`–` 3`, the current game's header and the question.
const PAGE: Address = Address::both(0x92_CB4D, 0x92_E1F3);
/// The page after a save: ` 1`–` 3` and "`n` saved".
const SAVED: Address = Address::both(0x92_CB77, 0x92_E21E);
/// Slot `n`'s name (`+12n`), from `$061C`.
const FILLED: Address = Address::both(0x92_CBA0, 0x92_E244);
/// Slot `n`'s "No Data" (`+12n`).
const EMPTY: Address = Address::both(0x92_CBC4, 0x92_E268);
/// The current game's name, from `$0610`.
const CURRENT: Address = Address::both(0x92_CBE8, 0x92_E28C);
/// Where `$87:CB4B` copies a slot's name, and the current name.
const SLOT_NAME: u16 = 0x061C;
const CURRENT_NAME: u16 = 0x0610;
/// The saved page's number: the slot plus 1.
const SAVED_NUMBER: u16 = 0x04C6;
/// Bytes a name may take, its `D4` end included.
const NAME_BYTES: u16 = 12;
/// The byte that ends a name.
const NAME_END: u8 = 0xD4;

/// What the screen shows of a valid slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot<'a> {
    /// The name's glyph codes, without its `D4` end.
    pub name: &'a [u8],
}

fn address(image: &[u8], address: Address) -> Result<u32, TextError> {
    address.of(image).ok_or(TextError {
        source: 0,
        reason: "no Records text in this revision",
    })
}

/// A name at `base` in WRAM, as the engine reads it: its codes, then `D4`.
fn name_at(base: u16, name: &[u8]) -> impl Fn(u16) -> Option<u8> + '_ {
    move |at| {
        let index = usize::from(at.checked_sub(base).filter(|&index| index < NAME_BYTES)?);
        Some(name.get(index).copied().unwrap_or(NAME_END))
    }
}

/// The slot list's requests (`$87:CB4B`): each slot's name, or "No Data",
/// with the readers of the names (`$061C`).
fn slot_requests<'a>(
    image: &[u8],
    slots: &[Option<Slot<'_>>; 3],
    readers: &'a [impl Fn(u16) -> Option<u8>; 3],
) -> Result<Vec<Request<'a>>, TextError> {
    let (filled, empty) = (address(image, FILLED)?, address(image, EMPTY)?);
    Ok(slots
        .iter()
        .zip(readers)
        .zip(0u32..)
        .map(|((slot, read), n)| {
            let base = if slot.is_some() { filled } else { empty };
            (base + 12 * n, read as &dyn Fn(u16) -> Option<u8>)
        })
        .collect())
}

/// A slot's name as `$87:CB4B` copies it; an empty slot's script reads none.
fn slot_names<'a>(slots: &[Option<Slot<'a>>; 3]) -> [impl Fn(u16) -> Option<u8> + 'a; 3] {
    slots.map(|slot| name_at(SLOT_NAME, slot.map_or(&[][..], |slot| slot.name)))
}

/// The screen as it opens: the page, the three slots, the current game
/// named `current`; one page per request, positions relative to the text
/// area at (24, 88).
///
/// # Errors
/// A script that does not decode.
pub fn entry_text(
    image: &[u8],
    slots: &[Option<Slot<'_>>; 3],
    current: &[u8],
) -> Result<Vec<DialoguePage>, TextError> {
    let none = |_| None;
    let current_name = name_at(CURRENT_NAME, current);
    let readers = slot_names(slots);
    let mut sources: Vec<Request<'_>> = vec![(address(image, PAGE)?, &none)];
    sources.extend(slot_requests(image, slots, &readers)?);
    sources.push((address(image, CURRENT)?, &current_name));
    HouseDialogue::decode_requests(image, &sources)
}

/// The screen after saving to slot `saved` (0–2): the saved page, then the
/// slots again.
///
/// # Errors
/// A script that does not decode.
pub fn saved_text(
    image: &[u8],
    saved: u8,
    slots: &[Option<Slot<'_>>; 3],
) -> Result<Vec<DialoguePage>, TextError> {
    let number = move |at: u16| match at {
        SAVED_NUMBER => Some(saved.min(2) + 1),
        _ if at == SAVED_NUMBER + 1 => Some(0),
        _ => None,
    };
    let readers = slot_names(slots);
    let mut sources: Vec<Request<'_>> = vec![(address(image, SAVED)?, &number)];
    sources.extend(slot_requests(image, slots, &readers)?);
    HouseDialogue::decode_requests(image, &sources)
}

/// BG1's characters: an LZ packet of 128 4bpp tiles (VRAM `$3000`).
const BG_TILES: Address = Address::both(0xAB_8ABA, 0xAD_9543);
/// BG1's 32×32 map, raw (VRAM `$3800`).
const BG_MAP: Address = Address::both(0xE7_198F, 0xE9_198F);
/// CGRAM `$20–7F`: BG palettes 2–7; colour `$20` is also the backdrop.
const BG_COLOURS: Address = Address::both(0xCC_7096, 0xCE_7096);
/// OBJ tiles `$100–1FF`: an LZ packet (VRAM `$5000`).
const OBJ_TILES: Address = Address::both(0xC8_0890, 0xC8_777E);
/// CGRAM `$B0–FF`: OBJ palettes 3–7.
const OBJ_COLOURS: Address = Address::both(0xCD_0953, 0xCF_0953);
/// The actors' animation table; the rollers' and the cursor's frames.
const ANIMATIONS: Address = Address::both(0xB0_BD4F, 0xB2_C159);
const ROLLER: u32 = 0x42;
const CURSOR: u32 = 0x18E;
/// The title's glyphs: 旅のきろく, European "Records".
const TITLE: Address = Address::both(0x92_CC26, 0x92_E2CA);

/// The screen's art.
#[derive(Debug, Clone)]
pub struct RecordsArt {
    /// BG1's characters.
    pub bg_tiles: Vec<Tile4bpp>,
    /// BG1's map entries, 32×32.
    pub bg_map: Vec<u16>,
    /// CGRAM `$20–7F`.
    pub bg_colours: [Bgr555; 0x60],
    /// OBJ tiles `$100–1FF`.
    pub obj_tiles: Vec<Tile4bpp>,
    /// CGRAM `$B0–FF`.
    pub obj_colours: [Bgr555; 0x50],
    /// Each roller: 45 pieces, OBJ palette 6.
    pub roller: SpriteFrame,
    /// The cursor: one piece, OBJ palette 4.
    pub cursor: SpriteFrame,
    /// The title's 16×16 glyphs, in OBJ palette 4.
    pub title: Vec<Glyph>,
}

fn located(image: &[u8], address: Address) -> Result<u32, ShopError> {
    address
        .of(image)
        .ok_or(ShopError::Invalid(0, "no Records art in this revision"))
}

fn tiles(image: &[u8], address: Address, bytes: usize) -> Result<Vec<Tile4bpp>, ShopError> {
    let at = located(image, address)?;
    packet(image, at)?
        .get(..bytes)
        .ok_or(ShopError::Invalid(at, "short packet"))
        .and_then(|bytes| decode_tiles_4bpp(bytes).map_err(|_| ShopError::Invalid(at, "tiles")))
}

/// The sprite frame at `offset` into the animation table.
fn frame(image: &[u8], offset: u32) -> Result<SpriteFrame, ShopError> {
    let at = located(image, ANIMATIONS)? + offset;
    let count = usize::from(read(image, at + 16, 1)?[0]);
    SpriteFrame::decode(read(image, at, 17 + 7 * count)?)
        .map_err(|_| ShopError::Invalid(at, "sprite frame"))
}

impl RecordsArt {
    /// Decodes the art.
    ///
    /// # Errors
    /// Refuses a packet, map, palette or frame outside the image or
    /// malformed.
    pub fn from_rom(image: &[u8]) -> Result<Self, ShopError> {
        let map = located(image, BG_MAP)?;
        Ok(Self {
            bg_tiles: tiles(image, BG_TILES, 0x1000)?,
            bg_map: read(image, map, 0x800)?
                .chunks_exact(2)
                .map(|entry| u16::from_le_bytes([entry[0], entry[1]]))
                .collect(),
            bg_colours: colours(image, located(image, BG_COLOURS)?)?,
            obj_tiles: tiles(image, OBJ_TILES, 0x2000)?,
            obj_colours: colours(image, located(image, OBJ_COLOURS)?)?,
            roller: frame(image, ROLLER)?,
            cursor: frame(image, CURSOR)?,
            title: label_glyphs(image, located(image, TITLE)?)?,
        })
    }
}

/// The LEVEL and TIME labels' BG3 entries, on the bottom row.
const LEVEL_LABEL: [u16; 2] = [0x283D, 0x283E];
const TIME_LABEL: [u16; 2] = [0x280E, 0x280F];
/// A digit `d`: `$2C21 + d` over `$3031 + d` (the shop's panel digits);
/// the colon `$2C49` over `$3059`.
const DIGIT: [u16; 2] = [0x2C21, 0x3031];
const COLON: [u16; 2] = [0x2C49, 0x3059];

/// The BG3 entries of a level and a play time (`$85:C352`, `$86:8178`),
/// as (column, row, entry), `row` the top row: 11 + 2·slot for a slot,
/// 21 for the current game. Each number shows its last two decimal
/// digits; a level's and the hours' tens digit is blank when 0; 100 hours
/// or more show 99:59.
#[must_use]
pub fn stats_tiles(level: u8, seconds: u32, row: u8) -> Vec<(u8, u8, u16)> {
    let (hours, minutes) = match seconds / 3600 {
        hours @ 0..=99 => (hours, seconds % 3600 / 60),
        _ => (99, 59),
    };
    let mut tiles = Vec::new();
    for (column, [top, bottom]) in [(15, LEVEL_LABEL), (20, TIME_LABEL)] {
        tiles.extend([(column, row + 1, top), (column + 1, row + 1, bottom)]);
    }
    tiles.extend([(24, row, COLON[0]), (24, row + 1, COLON[1])]);
    for (column, value, blank_tens) in [
        (17, u32::from(level), true),
        (22, hours, true),
        (25, minutes, false),
    ] {
        let digits = [(column, value % 100 / 10), (column + 1, value % 10)];
        for (column, digit) in digits {
            if blank_tens && column == digits[0].0 && digit == 0 {
                continue;
            }
            let digit = u16::try_from(digit).unwrap_or(0);
            tiles.extend([
                (column, row, DIGIT[0] + digit),
                (column, row + 1, DIGIT[1] + digit),
            ]);
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stats_blank_leading_zeros_but_the_minutes() {
        let tiles = stats_tiles(7, 3 * 60 + 59, 11);
        let at = |column, row| {
            tiles
                .iter()
                .find(|&&(c, r, _)| (c, r) == (column, row))
                .map(|&(_, _, entry)| entry)
        };
        assert_eq!(at(17, 11), None, "level 7: no tens");
        assert_eq!((at(18, 11), at(18, 12)), (Some(0x2C28), Some(0x3038)));
        assert_eq!((at(22, 11), at(23, 11)), (None, Some(0x2C21)), "0 hours");
        assert_eq!((at(25, 11), at(26, 11)), (Some(0x2C21), Some(0x2C24)), "03");
        assert_eq!(
            (at(15, 12), at(15, 11)),
            (Some(0x283D), None),
            "label below"
        );
        // Level 100 shows 0; 111 hours show 99:59.
        let tiles = stats_tiles(100, 111 * 3600, 21);
        let digits: Vec<u16> = tiles
            .iter()
            .filter(|&&(_, row, entry)| row == 21 && entry & 0xFF00 == 0x2C00 && entry != COLON[0])
            .map(|&(_, _, entry)| entry - DIGIT[0])
            .collect();
        assert_eq!(digits, [0, 9, 9, 5, 9]);
    }
}
