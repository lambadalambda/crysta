//! A player walking the slice, moving between maps through real exits.
//!
//! Movement is [`room_core`]'s, unchanged. This adds the two things free roam
//! needs on top: the current map's room, and the exit geometry that carries the
//! player into the next one. Two exact return records additionally own the
//! player through measured initialized-to-free arrival profiles.

use crate::actors::{Actor, Surroundings};
use crate::residents::{residents, talk_to, Conversation, Resident};
use crate::{room, room_candidate, MapRoom, RoomError, MAPS};
use assets::maps::actors::ResolveError;
use assets::maps::exits::{ExitError, ExitList, ExitRecord};
use assets::maps::scripts::EventFlags;
use room_core::arrival::{Arrival, ReturnRoute};
use room_core::{
    AnimationFrame, AnimationState, Direction, FrameInput, Room, Unqualified, WalkingState,
};
use std::fmt;

/// A map the player is standing in, and where they are standing.
#[derive(Clone)]
pub struct World<'a> {
    image: &'a [u8],
    map: u16,
    /// The map's room with the bodies present blocked in.
    room: MapRoom,
    /// The map's room as built, before any body is blocked in.
    base: MapRoom,
    exits: ExitList,
    walking: WalkingState,
    /// Source-bound initialized-to-free arrival; never ordinary walking.
    arrival: Option<Arrival>,
    /// Residents the spawn lists install for the current flags, where their
    /// running scripts have put them.
    residents: Vec<Resident>,
    /// The running script of each resident, aligned with `residents`.
    actors: Vec<Actor>,
    /// Collision cells the bodies blocked when `room` was last rebuilt.
    blocked: Vec<(u16, u16)>,
    /// The `$7E:06C0` event-flag bitmap, owned so it can be written to.
    events: Vec<u8>,
    /// Last direction the player moved in, which is the way they face.
    facing: Direction,
    /// Which of the player's ordinary frames is showing.
    animation: AnimationState,
    /// Whether an exit under the player may fire.
    ///
    /// The player arrives standing on geometry that is often an exit in its own
    /// right — a doorway leads back the way it came. Firing it immediately
    /// would ping-pong between two maps forever, so an arrival disarms the
    /// trigger and stepping clear of every exit rearms it.
    armed: bool,
}

/// A world that could not be entered or stepped.
#[derive(Debug)]
pub enum WorldError {
    /// A targeted return edge no longer matches its exact qualified record.
    Arrival {
        /// Source map.
        map: u16,
        /// Normalized offset of the rejected exit record.
        record: usize,
    },
    /// The map could not be built into a room.
    Room(RoomError),
    /// The map's resident spawn stream could not be resolved.
    Residents {
        /// Map whose roster failed to load.
        map: u16,
        /// Original spawn decoding or resolution failure.
        source: ResolveError,
    },
    /// The map's exit list did not decode.
    Exits {
        /// Map identifier.
        map: u16,
        /// Underlying failure.
        source: ExitError,
    },
}

impl From<RoomError> for WorldError {
    fn from(source: RoomError) -> Self {
        Self::Room(source)
    }
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arrival { map, record } => write!(
                f,
                "unqualified return arrival from map {map:#06x}, record {record:#08x}"
            ),
            Self::Room(source) => write!(f, "{source}"),
            Self::Residents { map, source } => write!(f, "map {map:#06x} residents: {source}"),
            Self::Exits { map, source } => write!(f, "map {map:#06x} exits: {source}"),
        }
    }
}

impl std::error::Error for WorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Arrival { .. } => None,
            Self::Room(source) => Some(source),
            Self::Residents { source, .. } => Some(source),
            Self::Exits { source, .. } => Some(source),
        }
    }
}

/// What one step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The player did not move.
    Stayed,
    /// The player moved within the current map.
    Walked,
    /// The player left through an exit.
    Entered {
        /// Map departed.
        from: u16,
        /// Map arrived in.
        to: u16,
    },
    /// Movement was refused by the core, which never modifies state on failure.
    Refused(Unqualified),
}

impl<'a> World<'a> {
    /// Places the player in a map at a pixel position.
    ///
    /// # Errors
    /// Refuses a map whose resident spawn stream cannot be resolved, whose room
    /// cannot be built, or whose exits do not decode. A failed roster is never
    /// replaced by an empty one.
    pub fn enter(image: &'a [u8], map: u16, x: u16, y: u16) -> Result<Self, WorldError> {
        Self::enter_with_events(image, map, x, y, new_game_flags())
    }

    /// Places the player with an explicit event-flag bitmap.
    ///
    /// The flags decide who is present and what they say, so they belong to the
    /// world rather than to each call.
    ///
    /// # Errors
    /// As [`Self::enter`].
    pub fn enter_with_events(
        image: &'a [u8],
        map: u16,
        x: u16,
        y: u16,
        events: Vec<u8>,
    ) -> Result<Self, WorldError> {
        Self::enter_with_policy(image, map, x, y, events, false)
    }

    /// Enters with the opt-in passive directional candidate (`$097C & 4 == 0`).
    ///
    /// The explicit flags are installed before residents and occupancy are built.
    /// Ordinary constructors remain conservative. The caller asserts the full
    /// [`Room::with_passive_directional_type8_special_bit_clear`] contract:
    /// ordinary special-player resolver, fixed (-8,-16) offsets and 16×16 bounds,
    /// inactive action hooks (`$0980 & $0050 == 0`), and `$097C & $0004 == 0`
    /// during ordinary walking. Source-bound arrivals suspend ordinary collision
    /// until their measured free boundary. Stop using ordinary movement if its
    /// contract changes.
    /// Host doorway interaction is not native interaction qualification.
    ///
    /// # Errors
    /// As [`Self::enter`].
    pub fn enter_candidate(
        image: &'a [u8],
        map: u16,
        x: u16,
        y: u16,
        events: Vec<u8>,
    ) -> Result<Self, WorldError> {
        Self::enter_with_policy(image, map, x, y, events, true)
    }

    fn enter_with_policy(
        image: &'a [u8],
        map: u16,
        x: u16,
        y: u16,
        events: Vec<u8>,
        candidate: bool,
    ) -> Result<Self, WorldError> {
        let present = residents(image, map, EventFlags::Bitmap(&events))
            .map_err(|source| WorldError::Residents { map, source })?;
        let actors = present
            .iter()
            .enumerate()
            .map(|(index, resident)| {
                let seed = u32::from(map)
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add(u32::try_from(index).unwrap_or(0).wrapping_mul(0x85EB_CA6B));
                Actor::new(resident.position, resident.script, resident.initial, seed)
            })
            .collect();
        let base = if candidate {
            room_candidate(image, map)?
        } else {
            room(image, map)?
        };
        let blocked = body_cells(&present);
        let built = occupy(base.clone(), &bodies(&present))?;
        // Every map in the slice has a list that decodes; a malformed one is a
        // refusal rather than a map the player silently cannot leave.
        let exits =
            ExitList::from_rom(image, map).map_err(|source| WorldError::Exits { map, source })?;
        Ok(Self {
            image,
            map,
            room: built,
            base,
            exits,
            walking: WalkingState::new(x, y),
            residents: present,
            actors,
            blocked,
            events,
            facing: Direction::Down,
            animation: AnimationState::standing(Direction::Down),
            armed: false,
            arrival: None,
        })
    }

    /// Map the player is standing in.
    #[must_use]
    pub const fn map(&self) -> u16 {
        self.map
    }

    /// Player position in pixels.
    #[must_use]
    pub fn position(&self) -> (u16, u16) {
        self.arrival
            .map_or((self.walking.x(), self.walking.y()), Arrival::position)
    }

    /// Measured return arrival currently owning the player, if any.
    ///
    /// The final free sample consumes an arrival advance, not a walking step.
    #[must_use]
    pub const fn arrival(&self) -> Option<Arrival> {
        self.arrival
    }

    /// The current map's room.
    #[must_use]
    pub const fn room(&self) -> &Room {
        &self.room.room
    }

    /// The current map's grid, in cells.
    #[must_use]
    pub const fn dimensions(&self) -> (u16, u16) {
        (self.room.width, self.room.height)
    }

    /// Direction the player faces.
    #[must_use]
    pub const fn facing(&self) -> Direction {
        self.facing
    }

    /// Turns to face a direction without moving.
    ///
    /// Holding a direction against a wall turns the player in the real game,
    /// which is how a doorway gets faced from the cell in front of it.
    pub fn face(&mut self, direction: Direction) {
        if self.arrival.is_some() {
            return;
        }
        self.facing = direction;
        self.animation = AnimationState::standing(direction);
    }

    /// The player's current ordinary frame.
    ///
    /// Advanced once per successful step from the walking state's active
    /// direction, as the qualified slice does, so a blocked step still walks
    /// in place and a released direction stands at once.
    #[must_use]
    pub const fn animation(&self) -> AnimationFrame {
        self.animation.frame()
    }

    /// Walks one frame, then applies any exit the player is standing on.
    pub fn step(&mut self, direction: Option<Direction>) -> Step {
        self.step_checked(direction).unwrap_or(Step::Stayed)
    }

    /// Walks one frame without hiding transition or occupancy build failures.
    ///
    /// A core refusal remains [`Step::Refused`], not a successful route action.
    /// On a build error the caller must discard the world: walking or actors
    /// may already have advanced. Use this API for discovery and replay.
    ///
    /// # Errors
    /// Propagates unqualified target-arrival records, destination resident-resolution,
    /// room, exit-list and actor occupancy rebuild failures.
    pub fn step_checked(&mut self, direction: Option<Direction>) -> Result<Step, WorldError> {
        let before = self.position();
        if let Some(mut arrival) = self.arrival {
            arrival.advance();
            let (x, y) = arrival.position();
            if arrival.owns_player() {
                self.arrival = Some(arrival);
            } else {
                self.arrival = None;
                // Fresh host input history, not emulation of native pad bookkeeping.
                self.walking = WalkingState::new(x, y);
                self.face(Direction::Down);
            }
            // Actors still run, but forced player movement never uses their
            // ordinary collision grid or scans/rearms reverse exits.
            self.run_actors()?;
            return Ok(if self.position() == before {
                Step::Stayed
            } else {
                Step::Walked
            });
        }
        if let Some(direction) = direction {
            self.facing = direction;
        }
        if let Err(refused) = self.walking.step(&self.room.room, FrameInput { direction }) {
            return Ok(Step::Refused(refused));
        }
        self.animation.advance(self.walking.active_direction());
        if let Some(step) = self.take_exit()? {
            return Ok(step);
        }
        self.run_actors()?;
        Ok(if self.position() == before {
            Step::Stayed
        } else {
            Step::Walked
        })
    }

    /// Walks one frame with opt-in interactive host recovery after a core refusal.
    ///
    /// Uses [`Self::step_checked`], including its scripted arrival ownership. On
    /// [`Step::Refused`], discards walking input/cadence at the identical position,
    /// stands facing the checked step's facing, and ticks residents once. Returns
    /// the original refusal, never a successful route action; no exit is tested
    /// or rearmed on that rejected frame and no collision admission is broadened.
    /// The next input starts with fresh walking latency.
    ///
    /// This is host recovery, not native movement qualification. [`Self::step`]
    /// and [`Self::step_checked`] retain the atomic-refused walking baseline for
    /// discovery and replay (including retained delayed input).
    ///
    /// # Errors
    /// As [`Self::step_checked`], including actor occupancy failures during
    /// recovery. On error the caller must discard the potentially advanced world.
    pub fn step_interactive(&mut self, direction: Option<Direction>) -> Result<Step, WorldError> {
        let step = self.step_checked(direction)?;
        if matches!(step, Step::Refused(_)) {
            let (x, y) = self.position();
            self.walking = WalkingState::new(x, y);
            self.animation = AnimationState::standing(self.facing);
            self.run_actors()?;
        }
        Ok(step)
    }

    /// Runs every resident's script for one frame and moves bodies.
    ///
    /// Each actor sees the player's cell and every other body's cell and
    /// destination as occupied, so nobody steps onto anybody. When a body's
    /// cell changes, the room is rebuilt from the base with the new cells
    /// blocked, so the player is stopped by residents wherever they are.
    fn run_actors(&mut self) -> Result<(), WorldError> {
        let (x, y) = self.position();
        let player = (x.saturating_sub(8) / 16, y.saturating_sub(16) / 16);
        for index in 0..self.actors.len() {
            let mut occupied = vec![player];
            for (other, actor) in self.actors.iter().enumerate() {
                if other != index && self.residents[other].body {
                    occupied.push(actor.collision_cell());
                    occupied.extend(actor.destination());
                }
            }
            let around = Surroundings {
                image: self.image,
                events: &self.events,
                cells: self.base.room.cells(),
                width: self.base.width,
                height: self.base.height,
                occupied: &occupied,
                player: (x, y),
                facing: self.facing,
            };
            self.actors[index].tick(&around);
        }
        let mut gone = Vec::new();
        for (index, (resident, actor)) in self.residents.iter_mut().zip(&self.actors).enumerate() {
            if actor.is_gone() {
                gone.push(index);
                continue;
            }
            resident.position = actor.position;
            resident.selector = actor.selector;
            resident.hflip = actor.hflip;
            resident.pose_age = actor.pose_age;
            resident.walking = actor.walking;
        }
        for index in gone.into_iter().rev() {
            self.residents.remove(index);
            self.actors.remove(index);
        }
        let cells = body_cells(&self.residents);
        if cells != self.blocked {
            self.room = occupy(self.base.clone(), &bodies(&self.residents))?;
            self.blocked = cells;
        }
        Ok(())
    }

    /// Residents present in the current map.
    #[must_use]
    pub fn residents(&self) -> &[Resident] {
        &self.residents
    }

    /// The event-flag bitmap in force.
    #[must_use]
    pub fn events(&self) -> &[u8] {
        &self.events
    }

    /// Talks to the resident the player is facing, if there is one.
    ///
    /// Applies the flags the conversation writes, so progression moves.
    pub fn talk(&mut self) -> Option<Conversation> {
        if self.arrival.is_some() {
            return None;
        }
        let (x, y) = self.position();
        let (dx, dy) = facing_delta(self.facing);
        let faced = (
            (x / 16).wrapping_add_signed(dx),
            (y / 16).wrapping_add_signed(dy),
        );
        let resident = self
            .residents
            .iter()
            .find(|resident| resident.cell() == faced)?
            .clone();
        let spoken = talk_to(self.image, &resident, EventFlags::Bitmap(&self.events));
        if let Conversation::Speaks { flags, .. } = &spoken {
            for flag in flags {
                let index = usize::from(flag & 0x0FFF);
                let Some(byte) = self.events.get_mut(index / 8) else {
                    continue;
                };
                // Bit 15 selects set over clear, the same encoding the loading
                // scripts and the spawn stream use.
                if flag & 0x8000 == 0 {
                    *byte &= !(1 << (index % 8));
                } else {
                    *byte |= 1 << (index % 8);
                }
            }
        }
        Some(spoken)
    }

    /// Opens the doorway the player is standing at or facing.
    ///
    /// Walking is not enough to leave most rooms. A town entrance is a single
    /// cell the player cannot stand on, with exactly one standable neighbour
    /// below it, and its trigger is one pixel tall: for the door into `$001D`
    /// exactly one player position in the whole map satisfies it, `(904, 752)`,
    /// and collision stops the player eight pixels short at `(904, 760)`.
    ///
    /// So a doorway is matched on **tiles**, not pixels. The player standing in
    /// front of it already has their collision origin in the door's own tile —
    /// the sub-tile test is the part they cannot satisfy — so interaction tests
    /// tile containment of the origin, and of the tile being faced, and ignores
    /// the fine position that walking through would need.
    pub fn interact(&mut self) -> Step {
        self.interact_checked().unwrap_or(Step::Stayed)
    }

    /// Opens a doorway without hiding destination entry failures.
    ///
    /// This host doorway operation is not native interaction qualification.
    ///
    /// # Errors
    /// Propagates unqualified target-arrival records, destination resident-resolution,
    /// room, exit-list and occupancy build failures.
    pub fn interact_checked(&mut self) -> Result<Step, WorldError> {
        if self.arrival.is_some() {
            return Ok(Step::Stayed);
        }
        let (x, y) = self.position();
        let (Some(origin_x), Some(origin_y)) = (x.checked_sub(8), y.checked_sub(16)) else {
            return Ok(Step::Stayed);
        };
        let (tile_x, tile_y) = (origin_x / 16, origin_y / 16);
        let (dx, dy) = facing_delta(self.facing);
        let faced = (
            tile_x.wrapping_add_signed(dx),
            tile_y.wrapping_add_signed(dy),
        );
        let Some(record) = self.exits.records().iter().find(|record| {
            let (left, top) = (u16::from(record.x()), u16::from(record.y()));
            let (right, bottom) = (
                left + u16::from(record.width()),
                top + u16::from(record.height()),
            );
            let covers = |(column, row): (u16, u16)| {
                (left..right).contains(&column) && (top..bottom).contains(&row)
            };
            covers((tile_x, tile_y)) || covers(faced)
        }) else {
            return Ok(Step::Stayed);
        };
        let Some(entered) = self.enter_exit(record)? else {
            return Ok(Step::Stayed);
        };
        let destination = entered.map;
        let from = self.map;
        *self = entered;
        Ok(Step::Entered {
            from,
            to: destination,
        })
    }

    /// Applies the exit under the player, if one is armed and leads into the
    /// slice.
    fn take_exit(&mut self) -> Result<Option<Step>, WorldError> {
        let (x, y) = self.position();
        // `ExitList::select` takes the bounding origin, not the player
        // position: the measured collision reference is (x - 8, y - 16).
        let (Some(x), Some(y)) = (x.checked_sub(8), y.checked_sub(16)) else {
            return Ok(None);
        };
        let origin = (x, y);
        if self.exits.select(origin.0, origin.1).is_none() {
            // Clear of every exit, so the next one may fire.
            self.armed = true;
            return Ok(None);
        }
        if !self.armed {
            return Ok(None);
        }
        self.transition_at(origin)
    }

    /// Follows the exit whose rectangle contains `origin`, if it stays in the
    /// slice.
    fn transition_at(&mut self, origin: (u16, u16)) -> Result<Option<Step>, WorldError> {
        let Some(record) = self.exits.select(origin.0, origin.1) else {
            return Ok(None);
        };
        let Some(mut entered) = self.enter_exit(record)? else {
            return Ok(None);
        };
        let destination = entered.map;
        let from = self.map;
        // Arriving stands the player facing the way they came in, as the
        // qualified slice does on its own transitions.
        entered.face(self.facing);
        *self = entered;
        Ok(Some(Step::Entered {
            from,
            to: destination,
        }))
    }

    // Both checked exit paths share source admission, initialized placement and
    // ownership. Explicit World::enter remains a raw placement operation.
    fn enter_exit(&self, record: &ExitRecord) -> Result<Option<Self>, WorldError> {
        // Validate pinned source identities before interpreting mutable operands.
        let arrival = qualified_arrival(self.map, record)?;
        let Ok(destination) = record.direct_destination() else {
            return Ok(None);
        };
        if !MAPS.contains(&destination) {
            return Ok(None);
        }
        let (x, y) = arrival.map_or(record.destination_position(), Arrival::position);
        let mut entered = self.enter_destination(destination, x, y)?;
        entered.arrival = arrival;
        Ok(Some(entered))
    }

    fn enter_destination(&self, map: u16, x: u16, y: u16) -> Result<Self, WorldError> {
        Self::enter_with_policy(
            self.image,
            map,
            x,
            y,
            self.events.clone(),
            self.base.room.passive_directional_type8_special_bit_clear(),
        )
    }
}

/// Only these two record/state witnesses have measured arrival profiles.
/// Other edges retain the legacy raw host transfer, not selector qualification.
fn qualified_arrival(map: u16, record: &ExitRecord) -> Result<Option<Arrival>, WorldError> {
    let source = record.source_range().start;
    let (offset, bytes, route) = match map {
        0x1E if source == 0x18F9B || record.raw_destination() == 0x0A => (
            0x18F9B,
            [39, 12, 1, 4, 10, 0, 0, 5, 16, 3, 240, 2],
            ReturnRoute::Town,
        ),
        0x19 if source == 0x18F42 || record.raw_destination() == 0x17 => (
            0x18F42,
            [58, 20, 1, 1, 23, 0, 0, 14, 192, 1, 96, 1],
            ReturnRoute::Stairs,
        ),
        _ => return Ok(None),
    };
    if record.source_range().start != offset || record.bytes() != &bytes {
        return Err(WorldError::Arrival {
            map,
            record: record.source_range().start,
        });
    }
    Ok(Some(Arrival::new(route)))
}

/// The residents that are bodies.
fn bodies(present: &[Resident]) -> Vec<Resident> {
    present
        .iter()
        .filter(|resident| resident.body)
        .cloned()
        .collect()
}

/// The collision cells the bodies stand on, in roster order.
fn body_cells(present: &[Resident]) -> Vec<(u16, u16)> {
    present
        .iter()
        .filter(|resident| resident.body)
        .map(Resident::collision_cell)
        .collect()
}

/// Cell offset one step in a direction.
const fn facing_delta(facing: Direction) -> (i16, i16) {
    match facing {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

/// The measured new-game flag state: 32 and 251 are set.
///
/// `EventFlags::AllClear` is deliberately not this; see its documentation.
#[must_use]
pub fn new_game_flags() -> Vec<u8> {
    let mut bitmap = vec![0u8; 512];
    for flag in [32usize, 251] {
        bitmap[flag / 8] |= 1 << (flag % 8);
    }
    bitmap
}

/// Rebuilds a room with each resident's cell made solid.
///
/// Applied by [`World::enter`] to residents that are bodies, and only those:
/// a spawn list holds more than people, and making every record solid takes
/// reachability from 19 maps to 2, because script-only records sit on the
/// cells doorway approaches need. A record that decodes to art is a body.
///
/// The cell blocked is [`Resident::collision_cell`], not the visual one:
/// movement samples at `(x - 8, y - 16)`.
///
/// # Errors
/// As [`World::enter`].
pub fn occupy(built: MapRoom, present: &[Resident]) -> Result<MapRoom, WorldError> {
    if present.is_empty() {
        return Ok(built);
    }
    let mut cells = built.room.cells().to_vec();
    for resident in present {
        let (column, row) = resident.collision_cell();
        if column >= built.width || row >= built.height {
            continue;
        }
        cells[usize::from(row) * usize::from(built.width) + usize::from(column)] = 14 << 9;
    }
    let map = built.map;
    let mut rebuilt = Room::new(built.width, built.height, cells)
        .map_err(|source| RoomError::Refused { map, source })?
        .with_material_policy(crate::qualified_policy(map, built.width, built.height))
        .map_err(|source| RoomError::Policy { map, source })?;
    if built.room.passive_directional_type8_special_bit_clear() {
        rebuilt = rebuilt.with_passive_directional_type8_special_bit_clear();
    } else if built.room.passive_directional_collision() {
        rebuilt = rebuilt.with_passive_directional_collision();
    }
    Ok(MapRoom {
        room: rebuilt,
        ..built
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resident() -> Resident {
        Resident {
            position: (24, 32),
            record: 0,
            script: None,
            body: true,
            initial: 0,
            selector: 0,
            hflip: false,
            pose_age: 0,
            walking: false,
        }
    }

    fn synthetic_world<'a>() -> World<'a> {
        let base = MapRoom {
            room: Room::new(8, 8, vec![0; 64]).unwrap(),
            map: 0xB,
            width: 8,
            height: 8,
        };
        // Synthetic exit encoding only; no ROM fixture is used by shared tests.
        let mut bytes = vec![0; 0x188B9];
        bytes[0x18016..0x18018].copy_from_slice(&0x88ACu16.to_le_bytes());
        bytes[0x188AC..0x188B8].copy_from_slice(&[3, 3, 1, 1, 0xC, 0, 0, 0, 56, 0, 64, 0]);
        bytes[0x188B8] = 0xFF;
        World {
            image: &[],
            map: 0xB,
            room: base.clone(),
            base,
            exits: ExitList::from_rom(&bytes, 0xB).unwrap(),
            walking: WalkingState::new(56, 64),
            residents: vec![],
            actors: vec![],
            blocked: vec![],
            events: new_game_flags(),
            facing: Direction::Down,
            animation: AnimationState::standing(Direction::Down),
            armed: true,
            arrival: None,
        }
    }

    #[test]
    fn return_admission_pins_every_operand_and_source_before_destination_decoding() {
        for (map, offset, bytes, route) in [
            (
                0x1E,
                0x18F9B,
                [39, 12, 1, 4, 10, 0, 0, 5, 16, 3, 240, 2],
                ReturnRoute::Town,
            ),
            (
                0x19,
                0x18F42,
                [58, 20, 1, 1, 23, 0, 0, 14, 192, 1, 96, 1],
                ReturnRoute::Stairs,
            ),
        ] {
            for changed in 0..14 {
                let at = offset + if changed == 13 { 16 } else { 0 };
                let mut image = vec![0; 0x19000];
                let pointer = 0x18000 + usize::from(map) * 2;
                image[pointer..pointer + 2]
                    .copy_from_slice(&u16::try_from(at - 0x10000).unwrap().to_le_bytes());
                image[at..at + 12].copy_from_slice(&bytes);
                image[at + 12] = 0xFF;
                if changed < 12 {
                    image[at + changed] ^= if changed == 5 { 0x80 } else { 2 };
                }
                let exits = ExitList::from_rom(&image, map).unwrap();
                let record = &exits.records()[0];
                if changed == 12 {
                    assert_eq!(
                        qualified_arrival(map, record).unwrap(),
                        Some(Arrival::new(route))
                    );
                    assert_eq!(qualified_arrival(0xB, record).unwrap(), None);
                    continue;
                }
                let origin = (u16::from(record.x()) * 16, u16::from(record.y()) * 16);
                let mut world = synthetic_world();
                world.map = map;
                world.walking = WalkingState::new(origin.0 + 8, origin.1 + 16);
                world.exits = exits;
                for error in [
                    world.interact_checked().unwrap_err(),
                    world.transition_at(origin).unwrap_err(),
                ] {
                    assert!(
                        matches!(error, WorldError::Arrival { map: source, record } if source == map && record == at)
                    );
                }
                assert_eq!(world.map(), map);
                assert!(world.arrival().is_none());
            }
        }
    }

    fn synthetic_spawn_image(map: u16, stream: &[u8]) -> Vec<u8> {
        // Synthetic stream and table only; no game data or complete ROM.
        let mut image = vec![0; 0x39002 + stream.len()];
        let pointer = 0x38000 + usize::from(map) * 2;
        image[pointer..pointer + 2].copy_from_slice(&0x9000u16.to_le_bytes());
        image[0x39002..].copy_from_slice(stream);
        image
    }

    #[test]
    fn entry_reports_resident_resolution_failure_in_every_mode() {
        let map = 0xC;
        for image in [
            vec![],                                 // Truncated table.
            vec![0; 0x3801A],                       // Absent list.
            synthetic_spawn_image(map, &[0xF0, 0]), // Unknown opcode.
            synthetic_spawn_image(map, &[0xFA, 0, 0x20, 0, 0, 0, 0, 0xFF, 0]),
        ] {
            let events = new_game_flags();
            let expected = residents(&image, map, EventFlags::Bitmap(&events)).unwrap_err();
            for result in [
                World::enter(&image, map, 56, 64),
                World::enter_with_events(&image, map, 56, 64, events.clone()),
                World::enter_candidate(&image, map, 56, 64, events.clone()),
            ] {
                let error = result.err().expect("failed roster must reject entry");
                assert_eq!(
                    error.to_string(),
                    format!("map {map:#06x} residents: {expected}")
                );
                assert!(
                    matches!(&error, WorldError::Residents { map: actual_map, source }
                    if *actual_map == map && *source == expected)
                );
                assert_eq!(
                    std::error::Error::source(&error)
                        .unwrap()
                        .downcast_ref::<ResolveError>(),
                    Some(&expected)
                );
            }
        }
    }

    #[test]
    fn occupancy_keeps_candidate_collision_without_changing_default() {
        for candidate in [false, true] {
            let mut built = synthetic_world().base;
            if candidate {
                built.room = built
                    .room
                    .with_passive_directional_type8_special_bit_clear();
            }
            let occupied = occupy(built, &[resident()]).unwrap();
            assert_eq!(
                occupied.room.passive_directional_type8_special_bit_clear(),
                candidate
            );
            assert_eq!(occupied.room.cells()[9], 14 << 9);
        }
    }

    #[test]
    fn checked_actions_report_destination_build_failure() {
        let image = synthetic_spawn_image(0xC, &[0xFF, 0]);
        let mut world = synthetic_world();
        world.image = &image;
        assert!(matches!(world.interact_checked(), Err(WorldError::Room(_))));
        assert_eq!(world.map(), 0xB);
        assert!(matches!(world.step_checked(None), Err(WorldError::Room(_))));
        assert_eq!(world.map(), 0xB);
        assert_eq!(world.interact(), Step::Stayed);
    }

    #[test]
    fn checked_transitions_report_resident_failure_without_installing_destination() {
        let image = synthetic_spawn_image(0xC, &[0xF0, 0]);
        let expected = residents(&image, 0xC, EventFlags::Bitmap(&new_game_flags())).unwrap_err();
        for candidate in [false, true] {
            let mut origin = synthetic_world();
            origin.image = &image;
            if candidate {
                origin.base.room = origin
                    .base
                    .room
                    .with_passive_directional_type8_special_bit_clear();
                origin.room = origin.base.clone();
            }
            for interact in [false, true] {
                let mut world = origin.clone();
                let result = if interact {
                    world.interact_checked()
                } else {
                    world.step_checked(None)
                };
                assert!(
                    matches!(result, Err(WorldError::Residents { map: 0xC, source })
                    if source == expected)
                );
                assert_eq!(world.map(), origin.map());
                assert_eq!(
                    world.walking.encode_snapshot(),
                    origin.walking.encode_snapshot()
                );
                assert_eq!(world.events(), origin.events());
                assert_eq!(world.residents(), origin.residents());
                assert_eq!(world.room().cells(), origin.room().cells());
                assert_eq!(
                    world.room().passive_directional_type8_special_bit_clear(),
                    candidate
                );
            }
            // The legacy wrappers still refuse entry rather than install an empty map.
            assert_eq!(origin.interact(), Step::Stayed);
            assert_eq!(origin.step(None), Step::Stayed);
            assert_eq!(origin.map(), 0xB);
        }
    }

    #[test]
    fn checked_step_reports_occupancy_rebuild_failure() {
        let mut world = synthetic_world();
        world.armed = false;
        // Invalid base metadata forces the rebuild to fail, not the walking step.
        world.base.width = 0;
        world.residents = vec![resident()];
        world.actors = vec![Actor::new((24, 32), None, 0, 0)];
        assert!(matches!(world.step_checked(None), Err(WorldError::Room(_))));
    }

    // Right's first displacement probes an unknown column; Left remains open.
    fn poised_at_unknown_boundary() -> World<'static> {
        let mut world = synthetic_world();
        world.armed = false;
        let mut cells = vec![0; 64];
        for row in 0..8 {
            cells[row * 8 + 4] = 1 << 9;
        }
        world.base.room = Room::new(8, 8, cells).unwrap();
        world.residents = vec![resident()];
        world.actors = vec![Actor::new((24, 32), None, 0, 0)];
        world.blocked = body_cells(&world.residents);
        world.room = occupy(world.base.clone(), &world.residents).unwrap();
        for _ in 0..2 {
            assert_eq!(
                world.step_checked(Some(Direction::Right)).unwrap(),
                Step::Stayed
            );
        }
        world
    }

    #[test]
    fn strict_refusal_keeps_delayed_input_locked_even_on_release_or_reversal() {
        for legacy in [false, true] {
            let mut world = poised_at_unknown_boundary();
            let before = world.clone();
            for direction in [Some(Direction::Right), None, Some(Direction::Left), None] {
                let step = if legacy {
                    world.step(direction)
                } else {
                    world.step_checked(direction).unwrap()
                };
                assert_eq!(step, Step::Refused(Unqualified::UnsupportedType(1)));
                assert_eq!(world.position(), before.position());
                assert_eq!(
                    world.walking.encode_snapshot(),
                    before.walking.encode_snapshot()
                );
                assert_eq!(world.residents(), before.residents());
                assert_eq!(world.events(), before.events());
            }
        }
    }

    #[test]
    fn interactive_refusal_resets_history_in_place_and_ticks_actors_once() {
        for direction in [None, Some(Direction::Right), Some(Direction::Left)] {
            let mut world = poised_at_unknown_boundary();
            // An armed exit underfoot would fail to build from this empty image.
            // Refusal must neither take it nor change its arming state.
            world.armed = true;
            let before = world.clone();
            let facing = direction.unwrap_or(before.facing());
            assert_eq!(
                world.step_interactive(direction).unwrap(),
                Step::Refused(Unqualified::UnsupportedType(1))
            );
            assert_eq!(world.position(), before.position());
            assert_eq!(world.map(), before.map());
            assert_eq!(world.arrival(), None);
            assert_eq!(world.armed, before.armed);
            assert_eq!(world.facing(), facing);
            assert_eq!(world.animation(), AnimationState::standing(facing).frame());
            assert_eq!(
                world.walking.encode_snapshot(),
                WalkingState::new(56, 64).encode_snapshot()
            );
            assert_eq!(world.events(), before.events());
            let mut residents = before.residents.clone();
            residents[0].pose_age += 1;
            assert_eq!(world.residents(), residents);
            assert_eq!(world.room(), before.room());
        }
    }

    #[test]
    fn interactive_refusal_allows_release_and_turning_away_but_not_unknown_tiles() {
        let mut world = poised_at_unknown_boundary();
        let position = world.position();
        // Continued pressure still refuses every attempted displacement: recovery
        // only restarts input latency, never admits the unknown column.
        for _ in 0..4 {
            assert_eq!(
                world.step_interactive(Some(Direction::Right)).unwrap(),
                Step::Refused(Unqualified::UnsupportedType(1))
            );
            assert_eq!(world.position(), position);
            for _ in 0..2 {
                assert_eq!(
                    world.step_interactive(Some(Direction::Right)).unwrap(),
                    Step::Stayed
                );
                assert_eq!(world.position(), position);
            }
        }
        assert!(matches!(
            world.step_interactive(None).unwrap(),
            Step::Refused(_)
        ));
        for _ in 0..4 {
            assert_eq!(world.step_interactive(None).unwrap(), Step::Stayed);
            assert_eq!(world.position(), position);
        }
        for _ in 0..2 {
            assert_eq!(
                world.step_interactive(Some(Direction::Left)).unwrap(),
                Step::Stayed
            );
        }
        assert_eq!(
            world.step_interactive(Some(Direction::Left)).unwrap(),
            Step::Walked
        );
        assert!(world.position().0 < position.0);
        assert_eq!(world.position().1, position.1);
    }

    #[test]
    fn interactive_arrivals_follow_checked_path_without_cancellation_or_extra_ticks() {
        for route in [ReturnRoute::Town, ReturnRoute::Stairs] {
            let mut world = synthetic_world();
            world.arrival = Some(Arrival::new(route));
            world.residents = vec![resident()];
            world.actors = vec![Actor::new((24, 32), None, 0, 0)];
            world.blocked = body_cells(&world.residents);
            let mut strict = world.clone();
            for frame in 0..100 {
                if world.arrival().is_none() {
                    break;
                }
                let direction = if frame % 2 == 0 {
                    Some(Direction::Left)
                } else {
                    None
                };
                assert_eq!(
                    world.step_interactive(direction).unwrap(),
                    strict.step_checked(direction).unwrap()
                );
                assert_eq!(world.position(), strict.position());
                assert_eq!(world.arrival(), strict.arrival());
                assert_eq!(
                    world.walking.encode_snapshot(),
                    strict.walking.encode_snapshot()
                );
                assert_eq!(world.facing(), strict.facing());
                assert_eq!(world.animation(), strict.animation());
                assert_eq!(world.residents(), strict.residents());
                assert_eq!(world.events(), strict.events());
                assert_eq!(world.armed, strict.armed);
            }
            assert_eq!(world.arrival(), None);
        }
    }

    #[test]
    fn interactive_propagates_transition_and_refused_frame_occupancy_errors() {
        let mut world = synthetic_world();
        assert!(matches!(
            world.step_interactive(None),
            Err(WorldError::Residents { .. })
        ));
        let mut world = poised_at_unknown_boundary();
        world.base.width = 0;
        world.blocked.clear();
        assert!(matches!(
            world.step_interactive(None),
            Err(WorldError::Room(_))
        ));
    }

    #[test]
    fn checked_step_keeps_core_refusals_distinct() {
        let mut world = synthetic_world();
        world.armed = false;
        world.room.room = Room::new(8, 8, vec![1 << 9; 64]).unwrap();
        for _ in 0..2 {
            world.step_checked(Some(Direction::Right)).unwrap();
        }
        assert_eq!(
            world.step_checked(Some(Direction::Right)).unwrap(),
            Step::Refused(Unqualified::UnsupportedType(1))
        );
    }
}
