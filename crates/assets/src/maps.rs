//! Qualified Japanese static and runtime map layers.
//!
//! The loader stores pixel dimensions at `$7E:0826/082A`, in 256-pixel units.
//! The first runtime layer at `$7E:A000` consists of row-major 16-bit cells for
//! 16×16-pixel tiles. Preserve the raw word: collision meanings are not yet a
//! portable movement specification. See `docs/maps.md` for qualification limits.

use std::fmt;

pub mod exits;
pub mod scripts;
mod static_layer;
pub mod visual;
pub use static_layer::{StaticLayer, StaticMapError};

const WRAM_SIZE: usize = 0x20000;
const LAYER_START: usize = 0xA000;
const LAYER_CAPACITY: usize = 0x4000;

/// A raw runtime tile/collision word, including dynamic bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCell(u16);

impl MapCell {
    /// Original little-endian cell word.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
    /// Low nine bits: candidate metatile index, not an SNES VRAM tile index.
    #[must_use]
    pub const fn tile_index(self) -> u16 {
        self.0 & 0x1FF
    }
    /// Upper byte excluding its low bit, as used by the community collision overlay.
    /// This is a code, not a claim that a tile is passable or blocked.
    #[must_use]
    pub const fn collision_code(self) -> u8 {
        self.0.to_le_bytes()[1] & 0xFE
    }
    /// Bits 9..15 as the loader wrote them at initialization.
    ///
    /// This inverts the qualified initialization transform
    /// `index | ((attributes[index] & $7F) << 9)` recorded in
    /// `docs/static-maps.md`. [`Self::collision_code`] is exactly twice this
    /// value for every 16-bit word, because the index contributes only bit 8
    /// and that bit is masked away.
    ///
    /// **This is only the loader's value on a freshly initialized layer.** The
    /// same document records bit 15 being set later during play, with its
    /// writer and meaning unclaimed, and that bit lies inside this field. On a
    /// runtime layer prefer [`Self::base_attribute`] and [`Self::dynamic_bit`].
    /// Note also that the loader's own `& $7F` discards bit 7 of the source
    /// table byte, so at most seven bits are ever recoverable.
    #[must_use]
    pub const fn attribute(self) -> u8 {
        ((self.0 >> 9) & 0x7F) as u8
    }
    /// Bits 9..14: the attribute without the bit a running game overwrites.
    ///
    /// Word bit 15 is attribute bit 6. Because gameplay reuses it, a runtime
    /// cell cannot distinguish a source attribute of `n | $40` from `n` with
    /// the dynamic bit set. This returns the unambiguous six bits. In the
    /// measured house map no initialized attribute has bit 6 set, so nothing is
    /// lost there; that is a property of that map, not a general guarantee.
    #[must_use]
    pub const fn base_attribute(self) -> u8 {
        ((self.0 >> 9) & 0x3F) as u8
    }
    /// Word bit 15, which `docs/static-maps.md` observes being set during play.
    ///
    /// Its writer and gameplay meaning are not claimed here.
    #[must_use]
    pub const fn dynamic_bit(self) -> bool {
        self.0 & 0x8000 != 0
    }
    /// Movement semantics, where measured play has established them.
    ///
    /// Keyed on [`Self::base_attribute`], so a cell whose dynamic bit has been
    /// set still resolves. Returns `None` for a base attribute no qualified
    /// sample covers: this deliberately refuses to guess, because an
    /// unqualified attribute is unknown rather than walkable. See
    /// `docs/collision.md` for the evidence and its bounds.
    #[must_use]
    pub const fn qualified_passability(self) -> Option<Passability> {
        match self.base_attribute() {
            0 | 2 | 22 => Some(Passability::Walkable),
            12 | 14 | 16 | 25 => Some(Passability::Solid),
            _ => None,
        }
    }
}

/// Whether the player's collision point may occupy a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passability {
    /// The collision point was observed standing on this attribute.
    Walkable,
    /// This attribute stopped a sustained directional press.
    Solid,
}

/// Attributes a qualified collision point was observed to occupy.
pub const QUALIFIED_WALKABLE: &[u8] = &[0, 2, 22];
/// Attributes observed to stop a sustained directional press.
pub const QUALIFIED_SOLID: &[u8] = &[12, 14, 16, 25];

/// Invalid input to the qualified runtime map reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// A complete 128 KiB WRAM image is required.
    WramSize {
        /// Actual input length.
        actual: usize,
    },
    /// Dimensions must be nonzero multiples of 256 pixels.
    InvalidDimensions {
        /// Width in pixels.
        width: u16,
        /// Height in pixels.
        height: u16,
    },
    /// Cells would cross into the next buffer at `$7E:E000`.
    LayerTooLarge {
        /// Requested bytes.
        bytes: usize,
    },
}
impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WramSize { actual } => write!(f, "expected 128 KiB WRAM, got {actual} bytes"),
            Self::InvalidDimensions { width, height } => {
                write!(f, "unsupported map dimensions {width}×{height} pixels")
            }
            Self::LayerTooLarge { bytes } => write!(
                f,
                "map layer needs {bytes} bytes, exceeds $A000..$E000 buffer"
            ),
        }
    }
}
impl std::error::Error for MapError {}

/// Validated metadata and cells from one loaded map checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedMap {
    id: u16,
    width: usize,
    height: usize,
    camera: (u16, u16),
    player: (u16, u16),
    cells: Vec<MapCell>,
}
impl LoadedMap {
    /// Reads the qualified runtime layout from a caller-owned WRAM image.
    ///
    /// The caller must establish ROM revision and checkpoint identity. This
    /// function validates structure, not whether a game is done loading a map.
    ///
    /// # Errors
    /// Rejects incorrect WRAM size, unsupported dimensions, and buffer overflow.
    pub fn from_wram(wram: &[u8]) -> Result<Self, MapError> {
        if wram.len() != WRAM_SIZE {
            return Err(MapError::WramSize { actual: wram.len() });
        }
        let word = |offset| u16::from_le_bytes([wram[offset], wram[offset + 1]]);
        let (width, height) = (word(0x826), word(0x82A));
        if width == 0 || height == 0 || width % 256 != 0 || height % 256 != 0 {
            return Err(MapError::InvalidDimensions { width, height });
        }
        let (width, height) = (usize::from(width / 16), usize::from(height / 16));
        let bytes = width * height * 2;
        if bytes > LAYER_CAPACITY {
            return Err(MapError::LayerTooLarge { bytes });
        }
        let cells = wram[LAYER_START..LAYER_START + bytes]
            .chunks_exact(2)
            .map(|bytes| MapCell(u16::from_le_bytes([bytes[0], bytes[1]])))
            .collect();
        Ok(Self {
            id: word(0x47E),
            width,
            height,
            camera: (word(0x80E), word(0x812)),
            player: (word(0x1000), word(0x1002)),
            cells,
        })
    }
    /// Full 16-bit runtime map ID.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        self.id
    }
    /// Width in 16-pixel cells.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }
    /// Height in 16-pixel cells.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }
    /// Runtime camera coordinates from `$080E/$0812`, in pixels.
    #[must_use]
    pub const fn camera(&self) -> (u16, u16) {
        self.camera
    }
    /// Player coordinates from `$1000/$1002`, in pixels.
    #[must_use]
    pub const fn player(&self) -> (u16, u16) {
        self.player
    }
    /// Original row-major cell sequence.
    #[must_use]
    pub fn cells(&self) -> &[MapCell] {
        &self.cells
    }
    /// Looks up a cell without wrapping out-of-bounds coordinates.
    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<MapCell> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(self.cells[y * self.width + x])
        }
    }
    /// Looks up a cell using map-relative pixel coordinates.
    #[must_use]
    pub fn cell_at_pixel(&self, x: usize, y: usize) -> Option<MapCell> {
        self.cell(x / 16, y / 16)
    }
    /// Losslessly reproduces the captured layer bytes, without reinterpreting flags.
    #[must_use]
    pub fn layer_bytes(&self) -> Vec<u8> {
        self.cells
            .iter()
            .flat_map(|cell| cell.raw().to_le_bytes())
            .collect()
    }
}

#[cfg(test)]
mod collision_tests {
    use super::{MapCell, Passability, QUALIFIED_SOLID, QUALIFIED_WALKABLE};

    /// Builds a cell exactly as the qualified loader transform does.
    fn initialized(index: u16, attribute: u8) -> MapCell {
        MapCell(index | (u16::from(attribute & 0x7F) << 9))
    }

    #[test]
    fn attribute_inverts_the_qualified_initialization_transform() {
        for attribute in 0..=0x7Fu8 {
            for index in [0u16, 1, 0x0C, 0xAC, 0x1FE, 0x1FF] {
                let cell = initialized(index, attribute);
                assert_eq!(cell.attribute(), attribute);
                assert_eq!(cell.tile_index(), index);
            }
        }
    }

    #[test]
    fn community_code_is_exactly_twice_the_attribute() {
        // The published overlay mask is an equivalent encoding, not a different
        // reading: the index reaches the upper byte only through bit 8, which
        // the mask clears. It has simply never carried a passability claim.
        for attribute in 0..=0x7Fu8 {
            for index in [0u16, 1, 0x100, 0x1FF] {
                let cell = initialized(index, attribute);
                assert_eq!(u16::from(cell.collision_code()), u16::from(attribute) * 2);
            }
        }
    }

    /// Words taken verbatim from the measured map `$000F` sweeps: every one of
    /// these was either stood on or stopped a sustained press.
    #[test]
    fn measured_house_words_resolve_to_their_observed_semantics() {
        for (raw, base, expected) in [
            (0x1c0au16, 14, Some(Passability::Solid)), // blocked a held press
            (0x1c0b, 14, Some(Passability::Solid)),
            (0x1c59, 14, Some(Passability::Solid)),
            (0x1868, 12, Some(Passability::Solid)),
            (0x1869, 12, Some(Passability::Solid)),
            (0x183b, 12, Some(Passability::Solid)),
            (0x1c20, 14, Some(Passability::Solid)),
            (0x0001, 0, Some(Passability::Walkable)), // stood on
            (0x00ac, 0, Some(Passability::Walkable)),
            (0x2cd2, 22, Some(Passability::Walkable)),
            (0x2cd5, 22, Some(Passability::Walkable)),
            // Walked through: map $000F cell (24,12) carried the player into
            // map $0010, so this attribute admits movement.
            (0x0436, 2, Some(Passability::Walkable)),
            // Refused a sustained press from the only reachable side.
            (0x2043, 16, Some(Passability::Solid)),
            // The town exterior's impassable band, solid under every
            // consistent offset of the map $000A sweep.
            (0x3243, 25, Some(Passability::Solid)),
        ] {
            let cell = MapCell(raw);
            assert_eq!(cell.base_attribute(), base, "base attribute of {raw:#06x}");
            assert_eq!(cell.qualified_passability(), expected, "cell {raw:#06x}");
        }
    }

    /// The two cells in map `$000F` whose dynamic bit is set.
    #[test]
    fn dynamic_bit_does_not_hide_a_qualified_attribute() {
        for (runtime, initialized, base) in [(0x9ce8u16, 0x1ce8u16, 14), (0x9845, 0x1845, 12)] {
            let cell = MapCell(runtime);
            assert!(cell.dynamic_bit());
            assert!(!MapCell(initialized).dynamic_bit());
            // The seven-bit reading splits one attribute into two...
            assert_eq!(cell.attribute(), base + 0x40);
            assert_eq!(MapCell(initialized).attribute(), base);
            // ...while the six-bit reading keeps them together, so a wall whose
            // dynamic bit is set is still solid rather than unknown.
            assert_eq!(cell.base_attribute(), base);
            assert_eq!(cell.qualified_passability(), Some(Passability::Solid));
            assert_eq!(cell.tile_index(), MapCell(initialized).tile_index());
        }
    }

    #[test]
    fn unqualified_attributes_are_unknown_rather_than_walkable() {
        // The Crysta slice also carries base attributes 5, 6, 7, 8, 21 and 29.
        // No sample resolves them, so the decoder must not silently admit the
        // player. 76 and 78 are deliberately absent: they are 12 and 14
        // with the dynamic bit, not attributes any map contains.
        for base in [5u8, 6, 7, 8, 21, 29] {
            assert_eq!(initialized(0x40, base).qualified_passability(), None);
        }
        let qualified: Vec<u8> = (0..0x40u8)
            .filter(|a| initialized(0, *a).qualified_passability().is_some())
            .collect();
        let mut expected = [QUALIFIED_WALKABLE, QUALIFIED_SOLID].concat();
        expected.sort_unstable();
        assert_eq!(qualified, expected);
    }

    #[test]
    fn walkable_and_solid_sets_stay_disjoint() {
        for solid in QUALIFIED_SOLID {
            assert!(!QUALIFIED_WALKABLE.contains(solid), "{solid} in both sets");
        }
    }
}
