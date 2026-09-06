//! Allowlisted ROM-only first backgrounds, not a general scene compositor.
use super::{
    scripts::{self, Command, Limits, ResourceKind},
    StaticLayer, StaticMapError,
};
use crate::{
    compression,
    graphics::{self, BgTileWord, Bgr555, GraphicsError, IndexedPixel, Tile4bpp},
};
use std::{fmt, ops::Range};

/// Invalid bytes or a loading recipe outside the qualified static-background subset.
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

/// Qualified first background for Japanese maps $000B–$000D, $000F–$0011 and $0128.
///
/// Preserves raw cells, definition words and natural ROM colors. Does not apply
/// animation, sprites, windows, color math, brightness or layer composition.
/// Room profiles recognize audited script shapes, including the audio-only
/// conditional tail; they do not evaluate game flags or execute that tail.
#[derive(Debug)]
pub struct StaticBackground {
    layer: StaticLayer,
    resources: Vec<VisualResource>,
    tiles: Vec<Tile4bpp>,
    metatiles: Vec<[BgTileWord; 4]>,
    palette: [Bgr555; 128],
}
impl StaticBackground {
    /// Decodes one explicitly allowlisted first-background recipe.
    /// The caller must authenticate the Japanese ROM. Resource pointers are read
    /// from validated script instructions, never substituted with resource offsets.
    ///
    /// # Errors
    /// Rejects other map IDs, changed script shapes, malformed/bank-crossing
    /// returned resources, output-size mismatches and unqualified cell/definition
    /// bits. Omitted BG2/sprite resources have their instruction shapes and source
    /// start addresses checked, not their payload contents or complete extents.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, VisualMapError> {
        let (loads, graphics_size) = match map_id {
            0x128 => (cavern_loads(image)?, 0x4000),
            0x000B..=0x000D | 0x000F..=0x0011 => (room_loads(image, map_id)?, 0x6000),
            _ => {
                return Err(VisualMapError::Unsupported(
                    "unqualified static background map ID",
                ))
            }
        };
        let graphics = resource(
            image,
            loads[0].1,
            ResourceKind::Graphics,
            graphics_size,
            true,
        )?;
        let colors = resource(image, loads[1].1, ResourceKind::Palette, 192, false)?;
        let definitions = resource(image, loads[2].1, ResourceKind::Metatiles, 0x1000, true)?;
        let attributes = resource(image, loads[3].1, ResourceKind::Metatiles, 512, true)?;
        let shared_colors = resource(image, loads[5].1, ResourceKind::Palette, 64, false)?;
        let layer = StaticLayer::from_rom(image, loads[4].1).map_err(VisualMapError::Layer)?;
        if layer.cells().iter().any(|cell| cell.raw() > 0x1ff) {
            return Err(VisualMapError::Unsupported(
                "unqualified high bits in static background cells",
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
                "nonzero background definition graphics adjustment",
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
    /// Ordered decoded BG1 loads: graphics, palette, definitions, attributes,
    /// shared palette. Excludes the separately retained layer and unused BG2/sprites.
    #[must_use]
    pub fn resources(&self) -> &[VisualResource] {
        &self.resources
    }
    /// Decoded 4bpp tiles from index zero: 512 for the cavern, 768 for rooms.
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
                "pixel outside static background layer",
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

/// Backwards-compatible map $0128 recipe. See [`StaticBackground`].
#[derive(Debug)]
pub struct CavernBackground(StaticBackground);
impl CavernBackground {
    /// Decodes the Japanese portal cavern's first background.
    /// # Errors
    /// See [`StaticBackground::from_rom`].
    pub fn from_rom(image: &[u8]) -> Result<Self, VisualMapError> {
        StaticBackground::from_rom(image, 0x128).map(Self)
    }
    /// Raw first layer.
    #[must_use]
    pub const fn layer(&self) -> &StaticLayer {
        self.0.layer()
    }
    /// Retained background resource loads.
    #[must_use]
    pub fn resources(&self) -> &[VisualResource] {
        self.0.resources()
    }
    /// Decoded 4bpp tiles.
    #[must_use]
    pub fn tiles(&self) -> &[Tile4bpp] {
        self.0.tiles()
    }
    /// Raw four-word definitions.
    #[must_use]
    pub fn metatiles(&self) -> &[[BgTileWord; 4]] {
        self.0.metatiles()
    }
    /// Natural ROM palette.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 128] {
        self.0.palette()
    }
    /// Samples a pixel, preserving transparency and priority.
    /// # Errors
    /// Rejects out-of-layer coordinates or unavailable graphics.
    pub fn pixel(&self, x: usize, y: usize) -> Result<IndexedPixel, VisualMapError> {
        self.0.pixel(x, y)
    }
}

type Load = (ResourceKind, usize, Vec<u8>);

fn cavern_loads(image: &[u8]) -> Result<Vec<Load>, VisualMapError> {
    let program =
        scripts::resolve_map(image, 0x128, Limits::default()).map_err(VisualMapError::Script)?;
    let loads: Vec<_> = program
        .instructions
        .iter()
        .filter_map(|i| match i.command {
            Command::Resource { kind, source } => {
                Some((kind, source.normalized().value() as usize, i.bytes.clone()))
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
    Ok(loads
        .into_iter()
        .enumerate()
        .filter_map(|(i, load)| (i != 5).then_some(load))
        .collect())
}

// These are fixed instruction windows, not a second loading-script interpreter.
// Only packed resource-pointer fields are variable. All control bytes and table
// pointers are checked, including every conditional audio alternative. No FD
// predicate is evaluated: each audited alternative reaches shared subscript $10.
// Reconstructed constraints below are labeled at loader-instruction boundaries;
// addresses in comments are Japanese runtime addresses (offset fields normalize
// them to the ROM image). Three zero bytes at each `pointers` offset stand for a
// masked packed source pointer, NOT literal extracted cartridge pointer bytes.
// FD predicate bit semantics and the labeled opaque fields are not decoded here.
// FE/FF descriptions below assume this profile's unflagged, non-call path unless
// explicitly describing their general flagged-return behavior.
struct RoomSpan {
    offset: usize,
    bytes: &'static [u8],
    pointers: &'static [usize],
}
const ROOM_ROOT: RoomSpan = RoomSpan {
    offset: 0x18_8496,
    bytes: &[
        // $98:8496 — FA: defer subscript $0001 (common room loads).
        8, 0xfa, 1, 0,
        // $98:849A — Graphics: source units $00..$10 ($200 bytes/unit), opaque mode $00;
        // masked pointer; opaque suffix bytes [$40,$00], qualified VRAM word $4000.
        0x80, 0, 0x10, 0, 0, 0, 0, 0x40, 0,
        // $98:84A3 — Palette: source colors $00..$20, CGRAM start $90; masked pointer.
        0x40, 0, 0x20, 0x90, 0, 0, 0,
        // $98:84AA — END: follow deferred subscript at root (or return/finish in other contexts).
        0,
    ],
    pointers: &[8, 17],
};
const ROOM_COMMON: RoomSpan = RoomSpan {
    offset: 0x18_8405,
    bytes: &[
        // $98:8405 — Palette: source colors $00..$60, CGRAM start $20; masked pointer.
        0x40, 0, 0x60, 0x20, 0, 0, 0,
        // $98:840C — FE: flagged return; otherwise skip opaque word $0002.
        8, 0xfe, 2, 0,
        // $98:8410 — Layer: selector $01 (BG1); masked dimension-prefixed resource pointer.
        0x10, 1, 0, 0, 0,
        // $98:8415 — Layer: selector $02 (BG2, omitted); masked resource pointer.
        0x10, 2, 0, 0, 0,
        // $98:841A — FE: flagged return; otherwise skip opaque word $0003.
        8, 0xfe, 3, 0,
        // $98:841E — Graphics: source units $00..$30 ($200 bytes/unit), opaque mode $03;
        // masked pointer; opaque suffix bytes [$00,$00], qualified VRAM word $0000.
        0x80, 0, 0x30, 3, 0, 0, 0, 0, 0,
        // $98:8427 — Metatiles: opaque parameters [$00,$40,$00,$01], qualified BG1 definitions; masked pointer.
        0x20, 0, 0x40, 0, 1, 0, 0, 0,
        // $98:842F — Metatiles: opaque parameters [$00,$08,$00,$81], qualified attributes; masked pointer.
        0x20, 0, 8, 0, 0x81, 0, 0, 0,
        // $98:8437 — Metatiles: opaque parameters [$00,$40,$00,$02], qualified BG2 definitions (omitted); masked pointer.
        0x20, 0, 0x40, 0, 2, 0, 0, 0,
        // $98:843F — FF: jump to subscript $0006 (conditional audio tail) when unflagged.
        8, 0xff, 6, 0,
    ],
    pointers: &[4, 13, 18, 29, 39, 47, 55],
};
const ROOM_AUDIO: RoomSpan = RoomSpan {
    offset: 0x18_8390,
    bytes: &[
        // $98:8390 — FD: conditional jump to subscript $00CF; opaque predicate word $81AC.
        8, 0xfd, 0xac, 0x81, 0xcf, 0,
        // $98:8396 — FD: conditional jump to subscript $00C5; opaque predicate word $8197.
        8, 0xfd, 0x97, 0x81, 0xc5, 0,
        // $98:839C — FD: conditional jump to subscript $0007; opaque predicate word $802C.
        8, 0xfd, 0x2c, 0x80, 7, 0,
        // $98:83A2 — FD: conditional jump to subscript $0014; opaque predicate word $802B.
        8, 0xfd, 0x2b, 0x80, 0x14, 0,
        // $98:83A8 — FD: conditional jump to subscript $0008; opaque predicate word $802A.
        8, 0xfd, 0x2a, 0x80, 8, 0,
        // $98:83AE — FD: conditional jump to subscript $0007; opaque predicate word $0023.
        8, 0xfd, 0x23, 0, 7, 0,
        // $98:83B4 — FD: conditional jump to subscript $0007; opaque predicate word $8101.
        8, 0xfd, 1, 0x81, 7, 0,
        // $98:83BA — FC: select audio-list entry $0005; audio-list execution is outside this projection.
        8, 0xfc, 5, 0,
        // $98:83BE — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83C2 — FE: flagged return; otherwise skip opaque word $0007.
        8, 0xfe, 7, 0,
        // $98:83C6 — FC: select audio-list entry $0003; branch entry from FD/subscript table.
        8, 0xfc, 3, 0,
        // $98:83CA — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83CE — FE: flagged return; otherwise skip opaque word $0014.
        8, 0xfe, 0x14, 0,
        // $98:83D2 — FC: select audio-list entry $000D; branch entry from FD/subscript table.
        8, 0xfc, 0xd, 0,
        // $98:83D6 — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83DA — FE: flagged return; otherwise skip opaque word $0008.
        8, 0xfe, 8, 0,
        // $98:83DE — FC: select audio-list entry $0000; branch entry from FD/subscript table.
        8, 0xfc, 0, 0,
        // $98:83E2 — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83E6 — FE: flagged return; otherwise skip opaque word $00C5.
        8, 0xfe, 0xc5, 0,
        // $98:83EA — FC: select audio-list entry $0014; branch entry from FD/subscript table.
        8, 0xfc, 0x14, 0,
        // $98:83EE — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83F2 — FE: flagged return; otherwise skip opaque word $00CF.
        8, 0xfe, 0xcf, 0,
        // $98:83F6 — FC: select audio-list entry $0036; branch entry from FD/subscript table.
        8, 0xfc, 0x36, 0,
        // $98:83FA — FF: jump to shared-resource subscript $0010 when unflagged.
        8, 0xff, 0x10, 0,
        // $98:83FE — END: return/follow pending/finish; unreachable after the preceding unflagged jump.
        0,
    ],
    pointers: &[],
};
const ROOM_SHARED: RoomSpan = RoomSpan {
    offset: 0x18_819c,
    bytes: &[
        // $98:819C — Graphics: source units $00..$08 ($200 bytes/unit), opaque mode $00;
        // masked pointer; opaque suffix bytes [$70,$00], qualified VRAM word $7000 (omitted).
        0x80, 0, 8, 0, 0, 0, 0, 0x70, 0,
        // $98:81A5 — Palette: source colors $00..$20, CGRAM start $00; masked pointer.
        0x40, 0, 0x20, 0, 0, 0, 0,
        // $98:81AC — FE: flagged return; otherwise skip opaque word $003F.
        8, 0xfe, 0x3f, 0,
        // $98:81B0 — END: return, follow pending stream, or finish; no pending stream on this profile.
        0,
    ],
    pointers: &[4, 13],
};
const ROOM_SUBSCRIPTS: &[(usize, u32)] = &[
    (1, 0x98_8405),
    (6, 0x98_8390),
    (7, 0x98_83c6),
    (0x14, 0x98_83d2),
    (8, 0x98_83de),
    (0xc5, 0x98_83ea),
    (0xcf, 0x98_83f6),
    (0x10, 0x98_819c),
];
fn room_loads(image: &[u8], id: u16) -> Result<Vec<Load>, VisualMapError> {
    let invalid = || VisualMapError::Unsupported("unqualified room script profile");
    let entry: u32 = match id {
        0xb => 0x98_8401,
        0xc => 0x98_8446,
        0xd => 0x98_844d,
        0xf => 0x98_8496,
        0x10 => 0x98_84ad,
        0x11 => 0x98_84b4,
        _ => return Err(invalid()),
    };
    let at = 0x06_959c + usize::from(id) * 3;
    if image.get(at..at + 3) != Some(&entry.to_le_bytes()[..3]) {
        return Err(invalid());
    }
    for &(index, entry) in ROOM_SUBSCRIPTS {
        let at = 0x06_a28c + index * 3;
        if image.get(at..at + 3) != Some(&entry.to_le_bytes()[..3]) {
            return Err(invalid());
        }
    }
    // B falls through FE $0001 into the common palette load; C/D/10/11
    // defer subscript $0001 and END. F additionally loads non-overlapping OBJ art.
    let root = RoomSpan {
        offset: (entry & 0x3f_ffff) as usize,
        bytes: if id == 0xb {
            &[8, 0xfe, 1, 0]
        } else {
            &[8, 0xfa, 1, 0, 0]
        },
        pointers: &[],
    };
    for span in [
        if id == 0xf { &ROOM_ROOT } else { &root },
        &ROOM_COMMON,
        &ROOM_AUDIO,
        &ROOM_SHARED,
    ] {
        let bytes = image
            .get(span.offset..span.offset + span.bytes.len())
            .ok_or_else(invalid)?;
        if bytes
            .iter()
            .zip(span.bytes)
            .enumerate()
            .any(|(i, (got, want))| {
                !span.pointers.iter().any(|&p| (p..p + 3).contains(&i)) && got != want
            })
        {
            return Err(invalid());
        }
        // Validate even omitted BG2/sprite pointers. Their transfer destinations
        // are fixed above and do not overlap the returned BG1 tiles or colors.
        for &p in span.pointers {
            let source = scripts::unpack_pointer(
                bytes[p..p + 3].try_into().expect("three-byte field"),
                0x98,
            )
            .map_err(VisualMapError::Script)?
            .normalized()
            .value() as usize;
            if source >= image.len() {
                return Err(invalid());
            }
        }
    }
    // Fixed offsets into validated windows; never discover instruction boundaries.
    [
        (ResourceKind::Graphics, 0x18_841e, 4, 9),
        (ResourceKind::Palette, 0x18_8405, 4, 7),
        (ResourceKind::Metatiles, 0x18_8427, 5, 8),
        (ResourceKind::Metatiles, 0x18_842f, 5, 8),
        (ResourceKind::Layer, 0x18_8410, 2, 5),
        (ResourceKind::Palette, 0x18_81a5, 4, 7),
    ]
    .into_iter()
    .map(|(kind, at, p, len)| {
        let bytes = &image[at..at + len];
        let source =
            scripts::unpack_pointer(bytes[p..p + 3].try_into().expect("three-byte field"), 0x98)
                .map_err(VisualMapError::Script)?
                .normalized()
                .value() as usize;
        Ok((kind, source, bytes.to_vec()))
    })
    .collect()
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

    fn room_fixture() -> Vec<u8> {
        let mut image = vec![0; 0x28_0000];
        for (id, entry) in [
            (0xb_usize, 0x98_8401_u32),
            (0xc, 0x98_8446),
            (0xd, 0x98_844d),
            (0xf, 0x98_8496),
            (0x10, 0x98_84ad),
            (0x11, 0x98_84b4),
        ] {
            let at = 0x06_959c + id * 3;
            image[at..at + 3].copy_from_slice(&entry.to_le_bytes()[..3]);
        }
        for &(index, entry) in ROOM_SUBSCRIPTS {
            let at = 0x06_a28c + index * 3;
            image[at..at + 3].copy_from_slice(&entry.to_le_bytes()[..3]);
        }
        for span in [&ROOM_ROOT, &ROOM_COMMON, &ROOM_AUDIO, &ROOM_SHARED] {
            image[span.offset..span.offset + span.bytes.len()].copy_from_slice(span.bytes);
        }
        image[0x18_8401..0x18_8405].copy_from_slice(&[8, 0xfe, 1, 0]);
        for at in [0x18_8446, 0x18_844d, 0x18_84ad, 0x18_84b4] {
            image[at..at + 5].copy_from_slice(&[8, 0xfa, 1, 0, 0]);
        }
        let mut graphics = vec![0; 0x6000];
        for row in 0..8 {
            graphics[32 + row * 2] = 255;
        }
        let mut definitions = vec![0; 4096];
        definitions[..2].copy_from_slice(&0x0801_u16.to_le_bytes());
        for (at, data) in [
            (0x20_8000, graphics),
            (0x22_8000, definitions),
            (0x23_8000, vec![0; 512]),
        ] {
            let packet = compression::encode(&data).unwrap();
            image[at..at + packet.len()].copy_from_slice(&packet);
        }
        image[0x21_8002..0x21_8004].copy_from_slice(&31_u16.to_le_bytes());
        let packet = compression::encode(&vec![0; 4096]).unwrap();
        image[0x24_8000..0x24_8002].copy_from_slice(&[2, 4]);
        image[0x24_8002..0x24_8002 + packet.len()].copy_from_slice(&packet);
        for (at, source) in [
            (0x18_8422, 0x20_8000_u32),
            (0x18_8409, 0x21_8000),
            (0x18_842c, 0x22_8000),
            (0x18_8434, 0x23_8000),
            (0x18_8412, 0x24_8000),
            (0x18_81a9, 0x25_8000),
        ] {
            let pointer = (((source >> 16) + 0x80 - 0x98) << 15) | (source & 0x7fff);
            image[at..at + 3].copy_from_slice(&pointer.to_le_bytes()[..3]);
        }
        image
    }

    #[test]
    fn room_profiles_decode_relocated_synthetic_resources_and_full_layer() {
        let image = room_fixture();
        for id in [0xb, 0xc, 0xd, 0xf, 0x10, 0x11] {
            let scene = StaticBackground::from_rom(&image, id).unwrap();
            assert_eq!((scene.layer().width(), scene.layer().height()), (32, 64));
            assert_eq!(scene.tiles().len(), 768);
            assert_eq!(scene.metatiles().len(), 512);
            assert_eq!(scene.palette()[33].rgb8(), [255, 0, 0]);
            assert_eq!(
                scene.pixel(0, 0).unwrap(),
                IndexedPixel::Opaque {
                    palette_index: 33,
                    priority: false
                }
            );
            assert_eq!(scene.pixel(511, 1023).unwrap(), IndexedPixel::Transparent);
            assert!(scene.pixel(512, 0).is_err());
            assert!(scene.pixel(0, 1024).is_err());
            for resource in scene.resources() {
                assert_eq!(resource.source_bytes(), &image[resource.source_range()]);
            }
        }
    }

    #[test]
    fn room_profiles_reject_changed_controls_destinations_tables_and_truncation() {
        let good = room_fixture();
        // Every fixed byte, including controls OUTSIDE the resource windows.
        for span in [&ROOM_ROOT, &ROOM_COMMON, &ROOM_AUDIO, &ROOM_SHARED] {
            for i in 0..span.bytes.len() {
                if span.pointers.iter().any(|&p| (p..p + 3).contains(&i)) {
                    continue;
                }
                let mut image = good.clone();
                image[span.offset + i] ^= 1;
                assert!(
                    StaticBackground::from_rom(&image, 0xf).is_err(),
                    "at {:x}",
                    span.offset + i
                );
            }
        }
        for at in [
            0x06_959c + 0x10 * 3,
            0x06_a28c + 6 * 3,
            0x06_a28c + 0xcf * 3,
            0x18_84af,
        ] {
            let mut image = good.clone();
            image[at] ^= 1;
            assert!(StaticBackground::from_rom(&image, 0x10).is_err());
        }
        assert!(StaticBackground::from_rom(&good[..0x18_8425], 0x10).is_err());
        // No packed-pointer masking of invalid source windows, even omitted BG2.
        let mut image = good;
        image[0x18_8417..0x18_841a].copy_from_slice(&[0, 0, 0x73]);
        assert!(StaticBackground::from_rom(&image, 0x10).is_err());
    }

    #[test]
    fn new_house_roots_reject_changed_bytes_tables_and_gated_maps() {
        let good = room_fixture();
        for (id, at, len) in [
            (0xb, 0x18_8401, 4),
            (0xc, 0x18_8446, 5),
            (0xd, 0x18_844d, 5),
            (0x11, 0x18_84b4, 5),
        ] {
            for byte in (at..at + len)
                .chain(0x06_959c + usize::from(id) * 3..0x06_959f + usize::from(id) * 3)
            {
                let mut image = good.clone();
                image[byte] ^= 1;
                assert!(
                    StaticBackground::from_rom(&image, id).is_err(),
                    "map {id:x} at {byte:x}"
                );
            }
            assert!(StaticBackground::from_rom(&good[..at + len - 1], id).is_err());
        }
        for id in [0xa, 0xe, 0x12, 0x20, 0x21, 0x122] {
            assert!(StaticBackground::from_rom(&good, id).is_err());
        }
    }

    #[test]
    fn room_profiles_reject_bad_payload_sizes_and_unqualified_bits() {
        for (at, size, word) in [
            (0x20_8000, 0x4000, 0_u16), // cavern-sized graphics is not this profile
            (0x22_8000, 0x1000, 0x200), // unqualified definition adjustment
            (0x24_8002, 0x1000, 0x200), // unqualified high cell bits
        ] {
            let mut image = room_fixture();
            let mut data = vec![0; size];
            data[..2].copy_from_slice(&word.to_le_bytes());
            let packet = compression::encode(&data).unwrap();
            image[at..at + packet.len()].copy_from_slice(&packet);
            assert!(StaticBackground::from_rom(&image, 0x10).is_err());
        }
    }

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
