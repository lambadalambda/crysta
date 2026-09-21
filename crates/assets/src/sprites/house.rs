//! ROM-backed fresh house actor presentation, not native scene behavior.

use super::{bank_range, take, word, SpriteError, SpriteFrame};
use crate::{
    compression,
    graphics::{decode_tiles_4bpp, Bgr555, Tile4bpp},
    maps::{
        actor_script::{self, ScriptError},
        actors::SpawnRecord,
        scripts::EventFlags,
    },
};
use std::{collections::HashMap, ops::Range, sync::Arc};

/// Clears the actor's horizontal mirror.
const CLEAR_HFLIP: u8 = 0xB6;
/// Sets the actor's horizontal mirror.
const SET_HFLIP: u8 = 0xB7;
/// Selects an animation sequence; one operand byte.
const SELECT_POSE: u8 = 0x80;
/// Resolves the pose through `$80:ED75` and yields until it is done.
///
/// Its counted sibling `COP 8F` is not a boundary: the map-`$0011` resident
/// pauses on one before their ordinary loop sets the mirror the frozen
/// roster shows.
const WAIT: u8 = 0x8E;

/// The pose an actor's script has selected by the time it first waits.
///
/// The ordinary interaction loop `docs/house-npc.md` records is: clear or set
/// H-flip (`COP B6`/`B7`), select animation (`COP 80 n`), then resolve and
/// wait (`COP 8E`). What is in force at that wait is the resident's steady
/// state, so the walk stops there. For a resident whose script walks them
/// about first, that is later than the frozen roster's setup-frame policy and
/// can differ from it in mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResidentPose {
    /// Sequence selected by `COP 80`, or `None` to keep the header's initial one.
    pub selector: Option<u8>,
    /// Horizontal mirror in force.
    pub hflip: bool,
}

impl ResidentPose {
    /// Derives the setup pose by walking the record's entry script.
    ///
    /// The walk follows the flags first. When it ends before any wait -- the
    /// map-`$0011` resident runs an intro on a new game that ends in native
    /// code the walker does not execute -- a walk with every flag set stands
    /// in. One-time sequences are gated on a clear flag they set when they
    /// finish, so that is the state the resident settles into once they have
    /// run. The branch-free walk would not do: it falls into the `COP 06`
    /// call those scripts keep on their not-taken arm, which is unaccounted.
    ///
    /// A record without a script, or one that reaches no wait either way,
    /// keeps the header's initial selector and no mirror.
    ///
    /// # Errors
    /// Propagates a script address that is not ROM-backed or is truncated.
    pub fn from_script(
        image: &[u8],
        record: &SpawnRecord,
        events: EventFlags<'_>,
    ) -> Result<Self, ScriptError> {
        let Some(script) = record.script() else {
            return Ok(Self::default());
        };
        let flagged = actor_script::walk_with_events(image, script, events)?;
        if let Some(pose) = Self::at_first_wait(image, &flagged) {
            return Ok(pose);
        }
        let every = [0xFFu8; 512];
        let settled = actor_script::walk_with_events(image, script, EventFlags::Bitmap(&every))?;
        Ok(Self::at_first_wait(image, &settled).unwrap_or_default())
    }

    /// The pose in force at the first wait, or `None` if the walk has none.
    fn at_first_wait(image: &[u8], walked: &actor_script::ScriptEffects) -> Option<Self> {
        let mut pose = Self::default();
        for command in &walked.commands {
            match command.service {
                CLEAR_HFLIP => pose.hflip = false,
                SET_HFLIP => pose.hflip = true,
                SELECT_POSE => pose.selector = image.get(command.offset + 2).copied(),
                WAIT => return Some(pose),
                _ => {}
            }
        }
        None
    }
}

/// Why a spawn record yields no actor.
#[derive(Debug)]
pub enum RecordRefusal {
    /// The record installs no graphics resource: a `$00` or `$FD` record is a
    /// script with a position, and the game draws nothing for it either.
    NoDescriptor,
    /// The record reuses its predecessor's resource or graphics, and that
    /// predecessor was refused.
    PredecessorRefused,
    /// The record's descriptor, header or frame is outside the qualified set.
    Invalid(SpriteError),
}

impl std::fmt::Display for RecordRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDescriptor => write!(f, "spawn record carries no resource descriptor"),
            Self::PredecessorRefused => write!(f, "the record this one reuses was refused"),
            Self::Invalid(error) => write!(f, "{error}"),
        }
    }
}

/// Whether a `$01` record leans on the record before it.
fn reuses_predecessor(image: &[u8], spawn: &[u8]) -> bool {
    let descriptor = &spawn[7..10];
    if descriptor == [0, 0, 0] {
        return true;
    }
    let Ok(at) = pointer(descriptor) else {
        return false;
    };
    // Descriptor: packet (3), mode (2), three more for mode `$0000`, palette
    // (4), then graphics, which is `$FFFF` for reuse.
    let extra = if image.get(at + 3..at + 5) == Some(&[0, 0]) {
        3
    } else {
        0
    };
    image.get(at + 9 + extra..at + 11 + extra) == Some(&[0xFF, 0xFF])
}

#[path = "pandora.rs"]
pub(super) mod pandora;

/// Source coordinates for a pose. Decoded offsets are never added to ROM addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HousePoseKey {
    /// Compressed composition packet CPU address and decoded composition offset.
    Compressed {
        /// CPU address of the compressed composition packet.
        packet: u32,
        /// Composition offset in decompressed bytes, four bytes after its anchor.
        offset: u16,
    },
    /// Actual direct-ROM composition CPU address (four bytes past its anchor).
    Direct(u32),
}
/// The ROM packet supplying source-indexed planar tiles, not a native VRAM slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HouseGraphicsKey {
    /// Compressed graphics packet CPU address.
    Compressed(u32),
}
/// One bounded ordinary-list record. Duration is retained, not scheduled.
#[derive(Debug)]
pub struct HouseFrame {
    key: HousePoseKey,
    duration: u8,
    facing: u8,
    source: SpriteFrame,
    composition: SpriteFrame,
}
impl HouseFrame {
    /// Packet/offset or direct-ROM source identity.
    #[must_use]
    pub const fn key(&self) -> HousePoseKey {
        self.key
    }
    /// Native record duration byte; not an elapsed-frame API or scheduler.
    #[must_use]
    pub const fn duration(&self) -> u8 {
        self.duration
    }
    /// Native facing after the actor's qualified H-flip adjustment.
    #[must_use]
    pub const fn facing(&self) -> u8 {
        self.facing
    }
    /// Original decompressed/direct source bytes before palette relocation.
    #[must_use]
    pub const fn source_composition(&self) -> &SpriteFrame {
        &self.source
    }
    /// Shared composition with native palette relocation, but no VRAM tile-slot offset.
    #[must_use]
    pub const fn composition(&self) -> &SpriteFrame {
        &self.composition
    }
}
#[derive(Debug, Clone)]
struct Packet {
    cpu: u32,
    bytes: Arc<[u8]>,
    extent: Range<usize>,
}
#[derive(Debug)]
struct Graphics {
    key: HouseGraphicsKey,
    tiles: Arc<[Tile4bpp]>,
}
#[derive(Debug, Clone)]
struct Resource {
    packet: Option<Packet>,
    graphics: Arc<Graphics>,
    palette: [Bgr555; 16],
    palette_base: u8,
    source_palette: u8,
}
/// One immutable, source-derived setup instance. No AI, collision, dialogue or clock.
#[derive(Debug)]
pub struct HouseActor {
    id: u32,
    map: u16,
    position: [u16; 2],
    /// The header's initial selector, before any script selects a pose.
    initial: u8,
    selector: u8,
    hflip: bool,
    tie_rank: u8,
    frames: Vec<HouseFrame>,
    resource: Resource,
    ranges: Vec<Range<usize>>,
}
impl HouseActor {
    /// Decodes the setup art a map's spawn records install, in list order.
    ///
    /// `HouseScenes` names its residents by profile; this follows each `$01`
    /// record's resource descriptor and takes the first record of the
    /// sequence `pose` selects, with the same shape checks. The result is
    /// aligned with `records`.
    ///
    /// The list is decoded as a whole because a record may lean on the one
    /// before it: a descriptor of `$000000` reuses its resource, and graphics
    /// of `$FFFF` reuse its graphics. The native loader always has that
    /// predecessor; this one may have refused it, and then the reuse is
    /// refused too rather than handed the last body that happened to decode.
    /// Whether a `$00` or `$FD` record moves the native predecessor is not
    /// established; here they do not. A `$00` record carries a descriptor
    /// too, but every one in the slice has a movement-resource mode outside
    /// the qualified set, so following it gains nothing and poisons three
    /// more reuses.
    pub fn from_records(
        image: &[u8],
        map: u16,
        records: &[SpawnRecord],
        mut pose: impl FnMut(&SpawnRecord) -> ResidentPose,
    ) -> Vec<Result<Self, RecordRefusal>> {
        // Index in `out` of the last descriptor-bearing record, which is the
        // predecessor a reuse reaches for, decoded or not.
        let mut previous: Option<usize> = None;
        let mut out: Vec<Result<Self, RecordRefusal>> = Vec::with_capacity(records.len());
        for record in records {
            if record.opcode() != 1 || record.bytes().len() != 10 {
                out.push(Err(RecordRefusal::NoDescriptor));
                continue;
            }
            let prior = previous.map(|index| out[index].as_ref());
            let result = match prior {
                Some(Err(_)) if reuses_predecessor(image, record.bytes()) => {
                    Err(RecordRefusal::PredecessorRefused)
                }
                _ => {
                    Self::from_record(image, map, record, pose(record), prior.and_then(Result::ok))
                        .map_err(RecordRefusal::Invalid)
                }
            };
            previous = Some(out.len());
            out.push(result);
        }
        out
    }

    /// One record, with the predecessor its reuse may reach for.
    ///
    /// # Errors
    /// Refuses a record without a descriptor, a descriptor shape outside the
    /// qualified set, a selector outside the packet's table, and a reuse with
    /// no predecessor.
    fn from_record(
        image: &[u8],
        map: u16,
        record: &SpawnRecord,
        pose: ResidentPose,
        previous: Option<&Self>,
    ) -> Result<Self, SpriteError> {
        let spawn = record.bytes();
        if record.opcode() != 1 || spawn.len() != 10 {
            return Err(SpriteError::Invalid(
                "spawn record carries no resource descriptor",
            ));
        }
        // Neither the record's flag byte nor the header's tail names art.
        // Byte 3 is `$00` on 58 of the slice's 61 descriptor-bearing records
        // and `$01`/`$02` on the rest; the header is `[selector, n, flags, 0,
        // n]` with flags `$41`/`$50`/`$51`/`$D1` seen. The descriptor alone
        // names the packet, palette and graphics, and the frozen nine
        // reproduce from it, so these bytes are read for the selector and
        // otherwise left to the actor VM they seed.
        let mut loader = Loader::new(image);
        let source = record.offset();
        loader.read(source, 10)?;
        let header = pointer(&spawn[4..7])?;
        let head = loader.read(header, 5)?;
        let selector = pose.selector.unwrap_or(head[0]);
        let resource = if spawn[7..10] == [0, 0, 0] {
            let prior =
                previous.ok_or(SpriteError::Invalid("missing descriptor reuse predecessor"))?;
            loader.ranges.extend(prior.ranges.clone());
            prior.resource.clone()
        } else {
            loader.resource(pointer(&spawn[7..10])?, previous)?
        };
        let packet = resource
            .packet
            .as_ref()
            .ok_or(SpriteError::Invalid("cannot reuse a direct-ROM resource"))?;
        let frames = decode_sequence(
            &packet.bytes,
            ListSpec {
                selector,
                count: 1,
                duration: 0,
                source_cpu: packet.cpu,
                direct: false,
                source_palette: resource.source_palette,
                palette_base: resource.palette_base,
                hflip: pose.hflip,
            },
        )?;
        Ok(Self {
            id: cpu_address(source)?,
            map,
            position: [record.origin().0, record.origin().1],
            initial: head[0],
            selector,
            hflip: pose.hflip,
            tie_rank: 0,
            frames,
            resource,
            ranges: loader.ranges,
        })
    }

    /// Stable source spawn record, or source child-creation instruction for F's object.
    #[must_use]
    pub const fn source_id(&self) -> u32 {
        self.id
    }
    /// Qualified room membership; this is not inferred from shared graphics.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        self.map
    }
    /// Source origin. D uses its creation origin, never a later wandering snapshot.
    #[must_use]
    pub const fn position(&self) -> [u16; 2] {
        self.position
    }
    /// Qualified ordinary selector; D deliberately uses the source header's initial selector.
    #[must_use]
    pub const fn selector(&self) -> u8 {
        self.selector
    }
    /// The header's initial selector, which a script starts from.
    #[must_use]
    pub const fn initial(&self) -> u8 {
        self.initial
    }
    /// Decodes any sequence of the actor's packet, for a pose a running
    /// script selects later: the walking sequences 3, 4 and 5, for instance.
    ///
    /// # Errors
    /// Refuses a selector outside the packet's table, a direct-ROM resource,
    /// and records outside the qualified shape.
    pub fn sequence(&self, selector: u8, hflip: bool) -> Result<Vec<HouseFrame>, SpriteError> {
        let packet = self
            .resource
            .packet
            .as_ref()
            .ok_or(SpriteError::Invalid("direct-ROM resource has no packet"))?;
        decode_sequence(
            &packet.bytes,
            ListSpec {
                selector,
                count: 1,
                duration: 0,
                source_cpu: packet.cpu,
                direct: false,
                source_palette: self.resource.source_palette,
                palette_base: self.resource.palette_base,
                hflip,
            },
        )
    }
    /// Qualified actor horizontal mirror. Vertical mirroring is not requested.
    #[must_use]
    pub const fn hflip(&self) -> bool {
        self.hflip
    }
    /// Ordinary equal-world-Y painter rank, derived from the qualified native list.
    #[must_use]
    pub const fn tie_rank(&self) -> u8 {
        self.tie_rank
    }
    /// Complete bounded ordinary record list, including repeated composition keys.
    #[must_use]
    pub fn frames(&self) -> &[HouseFrame] {
        &self.frames
    }
    /// Explicit frozen setup policy: first source list record, not native AI simulation.
    #[must_use]
    pub const fn setup_frame(&self) -> &HouseFrame {
        &self.frames.as_slice()[0]
    }
    /// Effective facing of the selected setup frame.
    #[must_use]
    pub const fn facing(&self) -> u8 {
        self.setup_frame().facing
    }
    /// Compressed resource key; the same raster can be shared by distinct instances.
    #[must_use]
    pub fn graphics_key(&self) -> HouseGraphicsKey {
        self.resource.graphics.key
    }
    /// Shared source-indexed 256-tile resource. Do not add native OAM name-select $100.
    #[must_use]
    pub fn graphics(&self) -> &[Tile4bpp] {
        &self.resource.graphics.tiles
    }
    /// CGRAM base to subtract from opaque `SpritePixel::palette_index`.
    #[must_use]
    pub const fn palette_base(&self) -> u8 {
        self.resource.palette_base
    }
    /// Sixteen natural BGR555 colors; index zero remains transparent.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 16] {
        &self.resource.palette
    }
    /// Exact headerless source extents, including shared-resource dependencies.
    /// Ranges can overlap/repeat; compressed extents end at their consumed terminator.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}
/// Nine fresh setup residents plus F's source-created table object.
/// Controllers, shadow, temporary scene text and gated E/20/21 visuals are excluded.
#[derive(Debug)]
pub struct HouseScenes {
    actors: Vec<HouseActor>,
    ranges: Vec<Range<usize>>,
}
impl HouseScenes {
    /// Decode the complete admitted setup roster from a caller-authenticated Japanese ROM.
    /// This projects bounded source records/lists; it does not execute event predicates.
    ///
    /// # Errors
    /// Rejects changed source shapes, unsupported resource modes/palettes/tile boundaries,
    /// missing reuse dependencies, malformed packets, bad pointers and truncated input.
    pub fn from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        Self::decode(image, true)
    }
    fn decode(image: &[u8], include_prop: bool) -> Result<Self, SpriteError> {
        let mut loader = Loader::new(image);
        let mut actors = Vec::new();
        for profile in PROFILES {
            let previous = actors.last().filter(|a: &&HouseActor| a.map == profile.map);
            actors.push(loader.actor(profile, previous)?);
        }
        if include_prop {
            actors.push(loader.prop()?);
        }
        Ok(Self {
            actors,
            ranges: loader.ranges,
        })
    }
    #[cfg(test)]
    fn residents_from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        Self::decode(image, false)
    }
    pub(super) fn legacy_first(image: &[u8]) -> Result<HouseActor, SpriteError> {
        Loader::new(image).actor(PROFILES[6], None)
    }
    /// Admitted instances. Transport should key by source ID, not by pose or runtime slot.
    #[must_use]
    pub fn actors(&self) -> &[HouseActor] {
        &self.actors
    }
    /// Exact stable instance lookup; no fallback to another resident.
    #[must_use]
    pub fn actor(&self, id: u32) -> Option<&HouseActor> {
        self.actors.iter().find(|a| a.id == id)
    }
    /// Ark's rank after the qualified ordinary actors at equal world Y.
    /// Shadows/text are different ordering classes and are not represented here.
    #[must_use]
    pub const fn ark_tie_rank(map: u16) -> Option<u8> {
        match map {
            11 | 13 | 15 | 17 => Some(1),
            12 => Some(4),
            16 => Some(2),
            _ => None,
        }
    }
    /// All exact input extents consumed by this projection, including cached/shared inputs.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}
#[derive(Clone, Copy)]
struct Ordinary {
    offset: usize,
    callback: usize,
    flip: Option<bool>,
    backedge: bool,
}
#[derive(Clone, Copy)]
struct Profile {
    map: u16,
    offset: usize,
    initial: u8,
    count: usize,
    rank: u8,
    ordinary: Option<Ordinary>,
}
const fn ordinary(offset: usize, callback: usize, flip: Option<bool>, backedge: bool) -> Ordinary {
    Ordinary {
        offset,
        callback,
        flip,
        backedge,
    }
}
const PROFILES: [Profile; 9] = [
    Profile {
        map: 11,
        offset: 26,
        initial: 6,
        count: 1,
        rank: 0,
        ordinary: None,
    },
    Profile {
        map: 12,
        offset: 26,
        initial: 1,
        count: 4,
        rank: 3,
        ordinary: Some(ordinary(0x5b, 0x5b, None, true)),
    },
    Profile {
        map: 12,
        offset: 36,
        initial: 2,
        count: 4,
        rank: 2,
        ordinary: Some(ordinary(0x46, 0x46, Some(false), true)),
    },
    Profile {
        map: 12,
        offset: 46,
        initial: 0,
        count: 4,
        rank: 1,
        ordinary: Some(ordinary(0x5b, 0x5b, None, true)),
    },
    Profile {
        map: 12,
        offset: 56,
        initial: 0,
        count: 4,
        rank: 0,
        ordinary: Some(ordinary(0x5b, 0x5b, None, true)),
    },
    Profile {
        map: 13,
        offset: 19,
        initial: 3,
        count: 4,
        rank: 0,
        ordinary: None,
    },
    Profile {
        map: 16,
        offset: 19,
        initial: 2,
        count: 1,
        rank: 1,
        ordinary: Some(ordinary(0x2a, 0x2a, Some(false), true)),
    },
    Profile {
        map: 16,
        offset: 29,
        initial: 2,
        count: 1,
        rank: 0,
        ordinary: Some(ordinary(0x2c, 0x2c, Some(true), true)),
    },
    Profile {
        map: 17,
        offset: 19,
        initial: 4,
        count: 4,
        rank: 0,
        ordinary: Some(ordinary(0x27, 0x21, Some(true), false)),
    },
];
struct Loader<'a> {
    image: &'a [u8],
    ranges: Vec<Range<usize>>,
    packets: HashMap<usize, Packet>,
    graphics: HashMap<usize, Arc<Graphics>>,
}
impl<'a> Loader<'a> {
    fn new(image: &'a [u8]) -> Self {
        Self {
            image,
            ranges: Vec::new(),
            packets: HashMap::new(),
            graphics: HashMap::new(),
        }
    }
    fn read(&mut self, at: usize, len: usize) -> Result<&'a [u8], SpriteError> {
        bank_range(at, len)?;
        let b = take(self.image, at, len)?;
        self.ranges.push(at..at + len);
        Ok(b)
    }
    fn packet(&mut self, cpu: u32) -> Result<Packet, SpriteError> {
        let start = pointer(&cpu.to_le_bytes()[..3])?;
        if let Some(p) = self.packets.get(&start) {
            self.ranges.push(p.extent.clone());
            return Ok(p.clone());
        }
        let end = (((start >> 16) + 1) << 16).min(self.image.len());
        let b = self
            .image
            .get(start..end)
            .ok_or(SpriteError::Invalid("truncated house packet"))?;
        let d = compression::decode(b, 0xffff)
            .map_err(|_| SpriteError::Invalid("invalid house compressed packet"))?;
        let p = Packet {
            cpu,
            bytes: d.data.into(),
            extent: start..start + d.consumed,
        };
        self.ranges.push(p.extent.clone());
        self.packets.insert(start, p.clone());
        Ok(p)
    }
    fn graphics(&mut self, cpu: u32) -> Result<Arc<Graphics>, SpriteError> {
        let p = self.packet(cpu)?;
        let start = p.extent.start;
        if p.bytes.len() != 0x2000 {
            return Err(SpriteError::Invalid("changed house graphics extent"));
        }
        if let Some(g) = self.graphics.get(&start) {
            return Ok(g.clone());
        }
        let g = Arc::new(Graphics {
            key: HouseGraphicsKey::Compressed(cpu),
            tiles: decode_tiles_4bpp(&p.bytes)?.into(),
        });
        self.graphics.insert(start, g.clone());
        Ok(g)
    }
    fn room_list(&mut self, map: u16) -> Result<usize, SpriteError> {
        let (xy, targets) = match map {
            11 => ([8, 13], [0xea28_u16, 0x8bdf]),
            12 => ([8, 22], [0xea34, 0x8c72]),
            13 => ([7, 39], [0xea5e, 0x8ce2]),
            15 => ([19, 7], [0xea6f, 0x8d58]),
            16 => ([24, 6], [0xea85, 0x8daf]),
            17 => ([13, 27], [0xeaa0, 0x8e0e]),
            _ => return Err(SpriteError::Invalid("unqualified house room")),
        };
        let index = usize::from(map) * 2;
        if self.read(0x2_8000 + index, 2)? != [0, 0] {
            return Err(SpriteError::Invalid("changed house actor table branch"));
        }
        let entry = self.read(0x3_8000 + index, 2)?;
        let list = pointer(&[entry[0], entry[1], 0x83])?;
        let mut prefix = vec![0, 6, 0xfd, xy[0], xy[1], 0, 0x29, 0xa1, 0x84];
        for (event, target) in [([0xac, 1], targets[0]), ([0x96, 1], targets[1])] {
            prefix.extend([0xfa, event[0], event[1]]);
            prefix.extend(target.to_le_bytes());
        }
        if map == 11 || map == 12 {
            prefix.extend([0xfa, 0xba, 0x10, 0xbb, 0]);
            prefix.extend((if map == 11 { 0x8bba_u16 } else { 0x8c4d }).to_le_bytes());
        }
        if self.read(list, prefix.len())? != prefix {
            return Err(SpriteError::Invalid("changed house ordinary spawn prefix"));
        }
        Ok(list)
    }
    fn actor(
        &mut self,
        p: Profile,
        previous: Option<&HouseActor>,
    ) -> Result<HouseActor, SpriteError> {
        let start = self.ranges.len();
        let list = self.room_list(p.map)?;
        bank_range(list, p.offset + 10)?;
        let source = list + p.offset;
        let spawn = self.read(source, 10)?;
        if spawn[0] != 1 || spawn[3] != 0 {
            return Err(SpriteError::Invalid("changed house resident spawn flags"));
        }
        let position = [u16::from(spawn[1]) * 16 + 8, u16::from(spawn[2]) * 16];
        let header = pointer(&spawn[4..7])?;
        if self.read(header, 5)? != [p.initial, 0, 0x51, 0, 0] {
            return Err(SpriteError::Invalid("changed house resident header"));
        }
        let (selector, hflip) = if let Some(o) = p.ordinary {
            self.selection(header, o)?
        } else {
            (p.initial, false)
        };
        let resource = if spawn[7..10] == [0, 0, 0] {
            let prior =
                previous.ok_or(SpriteError::Invalid("missing descriptor reuse predecessor"))?;
            if p.map != 12 || p.offset != 36 {
                return Err(SpriteError::Invalid("unqualified descriptor reuse"));
            }
            self.ranges.extend(prior.ranges.clone());
            prior.resource.clone()
        } else {
            if p.map == 12 && p.offset == 36 {
                return Err(SpriteError::Invalid("changed descriptor reuse"));
            }
            let descriptor = pointer(&spawn[7..10])?;
            self.resource(descriptor, previous)?
        };
        let packet = resource
            .packet
            .as_ref()
            .ok_or(SpriteError::Invalid("cannot reuse a direct-ROM resource"))?;
        let frames = decode_list(
            &packet.bytes,
            ListSpec {
                selector,
                count: p.count,
                duration: if p.count == 1 { 0 } else { 7 },
                source_cpu: packet.cpu,
                direct: false,
                source_palette: resource.source_palette,
                palette_base: resource.palette_base,
                hflip,
            },
        )?;
        Ok(HouseActor {
            id: cpu_address(source)?,
            map: p.map,
            position,
            initial: p.initial,
            selector,
            hflip,
            tie_rank: p.rank,
            frames,
            resource,
            ranges: self.ranges[start..].to_vec(),
        })
    }
    fn selection(&mut self, header: usize, o: Ordinary) -> Result<(u8, bool), SpriteError> {
        let len = 9 + usize::from(o.flip.is_some()) * 2 + usize::from(o.backedge) * 2;
        let at = header + o.offset;
        bank_range(header, o.offset + len)?;
        let b = self.read(at, len)?;
        let mut expected = vec![2, 0x23];
        expected.extend(
            u16::try_from((header + o.callback) & 0xffff)
                .expect("bank offset")
                .to_le_bytes(),
        );
        if let Some(f) = o.flip {
            expected.extend([2, if f { 0xb7 } else { 0xb6 }]);
        }
        let selector = b[expected.len() + 2];
        expected.extend([2, 0x80, selector, 2, 0x8e]);
        if o.backedge {
            expected.extend([0x80, u8::try_from(256 - len).unwrap()]);
        }
        if b != expected {
            return Err(SpriteError::Invalid("changed house ordinary pose script"));
        }
        Ok((selector, o.flip.unwrap_or(false)))
    }
    fn resource(
        &mut self,
        at: usize,
        previous: Option<&HouseActor>,
    ) -> Result<Resource, SpriteError> {
        let prefix = take(self.image, at, 5)?;
        let extra = match prefix[3..5] {
            [0x20, 0] => 0,
            [0, 0] => 3,
            _ => return Err(SpriteError::Invalid("unsupported house movement resource")),
        };
        let pal = 5 + extra;
        let gfx = pal + 4;
        let g = take(self.image, at + gfx, 2)?;
        let reuse = g == [0xff, 0xff];
        let d = self.read(at, gfx + if reuse { 2 } else { 4 })?;
        if extra != 0 {
            pointer(&d[5..8])?;
        }
        if ![0x80, 0x81].contains(&d[pal])
            || ![0, 2, 4].contains(&d[pal + 1])
            || d[pal + 2] != 2
            || ![8, 10].contains(&d[pal + 3])
        {
            return Err(SpriteError::Invalid("unsupported house palette descriptor"));
        }
        let palette_table = 0xfc72 + usize::from(d[pal] & 63) * 3;
        let base = pointer(self.read(palette_table, 3)?)?;
        let palette_start = base + usize::from(d[pal + 1]) * 16;
        bank_range(base, usize::from(d[pal + 1]) * 16 + 32)?;
        // Preserve the legacy first-resident source read ordering.
        let graphics_cpu = if reuse {
            None
        } else {
            if d[gfx..gfx + 3] != [0, 0, 0xc0] || ![0, 3].contains(&d[gfx + 3]) {
                return Err(SpriteError::Invalid("unsupported house graphics transfer"));
            }
            Some(cpu(self.read(0xfda4 + usize::from(d[gfx + 3]), 3)?))
        };
        let colors = self.read(palette_start, 32)?;
        let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
        let packet = self.packet(cpu(&d[..3]))?;
        let table_end = frame_table_end(&packet.bytes)?;
        let table_last = table_end - 2;
        let first_anchor = usize::from(word(take(&packet.bytes, table_last, 2)?, 0));
        let first = take(&packet.bytes, first_anchor, 24)?;
        if first[16] == 0 {
            return Err(SpriteError::Invalid("empty first packed house frame"));
        }
        let source_palette = ((word(first, 22) >> 9) & 7) as u8;
        let graphics = if let Some(g) = graphics_cpu {
            self.graphics(g)?
        } else {
            let prior =
                previous.ok_or(SpriteError::Invalid("missing graphics reuse predecessor"))?;
            self.ranges.extend(prior.ranges.clone());
            prior.resource.graphics.clone()
        };
        Ok(Resource {
            packet: Some(packet),
            graphics,
            palette,
            palette_base: 128 + d[pal + 3] * 8,
            source_palette,
        })
    }
    #[allow(clippy::too_many_lines)] // One source-created object, read end to end.
    fn prop(&mut self) -> Result<HouseActor, SpriteError> {
        let start = self.ranges.len();
        let list = self.room_list(15)?;
        bank_range(list, 56)?;
        let parent = self.read(list + 49, 7)?;
        if parent[0] != 0xfd || parent[3] != 0 {
            return Err(SpriteError::Invalid("changed house prop parent"));
        }
        let header_cpu = cpu(&parent[4..7]);
        let header = pointer(&parent[4..7])?;
        if self.read(header, 5)? != [0, 0, 0xd1, 0, 1] {
            return Err(SpriteError::Invalid("changed house prop parent header"));
        }
        bank_range(header, 20)?;
        let call = self.read(header + 11, 9)?;
        if call[..2] != [2, 0x9c] {
            return Err(SpriteError::Invalid("changed house child creation"));
        }
        let dx = i16::from_le_bytes([call[5], call[6]]);
        let dy = i16::from_le_bytes([call[7], call[8]]);
        let position = [
            u16::try_from(i32::from(parent[1]) * 16 + 8 + i32::from(dx)),
            u16::try_from(i32::from(parent[2]) * 16 + i32::from(dy)),
        ];
        let position = [
            position[0].map_err(|_| SpriteError::Invalid("house child X overflow"))?,
            position[1].map_err(|_| SpriteError::Invalid("house child Y overflow"))?,
        ];
        let child = self.read(pointer(&call[2..5])?, 24)?;
        if child[..14] != [0xa9, 0, 0, 0x9d, 4, 0, 0xa9, 0, 0, 0x9d, 6, 0, 2, 0xd8]
            || child[17..19] != [2, 0x80]
            || child[20..] != [2, 0x8e, 0x80, 0xf9]
        {
            return Err(SpriteError::Invalid("changed house prop setup script"));
        }
        let base_cpu = cpu(&child[14..17]);
        let base = pointer(&child[14..17])?;
        let selector = child[19];
        bank_range(base, usize::from(selector) * 2 + 2)?;
        let sequence = usize::from(word(self.read(base + usize::from(selector) * 2, 2)?, 0));
        bank_range(base, sequence + 6)?;
        let record = self.read(base + sequence, 6)?;
        let anchor = usize::from(word(record, 2));
        bank_range(base, anchor + 17)?;
        let count = usize::from(take(self.image, base + anchor, 17)?[16]);
        bank_range(base, anchor + 17 + count * 7)?;
        self.read(base + anchor, 17 + count * 7)?;
        let bytes = take(self.image, base, 0x1_0000 - (base & 0xffff))?;
        let frames = decode_list(
            bytes,
            ListSpec {
                selector,
                count: 1,
                duration: 15,
                source_cpu: base_cpu,
                direct: true,
                source_palette: 2,
                palette_base: 160,
                hflip: false,
            },
        )?;
        let root = self.read(0x6_95c9, 3)?;
        let stream_bank = root[2];
        let script = self.read(pointer(root)?, 23)?;
        if script[..8] != [8, 0xfa, 1, 0, 0x80, 0, 0x10, 0]
            || script[11..17] != [0x40, 0, 0x40, 0, 0x20, 0x90]
            || script[20..] != [0, 0x10, 0]
        {
            return Err(SpriteError::Invalid(
                "changed house prop graphics/palette load",
            ));
        }
        let packed = |p: usize| {
            crate::maps::scripts::unpack_pointer(
                [script[p], script[p + 1], script[p + 2]],
                stream_bank,
            )
            .map_err(|_| SpriteError::Invalid("invalid house prop packed pointer"))
        };
        let graphics = self.graphics(packed(8)?.value())?;
        let palette_start = packed(17)?.normalized().value() as usize;
        bank_range(palette_start, 64)?;
        let colors = self.read(palette_start + 32, 32)?;
        let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
        Ok(HouseActor {
            id: header_cpu + 11,
            map: 15,
            position,
            initial: selector,
            selector,
            hflip: false,
            tie_rank: 0,
            frames,
            resource: Resource {
                packet: None,
                graphics,
                palette,
                palette_base: 160,
                source_palette: 2,
            },
            ranges: self.ranges[start..].to_vec(),
        })
    }
}
#[derive(Clone, Copy)]
struct ListSpec {
    selector: u8,
    count: usize,
    duration: u8,
    source_cpu: u32,
    direct: bool,
    source_palette: u8,
    palette_base: u8,
    hflip: bool,
}
/// Offset of the sequence a selector names in a compressed packet's table.
fn packed_sequence(bytes: &[u8], selector: u8) -> Result<usize, SpriteError> {
    if usize::from(selector) * 2 + 2 > frame_table_end(bytes)? - 2 {
        return Err(SpriteError::Invalid("house selector outside packed table"));
    }
    Ok(usize::from(word(
        take(bytes, usize::from(selector) * 2, 2)?,
        0,
    )))
}
fn frame_table_end(bytes: &[u8]) -> Result<usize, SpriteError> {
    let end = usize::from(word(take(bytes, 0, 2)?, 0));
    if end < 4 || end % 2 != 0 {
        return Err(SpriteError::Invalid("invalid house frame table"));
    }
    take(bytes, 0, end)?;
    Ok(end)
}
fn decode_list(bytes: &[u8], spec: ListSpec) -> Result<Vec<HouseFrame>, SpriteError> {
    let ListSpec {
        selector,
        count,
        duration,
        direct,
        ..
    } = spec;
    let sequence = if direct {
        usize::from(word(take(bytes, usize::from(selector) * 2, 2)?, 0))
    } else {
        packed_sequence(bytes, selector)?
    };
    let records = take(bytes, sequence, count * 4 + 2)?;
    if records[count * 4..] != [0xff, 0xff] {
        return Err(SpriteError::Invalid("changed house frame list extent"));
    }
    records[..count * 4]
        .chunks_exact(4)
        .map(|r| {
            if r[0] != duration {
                return Err(SpriteError::Invalid("unsupported house frame record"));
            }
            decode_record(bytes, r, spec)
        })
        .collect()
}

/// Records one pose list may hold before it is treated as unterminated.
const MAX_SEQUENCE_RECORDS: usize = 64;

/// Every record of the sequence `spec.selector` names, up to its terminator.
///
/// The frozen loader knows each list's length and timing from its profile;
/// a record found by walking does not, so the list is read to its `$FFFF`
/// and each record keeps its own duration. The first is the setup frame.
fn decode_sequence(bytes: &[u8], spec: ListSpec) -> Result<Vec<HouseFrame>, SpriteError> {
    let sequence = packed_sequence(bytes, spec.selector)?;
    let mut frames = Vec::new();
    for index in 0..=MAX_SEQUENCE_RECORDS {
        let record = take(bytes, sequence + index * 4, 4)?;
        if record[..2] == [0xff, 0xff] {
            if frames.is_empty() {
                return Err(SpriteError::Invalid("empty house frame list"));
            }
            return Ok(frames);
        }
        frames.push(decode_record(bytes, record, spec)?);
    }
    Err(SpriteError::Invalid("unterminated house frame list"))
}

/// One four-byte list record: duration, facing, composition anchor.
fn decode_record(bytes: &[u8], r: &[u8], spec: ListSpec) -> Result<HouseFrame, SpriteError> {
    let ListSpec {
        source_cpu,
        direct,
        source_palette,
        palette_base,
        hflip,
        ..
    } = spec;
    if ![0, 1, 3].contains(&r[1]) {
        return Err(SpriteError::Invalid("unsupported house frame record"));
    }
    let anchor = usize::from(word(r, 2));
    let n = usize::from(take(bytes, anchor, 17)?[16]);
    let raw = take(bytes, anchor, 17 + n * 7)?;
    let original = SpriteFrame::decode(raw)?;
    let mut adjusted = raw.to_vec();
    for (i, c) in original.components().iter().enumerate() {
        let tile = c.word() & 511;
        if (c.word() >> 9) & 7 != u16::from(source_palette)
            || (c.word() >> 12) & 3 != 2
            || tile >= 256
            || (c.size() == 16 && (tile & 15 == 15 || tile + 17 >= 256))
        {
            return Err(SpriteError::Invalid(
                "unsupported house palette/priority/tile boundary",
            ));
        }
        let relocated = c
            .word()
            .wrapping_sub(u16::from(source_palette) << 9)
            .wrapping_add(u16::from((palette_base - 128) / 16) << 9);
        adjusted[22 + i * 7..24 + i * 7].copy_from_slice(&relocated.to_le_bytes());
    }
    let offset = u16::try_from(anchor + 4)
        .map_err(|_| SpriteError::Invalid("oversized house composition offset"))?;
    let key = if direct {
        HousePoseKey::Direct(source_cpu + u32::from(offset))
    } else {
        HousePoseKey::Compressed {
            packet: source_cpu,
            offset,
        }
    };
    Ok(HouseFrame {
        key,
        duration: r[0],
        facing: if r[1] == 3 && hflip { 2 } else { r[1] },
        source: original,
        composition: SpriteFrame::decode(&adjusted)?,
    })
}
fn pointer(p: &[u8]) -> Result<usize, SpriteError> {
    let bank = p[2];
    let offset = word(p, 0);
    if bank < 0x80 || (bank < 0xc0 && offset < 0x8000) {
        return Err(SpriteError::Invalid("unsupported house ROM pointer"));
    }
    Ok((usize::from(bank & 63) << 16) | usize::from(offset))
}
fn cpu(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], 0])
}
fn cpu_address(at: usize) -> Result<u32, SpriteError> {
    u32::try_from(at)
        .map(|a| a | 0x80_0000)
        .map_err(|_| SpriteError::Invalid("oversized house source address"))
}

#[cfg(test)]
#[path = "house_tests.rs"]
mod tests;
