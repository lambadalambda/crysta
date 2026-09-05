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
    pub fn collision_code(self) -> u8 {
        self.0.to_le_bytes()[1] & 0xFE
    }
}

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
