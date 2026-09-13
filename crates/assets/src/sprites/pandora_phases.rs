//! Finite source endpoints. These are not elapsed-time snapshots or NPC AI.
use super::{cpu, cpu_address, pointer, Loader, SpriteError};

/// One source instance in a parent-selected phase. Bounds are visual, not collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PandoraActorPhase {
    /// Stable source spawn record (never a WRAM slot).
    pub source_id: u32,
    /// Immutable art descriptor identity.
    pub art_id: u32,
    /// World endpoint in source pixels. Moving between endpoints is parent-owned.
    pub position: [u16; 2],
    /// Source spawn/COP instruction supplying this endpoint.
    pub position_source: u32,
    /// Exact source animation list selector, not a frame index.
    pub selector: u8,
    /// Actor horizontal mirror; the frame's facing 3 becomes facing 2 when set.
    pub hflip: bool,
    /// Relative equal-Y painter rank: later ranks paint on top. Ark follows ordinary actors.
    pub tie_rank: u8,
    /// Source per-actor OBJ priority override (tour COPBA $30); not baked into art.
    pub priority_override: Option<u8>,
}
/// Explicit unresolved visual work. Consumers must not mistake an art list for scene parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PandoraSceneLimit {
    /// A source spawn-frozen town presentation, not the native wandering schedule.
    FrozenTown,
    /// Ark shadow, transition poses, BG layers, masks/color math are outside this art library.
    SceneComposition,
    /// Required second-hit PPU window/color-math script $889CD8 is not rendered here.
    CellarColorMath,
    /// Scripted movement between finite endpoints is not implemented by this asset compiler.
    ScriptedMotion,
    /// Opening guide palette is temporarily all-white; effect program not compiled here.
    OpeningPalette,
}
/// A finite phase choice. Parent progression selects it; this library never reads event state.
#[derive(Debug)]
pub struct PandoraPhase {
    pub(super) id: &'static str,
    map: u16,
    actors: Vec<PandoraActorPhase>,
    limits: Vec<PandoraSceneLimit>,
}
impl PandoraPhase {
    /// Stable semantic phase identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }
    /// Source map, including forced tutorial destinations.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        self.map
    }
    /// Explicit membership, endpoints, source list choice and relative ordinary tie order.
    #[must_use]
    pub fn actors(&self) -> &[PandoraActorPhase] {
        &self.actors
    }
    /// Required omitted effects/behavior; never an implicit fidelity promise.
    #[must_use]
    pub fn limits(&self) -> &[PandoraSceneLimit] {
        &self.limits
    }
    /// Ark's ordinary painter rank after this roster. Special draw classes remain separate.
    #[must_use]
    pub fn ark_tie_rank(&self) -> usize {
        self.actors.len()
    }
}
fn spawn(
    loader: &mut Loader<'_>,
    id: u32,
    art: u32,
    selector: u8,
    rank: u8,
) -> Result<PandoraActorPhase, SpriteError> {
    let at = pointer(&id.to_le_bytes()[..3])?;
    let b = loader.read(at, 10)?;
    if b[0] > 1 {
        return Err(SpriteError::Invalid("changed Pandora visual spawn"));
    }
    let header = pointer(&b[4..7])?;
    loader.read(header, 5)?;
    let descriptor = cpu(&b[7..]);
    if descriptor != 0 && descriptor != art {
        return Err(SpriteError::Invalid("changed Pandora spawn descriptor"));
    }
    Ok(PandoraActorPhase {
        source_id: id,
        art_id: art,
        position: [u16::from(b[1]) * 16 + 8, u16::from(b[2]) * 16],
        position_source: id,
        selector,
        hflip: false,
        tie_rank: rank,
        priority_override: None,
    })
}
fn cop13(
    loader: &mut Loader<'_>,
    actor: &mut PandoraActorPhase,
    at: usize,
) -> Result<(), SpriteError> {
    let b = loader.read(at, 5)?;
    if b[..2] != [2, 0x13] {
        return Err(SpriteError::Invalid("changed Pandora relocation COP"));
    }
    actor.position = [u16::from(b[2]) * 16 + 8, u16::from(b[3]) * 16];
    actor.selector = b[4];
    actor.position_source = cpu_address(at)?;
    Ok(())
}
fn phase(
    id: &'static str,
    map: u16,
    mut actors: Vec<PandoraActorPhase>,
    extra: &[PandoraSceneLimit],
) -> PandoraPhase {
    for (i, actor) in actors.iter_mut().rev().enumerate() {
        actor.tie_rank = u8::try_from(i).expect("bounded roster");
    }
    let mut limits = vec![PandoraSceneLimit::SceneComposition];
    limits.extend(extra);
    PandoraPhase {
        id,
        map,
        actors,
        limits,
    }
}
#[allow(clippy::too_many_lines)] // Bounded declarative phase roster, not a scheduler.
pub(super) fn phases(loader: &mut Loader<'_>) -> Result<Vec<PandoraPhase>, SpriteError> {
    use PandoraSceneLimit::{CellarColorMath, FrozenTown, ScriptedMotion};
    let mut result = Vec::new();
    // Source spawn-frozen subset intersecting the expanded corridor/viewport. The
    // eastern workers and remote southern performers are not all-town admission.
    let mut town = Vec::new();
    for (id, art, selector) in [
        (0x83_89bf, 0x83_ebe4, 3),
        (0x83_89c9, 0x83_ebe4, 3),
        (0x83_8a05, 0x83_ec7e, 0),
        (0x83_8a19, 0x83_ed37, 0),
        (0x83_8a23, 0x83_ed37, 0),
        (0x83_8a2d, 0x83_ed37, 0),
    ] {
        town.push(spawn(loader, id, art, selector, 0)?);
    }
    for (i, actor) in town.iter_mut().rev().enumerate() {
        actor.tie_rank = u8::try_from(i).expect("bounded Pandora roster");
    }
    result.push(phase("town-source", 0xa, town, &[FrozenTown]));
    let resident = spawn(loader, 0x83_8ebe, 0x83_ecc6, 2, 0)?;
    result.push(phase("resident-13", 0x13, vec![resident], &[]));
    result.push(phase(
        "resident-13-request",
        0x13,
        vec![PandoraActorPhase {
            selector: 0,
            ..resident
        }],
        &[],
    ));
    let mut c = vec![
        spawn(loader, 0x83_8c0a, 0x83_ed5a, 2, 3)?,
        spawn(loader, 0x83_8c14, 0x83_ed5a, 2, 2)?,
        spawn(loader, 0x83_8c1e, 0x83_ed74, 1, 1)?,
        spawn(loader, 0x83_8c28, 0x83_edf8, 2, 0)?,
    ];
    cop13(loader, &mut c[0], 0x8_a387)?;
    cop13(loader, &mut c[2], 0x8_9ad0)?;
    cop13(loader, &mut c[3], 0x8_a20a)?;
    c[3].hflip = true;
    result.push(phase("c-entry", 0xc, c.clone(), &[ScriptedMotion]));
    // COP39 changes X to a tile-center endpoint. The source Y remains COP13's.
    let b = loader.read(0x8_9b01, 5)?;
    if b[..4] != [2, 0x39, 5, 0x70] {
        return Err(SpriteError::Invalid("changed C approach"));
    }
    let mut choice = c.clone();
    choice[2].position[0] = u16::from(b[4]) * 16 + 8;
    choice[2].position_source = 0x88_9b01;
    choice[2].selector = 0;
    choice[2].hflip = true;
    result.push(phase("c-choice", 0xc, choice, &[ScriptedMotion]));
    result.push(phase("c-direct", 0xc, c.clone(), &[]));
    result.push(phase("c-first-hit", 0xc, c.clone(), &[]));
    result.push(phase(
        "c-second-hit",
        0xc,
        c.clone(),
        &[CellarColorMath, ScriptedMotion],
    ));
    for (id, selector) in [("c-fourth-down", 0), ("c-fourth-up", 1)] {
        let mut cue = c.clone();
        cue[1].selector = selector;
        result.push(phase(id, 0xc, cue, &[ScriptedMotion]));
    }
    let mut reaction = vec![c[0], c[2], c[3]];
    result.push(phase(
        "c-color-math",
        0xc,
        reaction.clone(),
        &[CellarColorMath, ScriptedMotion],
    ));
    reaction[1].hflip = true;
    result.push(phase(
        "c-reaction-speaker",
        0xc,
        reaction.clone(),
        &[CellarColorMath, ScriptedMotion],
    ));
    reaction[2].selector = 0;
    result.push(phase(
        "c-reaction-right",
        0xc,
        reaction.clone(),
        &[ScriptedMotion],
    ));
    reaction.pop();
    reaction[0].selector = 0;
    result.push(phase(
        "c-reaction-left",
        0xc,
        reaction.clone(),
        &[ScriptedMotion],
    ));
    result.push(phase(
        "c-reaction-final",
        0xc,
        vec![reaction[1]],
        &[ScriptedMotion],
    ));
    result.push(phase("c-departed", 0xc, vec![], &[ScriptedMotion]));
    result.push(phase("cellar-e", 0xe, vec![], &[]));
    result.push(phase("cellar-20", 0x20, vec![], &[]));
    let box_actor = spawn(loader, 0x83_928f, 0x83_f984, 3, 0)?;
    result.push(phase("box-contact", 0x21, vec![box_actor], &[]));
    let guide = spawn(loader, 0x83_927b, 0x83_f8c0, 3, 0)?;
    result.push(phase(
        "box-opening",
        0x21,
        vec![guide],
        &[PandoraSceneLimit::OpeningPalette],
    ));
    result.push(phase(
        "box-opening-cue",
        0x21,
        vec![PandoraActorPhase {
            selector: 4,
            ..guide
        }],
        &[],
    ));
    let mut guide = spawn(loader, 0x83_9530, 0x83_f8a8, 3, 0)?;
    if loader.read(0x9_d2c1, 3)? != [2, 0xba, 0x30] {
        return Err(SpriteError::Invalid("changed tour priority"));
    }
    guide.priority_override = Some(3);
    result.push(phase("tour-41-0", 0x41, vec![guide], &[ScriptedMotion]));
    for (at, id) in [
        (0x9_d2f4, "tour-41-1"),
        (0x9_d30e, "tour-41-2"),
        (0x9_d328, "tour-41-3"),
        (0x9_d342, "tour-41-4"),
        (0x9_d35c, "tour-41-5"),
        (0x9_d376, "tour-41-6"),
        (0x9_d390, "tour-41-7"),
    ] {
        let b = loader.read(at, 7)?;
        if b[..3] != [2, 0xed, 3] {
            return Err(SpriteError::Invalid("changed tour motion"));
        }
        for axis in 0..2 {
            guide.position[axis] = u16::try_from(
                i32::from(guide.position[axis])
                    + i32::from(i16::from_le_bytes([b[3 + axis * 2], b[4 + axis * 2]])),
            )
            .map_err(|_| SpriteError::Invalid("tour endpoint overflow"))?;
        }
        guide.position_source = cpu_address(at)?;
        result.push(phase(id, 0x41, vec![guide], &[ScriptedMotion]));
    }
    for (id, map, source) in [
        ("tour-44", 0x44, 0x83_95fd),
        ("tour-42", 0x42, 0x83_9572),
        ("tour-43", 0x43, 0x83_95c0),
    ] {
        let mut guide = spawn(loader, source, 0x83_f8a8, 3, 0)?;
        guide.priority_override = Some(3);
        let mut actors = vec![guide];
        if map == 0x42 {
            let mut object = spawn(loader, 0x83_957c, 0x89_d9f9, 8, 0)?;
            let b = loader.read(0x9_d9fe, 4)?;
            if b[..2] != [2, 0xb2] {
                return Err(SpriteError::Invalid("changed tour object Y"));
            }
            object.position[1] = object.position[1]
                .checked_add_signed(i16::from_le_bytes([b[2], b[3]]))
                .ok_or(SpriteError::Invalid("tour object Y overflow"))?;
            object.position_source = 0x89_d9fe;
            actors[0].tie_rank = 1;
            actors.push(object);
        }
        result.push(phase(id, map, actors, &[ScriptedMotion]));
    }
    let mut guide = spawn(loader, 0x83_9530, 0x83_f8a8, 3, 0)?;
    guide.priority_override = Some(3);
    result.push(phase("tour-control", 0x41, vec![guide], &[]));
    Ok(result)
}

/// One source-scripted C departure segment, not a duration or scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PandoraMotion {
    /// Stable parent phase/segment key.
    pub id: &'static str,
    /// Source actor instance.
    pub actor: u32,
    /// COP39/COP3A source instruction.
    pub source: u32,
    /// Source endpoint before this segment.
    pub from: [u16; 2],
    /// Source endpoint after this segment.
    pub to: [u16; 2],
    /// Source list selector used while moving.
    pub selector: u8,
    /// Native packed motion operand, retained without claiming a portable clock.
    pub motion_operand: u8,
    /// Source COPA7 removal follows the final segment.
    pub removes_actor: bool,
}
pub(super) fn motions(
    loader: &mut Loader<'_>,
    phases: &[PandoraPhase],
) -> Result<Vec<PandoraMotion>, SpriteError> {
    let initial = phases
        .iter()
        .find(|p| p.id == "c-direct")
        .ok_or(SpriteError::Invalid("missing C source phase"))?;
    let mut result = Vec::new();
    for (actor, segments) in [
        (
            0x83_8c14,
            &[
                ("c-fourth-south", 0x8_a56d),
                ("c-fourth-align", 0x8_a578),
                ("c-fourth-exit", 0x8_a583),
            ][..],
        ),
        (
            0x83_8c28,
            &[("c-right-align", 0x8_a247), ("c-right-exit", 0x8_a256)][..],
        ),
        (
            0x83_8c0a,
            &[("c-left-align", 0x8_a3df), ("c-left-exit", 0x8_a3ea)][..],
        ),
        (
            0x83_8c1e,
            &[("c-speaker-align", 0x8_9c16), ("c-speaker-exit", 0x8_9c25)][..],
        ),
    ] {
        let mut position = initial
            .actors
            .iter()
            .find(|a| a.source_id == actor)
            .ok_or(SpriteError::Invalid("missing C source actor"))?
            .position;
        for (index, &(id, at)) in segments.iter().enumerate() {
            let bytes = loader.read(at, 5)?;
            if bytes[0] != 2 || ![0x39, 0x3a].contains(&bytes[1]) {
                return Err(SpriteError::Invalid("changed C departure motion"));
            }
            let mut to = position;
            let axis = usize::from(bytes[1] == 0x3a);
            to[axis] = u16::from(bytes[4]) * 16 + if axis == 0 { 8 } else { 0 };
            result.push(PandoraMotion {
                id,
                actor,
                source: cpu_address(at)?,
                from: position,
                to,
                selector: bytes[2],
                motion_operand: bytes[3],
                removes_actor: index + 1 == segments.len(),
            });
            position = to;
        }
    }
    // Pin the bounded control/pose/removal programs independently of any capture.
    for (at, len) in [
        (0x8_9a6a, 0x2ca),
        (0x8_a1a4, 0xd5),
        (0x8_a321, 0xf0),
        (0x8_a4eb, 0xa6),
        (0x8_acf5, 0xca),
        (0x8_aeb8, 0x7b),
        (0x9_d2ad, 0x103),
        (0x9_dc78, 0x93),
    ] {
        loader.read(at, len)?;
    }
    Ok(result)
}
