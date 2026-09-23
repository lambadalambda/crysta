//! A world map's Mode 7 background: the underworld `$03`.
//!
//! The layer is not in the loading script: the spawn stream's `$F0` record
//! (`$80:F689`) names it, uncompressed, one byte per 16-pixel cell. Each byte
//! picks a metatile of four 8bpp characters (top-left, top-right,
//! bottom-left, bottom-right) from the script's metatile load, and the
//! characters are 64 palette indices each. Rebuilt, the layer, metatiles and
//! characters match the native Mode 7 tilemap and character data. The native
//! view is in perspective; this is the flat plane it samples.

use super::{resource, VisualMapError};
use crate::graphics::Bgr555;
use crate::maps::actors::SpawnList;
use crate::maps::scripts::{self, Command, Limits, ResourceKind};

/// Maps whose world layer is qualified.
const WORLD_MAPS: [u16; 1] = [0x0003];
/// Metatiles: 256 of four character numbers.
const METATILES: usize = 256 * 4;
/// Characters: 256 of 8x8 palette indices.
const CHARACTERS: usize = 256 * 64;

/// A world map's plane, palette and cells.
#[derive(Debug, Clone)]
pub struct WorldMap {
    width: usize,
    height: usize,
    cells: Vec<u8>,
    metatiles: Vec<u8>,
    characters: Vec<u8>,
    palette: [Bgr555; 256],
}

impl WorldMap {
    /// Decodes a qualified world map.
    ///
    /// # Errors
    /// Rejects other maps, a stream without its layer record, and a loading
    /// script without the character, metatile and palette loads.
    pub fn from_rom(image: &[u8], map: u16) -> Result<Self, VisualMapError> {
        let unsupported = VisualMapError::Unsupported;
        if !WORLD_MAPS.contains(&map) {
            return Err(unsupported("unqualified world map ID"));
        }
        let layer = SpawnList::from_rom(image, map)
            .map_err(|_| unsupported("world map spawn stream"))?
            .world_layer()
            .ok_or(unsupported("world map without a layer record"))?;
        let (width, height) = (
            usize::from(layer.width_pages) * 16,
            usize::from(layer.height_pages) * 16,
        );
        let cells = image
            .get(layer.source..layer.source + width * height)
            .ok_or(unsupported("world layer outside the ROM"))?
            .to_vec();
        let program =
            scripts::resolve_map(image, map, Limits::default()).map_err(VisualMapError::Script)?;
        // The base loads, before the first `End`.
        let loads: Vec<_> = program
            .instructions
            .iter()
            .take_while(|instruction| !matches!(instruction.command, Command::End))
            .filter_map(|instruction| match instruction.command {
                Command::Resource { kind, source } => Some((
                    kind,
                    source.normalized().value() as usize,
                    instruction.bytes.clone(),
                )),
                _ => None,
            })
            .collect();
        let first = |wanted: ResourceKind| {
            loads
                .iter()
                .find(|(kind, _, _)| *kind == wanted)
                .ok_or(unsupported("world map without a required load"))
        };
        let characters = resource(
            image,
            first(ResourceKind::Graphics)?.1,
            ResourceKind::Graphics,
            CHARACTERS,
            true,
        )?
        .decoded;
        let metatiles = resource(
            image,
            first(ResourceKind::Metatiles)?.1,
            ResourceKind::Metatiles,
            METATILES,
            true,
        )?
        .decoded;
        let mut palette = [Bgr555::new(0); 256];
        for (_, source, bytes) in loads
            .iter()
            .filter(|(kind, _, _)| *kind == ResourceKind::Palette)
        {
            // `40 00 count first ptr24`: colours `first..first + count`.
            let (&count, &first) = (
                bytes.get(2).ok_or(unsupported("palette load"))?,
                bytes.get(3).ok_or(unsupported("palette load"))?,
            );
            let colors = resource(
                image,
                *source,
                ResourceKind::Palette,
                usize::from(count) * 2,
                false,
            )?
            .decoded;
            for (index, pair) in colors.chunks_exact(2).enumerate() {
                if let Some(color) = palette.get_mut(usize::from(first) + index) {
                    *color = Bgr555::new(u16::from_le_bytes([pair[0], pair[1]]));
                }
            }
        }
        Ok(Self {
            width,
            height,
            cells,
            metatiles,
            characters,
            palette,
        })
    }

    /// Width in cells.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Height in cells.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Every cell's byte, row-major.
    #[must_use]
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    /// One cell's byte.
    #[must_use]
    pub fn cell(&self, column: usize, row: usize) -> Option<u8> {
        (column < self.width)
            .then(|| self.cells.get(row * self.width + column).copied())
            .flatten()
    }

    /// The palette index at a pixel of the flat plane; the plane repeats.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        let (x, y) = (x % (self.width * 16), y % (self.height * 16));
        let cell = self.cells[(y / 16) * self.width + x / 16];
        let quarter = (y % 16) / 8 * 2 + (x % 16) / 8;
        let character = self.metatiles[usize::from(cell) * 4 + quarter];
        self.characters[usize::from(character) * 64 + (y % 8) * 8 + x % 8]
    }

    /// A palette colour.
    #[must_use]
    pub fn color(&self, index: u8) -> Bgr555 {
        self.palette[usize::from(index)]
    }
}
