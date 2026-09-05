//! Dimension-prefixed compressed map layers, qualified against Japanese loading.
use super::{MapCell, MapError, LAYER_CAPACITY};
use crate::compression::{self, DecodeError};
use std::{fmt, ops::Range};

/// Invalid or unsupported static layer container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticMapError {
    /// The normalized offset or two-byte header is outside the supported ROM bank.
    SourceBounds {
        /// Requested normalized file offset.
        offset: usize,
    },
    /// Dimensions exceed the supported runtime layout.
    Dimensions(MapError),
    /// The compressed packet is malformed, truncated, or exceeds the layer size.
    Compression(DecodeError),
    /// Packet output is smaller than the declared rectangular layer.
    SizeMismatch {
        /// Bytes required by the dimensions.
        expected: usize,
        /// Bytes produced by the packet.
        actual: usize,
    },
}
impl fmt::Display for StaticMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceBounds { offset } => {
                write!(f, "map source ${offset:X} is outside a supported ROM bank")
            }
            Self::Dimensions(error) => error.fmt(f),
            Self::Compression(error) => error.fmt(f),
            Self::SizeMismatch { expected, actual } => write!(
                f,
                "map dimensions require {expected} bytes, packet produced {actual}"
            ),
        }
    }
}
impl std::error::Error for StaticMapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dimensions(error) => Some(error),
            Self::Compression(error) => Some(error),
            _ => None,
        }
    }
}

/// Lossless static layer before the loader applies attributes or dynamic changes.
///
/// The two dimension bytes count 256-pixel pages. A compressed packet follows,
/// producing row-major little-endian words for 16-pixel cells. No map identity,
/// collision semantics or automatic map-script resolution is inferred here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticLayer {
    width: usize,
    height: usize,
    cells: Vec<MapCell>,
    source_offset: usize,
    source: Vec<u8>,
}
impl StaticLayer {
    /// Decodes a dimension-prefixed layer at a normalized, headerless ROM offset.
    ///
    /// The caller authenticates the ROM and establishes pointer provenance. This
    /// parser supports offsets below 4 MiB and containers wholly inside one
    /// 64 KiB bank; cross-bank container behavior is not qualified. It preserves
    /// the exact source encoding, including unused compression control bits.
    ///
    /// # Errors
    /// Rejects invalid offsets, truncated/bank-crossing containers, zero or
    /// oversized dimensions, unsupported compression, and output-size mismatch.
    pub fn from_rom(image: &[u8], offset: usize) -> Result<Self, StaticMapError> {
        let bounds = || StaticMapError::SourceBounds { offset };
        if offset >= 0x40_0000 {
            return Err(bounds());
        }
        let suffix = image.get(offset..).ok_or_else(bounds)?;
        let input = &suffix[..suffix.len().min(0x10000 - (offset & 0xFFFF))];
        let dimensions = input.get(..2).ok_or_else(bounds)?;
        let (width, height) = (
            u16::from(dimensions[0]) * 256,
            u16::from(dimensions[1]) * 256,
        );
        if width == 0 || height == 0 {
            return Err(StaticMapError::Dimensions(MapError::InvalidDimensions {
                width,
                height,
            }));
        }
        let (width, height) = (usize::from(width / 16), usize::from(height / 16));
        let expected = width * height * 2;
        if expected > LAYER_CAPACITY {
            return Err(StaticMapError::Dimensions(MapError::LayerTooLarge {
                bytes: expected,
            }));
        }
        let packet =
            compression::decode(&input[2..], expected).map_err(StaticMapError::Compression)?;
        if packet.data.len() != expected {
            return Err(StaticMapError::SizeMismatch {
                expected,
                actual: packet.data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            cells: packet
                .data
                .chunks_exact(2)
                .map(|bytes| MapCell(u16::from_le_bytes([bytes[0], bytes[1]])))
                .collect(),
            source_offset: offset,
            source: input[..2 + packet.consumed].to_vec(),
        })
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
    /// Raw words in row-major order, before runtime attribute processing.
    #[must_use]
    pub fn cells(&self) -> &[MapCell] {
        &self.cells
    }
    /// Looks up a cell without wrapping invalid coordinates.
    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<MapCell> {
        if x >= self.width || y >= self.height {
            None
        } else {
            Some(self.cells[y * self.width + x])
        }
    }
    /// Applies the trace-qualified loader's 512-entry metatile attribute lookup.
    ///
    /// For each cell, the loader keeps its low nine bits and replaces all upper
    /// bits with `(table[index] & 0x7F) << 9`. The static words remain unchanged.
    /// This reproduces initialization, not later object changes or passability.
    #[must_use]
    pub fn attributed_cells(&self, table: &[u8; 512]) -> Vec<MapCell> {
        self.cells
            .iter()
            .map(|cell| {
                let index = cell.tile_index();
                MapCell(index | (u16::from(table[usize::from(index)] & 0x7F) << 9))
            })
            .collect()
    }
    /// Original normalized source extent, including dimensions and packet.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.source_offset..self.source_offset + self.source.len()
    }
    /// Exact original container bytes, not a recompressed approximation.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    /// Losslessly reproduces the decoded layer's little-endian words.
    #[must_use]
    pub fn layer_bytes(&self) -> Vec<u8> {
        self.cells
            .iter()
            .flat_map(|cell| cell.raw().to_le_bytes())
            .collect()
    }
}
