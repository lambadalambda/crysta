//! ROM-only portal-cavern background recipe, not a general scene compositor.
use super::{
    scripts::{self, Command, Limits, ResourceKind},
    StaticLayer, StaticMapError,
};
use crate::{
    compression,
    graphics::{self, BgTileWord, Bgr555, GraphicsError, IndexedPixel, Tile4bpp},
};
use std::{fmt, ops::Range};

/// Invalid bytes or a loading recipe outside the qualified cavern subset.
#[derive(Debug)]
pub enum VisualMapError {
    /// Loading-script failure.
    Script(scripts::ScriptError),
    /// Static layer failure.
    Layer(StaticMapError),
    /// Compressed resource failure.
    Compression(compression::DecodeError),
    /// Graphics sampling failure.
    Graphics(GraphicsError),
    /// Unsupported recipe, resource extent or map coordinate.
    Unsupported(&'static str),
}
impl fmt::Display for VisualMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Script(e) => e.fmt(f),
            Self::Layer(e) => e.fmt(f),
            Self::Compression(e) => e.fmt(f),
            Self::Graphics(e) => e.fmt(f),
            Self::Unsupported(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for VisualMapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Script(e) => Some(e),
            Self::Layer(e) => Some(e),
            Self::Compression(e) => Some(e),
            Self::Graphics(e) => Some(e),
            Self::Unsupported(_) => None,
        }
    }
}

/// One losslessly retained visual resource and its decoded bytes.
#[derive(Debug, Clone)]
pub struct VisualResource {
    kind: ResourceKind,
    offset: usize,
    source: Vec<u8>,
    decoded: Vec<u8>,
}
impl VisualResource {
    /// Script resource category; metatile definitions and attributes share a category.
    #[must_use]
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    /// Normalized/headerless source extent, including packet framing if compressed.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.offset..self.offset + self.source.len()
    }
    /// Original encoded or raw source, without canonical re-encoding.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    /// Decompressed bytes, or the original raw palette bytes.
    #[must_use]
    pub fn decoded(&self) -> &[u8] {
        &self.decoded
    }
}

/// Qualified first background of Japanese map $0128, before gameplay effects.
///
/// Retains raw map cells and definition words. The cavern's graphics base and
/// definition adjustment are zero. Its low-nine-bit cells index four row-major
/// 8×8 words. This model does not execute sprite/shared graphics transfers,
/// palette animation, windows, color math, brightness or layer composition.
#[derive(Debug)]
pub struct CavernBackground {
    layer: StaticLayer,
    resources: Vec<VisualResource>,
    tiles: Vec<Tile4bpp>,
    metatiles: Vec<[BgTileWord; 4]>,
    palette: [Bgr555; 128],
}
impl CavernBackground {
    /// Resolves map $0128 and decodes the qualified complete-background recipe.
    /// The caller must authenticate the Japanese ROM. Pointers come from scripts,
    /// not fixed resource offsets. All seven ordered resource command shapes must
    /// match; only the shared, cached sprite graphics transfer is omitted.
    ///
    /// # Errors
    /// Rejects different resource modes/order/counts, malformed or bank-crossing
    /// data, output-size mismatches, and unqualified high bits in static cells.
    pub fn from_rom(image: &[u8]) -> Result<Self, VisualMapError> {
        let program = scripts::resolve_map(image, 0x128, Limits::default())
            .map_err(VisualMapError::Script)?;
        let loads: Vec<_> = program
            .instructions
            .iter()
            .filter_map(|i| match i.command {
                Command::Resource { kind, source } => {
                    Some((kind, source.normalized().value() as usize, &i.bytes))
                }
                _ => None,
            })
            .collect();
        // Check operands separately from packed pointer bytes. This is intentionally
        // one evidence-backed recipe, not a partial implementation of every mode.
        let expected: [(ResourceKind, &[u8], &[u8]); 7] = [
            (ResourceKind::Graphics, &[0, 0x20, 1], &[0, 0]),
            (ResourceKind::Palette, &[0, 0x60, 0x20], &[]),
            (ResourceKind::Metatiles, &[0, 0x40, 0, 1], &[]),
            (ResourceKind::Metatiles, &[0, 8, 0, 0x81], &[]),
            (ResourceKind::Layer, &[1], &[]),
            (ResourceKind::Graphics, &[0, 8, 0], &[0x70, 0]),
            (ResourceKind::Palette, &[0, 0x20, 0], &[]),
        ];
        if loads.len() != expected.len()
            || loads
                .iter()
                .zip(expected)
                .any(|((kind, _, bytes), (want, prefix, suffix))| {
                    *kind != want || bytes[1..=prefix.len()] != *prefix || !bytes.ends_with(suffix)
                })
        {
            return Err(VisualMapError::Unsupported(
                "unsupported cavern visual resource recipe",
            ));
        }
        let graphics = resource(image, loads[0].1, ResourceKind::Graphics, 0x4000, true)?;
        let colors = resource(image, loads[1].1, ResourceKind::Palette, 192, false)?;
        let definitions = resource(image, loads[2].1, ResourceKind::Metatiles, 0x1000, true)?;
        let attributes = resource(image, loads[3].1, ResourceKind::Metatiles, 512, true)?;
        let shared_colors = resource(image, loads[6].1, ResourceKind::Palette, 64, false)?;
        let layer = StaticLayer::from_rom(image, loads[4].1).map_err(VisualMapError::Layer)?;
        if layer.cells().iter().any(|cell| cell.raw() > 0x1ff) {
            return Err(VisualMapError::Unsupported(
                "unqualified high bits in static cavern cells",
            ));
        }
        let tiles =
            graphics::decode_tiles_4bpp(graphics.decoded()).map_err(VisualMapError::Graphics)?;
        let metatiles: Vec<_> = definitions
            .decoded()
            .chunks_exact(8)
            .map(|record| {
                std::array::from_fn(|i| {
                    BgTileWord::new(u16::from_le_bytes([record[i * 2], record[i * 2 + 1]]))
                })
            })
            .collect();
        // $86:92AC clears bit9 and ORs the graphics adjustment. Both are zero
        // for this recipe; do not silently erase unsupported definition bits.
        if metatiles
            .iter()
            .flatten()
            .any(|word| word.tile_index() >= 512)
        {
            return Err(VisualMapError::Unsupported(
                "nonzero cavern definition graphics adjustment",
            ));
        }
        let mut palette = [Bgr555::new(0); 128];
        for (color, bytes) in palette.iter_mut().zip(
            shared_colors
                .decoded()
                .chunks_exact(2)
                .chain(colors.decoded().chunks_exact(2)),
        ) {
            *color = Bgr555::new(u16::from_le_bytes([bytes[0], bytes[1]]));
        }
        Ok(Self {
            layer,
            resources: vec![graphics, colors, definitions, attributes, shared_colors],
            tiles,
            metatiles,
            palette,
        })
    }
    /// Raw dimension-prefixed first layer, before attribute/gameplay changes.
    #[must_use]
    pub const fn layer(&self) -> &StaticLayer {
        &self.layer
    }
    /// Ordered decoded visual loads, excluding the layer and cached shared graphics.
    #[must_use]
    pub fn resources(&self) -> &[VisualResource] {
        &self.resources
    }
    /// 512 decoded 4bpp tiles, addressed from graphics index zero.
    #[must_use]
    pub fn tiles(&self) -> &[Tile4bpp] {
        &self.tiles
    }
    /// 512 raw four-word definitions, TL/TR/BL/BR.
    #[must_use]
    pub fn metatiles(&self) -> &[[BgTileWord; 4]] {
        &self.metatiles
    }
    /// 128 raw background colors in destination order, before runtime changes.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 128] {
        &self.palette
    }
    /// Samples a map-relative pixel without flattening transparency or priority.
    ///
    /// # Errors
    /// Rejects coordinates outside the decoded map or unavailable graphics.
    pub fn pixel(&self, x: usize, y: usize) -> Result<IndexedPixel, VisualMapError> {
        if x >= self.layer.width() * 16 || y >= self.layer.height() * 16 {
            return Err(VisualMapError::Unsupported(
                "pixel outside static cavern layer",
            ));
        }
        let cell = self.layer.cells()[(y / 16) * self.layer.width() + x / 16];
        graphics::sample_metatile(
            &self.metatiles[usize::from(cell.raw() & 511)],
            &self.tiles,
            x % 16,
            y % 16,
        )
        .map_err(VisualMapError::Graphics)
    }
}

fn resource(
    image: &[u8],
    offset: usize,
    kind: ResourceKind,
    size: usize,
    compressed: bool,
) -> Result<VisualResource, VisualMapError> {
    let bounds = || VisualMapError::Unsupported("visual resource outside ROM or bank bounds");
    if offset >= 0x40_0000 {
        return Err(bounds());
    }
    let suffix = image.get(offset..).ok_or_else(bounds)?;
    let input = &suffix[..suffix.len().min(0x10000 - (offset & 0xffff))];
    let (decoded, consumed) = if compressed {
        let packet = compression::decode(input, size).map_err(VisualMapError::Compression)?;
        if packet.data.len() != size {
            return Err(VisualMapError::Unsupported(
                "visual resource decoded size mismatch",
            ));
        }
        (packet.data, packet.consumed)
    } else {
        (input.get(..size).ok_or_else(bounds)?.to_vec(), size)
    };
    Ok(VisualResource {
        kind,
        offset,
        source: input[..consumed].to_vec(),
        decoded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_and_compressed_resources_are_bounded_by_input_and_bank() {
        let bytes = vec![0; 0x20000];
        for (offset, size) in [(0x1ffff, 2), (0xffff, 2), (usize::MAX, 1)] {
            assert!(resource(&bytes, offset, ResourceKind::Palette, size, false).is_err());
        }
        let packet = compression::encode(&[1, 2, 3, 4]).unwrap();
        let mut image = vec![0; 0x20000];
        let offset = 0x10000 - packet.len() + 1;
        image[offset..offset + packet.len()].copy_from_slice(&packet);
        assert!(resource(&image, offset, ResourceKind::Graphics, 4, true).is_err());
        assert!(resource(&packet, 0, ResourceKind::Graphics, 5, true).is_err());
        assert!(resource(&packet, 0, ResourceKind::Graphics, 3, true).is_err());
    }
}
