//! Pure SNES 4bpp background primitives over caller-owned data.
//!
//! Coordinates start at the top left. Pixels and metatile words are row-major.
//! No VRAM addressing, resource extraction, compositing, or brightness effects
//! are inferred; callers supply decoded graphics with tile index zero at slot zero.

use std::fmt;

/// Invalid graphics input or an out-of-bounds metatile sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsError {
    /// A single 4bpp tile must contain exactly 32 bytes.
    TileSize {
        /// Supplied byte count.
        actual: usize,
    },
    /// A tile sequence must contain a multiple of 32 bytes.
    TilesSize {
        /// Supplied byte count.
        actual: usize,
    },
    /// The coordinate is outside a 16×16 metatile.
    PixelOutOfBounds {
        /// Requested horizontal coordinate.
        x: usize,
        /// Requested vertical coordinate.
        y: usize,
    },
    /// The sampled word references graphics absent from the supplied slice.
    MissingTile {
        /// Referenced ten-bit graphics index.
        index: u16,
    },
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TileSize { actual } => write!(f, "expected 32 tile bytes, got {actual}"),
            Self::TilesSize { actual } => {
                write!(f, "expected a multiple of 32 tile bytes, got {actual}")
            }
            Self::PixelOutOfBounds { x, y } => write!(f, "pixel ({x}, {y}) is outside 16×16"),
            Self::MissingTile { index } => write!(f, "missing graphics tile {index}"),
        }
    }
}

impl std::error::Error for GraphicsError {}

/// One decoded 8×8 tile of four-bit color indices, stored row-major.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile4bpp([u8; 64]);

impl Tile4bpp {
    /// Decodes SNES planar bytes: interleaved planes 0/1 in bytes 0..16,
    /// planes 2/3 in bytes 16..32, with bit 7 at the left of each row.
    ///
    /// # Errors
    /// Rejects input that is not exactly 32 bytes (including trailing bytes).
    pub fn decode(bytes: &[u8]) -> Result<Self, GraphicsError> {
        if bytes.len() != 32 {
            return Err(GraphicsError::TileSize {
                actual: bytes.len(),
            });
        }
        let mut pixels = [0; 64];
        for y in 0..8 {
            for x in 0..8 {
                for plane in 0..4 {
                    let byte = bytes[(plane / 2) * 16 + y * 2 + plane % 2];
                    pixels[y * 8 + x] |= ((byte >> (7 - x)) & 1) << plane;
                }
            }
        }
        Ok(Self(pixels))
    }

    /// All 64 color indices, each in 0..=15, row-major.
    #[must_use]
    pub const fn pixels(&self) -> &[u8; 64] {
        &self.0
    }

    /// Returns a color index, or `None` if either coordinate is outside 0..8.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x < 8 && y < 8 {
            Some(self.0[y * 8 + x])
        } else {
            None
        }
    }
}

/// Decodes consecutive 32-byte tiles without ignoring any trailing bytes.
/// Empty input represents an empty tile set; allocation is proportional to input.
///
/// # Errors
/// Rejects byte lengths that are not multiples of 32.
pub fn decode_tiles_4bpp(bytes: &[u8]) -> Result<Vec<Tile4bpp>, GraphicsError> {
    if !bytes.len().is_multiple_of(32) {
        return Err(GraphicsError::TilesSize {
            actual: bytes.len(),
        });
    }
    bytes.chunks_exact(32).map(Tile4bpp::decode).collect()
}

/// A raw SNES BGR555 word: red in bits 0..4, green 5..9, blue 10..14.
/// Unused bit 15 is retained but has no effect on RGB conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bgr555(u16);

impl Bgr555 {
    /// Retains an entire caller-supplied word, including bit 15.
    #[must_use]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    /// The unmodified word.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Full-brightness RGB, expanding each channel with `(v << 3) | (v >> 2)`.
    #[must_use]
    pub fn rgb8(self) -> [u8; 3] {
        [0, 5, 10].map(|shift| {
            let value = ((self.0 >> shift) & 31) as u8;
            (value << 3) | (value >> 2)
        })
    }

    /// Packs RGB by discarding each channel's low three bits; bit 15 is zero.
    /// This exactly reverses [`Self::rgb8`] except for the unused raw bit 15.
    #[must_use]
    pub fn from_rgb8([r, g, b]: [u8; 3]) -> Self {
        Self(u16::from(r >> 3) | (u16::from(g >> 3) << 5) | (u16::from(b >> 3) << 10))
    }
}

/// An SNES background tilemap word, without game-specific interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgTileWord(u16);

impl BgTileWord {
    /// Retains all bits of a caller-supplied word.
    #[must_use]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    /// The unmodified word.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Graphics index in bits 0..9 (0..=1023).
    #[must_use]
    pub const fn tile_index(self) -> u16 {
        self.0 & 0x03ff
    }

    /// 16-color palette selection in bits 10..12 (0..=7).
    #[must_use]
    pub const fn palette(self) -> u8 {
        ((self.0 >> 10) & 7) as u8
    }

    /// Priority flag in bit 13; no layer ordering is inferred.
    #[must_use]
    pub const fn priority(self) -> bool {
        self.0 & 0x2000 != 0
    }

    /// Horizontal flip flag in bit 14.
    #[must_use]
    pub const fn hflip(self) -> bool {
        self.0 & 0x4000 != 0
    }

    /// Vertical flip flag in bit 15.
    #[must_use]
    pub const fn vflip(self) -> bool {
        self.0 & 0x8000 != 0
    }
}

/// An indexed background sample, not an RGB value or a composited pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedPixel {
    /// Tile color zero, regardless of palette; not an opaque backdrop color.
    Transparent,
    /// A visible palette entry with the tile's priority flag.
    Opaque {
        /// CGRAM index: `palette * 16 + color` for 4bpp background samples.
        /// Callers can separately represent an opaque backdrop at index zero.
        palette_index: u8,
        /// Raw priority flag, without layer-compositing semantics.
        priority: bool,
    },
}

/// Samples four 8×8 words arranged as top-left, top-right, bottom-left,
/// bottom-right in a 16×16 metatile. Flips apply within each 8×8 tile only.
/// Graphics indices address `tiles` directly; color zero is always transparent.
/// Only the word covering the requested coordinate needs available graphics.
///
/// # Errors
/// Rejects coordinates outside 0..16 before indexing, or missing tile graphics.
pub fn sample_metatile(
    words: &[BgTileWord; 4],
    tiles: &[Tile4bpp],
    x: usize,
    y: usize,
) -> Result<IndexedPixel, GraphicsError> {
    if x >= 16 || y >= 16 {
        return Err(GraphicsError::PixelOutOfBounds { x, y });
    }
    let word = words[(y / 8) * 2 + x / 8];
    let tile = tiles
        .get(usize::from(word.tile_index()))
        .ok_or(GraphicsError::MissingTile {
            index: word.tile_index(),
        })?;
    let tx = if word.hflip() { 7 - x % 8 } else { x % 8 };
    let ty = if word.vflip() { 7 - y % 8 } else { y % 8 };
    let color = tile.pixels()[ty * 8 + tx];
    Ok(if color == 0 {
        IndexedPixel::Transparent
    } else {
        IndexedPixel::Opaque {
            palette_index: word.palette() * 16 + color,
            priority: word.priority(),
        }
    })
}
