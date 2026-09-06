//! Aggregate Pandora ownership. Source pacing/collision stay immutable compiler inputs.
use super::shared_sheet::{self, Sheet};
#[allow(clippy::wildcard_imports)] // Child integration shares the enclosing slice types.
use super::*;
use crate::pandora::{Node, Runtime};
use crate::pots::{self, PotState};

/// Explicit movement owner, independent of whether text is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOwner {
    /// Existing ordinary player input.
    Player,
    /// Text page/choice owns input.
    Dialogue,
    /// Finite source presentation owns input with no visible text required.
    Presentation,
    /// Qualified doorway/stair reconstruction.
    Transition,
    /// Pot lift/throw recovery; may overlap a story request.
    PotRecovery,
}
/// Read-only semantic presentation, not native sprite scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PandoraOutput {
    /// Stable source actor phase.
    pub scene: ScenePhase,
    /// Exact invocation, preserving repeated-resource identity.
    pub invocation: Option<Invocation>,
    /// Pending qualified source presentation completion.
    pub cue: Option<Cue>,
    /// Active motion key and number of samples already applied (zero at an exit handoff).
    /// Final samples commit the graph continuation and clear this presentation clock.
    pub motion: Option<(MotionKey, u16)>,
    /// Authoritative input owner.
    pub owner: ControlOwner,
    /// Actual hit counter, reset on every reconstruction.
    pub door_counter: u8,
    /// Room locals0..31, read from the sole story projection.
    pub locals: u32,
    /// Resident shared-sheet patches, independent of the current room visit.
    pub sheet: SharedSheetOutput,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct State {
    pub graph: Runtime,
    pub motion: Option<(u8, u16)>, // immutable motion index; next sample index
    pub pot: Option<PotState>,
    pub frozen: Option<CollisionKey>,
    pub sheet: Sheet,
    pub visit_consumed: u64,
    pub visit_cellar: CellarDoorPatch,
}
impl State {
    pub const fn new() -> Self {
        Self {
            graph: Runtime::new(),
            motion: None,
            pot: None,
            frozen: None,
            sheet: Sheet::new(true),
            visit_consumed: 0,
            visit_cellar: CellarDoorPatch::Closed,
        }
    }
    pub fn owns(self) -> bool {
        self.graph.node != Node::Control || self.motion.is_some()
    }
    pub fn collision(self, map: u16, flags: &StoryFlags) -> Option<CollisionKey> {
        let has = |bit| flags.contains(bit) == Ok(true);
        Some(match map {
            0xa => CollisionKey::Town,
            0x13 => CollisionKey::Resident,
            0xc if has(0x27) => {
                if self.graph.counter == 2 && self.graph.node != Node::Control {
                    CollisionKey::CReaction
                } else if self.sheet.cellar == CellarDoorPatch::Open {
                    CollisionKey::COpen
                } else if self.sheet.cellar == CellarDoorPatch::Damaged {
                    CollisionKey::CDamaged
                } else {
                    CollisionKey::CClosed
                }
            }
            0xe => CollisionKey::CellarE,
            0x20 => CollisionKey::Cellar20,
            0x21 if has(0x22) && (self.graph.node != Node::Cue(Cue::BoxReload) || !has(1)) => {
                CollisionKey::BoxOpened
            }
            0x21 => CollisionKey::Box,
            0x41 => CollisionKey::Tour41,
            0x44 => CollisionKey::Tour44,
            0x42 => CollisionKey::Tour42,
            0x43 => CollisionKey::Tour43,
            _ => return None,
        })
    }
    pub fn scene(self, map: u16, flags: &StoryFlags) -> ScenePhase {
        #[allow(clippy::enum_glob_use)]
        use Invocation::*;
        let invocation = match self.graph.node {
            Node::Request(i, _) | Node::Cue(Cue::Returned(i)) => Some(i),
            _ => None,
        };
        if let Some(i) = invocation {
            return match i {
                ResidentFirst | ResidentGrant | ResidentRetry | ResidentRefusal => {
                    ScenePhase::Resident13Request
                }
                CEntry => ScenePhase::CEntry,
                CApproach | CChoice | CDirect => ScenePhase::CChoice,
                FirstHit => ScenePhase::CFirstHit,
                SecondHit => ScenePhase::CSecondHit,
                ReactionSpeaker | ReactionRequest => ScenePhase::CReactionSpeaker,
                ReactionRight => ScenePhase::CReactionRight,
                ReactionLeft => ScenePhase::CReactionLeft,
                ReactionFinal => ScenePhase::CReactionFinal,
                BoxEntry | BoxWarning => ScenePhase::BoxContact,
                OpeningFirst | OpeningThird => ScenePhase::BoxOpening,
                OpeningSecond | OpeningFourth => ScenePhase::BoxOpeningCue,
                TourIntro | TourFinal => ScenePhase::Tour410,
                TourOne => ScenePhase::Tour411,
                TourTwo => ScenePhase::Tour412,
                TourThree => ScenePhase::Tour413,
                TourFour => ScenePhase::Tour414,
                TourFive => ScenePhase::Tour415,
                TourSix => ScenePhase::Tour416,
                TourLeave41 => ScenePhase::Tour417,
                Tour44 | TourLeave44 => ScenePhase::Tour44,
                Tour42 | TourLeave42 => ScenePhase::Tour42,
                Tour43 | TourLeave43 => ScenePhase::Tour43,
            };
        }
        match self.graph.node {
            Node::Cue(Cue::SecondHitPatched) => return ScenePhase::CSecondHit,
            Node::Cue(Cue::ReactionColorMath | Cue::ReactionColorReturn) => {
                return ScenePhase::CColorMath
            }
            Node::Cue(Cue::BoxAcquireControl) => return ScenePhase::BoxContact,
            Node::Cue(Cue::BoxReload) => {
                return if flags.contains(1) == Ok(true) {
                    ScenePhase::BoxContact // The old roster survives until reconstruction.
                } else {
                    ScenePhase::BoxOpening
                };
            }
            _ => {}
        }
        match self.collision(map, flags) {
            Some(CollisionKey::Town) => ScenePhase::TownSource,
            Some(CollisionKey::Resident) => ScenePhase::Resident13,
            Some(CollisionKey::CClosed | CollisionKey::CDamaged) => ScenePhase::CDirect,
            Some(CollisionKey::COpen) => ScenePhase::CDeparted,
            Some(CollisionKey::CellarE) => ScenePhase::CellarE,
            Some(CollisionKey::Cellar20) => ScenePhase::Cellar20,
            Some(CollisionKey::Box) => ScenePhase::BoxContact,
            Some(CollisionKey::Tour41) => ScenePhase::TourControl,
            _ => ScenePhase::House,
        }
    }
}
impl GameData {
    /// Enable the bounded graph on the SAME New Game, never on the live host implicitly.
    /// Identity must authenticate existing house plus all Pandora source/compiler data.
    /// # Errors
    /// Requires the B/exterior capability, a single extension and unchanged ROM identity.
    pub fn with_pandora(
        mut self,
        pandora: PandoraData,
        identity: DataIdentity,
    ) -> Result<Self, SliceError> {
        if self.pandora.is_some()
            || !self.conversation_progression()
            || identity.rom_sha256 != self.identity.rom_sha256
            || identity.content_sha256 == self.identity.content_sha256
        {
            return Err(SliceError::Data);
        }
        for motion in &pandora.motions {
            for frame in &motion.frames {
                if frame.map_id == 0xd {
                    self.room(0xd, false, true)?
                        .validate_position(frame.anchor.position.0, frame.anchor.position.1)
                        .map_err(|_| SliceError::Data)?;
                }
            }
        }
        self.pandora = Some(pandora);
        self.identity = identity;
        Ok(self)
    }
    /// Explicit immutable capability; false preserves profile9 behavior and bytes.
    #[must_use]
    pub const fn pandora_enabled(&self) -> bool {
        self.pandora.is_some()
    }
}
impl GameState {
    fn check_pandora<'a>(&self, data: &'a GameData) -> Result<&'a PandoraData, SliceError> {
        if self.identity != data.identity || self.pandora.is_none() {
            return Err(SliceError::Data);
        }
        data.pandora.as_ref().ok_or(SliceError::Data)
    }
    /// Stable story/art output. No host flag-derived scene selection is needed.
    /// # Errors
    /// Rejects disabled or incompatible aggregate data.
    pub fn pandora_output(&self, data: &GameData) -> Result<PandoraOutput, SliceError> {
        let spec = self.check_pandora(data)?;
        let state = self.pandora.ok_or(SliceError::Data)?;
        let mut scene = state.scene(self.map_id, &self.flags);
        if let Some((id, next)) = state.motion {
            if next > 0 {
                scene = spec.motions[usize::from(id)].frames[usize::from(next - 1)].scene;
            }
        }
        let recovery = state
            .pot
            .is_some_and(|p| matches!(p.phase(), pots::Phase::Lifting | pots::Phase::Throwing));
        let owner = if self.transition.is_some()
            || state
                .motion
                .is_some_and(|(id, _)| spec.motions[usize::from(id)].key.maps().2)
        {
            ControlOwner::Transition
        } else if recovery {
            ControlOwner::PotRecovery
        } else if state.motion.is_some() {
            ControlOwner::Presentation
        } else if self.dialogue.is_some() || matches!(state.graph.node, Node::Request(..)) {
            ControlOwner::Dialogue
        } else if state.owns() {
            ControlOwner::Presentation
        } else {
            ControlOwner::Player
        };
        Ok(PandoraOutput {
            scene,
            invocation: match state.graph.node {
                Node::Request(i, _) => Some(i),
                _ => None,
            },
            cue: match state.graph.node {
                Node::Cue(c) => Some(c),
                _ => None,
            },
            motion: state
                .motion
                .map(|(id, cursor)| (spec.motions[usize::from(id)].key, cursor)),
            owner,
            door_counter: state.graph.counter,
            sheet: SharedSheetOutput {
                resident: state.sheet.resident,
                cellar: state.sheet.cellar,
                consumed: state.consumed(),
            },
            locals: u32::from_le_bytes(
                self.flags.bytes()[..4]
                    .try_into()
                    .map_err(|_| SliceError::Snapshot)?,
            ),
        })
    }
    /// Read-only per-visit action component with the resident sheet's canonical ledger.
    /// Hand/reservation slots can be queried through `pot_slots`; no mutable access.
    #[must_use]
    pub fn pot_state(&self) -> Option<&PotState> {
        self.pandora.as_ref()?.pot.as_ref()
    }
    /// Source held/reserved kinds: FA=098A, FB=098F, None after release/break.
    /// # Errors
    /// Rejects disabled/incompatible data.
    pub fn pot_slots(&self, data: &GameData) -> Result<(Option<u16>, Option<u16>), SliceError> {
        let spec = self.check_pandora(data)?;
        let state = self.pandora.ok_or(SliceError::Data)?;
        let Some(pot) = state.pot else {
            return Ok((None, None));
        };
        let room = self.pot_room(spec, state)?;
        let a = self.pot_admission(spec, &room);
        Ok((pot.held_slot_in(&a), pot.reserved_slot_in(&a)))
    }
    fn pot_room(&self, spec: &PandoraData, state: State) -> Result<Room, SliceError> {
        let key = state
            .frozen
            .or_else(|| state.collision(self.map_id, &self.flags))
            .ok_or(SliceError::Data)?;
        let mut room = spec.room(key).clone();
        shared_sheet::wood(&mut room, self.wooden_door_open);
        Ok(room)
    }
    fn pot_admission<'a>(&self, spec: &'a PandoraData, room: &'a Room) -> pots::Admission<'a> {
        pots::Admission {
            room,
            objects: &spec.objects,
            cellar_up_lanes: spec.cellar_up_lanes,
            door_hit_enabled: self.flags.contains(0x28) == Ok(true)
                && self.flags.contains(0x292) == Ok(false),
        }
    }
    pub(super) fn ensure_pot(&mut self, spec: &PandoraData) -> Result<(), SliceError> {
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        if self.map_id == 0xc && self.flags.contains(0x2e) == Ok(true) && state.pot.is_none() {
            let room = self.pot_room(spec, state)?;
            state.pot = Some(
                PotState::with_ledger(
                    &self.pot_admission(spec, &room),
                    self.walking,
                    self.animation.facing(),
                    state.sheet.parked_consumed,
                )
                .map_err(SliceError::Pot)?,
            );
            state.sheet.parked_consumed = 0;
            self.pandora = Some(state);
        }
        Ok(())
    }
    fn advance_pot(&mut self, spec: &PandoraData, input: pots::Input) -> Result<(), SliceError> {
        self.ensure_pot(spec)?;
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        let mut pot = state.pot.ok_or(SliceError::Interaction)?;
        let key = state
            .frozen
            .or_else(|| state.collision(self.map_id, &self.flags))
            .ok_or(SliceError::Data)?;
        let before = pot.phase();
        let room = self.pot_room(spec, state)?;
        let output = pot
            .step(&self.pot_admission(spec, &room), input)
            .map_err(SliceError::Pot)?;
        if before != pots::Phase::Throwing && pot.phase() == pots::Phase::Throwing {
            state.frozen = Some(key);
        }
        if output.control_restored {
            state.frozen = None;
        }
        if output.door_hit {
            state.graph.hit(&mut self.flags)?;
            state.sheet.cellar = if state.graph.counter == 1 {
                CellarDoorPatch::Damaged
            } else {
                CellarDoorPatch::Open
            };
        }
        self.walking = *pot.walking();
        if output.movement.is_some() {
            self.animation.advance(self.walking.active_direction());
        } else {
            self.animation = AnimationState::standing(pot.facing());
        }
        state.pot = Some(pot);
        self.pandora = Some(state);
        Ok(())
    }
    /// Distinct one-frame native A pulse (lift/throw), NEVER resident Interact/B.
    /// # Errors
    /// Disabled data, wrong room/ownership/geometry/onset or repeated A reject atomically.
    pub fn pot_action(&mut self, data: &GameData) -> Result<FrameOutput, SliceError> {
        let spec = self.check_pandora(data)?;
        let mut next = self.clone();
        if next.map_id != 0xc
            || next.transition.is_some()
            || next.dialogue.is_some()
            || next.flags.contains(0x292) == Ok(true)
            || next.pandora.ok_or(SliceError::Data)?.owns()
        {
            return Err(SliceError::Interaction);
        }
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        next.advance_pot(
            spec,
            pots::Input {
                direction: None,
                action: true,
            },
        )?;
        *self = next;
        Ok(self.output())
    }
    pub(super) fn pandora_load(&mut self) -> Result<(), SliceError> {
        if let Some(state) = &mut self.pandora {
            state.load(self.map_id, &mut self.flags, &mut self.wooden_door_open)?;
        }
        Ok(())
    }
    pub(super) fn step_pandora(
        &mut self,
        data: &GameData,
        input: FrameInput,
    ) -> Result<FrameOutput, SliceError> {
        let spec = self.check_pandora(data)?;
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        next.ensure_pot(spec)?;
        let state = next.pandora.ok_or(SliceError::Data)?;
        if state
            .pot
            .is_some_and(|p| matches!(p.phase(), pots::Phase::Lifting | pots::Phase::Throwing))
        {
            // Flight/recovery must progress even when the real hit opened a story request.
            next.advance_pot(spec, pots::Input::default())?;
            if state.motion.is_some() || matches!(state.graph.node, Node::Cue(_)) {
                next.advance_pending_motion(spec)?;
            }
        } else if state.motion.is_some() || matches!(state.graph.node, Node::Cue(_)) {
            next.advance_pending_motion(spec)?;
        } else if matches!(state.graph.node, Node::Request(..)) || next.dialogue.is_some() {
            // Semantic text ownership: discard directions, never buffer them.
        } else if next.poll_box_opening(spec)? {
            // COP0D/local predicate takes control, but COPDF has not succeeded.
        } else {
            if state.pot.is_some() {
                next.advance_pot(
                    spec,
                    pots::Input {
                        direction: input.direction,
                        action: false,
                    },
                )?;
            } else {
                next.walking
                    .step(&next.effective_room(data)?, input)
                    .map_err(SliceError::Walking)?;
                next.animation.advance(next.walking.active_direction());
            }
            next.detect_pandora_contact(spec)?;
            next.poll_box_opening(spec)?;
            next.detect_pandora_exit(spec)?;
            next.detect_house_exit(data)?;
        }
        *self = next;
        Ok(self.output())
    }
    fn advance_motion(&mut self, spec: &PandoraData) -> Result<(), SliceError> {
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        let (id, cursor) = state.motion.ok_or(SliceError::Exit)?;
        let motion = &spec.motions[usize::from(id)];
        let frame = motion
            .frames
            .get(usize::from(cursor))
            .ok_or(SliceError::Exit)?;
        self.map_id = frame.map_id;
        self.walking = WalkingState::new(frame.anchor.position.0, frame.anchor.position.1);
        self.animation = AnimationState::standing(frame.anchor.facing);
        if frame.reload {
            state.load(self.map_id, &mut self.flags, &mut self.wooden_door_open)?;
            self.fresh_bedroom = false;
            self.d_open_loaded = self.map_id == 0xd && self.flags.contains(0x26) == Ok(true);
        }
        if !frame.reload {
            if let Some(pot) = &mut state.pot {
                if pot.phase() == pots::Phase::Empty {
                    pot.rebase(self.walking, self.animation.facing())
                        .map_err(SliceError::Pot)?;
                } else if pot.walking().position() != self.walking.position()
                    || pot.facing() != self.animation.facing()
                {
                    return Err(SliceError::Interaction);
                } else {
                    self.walking = *pot.walking();
                }
            }
        }
        if usize::from(cursor) + 1 == motion.frames.len() {
            state.motion = None;
            if let MotionKey::Cue(c) = motion.key {
                state.graph.complete(c, &mut self.flags)?;
            }
        } else {
            state.motion = Some((id, cursor + 1));
        }
        self.pandora = Some(state);
        Ok(())
    }
    fn detect_pandora_contact(&mut self, spec: &PandoraData) -> Result<(), SliceError> {
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        if self.map_id != 0x21
            || state.owns()
            || self.walking.active_direction() != Some(Direction::Down)
        {
            return Ok(());
        }
        if self.flags.contains(1) == Ok(true) {
            return Ok(());
        } // callback category cleared
        let contact = spec.contacts[1];
        if self.walking.position() != contact.trigger.position
            || self.animation.facing() != contact.trigger.facing
        {
            return Ok(());
        }
        state.graph.contact(&mut self.flags)?;
        self.walking = WalkingState::new(contact.result.position.0, contact.result.position.1);
        self.animation = AnimationState::standing(contact.result.facing);
        self.pandora = Some(state);
        Ok(())
    }
    fn poll_box_opening(&mut self, spec: &PandoraData) -> Result<bool, SliceError> {
        if self.map_id != 0x21 || !spec.opening_gate.contains(self.walking.position()) {
            return Ok(false);
        }
        let state = self.pandora.as_mut().ok_or(SliceError::Data)?;
        if !state.graph.poll_opening(&self.flags) {
            return Ok(false);
        }
        let (x, y) = self.walking.position();
        self.walking = WalkingState::new(x, y);
        self.animation = AnimationState::standing(self.animation.facing());
        Ok(true)
    }
    fn detect_pandora_exit(&mut self, spec: &PandoraData) -> Result<(), SliceError> {
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        if state.owns() || state.pot.is_some_and(|p| p.phase() != pots::Phase::Empty) {
            return Ok(());
        }
        for (id, motion) in spec.motions.iter().enumerate() {
            let MotionKey::Travel(travel) = motion.key else {
                continue;
            };
            let anchor = motion.trigger.ok_or(SliceError::Data)?;
            if travel.maps().0 != self.map_id
                || anchor.position != self.walking.position()
                || self.walking.active_direction() != Some(anchor.facing)
            {
                continue;
            }
            if self.flags.contains(0x26) != Ok(true)
                || (matches!(
                    travel,
                    Travel::CToCellar | Travel::ETo20 | Travel::TwentyToBox
                ) && self.flags.contains(0x292) != Ok(true))
            {
                return Err(SliceError::Exit);
            }
            state.motion = Some((u8::try_from(id).map_err(|_| SliceError::Data)?, 0));
            if state.pot.is_none() {
                self.walking = WalkingState::new(anchor.position.0, anchor.position.1);
            }
            self.animation = AnimationState::standing(anchor.facing);
            self.pandora = Some(state);
            return Ok(());
        }
        Ok(())
    }
}
impl GameState {
    pub(super) fn pandora_resident(&mut self, data: &GameData) -> Result<FrameOutput, SliceError> {
        let spec = self.check_pandora(data)?;
        let contact = spec.contacts[0];
        if self.transition.is_some()
            || self.walking.position() != contact.trigger.position
            || self.animation.facing() != contact.trigger.facing
        {
            return Err(SliceError::Interaction);
        }
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        next.pandora
            .as_mut()
            .ok_or(SliceError::Data)?
            .graph
            .resident(self.map_id, &self.flags)?;
        next.walking = WalkingState::new(contact.result.position.0, contact.result.position.1);
        next.animation = AnimationState::standing(contact.result.facing);
        *self = next;
        Ok(self.output())
    }
    pub(super) fn pandora_dialogue_action(
        &mut self,
        data: &GameData,
        choice: Option<u8>,
    ) -> Result<FrameOutput, SliceError> {
        let spec = self.check_pandora(data)?;
        if self.transition.is_some() || self.pandora.is_some_and(|p| p.motion.is_some()) {
            return Err(SliceError::Interaction);
        }
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        let graph = &mut next.pandora.as_mut().ok_or(SliceError::Data)?.graph;
        if let Some(result) = choice {
            graph.choose(&spec.text, result)?;
        } else {
            graph.acknowledge(&spec.text, &mut next.flags)?;
        }
        next.ensure_pot(spec)?;
        *self = next;
        Ok(self.output())
    }
}
impl State {
    #[allow(clippy::too_many_lines)] // Coupled graph, motion and pot ownership invariants.
    pub(super) fn decode(
        data: &GameData,
        bytes: &[u8],
        map: u16,
        flags: &StoryFlags,
    ) -> Result<Self, SliceError> {
        let spec = data.pandora.as_ref().ok_or(SliceError::Snapshot)?;
        let graph = Runtime::decode(
            bytes[245..249]
                .try_into()
                .map_err(|_| SliceError::Snapshot)?,
            map,
            flags,
        )?;
        let motion = match (bytes[249], u16::from_le_bytes([bytes[250], bytes[251]])) {
            (255, 0) => None,
            (id, cursor)
                if spec
                    .motions
                    .get(usize::from(id))
                    .is_some_and(|m| usize::from(cursor) < m.frames.len()) =>
            {
                Some((id, cursor))
            }
            _ => return Err(SliceError::Snapshot),
        };
        let frozen = if bytes[292] == 255 {
            None
        } else {
            Some(
                *CollisionKey::ALL
                    .get(usize::from(bytes[292]))
                    .ok_or(SliceError::Snapshot)?,
            )
        };
        let mut state = Self {
            graph,
            motion,
            pot: None,
            frozen,
            sheet: Sheet {
                resident: match bytes[300] {
                    0 => false,
                    1 => true,
                    _ => return Err(SliceError::Snapshot),
                },
                cellar: CellarDoorPatch::decode(bytes[301])?,
                parked_consumed: u64::from_le_bytes(
                    bytes[304..312]
                        .try_into()
                        .map_err(|_| SliceError::Snapshot)?,
                ),
            },
            visit_cellar: CellarDoorPatch::decode(bytes[302])?,
            visit_consumed: u64::from_le_bytes(
                bytes[312..320]
                    .try_into()
                    .map_err(|_| SliceError::Snapshot)?,
            ),
        };
        if bytes[252..292] != [0; 40] {
            if map != 0xc || flags.contains(0x2e) != Ok(true) {
                return Err(SliceError::Snapshot);
            }
            let key = frozen
                .or_else(|| state.collision(map, flags))
                .ok_or(SliceError::Snapshot)?;
            let mut room = spec.room(key).clone();
            shared_sheet::wood(&mut room, bytes[108] == 1);
            let admission = pots::Admission {
                room: &room,
                objects: &spec.objects,
                cellar_up_lanes: spec.cellar_up_lanes,
                door_hit_enabled: flags.contains(0x28) == Ok(true)
                    && flags.contains(0x292) == Ok(false),
            };
            state.pot = Some(
                PotState::decode_snapshot(&admission, &bytes[252..292])
                    .map_err(|_| SliceError::Snapshot)?,
            );
        }
        state.validate_sheet(spec, map, flags, bytes[108] == 1)?;
        if bytes[303] != 0 {
            return Err(SliceError::Snapshot);
        }
        // An active reservation must have been consumed during THIS visit.
        let object = bytes[283];
        if object > 0 && state.visit_consumed & (1 << (object - 1)) != 0 {
            return Err(SliceError::Snapshot);
        }
        if let Node::Cue(cue) = graph.node {
            let (source, destination, reload) = MotionKey::Cue(cue).maps();
            let loaded = reload
                && map == destination
                && (source != destination || flags.bytes()[..4] == [0; 4]);
            if loaded && motion.is_none() {
                return Err(SliceError::Snapshot);
            }
        }
        if state
            .pot
            .is_some_and(|p| p.phase() == pots::Phase::Throwing)
            != frozen.is_some()
            || frozen.is_some_and(|k| !matches!(k, CollisionKey::CClosed | CollisionKey::CDamaged))
        {
            return Err(SliceError::Snapshot);
        }
        if let Some((id, cursor)) = motion {
            let m = &spec.motions[usize::from(id)];
            match m.key {
                MotionKey::Cue(c) if graph.node == Node::Cue(c) => {}
                MotionKey::Travel(_)
                    if graph.node == Node::Control
                        || (map == 0x21
                            && matches!(graph.node, Node::Request(Invocation::BoxEntry, _))) => {}
                _ => return Err(SliceError::Snapshot),
            }
            if cursor == 0 {
                if map != m.key.maps().0 {
                    return Err(SliceError::Snapshot);
                }
            } else if m.frames[usize::from(cursor - 1)].map_id != map {
                return Err(SliceError::Snapshot);
            }
        }
        Ok(state)
    }
}
impl GameState {
    pub(super) fn append_pandora_snapshot(&self, bytes: &mut Vec<u8>) {
        let Some(state) = self.pandora else {
            return;
        };
        bytes.extend(&self.flags.bytes()[64..]);
        bytes.extend(state.graph.encode());
        bytes.push(state.motion.map_or(255, |x| x.0));
        bytes.extend(state.motion.map_or(0, |x| x.1).to_le_bytes());
        bytes.extend(state.pot.map_or([0; 40], |p| p.encode_snapshot()));
        bytes.push(state.frozen.map_or(255, |k| k as u8));
        bytes.extend([0; 3]);
        let frozen = state.owns()
            && state.motion.is_none()
            && self.transition.is_none()
            && self.dialogue.is_none();
        let (x, y) = if frozen {
            self.walking.position()
        } else {
            (0, 0)
        };
        bytes.extend(x.to_le_bytes());
        bytes.extend(y.to_le_bytes());
        bytes.extend([
            u8::from(state.sheet.resident),
            state.sheet.cellar as u8,
            state.visit_cellar as u8,
            0,
        ]);
        bytes.extend(state.sheet.parked_consumed.to_le_bytes());
        bytes.extend(state.visit_consumed.to_le_bytes());
    }
    pub(super) fn validate_pandora_restore(
        &self,
        data: &GameData,
        bytes: &[u8],
    ) -> Result<(), SliceError> {
        let spec = self.check_pandora(data)?;
        let state = self.pandora.ok_or(SliceError::Snapshot)?;
        if state.owns() && self.dialogue.is_some() {
            return Err(SliceError::Snapshot);
        }
        if state.motion.is_some() && self.transition.is_some() {
            return Err(SliceError::Snapshot);
        }
        if let Some(t) = self.transition {
            if t.map_id() != t.source_map() {
                // Reconstruction already happened; a departing visit's action may
                // not survive into this arrival, even if its map is also C.
                if state.pot.is_some()
                    || state.graph.counter != 0
                    || self.flags.bytes()[..4] != [0; 4]
                    || state.visit_consumed != state.consumed()
                    || state.visit_cellar != state.sheet.cellar
                {
                    return Err(SliceError::Snapshot);
                }
            } else if state.owns()
                || state.pot.is_some_and(|p| {
                    !p.idle_empty() || p.position() != t.handoff() || p.facing() != t.direction()
                })
            {
                return Err(SliceError::Snapshot);
            }
        }
        if let Some((id, cursor)) = state.motion {
            let motion = &spec.motions[usize::from(id)];
            let expected = if cursor == 0 {
                motion.trigger
            } else {
                Some(motion.frames[usize::from(cursor - 1)].anchor)
            };
            if expected.is_some_and(|a| {
                a.position != self.walking.position() || a.facing != self.animation.facing()
            }) {
                return Err(SliceError::Snapshot);
            }
        }
        if self.transition.is_none() && self.dialogue.is_none() {
            if let Some(pot) = state.pot {
                if self.walking != *pot.walking() || self.animation.facing() != pot.facing() {
                    return Err(SliceError::Snapshot);
                }
                if !state.owns()
                    && matches!(pot.phase(), pots::Phase::Empty | pots::Phase::Held)
                    && self.animation.is_walking() != self.walking.active_direction().is_some()
                {
                    return Err(SliceError::Snapshot);
                }
                if let Some(frozen) = state.frozen {
                    // At most one hit per flight; the visit baseline, not counter0,
                    // determines whether this launch began on retained damage.
                    let hit = pot.contact_reached();
                    let before = state
                        .graph
                        .counter
                        .checked_sub(u8::from(hit))
                        .ok_or(SliceError::Snapshot)?;
                    if frozen
                        != if before == 0 && state.visit_cellar == CellarDoorPatch::Closed {
                            CollisionKey::CClosed
                        } else {
                            CollisionKey::CDamaged
                        }
                        || before > 1
                    {
                        return Err(SliceError::Snapshot);
                    }
                }
            } else if state.owns() {
                if self.animation.is_walking()
                    || self.walking
                        != WalkingState::new(self.walking.position().0, self.walking.position().1)
                {
                    return Err(SliceError::Snapshot);
                }
                WalkingState::decode_snapshot(
                    &self.effective_room(data)?,
                    &self.walking.encode_snapshot(),
                )
                .map_err(|_| SliceError::Snapshot)?;
            } else if self.map_id == 0xc && self.flags.contains(0x2e) == Ok(true) {
                return Err(SliceError::Snapshot);
            }
        }
        if self.snapshot() != bytes {
            return Err(SliceError::Snapshot);
        }
        Ok(())
    }
}
impl GameState {
    fn detect_house_exit(&mut self, data: &GameData) -> Result<(), SliceError> {
        let state = self.pandora.ok_or(SliceError::Data)?;
        if state.owns() || state.pot.is_some_and(|p| p.phase() != pots::Phase::Empty) {
            return Ok(());
        }
        let Some((index, _)) = data.exit(self.map_id, self.walking.position()) else {
            return Ok(());
        };
        let transition = Transition::select(self.map_id, index, self.walking.position())
            .ok_or(SliceError::Exit)?;
        if (transition.exterior() && !self.d_open_loaded)
            || (self.map_id == 0xc && index == 2 && !self.wooden_door_open)
            || self.walking.active_direction() != Some(transition.direction())
        {
            return Err(SliceError::Exit);
        }
        let (x, y) = transition.handoff();
        self.walking = WalkingState::new(x, y);
        self.animation = AnimationState::standing(transition.direction());
        self.transition = Some(transition);
        Ok(())
    }
    /// Source-cell removals for rendering, retained for the resident sheet lifetime.
    /// # Errors
    /// Rejects incompatible/disabled data; never changes the immutable collision catalog.
    pub fn consumed_pots(
        &self,
        data: &GameData,
    ) -> Result<Vec<crate::pots::SourceObject>, SliceError> {
        let spec = self.check_pandora(data)?;
        let state = self.pandora.ok_or(SliceError::Data)?;
        Ok(spec
            .objects
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(i, object)| (state.consumed() & (1 << i) != 0).then_some(object))
            .collect())
    }
}
impl GameState {
    fn advance_pending_motion(&mut self, spec: &PandoraData) -> Result<(), SliceError> {
        let mut state = self.pandora.ok_or(SliceError::Data)?;
        if state.motion.is_none() {
            let Node::Cue(cue) = state.graph.node else {
                return Err(SliceError::Interaction);
            };
            let (id, motion) = spec.motion(MotionKey::Cue(cue)).ok_or(SliceError::Exit)?;
            if motion.trigger.is_some_and(|a| {
                a.position != self.walking.position() || a.facing != self.animation.facing()
            }) {
                return Err(SliceError::Exit);
            }
            state.motion = Some((u8::try_from(id).map_err(|_| SliceError::Data)?, 0));
            self.pandora = Some(state);
        }
        self.advance_motion(spec)
    }
}
