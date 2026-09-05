//! Explicit semantic room preview over immutable data; not classic frame fidelity.
use crate::transition::Transition;
use crate::{FrameInput, Room, Unqualified, WalkingState};
use alloc::{vec, vec::Vec};
use core::fmt;

/// Semantic profile version; v6 adds ROM-compiled fresh bedroom initialization.
pub const PROFILE_VERSION: u8 = 6;

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
    fn fine(self, x: u16, y: u16) -> bool {
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

/// Immutable two-room data. Asset extraction and hashing happen outside the core.
#[derive(Debug)]
pub struct GameData {
    rooms: [Room; 2],
    exits: [Vec<Exit>; 2],
    identity: DataIdentity,
    new_game: NewGameData,
}
impl GameData {
    /// Builds this narrow profile. Exit source, adjustment and spawn qualification
    /// must be checked by the asset adapter; both first exits are validated here too.
    /// `new_game` must be compiled from the fresh bootstrap source, not
    /// supplied SRAM or a restored checkpoint. Intro presentation is omitted by policy.
    /// # Errors
    /// Rejects wrong room dimensions or missing/changed doorway metadata.
    pub fn new(
        rooms: [Room; 2],
        exits: [Vec<Exit>; 2],
        identity: DataIdentity,
        new_game: NewGameData,
    ) -> Result<Self, SliceError> {
        let e = exits[0].first().ok_or(SliceError::Data)?;
        let reverse = Exit([24, 20, 1, 1, 15, 0, 0, 6, 128, 1, 176, 0]);
        if e.0[..4] != [24, 12, 1, 2]
            || e.0[4..8] != [16, 0, 0, 5]
            || u16::from_le_bytes([e.0[8], e.0[9]]) != 384
            || u16::from_le_bytes([e.0[10], e.0[11]]) != 336
            || new_game.position != (304, 112)
            || exits[1].first() != Some(&reverse)
            || rooms
                .iter()
                .chain(core::iter::once(&new_game.bedroom))
                .any(|r| r.width() != 32 || r.height() != 64)
        {
            return Err(SliceError::Data);
        }
        Ok(Self {
            rooms,
            exits,
            identity,
            new_game,
        })
    }
    fn room(&self, id: u16, fresh: bool) -> Result<&Room, SliceError> {
        if fresh {
            return if id == 15 {
                Ok(&self.new_game.bedroom)
            } else {
                Err(SliceError::Data)
            };
        }
        match id {
            15 => Ok(&self.rooms[0]),
            16 => Ok(&self.rooms[1]),
            _ => Err(SliceError::Data),
        }
    }
    fn exit(&self, id: u16, position: (u16, u16)) -> Option<(usize, Exit)> {
        let origin = (position.0.wrapping_sub(8), position.1.wrapping_sub(16));
        self.exits[usize::from(id - 15)]
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
    /// An exit or handoff is outside the supported semantic doorway pair.
    Exit,
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
    /// Current map ID ($000F or $0010).
    pub map_id: u16,
    /// Player anchor in map pixels.
    pub position: (u16, u16),
    /// Owner of movement for the next update.
    pub phase: Phase,
}

/// Minimal mutable slice state. No clock, filesystem, original CPU or RNG usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    identity: DataIdentity,
    tick: u64,
    map_id: u16,
    walking: WalkingState,
    transition: Option<Transition>,
    fresh_bedroom: bool,
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
            transition: None,
            fresh_bedroom: false,
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
            transition: None,
            fresh_bedroom: true,
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
        if self.identity != data.identity {
            return Err(SliceError::Data);
        }
        let mut next = self.clone();
        next.tick = next.tick.checked_add(1).ok_or(SliceError::TickOverflow)?;
        if let Some(mut transition) = next.transition {
            transition.advance();
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
            next.walking
                .step(data.room(next.map_id, next.fresh_bedroom)?, input)
                .map_err(SliceError::Walking)?;
            if let Some((index, _)) = data.exit(next.map_id, next.walking.position()) {
                let transition = Transition::start(next.map_id, next.walking.position())
                    .ok_or(SliceError::Exit)?;
                if index != 0 || next.walking.active_direction() != Some(transition.direction()) {
                    return Err(SliceError::Exit);
                }
                // Walking no longer owns control; discard its unused history.
                // A canonical anchor keeps the private representation stable.
                let (x, y) = transition.handoff();
                next.walking = WalkingState::new(x, y);
                next.transition = Some(transition);
            }
        }
        let output = next.output();
        *self = next;
        Ok(output)
    }
    /// Current stable semantic result, without advancing simulation.
    #[must_use]
    pub fn output(&self) -> FrameOutput {
        FrameOutput {
            tick: self.tick,
            map_id: self.map_id,
            position: self
                .transition
                .map_or_else(|| self.walking.position(), Transition::position),
            phase: self.transition.map_or(Phase::Walking, |t| {
                if t.elapsed() <= 17 {
                    Phase::Departing
                } else {
                    Phase::Arriving
                }
            }),
        }
    }
    /// Fixed little-endian v1 snapshot; immutable content is identified, not embedded.
    /// Bytes include profile/schema and RNG-policy versions (0 means no RNG).
    #[must_use]
    pub fn snapshot(&self) -> Vec<u8> {
        let mut bytes = vec![b'R', b'S', b'L', b'C', 1, PROFILE_VERSION, 0, 1];
        bytes.extend(self.identity.rom_sha256);
        bytes.extend(self.identity.content_sha256);
        bytes.extend(self.tick.to_le_bytes());
        bytes.extend(self.map_id.to_le_bytes());
        bytes.push(self.transition.map_or(255, Transition::encoded));
        // No walking component is serialized while a doorway owns control.
        // Erasing its marker cannot turn a transition into a valid walking state.
        bytes.extend(if self.transition.is_some() {
            [0; crate::SNAPSHOT_SIZE]
        } else {
            self.walking.encode_snapshot()
        });
        bytes.push(u8::from(self.fresh_bedroom));
        bytes
    }
    /// Restores only a compatible, internally valid snapshot, without data or I/O.
    /// # Errors
    /// Rejects versions, identities, malformed walking state, or invalid transition ownership.
    pub fn restore(data: &GameData, bytes: &[u8]) -> Result<Self, SliceError> {
        if bytes.len() != 100
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
        let transition = if bytes[82] == 255 {
            None
        } else {
            Some(Transition::restore(bytes[82]).ok_or(SliceError::Snapshot)?)
        };
        let walking = if let Some(t) = transition {
            if t.map_id() != map_id
                || bytes[83..99] != [0; crate::SNAPSHOT_SIZE]
                || (fresh_bedroom && t.source_map() != 15)
            {
                return Err(SliceError::Snapshot);
            }
            let (x, y) = t.handoff();
            WalkingState::new(x, y)
        } else {
            let walking =
                WalkingState::decode_snapshot(data.room(map_id, fresh_bedroom)?, &bytes[83..99])
                    .map_err(|_| SliceError::Snapshot)?;
            if data.exit(map_id, walking.position()).is_some() {
                return Err(SliceError::Snapshot);
            }
            walking
        };
        Ok(Self {
            identity: data.identity,
            tick,
            map_id,
            walking,
            transition,
            fresh_bedroom,
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
        assert_eq!(data.room(15, fresh.fresh_bedroom).unwrap(), &data.rooms[0]);
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
                phase: Phase::Walking
            }
        );
    }
    #[test]
    fn reverse_route_restores_ownership_and_replays_every_snapshot() {
        let mut data = synthetic_data();
        let mut cells = vec![0; 2048];
        cells[19 * 32 + 24] = 14 << 9; // clamps the last Up step onto Y336
        data.rooms[1] = Room::new(32, 64, cells).unwrap();
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
        data.exits[0] = vec![
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
        data.exits[0].push(Exit([29, 10, 1, 1, 99, 0, 0, 0, 0, 0, 0, 0]));
        let before = state.snapshot();
        assert_eq!(
            state.step(&data, FrameInput::default()),
            Err(SliceError::Exit)
        );
        assert_eq!(state.snapshot(), before);
        data.exits[0].pop();
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
