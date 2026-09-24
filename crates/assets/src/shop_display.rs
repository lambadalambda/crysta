//! The art of a shop's display ([notes](../../../docs/shops.md)): the
//! sprite sheet of the count row and coin, the BG3 characters of the price
//! and the money, their palettes, an item's icon and its name.
//!
//! The addresses are the Japanese ones; the European ROM holds the same
//! bytes elsewhere ([`crate::layout::at`]), and its names in English.

use crate::compression;
use crate::graphics::{decode_tiles_4bpp, Bgr555, Tile4bpp};
use crate::labels::located;
use crate::shops::{read, ShopError};

/// `$A9:F02F`: the sprite sheet `$85:A48D` unpacks; tiles `$40..$5F` sit
/// at `$800` (VRAM `$4200` onward).
const SPRITES: (u32, usize) = (0xA9_F02F, 0x800);
/// `$A9:9000`: BG3's 2bpp characters, from the map's resource list.
const PANEL: u32 = 0xA9_9000;
/// `$B2:8B78`: BG3's eight 4-colour palettes.
const PANEL_PALETTE: u32 = 0xB2_8B78;
/// The icon's tile index path (`$84:D628`): `$A8:8000` by item, then
/// `+$8002`, then `+$8016` in the same bank; tiles from `$A2:8000`, 32
/// bytes each.
const ICON_INDEX: u32 = 0xA8_8000;
const ICON_TILES: u32 = 0xA2_8000;
/// The icon's 8 colours (`$84:C278`): `$AF:E43B` by the byte at
/// `$B1:DA31` + item.
const ICON_PALETTES: (u32, u32) = (0xAF_E43B, 0xB1_DA31);
/// Item names: `C9 <width>`, glyph codes, `D4` (pointers at `$92:8179`).
const NAMES: u32 = 0x92_8179;
/// Bytes one LZ packet may unpack to.
const PACKET: usize = 0x2000;

/// The display's fixed art.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopArt {
    /// Sprite tiles `$40..$5F`: the count digits `$40..$49`, `×`, `=` and
    /// the coin.
    pub sprites: Vec<Tile4bpp>,
    /// OBJ palette 2.
    pub sprite_palette: [Bgr555; 16],
    /// OBJ palette 4 as the count digits use it: colour 1 black
    /// (`$8D:8B93`) and 14 white (`$8D:AC30`); the rest is not drawn.
    pub count_palette: [Bgr555; 16],
    /// BG3 characters `$00..$5F`, 2bpp, row-major indices.
    pub panel: Vec<[u8; 64]>,
    /// BG3's palettes, 4 colours each.
    pub panel_palette: [Bgr555; 32],
}

impl ShopArt {
    /// `×` in the count row.
    pub const TIMES: u8 = 0x5A;
    /// `=` in the count row.
    pub const EQUALS: u8 = 0x5B;
    /// The coin before the price.
    pub const COIN: u8 = 0x5E;
    /// The count digit `d` is sprite tile `DIGITS + d`.
    pub const DIGITS: u8 = 0x40;
    /// A panel digit `d` is BG3 tile `TOP + d` (palette 3) over
    /// `BOTTOM + d` (palette 4), `$85:ECB7`.
    pub const TOP: u8 = 0x21;
    /// The digits' lower halves.
    pub const BOTTOM: u8 = 0x31;
    /// The money bag, 2 × 2 tiles, palette 3 (`$CB:2CF7`).
    pub const BAG: [u8; 4] = [0x40, 0x41, 0x50, 0x51];

    /// Decodes the art.
    ///
    /// # Errors
    /// Refuses a packet or palette outside the image or malformed.
    pub fn from_rom(image: &[u8]) -> Result<Self, ShopError> {
        let sheet = packet(image, located(image, SPRITES.0)?)?;
        let sprites = sheet
            .get(SPRITES.1..SPRITES.1 + 32 * 32)
            .ok_or(ShopError::Invalid(SPRITES.0, "short sprite sheet"))
            .and_then(|bytes| {
                decode_tiles_4bpp(bytes).map_err(|_| ShopError::Invalid(SPRITES.0, "tiles"))
            })?;
        let panel = packet(image, located(image, PANEL)?)?
            .get(..0x60 * 16)
            .ok_or(ShopError::Invalid(PANEL, "short panel characters"))?
            .chunks_exact(16)
            .map(crate::graphics::decode_tile_2bpp)
            .collect();
        let mut count_palette = [Bgr555::new(0); 16];
        count_palette[14] = Bgr555::new(0x7FFF);
        Ok(Self {
            sprites,
            sprite_palette: crate::labels::label_palette(image)?,
            count_palette,
            panel,
            panel_palette: colours(image, located(image, PANEL_PALETTE)?)?,
        })
    }
}

/// An item's 16×16 icon, top-left, top-right, bottom-left, bottom-right,
/// and its 8 colours (OBJ palette 7, colours 8..15).
///
/// # Errors
/// Refuses reads outside the image.
pub fn item_icon(image: &[u8], item: u8) -> Result<([Tile4bpp; 4], [Bgr555; 8]), ShopError> {
    let index = located(image, ICON_INDEX)?;
    let bank = index & 0xFF_0000;
    let first = word(image, index + u32::from(item) * 2)?;
    let second = word(image, bank | u32::from(first).wrapping_add(0x8002) & 0xFFFF)?;
    let tile = word(
        image,
        bank | u32::from(second).wrapping_add(0x8016) & 0xFFFF,
    )? & 0x1FF;
    let source = located(image, ICON_TILES)? + u32::from(tile) * 32;
    let mut tiles = Vec::with_capacity(4);
    for row in [source, source + 0x200] {
        tiles.extend(
            decode_tiles_4bpp(read(image, row, 64)?)
                .map_err(|_| ShopError::Invalid(row, "icon tiles"))?,
        );
    }
    let entry = read(image, located(image, ICON_PALETTES.1)? + u32::from(item), 1)?[0];
    let palette = read(
        image,
        located(image, ICON_PALETTES.0)? + u32::from(entry) * 32,
        16,
    )?;
    Ok((
        [tiles[0], tiles[1], tiles[2], tiles[3]],
        std::array::from_fn(|i| {
            Bgr555::new(u16::from_le_bytes([palette[i * 2], palette[i * 2 + 1]]))
        }),
    ))
}

/// A colour at half brightness, as `$84:C29F` dims an icon.
#[must_use]
pub const fn halve(colour: Bgr555) -> Bgr555 {
    Bgr555::new((colour.raw() & 0xFBDE) >> 1)
}

/// An item's name: the width byte that places it, and its 16×16 glyphs in
/// the dialogue font, colour 3 cleared (0 and 3 transparent, 1 and 2 drawn
/// with OBJ palette 2).
///
/// # Errors
/// Refuses a name that leaves the image, does not end, or holds a code
/// other than glyphs and the kana switches.
pub fn name_glyphs(image: &[u8], item: u8) -> Result<(u8, Vec<[u8; 256]>), ShopError> {
    let names = located(image, NAMES)?;
    let start = names & 0xFF_0000 | u32::from(word(image, names + u32::from(item) * 2)?);
    let head = read(image, start, 2)?;
    if head[0] != 0xC9 {
        return Err(ShopError::Invalid(start, "name without its width"));
    }
    Ok((head[1], crate::labels::label_glyphs(image, start + 2)?))
}

fn packet(image: &[u8], at: u32) -> Result<Vec<u8>, ShopError> {
    let start = usize::try_from(at & 0x3F_FFFF).map_err(|_| ShopError::Truncated(at))?;
    let input = image.get(start..).ok_or(ShopError::Truncated(at))?;
    compression::decode(input, PACKET)
        .map(|packet| packet.data)
        .map_err(|_| ShopError::Invalid(at, "malformed packet"))
}

fn colours<const N: usize>(image: &[u8], at: u32) -> Result<[Bgr555; N], ShopError> {
    let bytes = read(image, at, N * 2)?;
    Ok(std::array::from_fn(|i| {
        Bgr555::new(u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]))
    }))
}

fn word(image: &[u8], at: u32) -> Result<u16, ShopError> {
    let bytes = read(image, at, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_drops_each_channels_low_bit() {
        assert_eq!(halve(Bgr555::new(0x7FFF)).raw(), 0x3DEF);
    }
}
