//! Explicit semantic room preview over immutable data; not classic frame fidelity.
use crate::conversation::{self, Active, ConversationSpec, DialogueOutput};
use crate::events::EventFlags;
pub use crate::pandora::{Invocation, PandoraText, RequestPages};
use crate::transition::Transition;
use crate::{
    AnimationFrame, AnimationState, Direction, FrameInput, Room, Unqualified, WalkingState,
};
use alloc::{vec, vec::Vec};
use core::fmt;

/// Semantic profile version; v9 adds conversation ownership and bounded exterior progression.
pub const PROFILE_VERSION: u8 = 9;

/// Only supported policy. Doorway updates are logical, not reference video frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Endpoint-calibrated doorway pacing, never classic video-frame fidelity.
    SemanticPreview,
}

/// Caller-authenticated immutable source identity, included in every snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataIdentity {
    /// SHA-256 of the normalized Japanese source image.
    pub rom_sha256: [u8; 32],
    /// SHA-256 of the ordered compiled collision/exit/profile data.
    pub content_sha256: [u8; 32],
}

/// Ordered raw exit metadata supplied by the bounded asset decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit(pub [u8; 12]);
impl Exit {
    fn coarse(self, x: u16, y: u16) -> bool {
        let tile = |v: u16| ((v >> 4) & 255) as u8;
        tile(x).wrapping_sub(self.0[0]) < self.0[2] && tile(y).wrapping_sub(self.0[1]) < self.0[3]
    }
    pub(crate) fn fine(self, x: u16, y: u16) -> bool {
        self.0[2] != 0
            && self.0[3] != 0
            && x.wrapping_sub(u16::from(self.0[0]) * 16) < u16::from(self.0[2]) * 16 - 15
            && y.wrapping_sub(u16::from(self.0[1]) * 16) < u16::from(self.0[3]) * 16 - 15
    }
}

/// Source-compiled fresh bedroom profile, distinct from a reloaded bedroom.
#[derive(Debug)]
pub struct NewGameData {
    /// Frozen post-intro runtime overlay, compiled over the static ROM grid.
    pub bedroom: Room,
    /// Queue-derived player anchor after explicit semantic intro completion.
    pub position: (u16, u16),
}

/// One authenticated source-compiled room and its complete ordered exit list.
/// Collision must include qualified fresh occupancy; raw flags are not erased.
#[derive(Debug)]
pub struct HouseRoom {
    /// Source map ID, one of B,C,D,F,10,11.
    pub map_id: u16,
    /// Immutable collision profile, compiled from ROM (not a runtime capture).
    pub collision: Room,
    /// Complete source order, including closed/unqualified outgoing boundaries.
    pub exits: Vec<Exit>,
}

/// Immutable source data. Extraction, occupancy admission and hashing are external.
#[derive(Debug)]
pub struct GameData {
    rooms: Vec<HouseRoom>,
    identity: DataIdentity,
    new_game: NewGameData,
    open_corridor: Option<Room>,
    progression: Option<ProgressionData>,
}
#[derive(Debug)]
struct ProgressionData {
    conversation: ConversationSpec,
    open_d: Room,
    exterior: Room,
}
impl GameData {
    /// Compatible F/10-only constructor. Does not enable door interaction.
    /// `new_game` is compiled from fresh bootstrap sources, never supplied SRAM.
    /// # Errors
    /// Rejects dimensions, bootstrap anchor, or changed first doorway records.
    pub fn new(
        rooms: [Room; 2],
        exits: [Vec<Exit>; 2],
        identity: DataIdentity,
        new_game: NewGameData,
    ) -> Result<Self, SliceError> {
        let [f, ten] = rooms;
        let [f_exits, ten_exits] = exits;
        let profiles = vec![
            HouseRoom {
                map_id: 15,
                collision: f,
                exits: f_exits,
            },
            HouseRoom {
                map_id: 16,
                collision: ten,
                exits: ten_exits,
            },
        ];
        if profiles
            .iter()
            .any(|p| p.exits.first() != crate::house::exits(p.map_id).and_then(|e| e.first()))
        {
            return Err(SliceError::Data);
        }
        Self::build(profiles, identity, new_game, None)
    }
    /// Six fresh room profiles in source ID order B,C,D,F,10,11.
    ///
    /// The adapter must authenticate the complete source recipe, including static
    /// occupancy, fresh flags ($0020/$00FB only), collision attributes and visual
    /// descriptors. Identity covers these inputs and this semantic policy. D's
    /// hidden exterior blocker must be included; wandering AI is not simulated.
    /// Closed C cells (8,19)/(8,20) must be $1CF2/$1CF3. The core compiles an
    /// immutable open-C variant with $1CF6/$00F7; rendering uses the same IDs.
    /// The variant is selected by state, not by mutating caller-owned data.
    /// # Errors
    /// Rejects missing/reordered rooms, changed exits, dimensions or door cells.
    pub fn new_house(
        rooms: [HouseRoom; 6],
        identity: DataIdentity,
        new_game: NewGameData,
    ) -> Result<Self, SliceError> {
        for (room, id) in rooms.iter().zip(crate::house::MAPS) {
            if room.map_id != id || Some(room.exits.as_slice()) != crate::house::exits(id) {
                return Err(SliceError::Data);
            }
        }
        let c = &rooms[1].collision;
        if c.cells().get(19 * 32 + 8) != Some(&0x1cf2)
            || c.cells().get(20 * 32 + 8) != Some(&0x1cf3)
        {
            return Err(SliceError::Data);
        }
        let mut open = c.clone();
        open.replace_cell(19 * 32 + 8, 0x1cf6);
        open.replace_cell(20 * 32 + 8, 0x00f7);
        Self::build(Vec::from(rooms), identity, new_game, Some(open))
    }
    fn build(
        rooms: Vec<HouseRoom>,
        identity: DataIdentity,
        new_game: NewGameData,
        open_corridor: Option<Room>,
    ) -> Result<Self, SliceError> {
        if new_game.position != (304, 112)
            || rooms
                .iter()
                .map(|p| &p.collision)
                .chain(core::iter::once(&new_game.bedroom))
                .any(|r| r.width() != 32 || r.height() != 64)
        {
            return Err(SliceError::Data);
        }
        Ok(Self {
            rooms,
            identity,
            new_game,
            open_corridor,
            progression: None,
        })
    }
    /// Extend a six-room house with the source-qualified B conversation and A landing.
    ///
    /// `identity` must authenticate the ordered conversation keys, full A grid,
    /// fixed sample admission policy and existing house data together. The ROM
    /// identity must remain unchanged. No A exits or actors are admitted.
    /// The source D cell $8592 is privately cloned to $0592 for post-grant reloads.
    /// A is 64x80 cells; every ordinary collision sample is restricted to the
    /// half-open source-qualified halo [29,47,36,53], independently of map bounds.
    /// # Errors
    /// Rejects legacy/twice-extended data, changed gate/dimensions/ROM, or non-open
    /// material/occupancy in the 42-cell admitted exterior halo.
    pub fn with_progression(
        mut self,
        conversation: ConversationSpec,
        exterior: Room,
        identity: DataIdentity,
    ) -> Result<Self, SliceError> {
        if self.rooms.len() != 6
            || !self.door_interaction()
            || self.progression.is_some()
            || identity.rom_sha256 != self.identity.rom_sha256
            || exterior.width() != 64
            || exterior.height() != 80
            || self.room(13, false, false)?.cells()[1415] != 0x8592
        {
            return Err(SliceError::Data);
        }
        for y in 47..53 {
            for x in 29..36 {
                let raw = exterior.cells()[y * 64 + x];
                if raw & 0x8000 != 0 || !matches!((raw >> 9) & 31, 0 | 22) {
                    return Err(SliceError::Data);
                }
            }
        }
        let exterior = exterior
            .with_sample_halo([29, 47, 36, 53])
            .map_err(|_| SliceError::Data)?;
        let mut open_d = self.room(13, false, false)?.clone();
        open_d.replace_cell(1415, 0x592);
        self.progression = Some(ProgressionData {
            conversation,
            open_d,
            exterior,
        });
        self.identity = identity;
        Ok(self)
    }
    /// Capability, not current target availability.
    #[must_use]
    pub fn conversation_progression(&self) -> bool {
        self.progression.is_some()
    }
    /// Capability, not current target availability. False for the legacy wrapper.
    #[must_use]
    pub fn door_interaction(&self) -> bool {
        self.open_corridor.is_some()
    }
    fn room(&self, id: u16, fresh: bool, door_open: bool) -> Result<&Room, SliceError> {
        if fresh {
            return if id == 15 {
                Ok(&self.new_game.bedroom)
            } else {
                Err(SliceError::Data)
            };
        }
        if id == 12 && door_open {
            return self.open_corridor.as_ref().ok_or(SliceError::Data);
        }
        if id == 10 {
            return self
                .progression
                .as_ref()
                .map(|p| &p.exterior)
                .ok_or(SliceError::Data);
        }
        self.rooms
            .iter()
            .find(|p| p.map_id == id)
            .map(|p| &p.collision)
            .ok_or(SliceError::Data)
    }
    fn exit(&self, id: u16, position: (u16, u16)) -> Option<(usize, Exit)> {
        let origin = (position.0.wrapping_sub(8), position.1.wrapping_sub(16));
        self.rooms
            .iter()
            .find(|p| p.map_id == id)?
            .exits
            .iter()
            .copied()
            .enumerate()
            .find(|(_, e)| e.coarse(origin.0, origin.1))
            .filter(|(_, e)| e.fine(origin.0, origin.1))
    }
}

/// Failure leaves the whole state unchanged; no unsupported behavior becomes a wall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceError {
    /// Immutable data does not match this profile.
    Data,
    /// Walking component left its qualified scope.
    Walking(Unqualified),
    /// An exit or handoff is outside the supported semantic doorways.
    Exit,
    /// No admitted target/wait, wrong dialogue action, or choice outside 0..2.
    Interaction,
    /// Snapshot version, source identity, fields or consistency were invalid.
    Snapshot,
    /// Logical tick counter overflowed.
    TickOverflow,
}
impl core::error::Error for SliceError {}
impl fmt::Display for SliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "room preview: {self:?}")
    }
}

/// Semantic output phase, explicitly distinct from the native scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The B conversation owns control; movement input is discarded.
    Dialogue,
    /// Ordinary qualified walking owns control.
    Walking,
    /// Logical unchecked departure translation owns control.
    Departing,
    /// Logical destination arrival translation owns control.
    Arriving,
}

/// Deterministic render/inspection result. No graphics or device commands enter state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOutput {
    /// Number of logical preview updates since reset.
    pub tick: u64,
    /// Current admitted source map ID.
    pub map_id: u16,
    /// Player anchor in map pixels.
    pub position: (u16, u16),
    /// Owner of movement for the next update.
    pub phase: Phase,
    /// Ordinary sprite asset key; doorway poses use explicit standing policy.
    pub animation: AnimationFrame,
}

/// Minimal mutable slice state. No clock, filesystem, original CPU or RNG usage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Independent source/capability/load-time predicates, not mutually exclusive phases.
pub struct GameState {
    identity: DataIdentity,
    tick: u64,
    map_id: u16,
    walking: WalkingState,
    animation: AnimationState,
    transition: Option<Transition>,
    fresh_bedroom: bool,
    wooden_door_open: bool,
    flags: EventFlags,
    dialogue: Option<Active>,
    d_open_loaded: bool,
    progression_enabled: bool,
}
impl GameState {
    /// Starts the authenticated ordinary bedroom checkpoint. Policy opt-in is explicit.
    #[must_use]
    pub fn new(data: &GameData, _policy: Policy) -> Self {
        Self {
            identity: data.identity,
            tick: 0,
            map_id: 15,
            walking: WalkingState::new(472, 176),
            animation: AnimationState::standing(Direction::Down),
            transition: None,
            fresh_bedroom: false,
            wooden_door_open: false,
            flags: conversation::initial_flags(false),
            dialogue: None,
            d_open_loaded: false,
            progression_enabled: data.conversation_progression(),
        }
    }
    /// Starts the source-compiled fresh bedroom after semantic intro completion.
    /// Default name only; no menu/dialogue presentation or native intro scheduler.
    /// No SRAM, snapshot, original CPU, host clock or RNG is consumed.
    #[must_use]
    pub fn new_game(data: &GameData, _policy: Policy) -> Self {
        let (x, y) = data.new_game.position;
        Self {
            identity: data.identity,
            tick: 0,
            map_id: 15,
            walking: WalkingState::new(x, y),
            animation: AnimationState::standing(Direction::Down),
            transition: None,
            fresh_bedroom: true,
            wooden_door_open: false,
            flags: conversation::initial_flags(false),
            dialogue: None,
            d_open_loaded: false,
            progression_enabled: data.conversation_progression(),
        }
    }
    /// Advances one walking frame or one *logical* doorway update. While the
    /// doorway owns control, inputs are discarded rather than buffered. Completion
    /// begins a fresh input-admission epoch at the measured arrival endpoint.
    /// This history reset is preview policy, not native admission qualification.
    /// # Errors
    /// Rejects incompatible data, unsupported walking/exit behavior, or overflow;
    /// failure is atomic and the caller decides how to report/pause it.
    pub fn step(&mut self, data: &GameData, input: FrameInput) -> Result<FrameOutput, SliceError> {
        if self.identity != data.identity
            || self.progression_enabled != data.conversation_progression()
        {
            return Err(SliceError::Data);
        }
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        if next.dialogue.is_some() {
            // Explicit semantic policy: keep standing pose/history, discard inputs.
        } else if let Some(mut transition) = next.transition {
            transition.advance();
            next.animation = AnimationState::standing(transition.direction());
            if next.map_id != transition.map_id() {
                // Membership is selected only at reconstruction, never on a live flag write.
                next.d_open_loaded =
                    transition.map_id() == 13 && (next.flags.contains(0x26) == Ok(true));
            }
            next.map_id = transition.map_id();
            if next.map_id != transition.source_map() {
                next.fresh_bedroom = false;
            }
            if transition.complete() {
                let (x, y) = transition.position();
                next.walking = WalkingState::new(x, y);
                next.transition = None;
            } else {
                next.transition = Some(transition);
            }
        } else {
            let room = next.current_room(data)?;
            next.walking
                .step(room, input)
                .map_err(SliceError::Walking)?;
            next.animation.advance(next.walking.active_direction());
            if let Some((index, _)) = data.exit(next.map_id, next.walking.position()) {
                let transition = Transition::select(next.map_id, index, next.walking.position())
                    .ok_or(SliceError::Exit)?;
                if data
                    .room(transition.source_map(), false, next.wooden_door_open)
                    .is_err()
                    || (!data.door_interaction() && transition.route() > 1)
                    || (transition.exterior()
                        && (!next.d_open_loaded || !data.conversation_progression()))
                    || (next.map_id == 12 && index == 2 && !next.wooden_door_open)
                    || next.walking.active_direction() != Some(transition.direction())
                {
                    return Err(SliceError::Exit);
                }
                // Walking no longer owns control; discard its unused history.
                // A canonical anchor keeps the private representation stable.
                let (x, y) = transition.handoff();
                next.walking = WalkingState::new(x, y);
                next.animation = AnimationState::standing(transition.direction());
                next.transition = Some(transition);
            }
        }
        let output = next.output();
        *self = next;
        Ok(output)
    }
    /// Final shared-sheet mutation, not event0026 or a native saved-game flag.
    #[must_use]
    pub const fn wooden_door_open(&self) -> bool {
        self.wooden_door_open
    }

    /// Commit the bounded C/B wooden-door action in one successful logical tick.
    ///
    /// Only C at (136,352), facing Up, with the door closed and no transition is
    /// admitted. The host submits a one-shot action after releasing direction.
    /// Native A takes control and schedules intermediate tiles/waits; this policy
    /// commits only final $F6/$F7 and cancels delayed walking without requiring an
    /// extra release tick. The input-history reset is semantic, not native timing.
    /// In progression-enabled data, B at (120,128), facing Up starts the fixed
    /// resident conversation instead. Entry text is not a progression action.
    /// # Errors
    /// Wrong data, overflow, already-open door or any other target reject atomically.
    pub fn interact(&mut self, data: &GameData) -> Result<FrameOutput, SliceError> {
        if self.identity != data.identity
            || self.progression_enabled != data.conversation_progression()
        {
            return Err(SliceError::Data);
        }
        if self.dialogue.is_some() {
            return Err(SliceError::Interaction);
        }
        if self.map_id == 11
            && self.walking.position() == (120, 128)
            && self.animation.facing() == Direction::Up
            && self.transition.is_none()
            && !self.fresh_bedroom
            && self.wooden_door_open
        {
            let spec = &data
                .progression
                .as_ref()
                .ok_or(SliceError::Interaction)?
                .conversation;
            let mut next = self.clone();
            next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
            next.dialogue = Some(spec.begin(&mut next.flags));
            next.walking = WalkingState::new(120, 128);
            next.animation = AnimationState::standing(Direction::Up);
            *self = next;
            return Ok(self.output());
        }
        if !data.door_interaction()
            || self.map_id != 12
            || self.fresh_bedroom
            || self.transition.is_some()
            || self.wooden_door_open
            || self.walking.position() != (136, 352)
            || self.animation.facing() != Direction::Up
        {
            return Err(SliceError::Interaction);
        }
        let tick = self.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        self.tick = tick;
        self.wooden_door_open = true;
        self.walking = WalkingState::new(136, 352);
        self.animation = AnimationState::standing(Direction::Up);
        Ok(self.output())
    }
    /// Canonical source flags: initial $0020/$00FB, with only $0026 mutable here.
    #[must_use]
    pub const fn event_flags(&self) -> &EventFlags {
        &self.flags
    }

    /// Current immutable collision variant, including load-time D gate selection.
    /// # Errors
    /// Rejects incompatible data or unavailable profiles.
    pub fn current_room<'a>(&self, data: &'a GameData) -> Result<&'a Room, SliceError> {
        if self.identity != data.identity
            || self.progression_enabled != data.conversation_progression()
        {
            return Err(SliceError::Data);
        }
        if self.map_id == 13 && self.d_open_loaded {
            return data
                .progression
                .as_ref()
                .map(|p| &p.open_d)
                .ok_or(SliceError::Data);
        }
        data.room(self.map_id, self.fresh_bedroom, self.wooden_door_open)
    }
    /// Inspect the current request and page/choice wait without advancing a tick.
    /// # Errors
    /// Rejects incompatible data or invalid ownership.
    pub fn dialogue(&self, data: &GameData) -> Result<Option<DialogueOutput>, SliceError> {
        if self.identity != data.identity
            || self.progression_enabled != data.conversation_progression()
        {
            return Err(SliceError::Data);
        }
        self.dialogue
            .map(|active| {
                data.progression
                    .as_ref()
                    .ok_or(SliceError::Data)?
                    .conversation
                    .output(active)
            })
            .transpose()
    }
    /// Acknowledge exactly one real page in one atomic tick; choices are rejected.
    /// # Errors
    /// Rejects wrong data, absence of a page wait or tick overflow atomically.
    pub fn acknowledge(&mut self, data: &GameData) -> Result<FrameOutput, SliceError> {
        self.dialogue_action(data, None)
    }
    /// Select explicit result 0=cancel or 1/2=option in one atomic tick.
    /// No frontend selection state or implicit/default result enters simulation.
    /// # Errors
    /// Rejects wrong data, absence of a choice, invalid result or tick overflow atomically.
    pub fn choose(&mut self, data: &GameData, selection: u8) -> Result<FrameOutput, SliceError> {
        self.dialogue_action(data, Some(selection))
    }
    fn dialogue_action(
        &mut self,
        data: &GameData,
        choice: Option<u8>,
    ) -> Result<FrameOutput, SliceError> {
        self.dialogue(data)?;
        let active = self.dialogue.ok_or(SliceError::Interaction)?;
        let spec = &data
            .progression
            .as_ref()
            .ok_or(SliceError::Interaction)?
            .conversation;
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        next.dialogue = if let Some(selection) = choice {
            Some(spec.choose(active, selection, &mut next.flags)?)
        } else {
            spec.acknowledge(active, &mut next.flags)?
        };
        // Every active/finished conversation keeps one canonical frozen pose/history.
        next.walking = WalkingState::new(120, 128);
        next.animation = AnimationState::standing(Direction::Up);
        *self = next;
        Ok(self.output())
    }
    /// Current stable semantic result, without advancing simulation.
    #[must_use]
    pub fn output(&self) -> FrameOutput {
        FrameOutput {
            tick: self.tick,
            animation: self.animation.frame(),
            map_id: self.map_id,
            position: self
                .transition
                .map_or_else(|| self.walking.position(), Transition::position),
            phase: self.transition.map_or(
                if self.dialogue.is_some() {
                    Phase::Dialogue
                } else {
                    Phase::Walking
                },
                |t| {
                    if t.elapsed() <= 17 {
                        Phase::Departing
                    } else {
                        Phase::Arriving
                    }
                },
            ),
        }
    }
    /// Fixed 181-byte little-endian snapshot; immutable content is identified, not embedded.
    /// Bytes include profile/schema and RNG-policy versions (0 means no RNG).
    #[must_use]
    pub fn snapshot(&self) -> Vec<u8> {
        let mut bytes = vec![b'R', b'S', b'L', b'C', 1, PROFILE_VERSION, 0, 1];
        bytes.extend(self.identity.rom_sha256);
        bytes.extend(self.identity.content_sha256);
        bytes.extend(self.tick.to_le_bytes());
        bytes.extend(self.map_id.to_le_bytes());
        bytes.push(self.transition.map_or(255, Transition::route));
        // No walking component is serialized while a doorway owns control.
        // Erasing its marker cannot turn a transition into a valid walking state.
        bytes.extend(if self.transition.is_some() || self.dialogue.is_some() {
            [0; crate::SNAPSHOT_SIZE]
        } else {
            self.walking.encode_snapshot()
        });
        bytes.push(u8::from(self.fresh_bedroom));
        bytes.extend([
            self.animation.facing() as u8,
            u8::from(self.animation.is_walking()),
            self.animation.phase(),
        ]);
        if let Some(t) = self.transition {
            bytes.push(t.elapsed());
            bytes.extend(t.handoff().0.to_le_bytes());
            bytes.extend(t.handoff().1.to_le_bytes());
        } else {
            bytes.extend([0; 5]);
        }
        bytes.push(u8::from(self.wooden_door_open));
        bytes.extend(self.flags.bytes());
        bytes.push(u8::from(self.d_open_loaded));
        bytes.extend(self.dialogue.map_or(0, |a| a.request).to_le_bytes());
        bytes.extend(
            self.dialogue
                .map_or(0, |a| a.cursor.position())
                .to_le_bytes(),
        );
        bytes.push(u8::from(self.progression_enabled));
        bytes
    }
    /// Restores only a compatible, internally valid snapshot, without data or I/O.
    /// # Errors
    /// Rejects versions, identities, malformed walking state, or invalid transition ownership.
    #[allow(clippy::too_many_lines)] // Keep the coupled ownership/schema checks together.
    pub fn restore(data: &GameData, bytes: &[u8]) -> Result<Self, SliceError> {
        if bytes.len() != 181
            || bytes[..8] != [b'R', b'S', b'L', b'C', 1, PROFILE_VERSION, 0, 1]
            || bytes[8..40] != data.identity.rom_sha256
            || bytes[40..72] != data.identity.content_sha256
        {
            return Err(SliceError::Snapshot);
        }
        let tick = u64::from_le_bytes(bytes[72..80].try_into().map_err(|_| SliceError::Snapshot)?);
        let map_id = u16::from_le_bytes([bytes[80], bytes[81]]);
        let fresh_bedroom = match bytes[99] {
            0 => false,
            1 if map_id == 15 => true,
            _ => return Err(SliceError::Snapshot),
        };
        let wooden_door_open = match bytes[108] {
            0 if map_id != 11 => false,
            1 if data.door_interaction() && !fresh_bedroom => true,
            _ => return Err(SliceError::Snapshot),
        };
        data.room(map_id, fresh_bedroom, wooden_door_open)
            .map_err(|_| SliceError::Snapshot)?;
        let progression_enabled = match bytes[180] {
            0 => false,
            1 => true,
            _ => return Err(SliceError::Snapshot),
        };
        if progression_enabled != data.conversation_progression() {
            return Err(SliceError::Snapshot);
        }
        let flags = EventFlags::new(
            bytes[109..173]
                .try_into()
                .map_err(|_| SliceError::Snapshot)?,
        );
        let granted = flags.contains(0x26) == Ok(true);
        if flags != conversation::initial_flags(granted)
            || (granted && (!progression_enabled || !wooden_door_open || fresh_bedroom))
            || (map_id == 10 && !granted)
        {
            return Err(SliceError::Snapshot);
        }
        let d_open_loaded = match bytes[173] {
            0 => false,
            1 if map_id == 13 && granted && progression_enabled => true,
            _ => return Err(SliceError::Snapshot),
        };
        let request = u32::from_le_bytes(
            bytes[174..178]
                .try_into()
                .map_err(|_| SliceError::Snapshot)?,
        );
        let cursor = u16::from_le_bytes([bytes[178], bytes[179]]);
        let dialogue = if request == 0 {
            if cursor != 0 {
                return Err(SliceError::Snapshot);
            }
            None
        } else {
            if map_id != 11 || !wooden_door_open || fresh_bedroom || bytes[82] != 255 {
                return Err(SliceError::Snapshot);
            }
            Some(
                data.progression
                    .as_ref()
                    .ok_or(SliceError::Snapshot)?
                    .conversation
                    .restore(request, cursor, &flags)?,
            )
        };
        let transition = if bytes[82] == 255 {
            if bytes[103..108] != [0; 5] {
                return Err(SliceError::Snapshot);
            }
            None
        } else {
            let handoff = (
                u16::from_le_bytes([bytes[104], bytes[105]]),
                u16::from_le_bytes([bytes[106], bytes[107]]),
            );
            let t =
                Transition::restore(bytes[82], bytes[103], handoff).ok_or(SliceError::Snapshot)?;
            if (!data.door_interaction() && t.route() > 1)
                || (t.exterior()
                    && (!progression_enabled || !granted || (map_id == 13 && !d_open_loaded)))
                // A retained arrival proves reconstruction already selected this
                // profile with the same flags; unlike an old loaded D, it cannot
                // legitimately preserve a pre-grant gate.
                || (t.map_id() == 13 && t.source_map() != 13 && d_open_loaded != granted)
                || (!wooden_door_open
                    && (t.source_map() == 11 || (t.source_map() == 12 && t.index() == 2)))
                || data.exit(t.source_map(), handoff).map(|(i, _)| i) != Some(t.index())
            {
                return Err(SliceError::Snapshot);
            }
            Some(t)
        };
        let walking = if dialogue.is_some() {
            if bytes[83..99] != [0; crate::SNAPSHOT_SIZE] {
                return Err(SliceError::Snapshot);
            }
            WalkingState::new(120, 128)
        } else if let Some(t) = transition {
            if t.map_id() != map_id
                || bytes[83..99] != [0; crate::SNAPSHOT_SIZE]
                || (fresh_bedroom && t.source_map() != 15)
            {
                return Err(SliceError::Snapshot);
            }
            let (x, y) = t.handoff();
            WalkingState::new(x, y)
        } else {
            let walking = WalkingState::decode_snapshot(
                data.room(map_id, fresh_bedroom, wooden_door_open)?,
                &bytes[83..99],
            )
            .map_err(|_| SliceError::Snapshot)?;
            if data.exit(map_id, walking.position()).is_some() {
                return Err(SliceError::Snapshot);
            }
            walking
        };
        let facing = match bytes[100] {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            3 => Direction::Right,
            _ => return Err(SliceError::Snapshot),
        };
        let is_walking = match bytes[101] {
            0 => false,
            1 => true,
            _ => return Err(SliceError::Snapshot),
        };
        let animation = AnimationState::from_parts(facing, is_walking, bytes[102])
            .ok_or(SliceError::Snapshot)?;
        let coherent = if dialogue.is_some() {
            animation == AnimationState::standing(Direction::Up)
        } else if let Some(t) = transition {
            !is_walking && facing == t.direction()
        } else if let Some(active) = walking.active_direction() {
            // Animation and horizontal movement share a 54-tick cycle. Vertical
            // movement keeps parity across animation wraps (phase zero may be
            // setup or an even tick at a later wrap).
            let phase_matches = if active.horizontal() {
                walking.phase() == animation.phase()
            } else {
                match walking.phase() {
                    0 => animation.phase() == 0,
                    1 => animation.phase() % 2 == 1,
                    2 => animation.phase() % 2 == 0,
                    _ => false,
                }
            };
            is_walking && facing == active && phase_matches
        } else {
            !is_walking
        };
        if !coherent {
            return Err(SliceError::Snapshot);
        }
        Ok(Self {
            identity: data.identity,
            tick,
            map_id,
            walking,
            animation,
            transition,
            fresh_bedroom,
            wooden_door_open,
            flags,
            dialogue,
            d_open_loaded,
            progression_enabled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Direction;
    fn synthetic_data() -> GameData {
        let room = || Room::new(32, 64, vec![0; 2048]).unwrap();
        let e = Exit([24, 12, 1, 2, 16, 0, 0, 5, 128, 1, 80, 1]);
        GameData::new(
            [room(), room()],
            [
                vec![e],
                vec![Exit([24, 20, 1, 1, 15, 0, 0, 6, 128, 1, 176, 0])],
            ],
            DataIdentity {
                rom_sha256: [1; 32],
                content_sha256: [2; 32],
            },
            NewGameData {
                bedroom: room(),
                position: (304, 112),
            },
        )
        .unwrap()
    }
    #[test]
    fn new_game_uses_compiled_fresh_initialization_not_saved_checkpoint() {
        let data = synthetic_data();
        let mut state = GameState::new_game(&data, Policy::SemanticPreview);
        assert_eq!(
            state.output(),
            FrameOutput {
                tick: 0,
                map_id: 15,
                position: (304, 112),
                animation: AnimationState::standing(Direction::Down).frame(),
                phase: Phase::Walking
            }
        );
        assert_ne!(state, GameState::new(&data, Policy::SemanticPreview));
        assert_eq!(state.walking.last_activation_direction(), None);
        assert_eq!(state.walking.onset_remaining(), 0);
        for direction in (0..20)
            .map(|_| Some(Direction::Right))
            .chain((0..80).map(|_| None))
            .chain((0..20).map(|_| Some(Direction::Down)))
            .chain((0..80).map(|_| None))
        {
            let mut restored = GameState::restore(&data, &state.snapshot()).unwrap();
            let input = FrameInput { direction };
            assert_eq!(state.step(&data, input), restored.step(&data, input));
            assert_eq!(state, restored);
        }
        assert_eq!(state.output().position, (332, 140));
        assert_eq!(
            GameState::new_game(&data, Policy::SemanticPreview)
                .output()
                .tick,
            0
        );
    }
    #[test]
    fn fresh_grid_is_not_reused_after_loading_and_snapshot_binds_its_phase() {
        let mut data = synthetic_data();
        data.new_game.bedroom = Room::new(32, 64, vec![14 << 9; 2048]).unwrap();
        let mut fresh = GameState::new_game(&data, Policy::SemanticPreview);
        for _ in 0..3 {
            fresh
                .step(
                    &data,
                    FrameInput {
                        direction: Some(Direction::Right),
                    },
                )
                .unwrap();
        }
        assert_eq!(fresh.output().position, (304, 112));
        assert_eq!(fresh.snapshot()[99], 1);
        assert_eq!(GameState::restore(&data, &fresh.snapshot()).unwrap(), fresh);
        // Construct a synthetic fresh handoff, then check every logical phase.
        data.new_game.bedroom = Room::new(32, 64, vec![0; 2048]).unwrap();
        fresh = GameState::new_game(&data, Policy::SemanticPreview);
        fresh.walking = WalkingState::new(392, 205);
        for _ in 0..4 {
            fresh
                .step(
                    &data,
                    FrameInput {
                        direction: Some(Direction::Down),
                    },
                )
                .unwrap();
        }
        assert_eq!(fresh.output().position, (392, 208));
        for elapsed in 0..35 {
            let bytes = fresh.snapshot();
            assert_eq!(GameState::restore(&data, &bytes).unwrap(), fresh);
            assert_eq!(bytes[99], u8::from(elapsed <= 17));
            let mut erased = bytes.clone();
            erased[82] = 255;
            assert!(GameState::restore(&data, &erased).is_err());
            let mut invalid = bytes;
            invalid[99] = if elapsed <= 17 { 2 } else { 1 };
            assert!(GameState::restore(&data, &invalid).is_err());
            fresh.step(&data, FrameInput::default()).unwrap();
        }
        assert!(!fresh.fresh_bedroom);
        fresh.map_id = 15;
        assert_eq!(
            data.room(15, fresh.fresh_bedroom, false).unwrap(),
            &data.rooms[0].collision
        );
    }
    #[test]
    fn synthetic_replay_snapshots_resume_at_every_logical_step() {
        let data = synthetic_data();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        let inputs = (0..56)
            .map(|_| Some(Direction::Left))
            .chain((0..24).map(|_| Some(Direction::Down)))
            .chain((0..35).map(|_| None));
        for input in inputs {
            let mut restored = GameState::restore(&data, &state.snapshot()).unwrap();
            assert_eq!(
                state.step(&data, FrameInput { direction: input }),
                restored.step(&data, FrameInput { direction: input })
            );
            assert_eq!(state.snapshot(), restored.snapshot());
        }
        assert_eq!(
            state.output(),
            FrameOutput {
                tick: 115,
                map_id: 16,
                position: (392, 353),
                animation: AnimationState::standing(Direction::Down).frame(),
                phase: Phase::Walking
            }
        );
    }
    #[test]
    fn reverse_route_restores_ownership_and_replays_every_snapshot() {
        let mut data = synthetic_data();
        let mut cells = vec![0; 2048];
        cells[19 * 32 + 24] = 14 << 9; // clamps the last Up step onto Y336
        data.rooms[1].collision = Room::new(32, 64, cells).unwrap();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        let inputs = (0..56)
            .map(|_| Some(Direction::Left))
            .chain((0..24).map(|_| Some(Direction::Down)))
            .chain((0..35).map(|_| None))
            .chain((0..14).map(|_| Some(Direction::Up)))
            .chain((0..35).map(|_| None));
        for direction in inputs {
            let mut restored = GameState::restore(&data, &state.snapshot()).unwrap();
            let input = FrameInput { direction };
            assert_eq!(state.step(&data, input), restored.step(&data, input));
            assert_eq!(state, restored);
            if state.transition.is_some() {
                let mut erased = state.snapshot();
                erased[82] = 255;
                assert!(GameState::restore(&data, &erased).is_err());
            }
        }
        assert_eq!(
            state.output(),
            FrameOutput {
                tick: 164,
                map_id: 15,
                position: (392, 191),
                animation: AnimationState::standing(Direction::Up).frame(),
                phase: Phase::Walking
            }
        );
        assert_eq!(state.walking.last_activation_direction(), None);
        // Wrong source map/direction cannot be smuggled into a return snapshot.
        state.map_id = 16;
        state.walking = WalkingState::new(392, 339);
        for _ in 0..4 {
            state
                .step(
                    &data,
                    FrameInput {
                        direction: Some(Direction::Up),
                    },
                )
                .unwrap();
        }
        assert!(state.transition.is_some());
        let mut wrong_route = state.snapshot();
        wrong_route[82] = 0;
        assert!(GameState::restore(&data, &wrong_route).is_err());
    }
    #[test]
    fn failure_is_atomic_and_snapshots_bind_versions_and_data() {
        let data = synthetic_data();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        for input in [Some(Direction::Left), None] {
            state.step(&data, FrameInput { direction: input }).unwrap();
        }
        let before = state.snapshot();
        assert!(state
            .step(
                &data,
                FrameInput {
                    direction: Some(Direction::Left)
                }
            )
            .is_err());
        assert_eq!(state.snapshot(), before);
        for at in [0, 4, 5, 6, 7, 8, 40, 80, 82] {
            let mut corrupt = before.clone();
            corrupt[at] ^= 0x80;
            assert!(GameState::restore(&data, &corrupt).is_err(), "field {at}");
        }
        for len in 0..before.len() {
            assert!(GameState::restore(&data, &before[..len]).is_err());
        }
        let mut trailing = before.clone();
        trailing.push(0);
        assert!(GameState::restore(&data, &trailing).is_err());
        let mut other = synthetic_data();
        other.identity.content_sha256[0] ^= 1;
        assert!(state.step(&other, FrameInput::default()).is_err());
        assert!(GameState::restore(&other, &before).is_err());
    }
    #[test]
    fn ordered_exit_selection_does_not_fall_through_failed_fine_match() {
        let mut data = synthetic_data();
        data.rooms[0].exits = vec![
            Exit([24, 12, 1, 2, 16, 0, 0, 5, 128, 1, 80, 1]),
            Exit([24, 12, 2, 2, 16, 0, 0, 5, 128, 1, 80, 1]),
        ];
        assert!(data.exit(15, (393, 209)).is_none());
        assert_eq!(data.exit(15, (392, 209)).unwrap().0, 0);
    }
    #[test]
    fn restore_rejects_erased_transition_ownership() {
        let data = synthetic_data();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        for direction in (0..56)
            .map(|_| Direction::Left)
            .chain((0..24).map(|_| Direction::Down))
        {
            state
                .step(
                    &data,
                    FrameInput {
                        direction: Some(direction),
                    },
                )
                .unwrap();
        }
        let mut bytes = state.snapshot();
        bytes[82] = 255;
        assert!(GameState::restore(&data, &bytes).is_err());
    }

    #[test]
    fn unsupported_handoffs_and_overflow_are_atomic() {
        let mut data = synthetic_data();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        data.rooms[0]
            .exits
            .push(Exit([29, 10, 1, 1, 99, 0, 0, 0, 0, 0, 0, 0]));
        let before = state.snapshot();
        assert_eq!(
            state.step(&data, FrameInput::default()),
            Err(SliceError::Exit)
        );
        assert_eq!(state.snapshot(), before);
        data.rooms[0].exits.pop();
        state.walking = WalkingState::new(392, 210);
        let before = state.snapshot();
        assert_eq!(
            state.step(
                &data,
                FrameInput {
                    direction: Some(Direction::Down)
                }
            ),
            Err(SliceError::Exit)
        );
        assert_eq!(state.snapshot(), before);
        state.tick = u64::MAX;
        let before = state.snapshot();
        assert_eq!(
            state.step(&data, FrameInput::default()),
            Err(SliceError::TickOverflow)
        );
        assert_eq!(state.snapshot(), before);
    }
    #[test]
    fn transition_discards_inputs_and_starts_fresh_admission_epoch() {
        let data = synthetic_data();
        let mut state = GameState::new(&data, Policy::SemanticPreview);
        for direction in (0..56)
            .map(|_| Direction::Left)
            .chain((0..24).map(|_| Direction::Down))
        {
            state
                .step(
                    &data,
                    FrameInput {
                        direction: Some(direction),
                    },
                )
                .unwrap();
        }
        let mut neutral = state.clone();
        for _ in 0..35 {
            state
                .step(
                    &data,
                    FrameInput {
                        direction: Some(Direction::Left),
                    },
                )
                .unwrap();
            neutral.step(&data, FrameInput::default()).unwrap();
            assert_eq!(state, neutral);
        }
        assert_eq!(state.walking.last_activation_direction(), None);
        assert_eq!(state.walking.onset_remaining(), 0);
    }
}

#[cfg(test)]
#[path = "house_tests.rs"]
mod house_tests;

#[cfg(test)]
#[path = "progression_tests.rs"]
mod progression_tests;
