//! The Restart file select ([notes](../../../docs/restart-screen.md)): map
//! `$04`'s scroll, its title and its page. The rest of its art is the
//! Records screen's ([`crate::records::RecordsArt`]).

use crate::labels::{label_glyphs, Glyph};
use crate::layout::Address;
use crate::maps::StaticLayer;
use crate::records::{address, located, slot_names, slot_requests, Slot};
use crate::shop_display::packet;
use crate::shops::ShopError;
use crate::text::{DialoguePage, HouseDialogue, Request, TextError};

/// Map `$04`'s layer, 16×48 cells, and its metatiles (an LZ packet, 8
/// bytes each).
const LAYER: Address = Address::both(0xCD_16C7, 0xCF_16F8);
const METATILES: Address = Address::both(0xCA_5513, 0xCC_5513);
/// The title's glyphs: 旅の再開, European "Restart".
const TITLE: Address = Address::both(0x92_CBFB, 0x92_E29F);
/// The page: ` 1`–` 3`, New Game, Copy Data, Erase Data.
const PAGE: Address = Address::both(0x92_CB21, 0x92_E1BE);

/// The screen's own art.
#[derive(Debug, Clone)]
pub struct RestartArt {
    /// BG1's map entries, 32×32, with the camera at 0,0.
    pub bg_map: Vec<u16>,
    /// The title's 16×16 glyphs, in OBJ palette 4.
    pub title: Vec<Glyph>,
}

impl RestartArt {
    /// Decodes the art.
    ///
    /// # Errors
    /// Refuses a layer, packet or label outside the image or malformed.
    pub fn from_rom(image: &[u8]) -> Result<Self, ShopError> {
        let at = located(image, LAYER)?;
        let layer = StaticLayer::from_rom(image, (at & 0x3F_FFFF) as usize)
            .map_err(|_| ShopError::Invalid(at, "map $04's layer"))?;
        let metatiles = packet(image, located(image, METATILES)?)?;
        // Tile row r, column c: word (r mod 2)·2 + (c mod 2) of the cell's
        // metatile.
        let bg_map = (0..32 * 32)
            .map(|index| {
                let (row, column) = (index / 32, index % 32);
                let cell = layer
                    .cell(column / 2, row / 2)
                    .map_or(0, |cell| cell.raw() & 0x1FF);
                let word = usize::from(cell) * 8 + (row % 2) * 4 + (column % 2) * 2;
                metatiles
                    .get(word..word + 2)
                    .map_or(0, |bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            })
            .collect();
        Ok(Self {
            bg_map,
            title: label_glyphs(image, located(image, TITLE)?)?,
        })
    }
}

/// The page and the three slots, as the screen types them: one page per
/// request, positions relative to the text area at (24, 88).
///
/// # Errors
/// A script that does not decode.
pub fn page_text(
    image: &[u8],
    slots: &[Option<Slot<'_>>; 3],
) -> Result<Vec<DialoguePage>, TextError> {
    let none = |_| None;
    let readers = slot_names(slots);
    let mut sources: Vec<Request<'_>> = vec![(address(image, PAGE)?, &none)];
    sources.extend(slot_requests(image, slots, &readers)?);
    HouseDialogue::decode_requests(image, &sources)
}
