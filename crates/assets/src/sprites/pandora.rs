//! Additive bounded Pandora presentation; no event VM or NPC scheduler.
use super::{
    bank_range, cpu, cpu_address, decode_tiles_4bpp, frame_table_end, pointer, take, word, Arc,
    Bgr555, Graphics, HouseFrame, HouseGraphicsKey, HousePoseKey, Loader, Range, SpriteError,
    SpriteFrame, Tile4bpp,
};
#[path = "pandora_phases.rs"]
mod phase_data;
use phase_data::{motions, phases};
pub use phase_data::{PandoraActorPhase, PandoraMotion, PandoraPhase, PandoraSceneLimit};

/// Graphics origin, independent of native dynamic VRAM placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandoraGraphicsKey {
    /// An LZ packet in the caller's ROM.
    Compressed(u32),
    /// Uncompressed source planar tiles.
    Direct(u32),
    /// Two 64-byte planar strips copied into source-indexed OBJ tiles $6A/$7A.
    HeldTile {
        /// First 64-byte (two-tile) source strip.
        top: u32,
        /// Second 64-byte source strip, one native tile row lower.
        bottom: u32,
    },
}
/// A finite source animation list. Records include repeats and native durations.
#[derive(Debug)]
pub struct PandoraPoseList {
    selector: u8,
    frames: Vec<HouseFrame>,
}
impl PandoraPoseList {
    /// Source selector; elapsed time and scripted choice belong to the parent.
    #[must_use]
    pub const fn selector(&self) -> u8 {
        self.selector
    }
    /// Effective facing for one record and the parent's actor mirror choice.
    #[must_use]
    pub fn effective_facing(&self, index: usize, hflip: bool) -> Option<u8> {
        self.frames.get(index).map(|f| {
            if f.facing() == 3 && hflip {
                2
            } else {
                f.facing()
            }
        })
    }
    /// All records, in source order. Here `HouseFrame::facing()` is the unmirrored
    /// source facing; use `effective_facing` with the phase's independent H-flip.
    /// No fallback frame is invented.
    #[must_use]
    pub fn frames(&self) -> &[HouseFrame] {
        &self.frames
    }
}
/// Immutable source rasters shared by instances and finite phases.
#[derive(Debug)]
pub struct PandoraArt {
    id: u32,
    graphics_key: PandoraGraphicsKey,
    graphics: Arc<[Tile4bpp]>,
    palette_base: u8,
    palette: [Bgr555; 16],
    lists: Vec<PandoraPoseList>,
}
impl PandoraArt {
    /// Descriptor address, or Ark's six-byte resource-table entry address.
    #[must_use]
    pub const fn source_id(&self) -> u32 {
        self.id
    }
    /// Source-indexed graphics identity (not OAM name-select or dynamic slot).
    #[must_use]
    pub const fn graphics_key(&self) -> PandoraGraphicsKey {
        self.graphics_key
    }
    /// Shared planar-decoded tiles.
    #[must_use]
    pub fn graphics(&self) -> &[Tile4bpp] {
        &self.graphics
    }
    /// Natural source colors. Zero is transparent; effects are not baked in.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 16] {
        &self.palette
    }
    /// CGRAM base to subtract from opaque sprite samples.
    #[must_use]
    pub const fn palette_base(&self) -> u8 {
        self.palette_base
    }
    /// Bounded allowlisted lists only.
    #[must_use]
    pub fn lists(&self) -> &[PandoraPoseList] {
        &self.lists
    }
    /// List `selector` of the object sheet at `base` (`COP D8`), as a
    /// script that points its art there draws it: the blue door's hit
    /// target (`$88:AAEE`, `$A2:C000` list 0), the spear's display.
    ///
    /// # Errors
    /// Refuses a list outside the qualified shapes.
    pub fn object(image: &[u8], base: u32, selector: u8) -> Result<Self, SpriteError> {
        object_list(&mut Loader::new(image), base, base, selector)
    }
    /// Exact source selector lookup.
    #[must_use]
    pub fn list(&self, selector: u8) -> Option<&PandoraPoseList> {
        self.lists.iter().find(|l| l.selector == selector)
    }
}

fn pose_list(
    bytes: &[u8],
    base: u32,
    direct: bool,
    selector: u8,
    palette_base: u8,
    tile_count: usize,
) -> Result<PandoraPoseList, SpriteError> {
    if palette_base < 128 || !palette_base.is_multiple_of(16) {
        return Err(SpriteError::Invalid("Pandora palette base"));
    }
    let table = usize::from(selector) * 2;
    if !direct && table + 2 > frame_table_end(bytes)? - 2 {
        return Err(SpriteError::Invalid(
            "Pandora selector outside packed table",
        ));
    }
    let sequence = usize::from(word(take(bytes, table, 2)?, 0));
    if sequence < table + 2 {
        return Err(SpriteError::Invalid("Pandora list overlaps selector table"));
    }
    let mut frames = Vec::new();
    for i in 0..64 {
        let at = sequence + i * 4;
        let prefix = take(bytes, at, 2)?;
        if prefix == [255, 255] {
            if frames.is_empty() {
                return Err(SpriteError::Invalid("empty Pandora list"));
            }
            return Ok(PandoraPoseList { selector, frames });
        }
        if prefix[0] >= 128 || ![0, 1, 3].contains(&prefix[1]) {
            return Err(SpriteError::Invalid("unsupported Pandora list command"));
        }
        let anchor = usize::from(word(take(bytes, at + 2, 2)?, 0));
        let n = usize::from(take(bytes, anchor, 17)?[16]);
        let raw = take(bytes, anchor, 17 + n * 7)?;
        let source = SpriteFrame::decode(raw)?;
        let mut relocated = raw.to_vec();
        let source_palette = (source.components()[0].word() >> 9) & 7;
        for (j, c) in source.components().iter().enumerate() {
            let word = c.word();
            let tile = usize::from(word & 511);
            if (word >> 9) & 7 != source_palette
                || tile + if c.size() == 16 { 17 } else { 0 } >= tile_count
                || (c.size() == 16 && tile & 15 == 15)
            {
                return Err(SpriteError::Invalid(
                    "Pandora component palette/tile boundary",
                ));
            }
            let adjusted = (word & !0x0e00) | (u16::from((palette_base - 128) / 16) << 9);
            relocated[22 + j * 7..24 + j * 7].copy_from_slice(&adjusted.to_le_bytes());
        }
        let offset =
            u16::try_from(anchor + 4).map_err(|_| SpriteError::Invalid("Pandora frame offset"))?;
        let key = if direct {
            HousePoseKey::Direct(
                base.checked_add(u32::from(offset))
                    .ok_or(SpriteError::Invalid("Pandora frame address"))?,
            )
        } else {
            HousePoseKey::Compressed {
                packet: base,
                offset,
            }
        };
        frames.push(HouseFrame {
            key,
            duration: prefix[0],
            facing: prefix[1],
            source,
            composition: SpriteFrame::decode(&relocated)?,
        });
    }
    Err(SpriteError::Invalid("unbounded Pandora list"))
}

/// Bounded Pandora art library. No captures, script interpreter, collision or clock.
#[derive(Debug)]
pub struct PandoraSprites {
    art: Vec<PandoraArt>,
    ranges: Vec<Range<usize>>,
    phases: Vec<PandoraPhase>,
    motions: Vec<PandoraMotion>,
}
impl PandoraSprites {
    /// Decode from the caller-authenticated headerless Japanese ROM.
    ///
    /// # Errors
    /// Rejects unsupported source shapes, list commands, palette modes, pointers,
    /// truncated packets and out-of-resource component tiles.
    pub fn from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        let mut loader = Loader::new(image);
        let mut art = Vec::new();
        carry_program(&mut loader)?;
        // Explicit reuse dependencies are source descriptors, never runtime slots.
        for (descriptor, gfx_descriptor, selectors) in DESCRIPTORS {
            art.push(descriptor_art(
                &mut loader,
                *descriptor,
                *gfx_descriptor,
                selectors,
            )?);
        }
        for (resource, selectors) in [
            (0, &[3, 4, 5][..]),
            (1, &[9, 10, 11][..]),
            (3, &[15, 16, 17, 24, 25, 26][..]),
        ] {
            art.push(ark_art(&mut loader, resource, selectors)?);
        }
        art.push(box_art(&mut loader)?);
        art.extend(pot_art(&mut loader)?);
        art.push(tour_object(&mut loader)?);
        let phases = phases(&mut loader)?;
        let motions = motions(&mut loader, &phases)?;
        for phase in &phases {
            for actor in phase.actors() {
                if !art
                    .iter()
                    .any(|a| a.id == actor.art_id && a.list(actor.selector).is_some())
                {
                    return Err(SpriteError::Invalid("unresolved Pandora phase art/list"));
                }
            }
        }
        for motion in &motions {
            if !phases.iter().flat_map(PandoraPhase::actors).any(|actor| {
                actor.source_id == motion.actor
                    && art
                        .iter()
                        .any(|a| a.id == actor.art_id && a.list(motion.selector).is_some())
            }) {
                return Err(SpriteError::Invalid(
                    "unresolved Pandora motion actor/art/list",
                ));
            }
        }
        Ok(Self {
            art,
            ranges: loader.ranges,
            phases,
            motions,
        })
    }
    /// Shared immutable art, keyed by source identity.
    #[must_use]
    pub fn art(&self) -> &[PandoraArt] {
        &self.art
    }
    /// Exact source lookup; missing art is not silently another actor.
    #[must_use]
    pub fn get(&self, id: u32) -> Option<&PandoraArt> {
        self.art.iter().find(|a| a.id == id)
    }
    /// Finite source endpoint choices; selection and transitions belong to parent state.
    #[must_use]
    pub fn phases(&self) -> &[PandoraPhase] {
        &self.phases
    }
    /// Source C departure endpoints. Parent state owns interpolation and completion.
    #[must_use]
    pub fn motions(&self) -> &[PandoraMotion] {
        &self.motions
    }
    /// Exact finite phase lookup, without a state/event interpreter.
    #[must_use]
    pub fn phase(&self, id: &str) -> Option<&PandoraPhase> {
        self.phases.iter().find(|p| p.id == id)
    }
    /// Exact source extents consumed, including pointer/list dependencies.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}
const DESCRIPTORS: &[(u32, u32, &[u8])] = &[
    (0x83_ebe4, 0x83_ebe4, &[0, 3]),                      // northern pair
    (0x83_ec7e, 0x83_ebe4, &[0]),                         // northern resident
    (0x83_ed37, 0x83_ebe4, &[0, 1, 2, 3, 4, 5, 6, 7, 8]), // corridor birds
    (0x83_ecc6, 0x83_ecc6, &[0, 2]),                      // map13 resident
    (0x83_ed5a, 0x83_ed5a, &[0, 1, 2, 3, 5]),             // C pair, direct branch
    (0x83_ed74, 0x83_ed5a, &[0, 1, 2, 3, 5]),
    (0x83_edf8, 0x83_ed5a, &[0, 2, 3, 5]),
    (0x83_f8c0, 0x83_f8c0, &[1, 3, 4, 11]), // opening guide
    (0x83_f8a8, 0x83_f8a8, &[3]),           // tour guide
];
fn descriptor_parts(loader: &mut Loader<'_>, id: u32) -> Result<(u32, usize), SpriteError> {
    let at = pointer(&id.to_le_bytes()[..3])?;
    let prefix = loader.read(at, 5)?;
    let mode = word(prefix, 3);
    if ![0, 0x20, 0x22, 0x23, 0xa3].contains(&mode) {
        return Err(SpriteError::Invalid("unsupported Pandora descriptor mode"));
    }
    let extra = if mode & 0x20 == 0 { 3 } else { 0 };
    if extra != 0 {
        pointer(loader.read(at + 5, 3)?)?;
    }
    Ok((cpu(&prefix[..3]), at + 5 + extra))
}
fn descriptor_graphics(loader: &mut Loader<'_>, id: u32) -> Result<Arc<Graphics>, SpriteError> {
    let (_, pal) = descriptor_parts(loader, id)?;
    let transfer = loader.read(pal + 4, 4)?;
    if transfer[..3] != [0, 0, 0xc0] {
        return Err(SpriteError::Invalid(
            "unsupported Pandora graphics transfer",
        ));
    }
    let cpu = cpu(loader.read(0xfda4 + usize::from(transfer[3]), 3)?);
    loader.graphics(cpu)
}
fn descriptor_art(
    loader: &mut Loader<'_>,
    id: u32,
    dependency: u32,
    selectors: &[u8],
) -> Result<PandoraArt, SpriteError> {
    let (packet_cpu, pal) = descriptor_parts(loader, id)?;
    let p = loader.read(pal, 4)?;
    if !(0x80..=0x90).contains(&p[0]) || p[2] != 2 || ![6, 8, 10, 12].contains(&p[3]) {
        return Err(SpriteError::Invalid(
            "unsupported Pandora palette descriptor",
        ));
    }
    let palette_pointer = pointer(loader.read(0xfc72 + usize::from(p[0] & 63) * 3, 3)?)?;
    bank_range(palette_pointer, usize::from(p[1]) * 16 + 32)?;
    let colors = loader.read(palette_pointer + usize::from(p[1]) * 16, 32)?;
    let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
    let palette_base = 128 + p[3] * 8;
    if dependency != id && loader.read(pal + 4, 2)? != [255, 255] {
        return Err(SpriteError::Invalid("changed Pandora graphics reuse"));
    }
    let graphics = descriptor_graphics(loader, dependency)?;
    let packet = loader.packet(packet_cpu)?;
    let lists = selectors
        .iter()
        .map(|&s| {
            pose_list(
                &packet.bytes,
                packet.cpu,
                false,
                s,
                palette_base,
                graphics.tiles.len(),
            )
        })
        .collect::<Result<_, _>>()?;
    let HouseGraphicsKey::Compressed(key) = graphics.key;
    Ok(PandoraArt {
        id,
        graphics_key: PandoraGraphicsKey::Compressed(key),
        graphics: graphics.tiles.clone(),
        palette_base,
        palette,
        lists,
    })
}
fn ark_art(
    loader: &mut Loader<'_>,
    resource: u8,
    selectors: &[u8],
) -> Result<PandoraArt, SpriteError> {
    let table = 0xa24f + usize::from(resource) * 6;
    let entry = loader.read(table, 6)?;
    let base_cpu = cpu(&entry[..3]);
    let gfx_cpu = cpu(&entry[3..]);
    let gfx = pointer(&entry[3..])?;
    let graphics: Arc<[Tile4bpp]> = decode_tiles_4bpp(loader.read(gfx, 0x4000)?)?.into();
    let cop = loader.read(0xf941, 7)?;
    if cop[..2] != [2, 0x5a] || cop[5..] != [128, 16] {
        return Err(SpriteError::Invalid("changed Pandora Ark palette COP"));
    }
    let colors = loader.read(pointer(&[cop[3], cop[4], cop[2]])?, 32)?;
    let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
    let lists = direct_lists(loader, base_cpu, selectors, 128, graphics.len())?;
    Ok(PandoraArt {
        id: cpu_address(table)?,
        graphics_key: PandoraGraphicsKey::Direct(gfx_cpu),
        graphics,
        palette_base: 128,
        palette,
        lists,
    })
}

#[cfg(test)]
#[path = "pandora_tests.rs"]
mod tests;

fn box_art(loader: &mut Loader<'_>) -> Result<PandoraArt, SpriteError> {
    let d = loader.read(0x3_f984, 23)?;
    if d[3..5] != [4, 0] || d[8] != 0x40 || d[12..17] != [4, 2, 8, 0x30, 0x30] || d[17] != 0x10 {
        return Err(SpriteError::Invalid("changed Pandora box descriptor"));
    }
    pointer(&d[5..8])?; // separate movement program, not a raster pointer
    let packet = loader.packet(cpu(&d[..3]))?;
    let palette_pointer = pointer(&d[9..12])?;
    bank_range(palette_pointer, usize::from(d[12]) * 16 + 32)?;
    let palette_at = palette_pointer + usize::from(d[12]) * 16;
    let colors = loader.read(palette_at, 32)?;
    let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
    let gfx_cpu = cpu(&d[18..21]);
    let graphics_packet = loader.packet(gfx_cpu)?;
    // The descriptor transfers only the $400-byte source band in paired rows.
    // Keep its full source-indexed packet; only selected composition tiles are admitted.
    let graphics: Arc<[Tile4bpp]> = decode_tiles_4bpp(&graphics_packet.bytes)?.into();
    let lists = vec![pose_list(
        &packet.bytes,
        packet.cpu,
        false,
        3,
        192,
        graphics.len(),
    )?];
    Ok(PandoraArt {
        id: 0x83_f984,
        graphics_key: PandoraGraphicsKey::Compressed(gfx_cpu),
        graphics,
        palette_base: 192,
        palette,
        lists,
    })
}
fn direct_lists(
    loader: &mut Loader<'_>,
    base: u32,
    selectors: &[u8],
    palette: u8,
    tiles: usize,
) -> Result<Vec<PandoraPoseList>, SpriteError> {
    let at = pointer(&base.to_le_bytes()[..3])?;
    let bytes = take(loader.image, at, 0x1_0000 - (at & 65535))?;
    let lists = selectors
        .iter()
        .map(|&s| pose_list(bytes, base, true, s, palette, tiles))
        .collect::<Result<Vec<_>, _>>()?;
    for list in &lists {
        let entry = at + usize::from(list.selector) * 2;
        let seq = at + usize::from(word(loader.read(entry, 2)?, 0));
        loader.read(seq, list.frames.len() * 4 + 2)?;
        for f in &list.frames {
            let HousePoseKey::Direct(p) = f.key else {
                unreachable!()
            };
            loader.read(
                pointer(&p.to_le_bytes()[..3])? - 4,
                f.source.source_bytes().len(),
            )?;
        }
    }
    Ok(lists)
}
fn tour_object(loader: &mut Loader<'_>) -> Result<PandoraArt, SpriteError> {
    let b = loader.read(0x9_d9fe, 16)?;
    if b[..6] != [2, 0xb2, 0xf8, 0xff, 2, 0xd8] || b[9..] != [2, 0x48, 0x42, 0x82, 2, 0x80, 8] {
        return Err(SpriteError::Invalid("changed tour object script"));
    }
    object_list(loader, 0x89_d9f9, cpu(&b[6..9]), b[15])
}
/// A list of the object sheet a script points its art at (`COP D8 base`),
/// with the source-qualified shared object sheet and palette load (also
/// used by F's prop).
fn object_list(
    loader: &mut Loader<'_>,
    id: u32,
    base: u32,
    selector: u8,
) -> Result<PandoraArt, SpriteError> {
    let resource = loader.prop()?.resource;
    let lists = direct_lists(
        loader,
        base,
        &[selector],
        resource.palette_base,
        resource.graphics.tiles.len(),
    )?;
    let HouseGraphicsKey::Compressed(key) = resource.graphics.key;
    Ok(PandoraArt {
        id,
        graphics_key: PandoraGraphicsKey::Compressed(key),
        graphics: resource.graphics.tiles.clone(),
        palette_base: resource.palette_base,
        palette: resource.palette,
        lists,
    })
}
fn pot_art(loader: &mut Loader<'_>) -> Result<Vec<PandoraArt>, SpriteError> {
    // Follow C's deferred common source loads, but decode only the metatile
    // definitions and palette needed by FA/FB. No map raster dependency/VM.
    if loader.read(0x6_959c + 0xc * 3, 3)? != [0x46, 0x84, 0x98]
        || loader.read(0x18_8446, 5)? != [8, 0xfa, 1, 0, 0]
        || loader.read(0x6_a28f, 3)? != [5, 0x84, 0x98]
    {
        return Err(SpriteError::Invalid(
            "changed C common object palette dependency",
        ));
    }
    let pal_load = loader.read(0x18_8405, 7)?;
    let def_load = loader.read(0x18_8427, 8)?;
    if pal_load[..4] != [0x40, 0, 0x60, 0x20] || def_load[..5] != [0x20, 0, 0x40, 0, 1] {
        return Err(SpriteError::Invalid("changed pot palette/definition loads"));
    }
    let unpack = |p: &[u8]| {
        crate::maps::scripts::unpack_pointer([p[0], p[1], p[2]], 0x98)
            .map(rom::RuntimeRomAddress::value)
            .map_err(|_| SpriteError::Invalid("pot packed source pointer"))
    };
    let palette_pointer = pointer(&unpack(&pal_load[4..])?.to_le_bytes()[..3])?;
    let definitions = loader.packet(unpack(&def_load[5..])?)?;
    if definitions.bytes.len() != 0x1000 {
        return Err(SpriteError::Invalid("pot definition extent"));
    }
    // Map-indexed held records: first matching source map entry, not WRAM $098A.
    let mut record = None;
    for i in 0..256 {
        let entry = loader.read(0x16_ddbd + i * 4, 4)?;
        if word(entry, 0) & 0x8000 != 0 {
            break;
        }
        if word(entry, 0) == 0xc {
            record = Some(0x16_0000 + usize::from(word(entry, 2)));
            break;
        }
    }
    let record = record.ok_or(SpriteError::Invalid("missing C held source records"))?;
    let script = loader.read(0x7_973d, 0x8d)?;
    // Source animation list and queued transfer operands from the lift consumer.
    if script[0x36..0x39] != [0x69, 0, 0x80] || script[0x51..0x54] != [0xa9, 0xa4, 1] {
        return Err(SpriteError::Invalid("changed pot upload source"));
    }
    let mut arts = Vec::new();
    for (i, tile_id) in [0xfa_usize, 0xfb].into_iter().enumerate() {
        let at = record + i * 5;
        let held = loader.read(at, 5)?;
        let selector = held[2];
        let entry = loader.read(0x30_8000 + usize::from(selector) * 2, 2)?;
        let seq = 0x30_8000 + usize::from(word(entry, 0));
        let list = loader.read(seq, 6)?;
        let anchor = 0x30_8000 + usize::from(word(list, 2));
        let first = loader.read(anchor, 24)?;
        let tile = word(first, 22) & 511;
        let graphics_base =
            (usize::from(script[0x52] & 63) << 16) | usize::from(word(script, 0x74));
        let top = graphics_base + usize::from(tile) * 32;
        let bottom = top + usize::from(word(script, 0x7b));
        let mut planar = vec![0; 256 * 32];
        planar[0x6a * 32..0x6c * 32].copy_from_slice(loader.read(top, 64)?);
        planar[0x7a * 32..0x7c * 32].copy_from_slice(loader.read(bottom, 64)?);
        let graphics: Arc<[Tile4bpp]> = decode_tiles_4bpp(&planar)?.into();
        let pal = usize::from((word(&definitions.bytes, tile_id * 8) >> 10) & 7);
        if pal < 2 {
            return Err(SpriteError::Invalid("pot palette outside common load"));
        }
        let mut palette = [Bgr555::new(0); 16];
        let offset = (pal * 16 + 8 - 32) * 2;
        bank_range(palette_pointer, offset + 16)?;
        let colors = loader.read(palette_pointer + offset, 16)?;
        for (i, color) in palette[8..].iter_mut().enumerate() {
            *color = Bgr555::new(word(colors, i * 2));
        }
        let lists = direct_lists(
            loader,
            0xa2_c000,
            &[25, 26, 29, 30, 43, 44, 45, 46, 47, 48, 60],
            240,
            256,
        )?;
        for list in &lists {
            for frame in &list.frames {
                for c in frame.composition.components() {
                    if c.word() & 511 != 0x6a {
                        return Err(SpriteError::Invalid("changed pot destination tile"));
                    }
                }
            }
        }
        arts.push(PandoraArt {
            id: cpu_address(at)?,
            graphics_key: PandoraGraphicsKey::HeldTile {
                top: cpu_address(top)?,
                bottom: cpu_address(bottom)?,
            },
            graphics,
            palette_base: 240,
            palette,
            lists,
        });
    }
    Ok(arts)
}

/// Finite carry presentation choice. Core/parent owns actions, durations and trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandoraCarryMotion {
    /// Source lift choreography, before ordinary held control.
    Lifting,
    /// Action-free held standing (not ordinary Ark idle).
    Standing,
    /// Held walking (not ordinary Ark walking).
    Walking,
    /// Throw windup/release; subsequent free-flight pot list is selector 60.
    Throwing,
}
/// Ark's run (`docs/input-admission.md`): the dash a double tap starts and
/// the brake that ends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandoraRunMotion {
    /// Dashing: resource 1 (`$80:A255`) lists 23..25 (`COP 83 sel,01`).
    Dashing,
    /// Braking: resource 0 (`$80:A24F`) lists 9..11 (`COP 84 sel`).
    Braking,
}
/// Explicit Ark/pot list pairing. FA and FB share composition but not graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PandoraCarryPose {
    /// Ark resource-table source ID for `PandoraSprites::get`.
    pub ark_art: u32,
    /// Source Ark animation selector.
    pub ark_selector: u8,
    /// Source pot animation selector; select FA/FB art independently by held record.
    pub pot_selector: u8,
    /// Ark's horizontal mirror.
    pub ark_hflip: bool,
    /// Pot's independent horizontal mirror.
    pub pot_hflip: bool,
}
impl PandoraSprites {
    /// Ark's run lists, apart from the Pandora art: the brake's (resource 0,
    /// lists 9..11) and the dash's (resource 1, lists 23..25), keyed by the
    /// same source ids as [`Self::run_pose`] names.
    ///
    /// # Errors
    /// As [`Self::from_rom`] for these lists.
    pub fn run_art(image: &[u8]) -> Result<[PandoraArt; 2], SpriteError> {
        let mut loader = Loader::new(image);
        Ok([
            ark_art(&mut loader, 0, &[9, 10, 11])?,
            ark_art(&mut loader, 1, &[23, 24, 25])?,
        ])
    }
    /// Ark's art, list and mirror for a run motion and facing (0 Down, 1 Up,
    /// 2 Left, 3 Right): one list per axis, Left mirrored.
    #[must_use]
    pub fn run_pose(motion: PandoraRunMotion, facing: u8) -> Option<(u32, u8, bool)> {
        let axis = facing.min(2);
        let (art, first) = match motion {
            PandoraRunMotion::Dashing => (0x80_a255, 23),
            PandoraRunMotion::Braking => (0x80_a24f, 9),
        };
        (facing <= 3).then_some((art, first + axis, facing == 2))
    }
    /// Pure source pose selection for native facing 0=Down, 1=Up, 2=Left, 3=Right.
    /// Free-flight pots use list 60 after the windup; source frame anchors already
    /// include visual lift. Do not add a second hardcoded carrying Y offset.
    /// Per-actor priority override during lifting is separate from source rasters.
    #[must_use]
    pub fn carry_pose(motion: PandoraCarryMotion, facing: u8) -> Option<PandoraCarryPose> {
        if facing > 3 {
            return None;
        }
        let axis = facing.min(2);
        let (ark_art, ark_selector, pot_selector) = match motion {
            PandoraCarryMotion::Lifting => (0x80_a261, 24 + axis, 43 + axis),
            PandoraCarryMotion::Standing => {
                (0x80_a24f, 3 + axis, if facing == 0 { 25 } else { 26 })
            }
            PandoraCarryMotion::Walking => (0x80_a255, 9 + axis, if facing < 2 { 29 } else { 30 }),
            PandoraCarryMotion::Throwing => (0x80_a261, 15 + axis, 46 + axis),
        };
        Some(PandoraCarryPose {
            ark_art,
            ark_selector,
            pot_selector,
            ark_hflip: facing == 2,
            pot_hflip: facing == 2,
        })
    }
}
fn carry_program(loader: &mut Loader<'_>) -> Result<(), SpriteError> {
    for (at, selector, resource) in [
        (0x4_b4c8, 3, 0),
        (0x4_b4da, 4, 0),
        (0x4_b4f7, 5, 0),
        (0x4_b540, 15, 3),
        (0x4_b553, 16, 3),
        (0x4_b56a, 17, 3),
        (0x4_be7c, 24, 3),
        (0x4_be88, 25, 3),
        (0x4_be98, 26, 3),
    ] {
        if loader.read(at, 5)? != [2, 0x84, selector, 0, resource] {
            return Err(SpriteError::Invalid("changed Ark carry source selection"));
        }
    }
    for (at, selector) in [(0x4_b509, 9), (0x4_b51a, 10), (0x4_b52f, 11)] {
        if loader.read(at, 4)? != [2, 0x83, selector, 1] {
            return Err(SpriteError::Invalid(
                "changed Ark held-walk source selection",
            ));
        }
    }
    // Release changes to selector $3C, then applies the projectile's source path.
    if loader.read(0x4_c701, 7)? != [0xa9, 0x3c, 0, 0x9f, 8, 0, 0x7f] {
        return Err(SpriteError::Invalid("changed flying pot selector"));
    }
    loader.read(0x4_c2b8, 0x520)?; // bounded lift/held/throw consumer, not executed
    Ok(())
}
