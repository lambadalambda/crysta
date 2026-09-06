//! Bounded ROM-backed Ark and house actor sprites, not a sprite VM or scene renderer.
mod house;
pub use house::{HouseActor, HouseFrame, HouseGraphicsKey, HousePoseKey, HouseScenes};
mod house_npc;
pub use house_npc::HouseNpc;

use crate::graphics::{Bgr555, GraphicsError, Tile4bpp, decode_tiles_4bpp};
use std::{fmt, ops::Range};

/// Invalid or unsupported sprite source.
#[derive(Debug)]
pub enum SpriteError {
    /// Truncated, trailing, or unqualified metadata.
    Invalid(&'static str),
    /// Missing or malformed planar graphics.
    Graphics(GraphicsError),
}
impl fmt::Display for SpriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(s) => f.write_str(s),
            Self::Graphics(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SpriteError {}
impl From<GraphicsError> for SpriteError {
    fn from(e: GraphicsError) -> Self {
        Self::Graphics(e)
    }
}

/// One actor-only sample. Priority is retained, not interpreted against BG layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpritePixel {
    /// No component covers the coordinate with a nonzero color.
    Transparent,
    /// First nontransparent component in ROM/OAM order.
    Opaque {
        /// SNES CGRAM index, including OBJ base 128.
        palette_index: u8,
        /// Raw two-bit OBJ priority.
        priority: u8,
        /// Zero-based component order in the source frame.
        component: usize,
    },
}

/// Seven-byte component, including both normal and mirrored placement coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteComponent([u8; 7]);
impl SpriteComponent {
    /// Original component bytes; no attributes or alternate offsets discarded.
    #[must_use]
    pub const fn raw(self) -> [u8; 7] {
        self.0
    }
    /// Component side length, in pixels (8 or 16).
    #[must_use]
    pub const fn size(self) -> u8 {
        if self.0[0] == 1 { 16 } else { 8 }
    }
    /// Original tile/attribute word. Low nine bits address source tiles, not VRAM slots.
    #[must_use]
    pub fn word(self) -> u16 {
        u16::from_le_bytes([self.0[5], self.0[6]])
    }
}

/// Lossless four-byte anchor + twelve-byte prefix + count + seven-byte components.
#[derive(Debug, Clone)]
pub struct SpriteFrame {
    source: Vec<u8>,
    components: Vec<SpriteComponent>,
}
impl SpriteFrame {
    /// Decodes an exact frame extent. The composition pointer used by the game
    /// points four bytes into this slice. Prefix bytes are retained, not interpreted.
    ///
    /// # Errors
    /// Rejects zero/over-128 counts, unsupported size flags, truncation and trailing bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SpriteError> {
        let count = usize::from(
            *bytes
                .get(16)
                .ok_or(SpriteError::Invalid("short sprite header"))?,
        );
        if count == 0 || count > 128 || bytes.len() != 17 + 7 * count {
            return Err(SpriteError::Invalid("invalid sprite component extent"));
        }
        let components = bytes[17..]
            .chunks_exact(7)
            .map(|b| {
                if b[0] > 1 {
                    return Err(SpriteError::Invalid("unsupported sprite size flags"));
                }
                Ok(SpriteComponent(std::array::from_fn(|i| b[i])))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            source: bytes.to_vec(),
            components,
        })
    }
    /// Exact source extent, including anchors and uninterpreted prefix.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    /// Components in ROM order (earlier wins OBJ overlap, regardless of priority).
    #[must_use]
    pub fn components(&self) -> &[SpriteComponent] {
        &self.components
    }
    /// Placement anchor selected independently for horizontal and vertical mirroring.
    #[must_use]
    pub fn anchor(&self, hflip: bool, vflip: bool) -> (i16, i16) {
        (
            i16::from(self.source[usize::from(hflip)].cast_signed()),
            i16::from(self.source[2 + usize::from(vflip)]),
        )
    }
    /// Half-open `(left, top, right, bottom)` bounds relative to the actor's world origin.
    /// Not a collision box. Effective pixel Y is OAM Y + 1.
    #[must_use]
    pub fn bounds(&self, hflip: bool, vflip: bool) -> (i16, i16, i16, i16) {
        let (ax, ay) = self.anchor(hflip, vflip);
        self.components.iter().fold(
            (i16::MAX, i16::MAX, i16::MIN, i16::MIN),
            |(l, t, r, b), c| {
                let x = i16::from(c.0[1 + usize::from(hflip)]) - ax;
                let y = i16::from(c.0[3 + usize::from(vflip)]) - ay;
                (
                    l.min(x),
                    t.min(y),
                    r.max(x + i16::from(c.size())),
                    b.max(y + i16::from(c.size())),
                )
            },
        )
    }
    /// Samples transparent 4bpp composition at an actor-relative pixel coordinate.
    /// Actor flips select alternate placements and XOR component flips; a large
    /// component flips as a whole and uses source tile stride 16, not packed stride 2.
    /// No shadow, color math, brightness, clipping, OAM limits or BG ordering is added.
    ///
    /// # Errors
    /// A covering component references a tile missing from the caller's graphics.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // Coordinates checked in 0..16 before narrowing.
    pub fn sample(
        &self,
        tiles: &[Tile4bpp],
        hflip: bool,
        vflip: bool,
        x: i16,
        y: i16,
    ) -> Result<SpritePixel, SpriteError> {
        let (ax, ay) = self.anchor(hflip, vflip);
        for (component, c) in self.components.iter().enumerate() {
            let dx = i32::from(x) + i32::from(ax) - i32::from(c.0[1 + usize::from(hflip)]);
            let dy = i32::from(y) + i32::from(ay) - i32::from(c.0[3 + usize::from(vflip)]);
            let size = i32::from(c.size());
            if !(0..size).contains(&dx) || !(0..size).contains(&dy) {
                continue;
            }
            let word = c.word();
            let tx = if (word & 0x4000 != 0) ^ hflip {
                size - 1 - dx
            } else {
                dx
            } as usize;
            let ty = if (word & 0x8000 != 0) ^ vflip {
                size - 1 - dy
            } else {
                dy
            } as usize;
            let index = (word & 511) + (tx / 8) as u16 + (ty / 8) as u16 * 16;
            let tile = tiles
                .get(usize::from(index))
                .ok_or(GraphicsError::MissingTile { index })?;
            let color = tile.pixels()[(ty % 8) * 8 + tx % 8];
            if color != 0 {
                return Ok(SpritePixel::Opaque {
                    palette_index: 128 + ((word >> 9) & 7) as u8 * 16 + color,
                    priority: ((word >> 12) & 3) as u8,
                    component,
                });
            }
        }
        Ok(SpritePixel::Transparent)
    }
}

/// One allowlisted frame, identified by its runtime ROM composition pointer.
#[derive(Debug)]
pub struct ArkFrame {
    id: u32,
    resource: u8,
    frame: SpriteFrame,
}
impl ArkFrame {
    /// Runtime ROM composition address (four bytes after the anchor).
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }
    /// Graphics table resource: zero for standing, one for walking.
    #[must_use]
    pub const fn resource(&self) -> u8 {
        self.resource
    }
    /// Lossless composition metadata.
    #[must_use]
    pub const fn composition(&self) -> &SpriteFrame {
        &self.frame
    }
}

/// Three standing and eighteen walking frames from the Japanese ROM.
/// Left uses the horizontal frames with actor H-flip. Timing/facing selection,
/// special idle gestures, equipment changes, shadows and scene effects are not modeled.
#[derive(Debug)]
pub struct ArkSprites {
    frames: Vec<ArkFrame>,
    graphics: [Vec<Tile4bpp>; 2],
    palette: [Bgr555; 16],
    ranges: Vec<Range<usize>>,
}
impl ArkSprites {
    /// Resolves the three bounded frame lists at $A4:A1E4 (standing) and $9A:D064
    /// (walking), graphics pointers at $80:A252/$A258 and the palette COP at $80:F941.
    /// The caller authenticates the headerless Japanese ROM, as with static backgrounds.
    /// This reads frame-list entries but does not execute their duration/direction bytes.
    ///
    /// # Errors
    /// Rejects malformed/out-of-bank pointers, changed list lengths/palette-load shape,
    /// unsupported component palettes and graphics extending outside the 512-tile resources.
    pub fn from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        let mut ranges = Vec::new();
        let mut graphics = [Vec::new(), Vec::new()];
        for (resource, tiles) in graphics.iter_mut().enumerate() {
            let table = 0xa252 + resource * 6;
            let p = take(image, table, 3)?;
            let start = rom_pointer(p[2], word(p, 0))?;
            let range = bank_range(start, 0x4000)?;
            *tiles = decode_tiles_4bpp(take(image, start, range.len())?)?;
            ranges.extend([table..table + 3, range]);
        }
        let cop = take(image, 0xf941, 7)?;
        if cop[0..2] != [2, 0x5a] || cop[5..7] != [0x80, 0x10] {
            return Err(SpriteError::Invalid("changed Ark palette COP"));
        }
        let start = rom_pointer(cop[2], word(cop, 3))?;
        let palette_range = bank_range(start, 32)?;
        let colors = take(image, start, 32)?;
        let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
        ranges.extend([0xf941..0xf948, palette_range]);
        let mut frames = Vec::new();
        for (resource, base, count) in [(0, 0x24_a1e4, 1), (1, 0x1a_d064, 6)] {
            ranges.push(base..base + 6);
            for axis in 0..3 {
                let offset = usize::from(word(take(image, base + axis * 2, 2)?, 0));
                let seq = base + offset;
                same_bank(base, seq, 4 * count + 2)?;
                let entries = take(image, seq, 4 * count + 2)?;
                if word(entries, count * 4) != 0xffff {
                    return Err(SpriteError::Invalid("changed Ark frame list length"));
                }
                ranges.push(seq..seq + entries.len());
                for entry in entries[..count * 4].chunks_exact(4) {
                    if word(entry, 0) & 0x8000 != 0 {
                        return Err(SpriteError::Invalid("early Ark frame list terminator"));
                    }
                    let start = base + usize::from(word(entry, 2));
                    same_bank(base, start, 17)?;
                    let n = usize::from(take(image, start, 17)?[16]);
                    same_bank(base, start, 17 + n * 7)?;
                    let range = start..start + 17 + n * 7;
                    let frame = SpriteFrame::decode(take(image, start, range.len())?)?;
                    for c in frame.components() {
                        let tile = c.word() & 511;
                        if c.word() & 0x0e00 != 0
                            || usize::from(tile) + if c.size() == 16 { 17 } else { 0 } >= 512
                        {
                            return Err(SpriteError::Invalid(
                                "unqualified Ark component palette or tile",
                            ));
                        }
                    }
                    let id = u32::try_from(start + 4)
                        .map_err(|_| SpriteError::Invalid("oversized frame pointer"))?
                        | 0x80_0000;
                    if frames.iter().any(|f: &ArkFrame| f.id == id) {
                        return Err(SpriteError::Invalid("duplicate Ark composition ID"));
                    }
                    frames.push(ArkFrame {
                        id,
                        resource,
                        frame,
                    });
                    ranges.push(range);
                }
            }
        }
        Ok(Self {
            frames,
            graphics,
            palette,
            ranges,
        })
    }
    /// Frames in standing down/up/horizontal, then walking down/up/horizontal list order.
    #[must_use]
    pub fn frames(&self) -> &[ArkFrame] {
        &self.frames
    }
    /// Exact qualified runtime composition pointer lookup; no fallback pose.
    #[must_use]
    pub fn frame(&self, id: u32) -> Option<&ArkFrame> {
        self.frames.iter().find(|f| f.id == id)
    }
    /// Source-indexed tiles for resource zero or one, not dynamic VRAM slots.
    #[must_use]
    pub fn graphics(&self, resource: u8) -> Option<&[Tile4bpp]> {
        self.graphics.get(usize::from(resource)).map(Vec::as_slice)
    }
    /// Natural full-brightness colors corresponding to CGRAM 128..143; index zero is transparent.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 16] {
        &self.palette
    }
    /// Headerless source extents used by extraction, including pointer/list metadata.
    /// Ranges may overlap; all are bounded inside their source bank.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}
fn word(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn take(image: &[u8], start: usize, len: usize) -> Result<&[u8], SpriteError> {
    image
        .get(start..start + len)
        .ok_or(SpriteError::Invalid("truncated sprite ROM source"))
}
fn rom_pointer(bank: u8, offset: u16) -> Result<usize, SpriteError> {
    if !(0x80..=0xbf).contains(&bank) || offset < 0x8000 {
        return Err(SpriteError::Invalid("unsupported sprite ROM pointer"));
    }
    Ok((usize::from(bank & 63) << 16) | usize::from(offset))
}
fn bank_range(start: usize, len: usize) -> Result<Range<usize>, SpriteError> {
    same_bank(start, start, len)?;
    Ok(start..start + len)
}
fn same_bank(base: usize, start: usize, len: usize) -> Result<(), SpriteError> {
    if start >> 16 != base >> 16 || (start + len - 1) >> 16 != base >> 16 {
        return Err(SpriteError::Invalid("bank-crossing sprite source"));
    }
    Ok(())
}
