//! A player walking the slice, moving between maps through real exits.
//!
//! Movement is [`room_core`]'s, unchanged. This adds the two things free roam
//! needs on top: the current map's room, and the exit geometry that carries the
//! player into the next one.

use crate::actors::{Actor, Surroundings};
use crate::residents::{residents, talk_to, Conversation, Resident};
use crate::{room, MapRoom, RoomError, MAPS};
use assets::maps::exits::{ExitError, ExitList};
use assets::maps::scripts::EventFlags;
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
    /// The map could not be built into a room.
    Room(RoomError),
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
            Self::Room(source) => write!(f, "{source}"),
            Self::Exits { map, source } => write!(f, "map {map:#06x} exits: {source}"),
        }
    }
}

impl std::error::Error for WorldError {}

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
    /// Refuses a map that cannot be built into a room or whose exits do not
    /// decode.
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
        let present = residents(image, map, EventFlags::Bitmap(&events)).unwrap_or_default();
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
        let base = room(image, map)?;
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
        (self.walking.x(), self.walking.y())
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
        let before = self.position();
        if let Some(direction) = direction {
            self.facing = direction;
        }
        if let Err(refused) = self.walking.step(&self.room.room, FrameInput { direction }) {
            return Step::Refused(refused);
        }
        self.animation.advance(self.walking.active_direction());
        if let Some(step) = self.take_exit() {
            return step;
        }
        self.run_actors();
        if self.position() == before {
            Step::Stayed
        } else {
            Step::Walked
        }
    }

    /// Runs every resident's script for one frame and moves bodies.
    ///
    /// Each actor sees the player's cell and every other body's cell and
    /// destination as occupied, so nobody steps onto anybody. When a body's
    /// cell changes, the room is rebuilt from the base with the new cells
    /// blocked, so the player is stopped by residents wherever they are.
    fn run_actors(&mut self) {
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
            if let Ok(rebuilt) = occupy(self.base.clone(), &bodies(&self.residents)) {
                self.room = rebuilt;
                self.blocked = cells;
            }
        }
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
        let (x, y) = self.position();
        let (Some(origin_x), Some(origin_y)) = (x.checked_sub(8), y.checked_sub(16)) else {
            return Step::Stayed;
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
            return Step::Stayed;
        };
        let Ok(destination) = record.direct_destination() else {
            return Step::Stayed;
        };
        if !MAPS.contains(&destination) {
            return Step::Stayed;
        }
        let (arrival_x, arrival_y) = record.destination_position();
        match Self::enter(self.image, destination, arrival_x, arrival_y) {
            Ok(mut entered) => {
                let from = self.map;
                entered.events.clone_from(&self.events);
                *self = entered;
                Step::Entered {
                    from,
                    to: destination,
                }
            }
            Err(_) => Step::Stayed,
        }
    }

    /// Applies the exit under the player, if one is armed and leads into the
    /// slice.
    fn take_exit(&mut self) -> Option<Step> {
        let (x, y) = self.position();
        // `ExitList::select` takes the bounding origin, not the player
        // position: the measured collision reference is (x - 8, y - 16).
        let origin = (x.checked_sub(8)?, y.checked_sub(16)?);
        if self.exits.select(origin.0, origin.1).is_none() {
            // Clear of every exit, so the next one may fire.
            self.armed = true;
            return None;
        }
        if !self.armed {
            return None;
        }
        self.transition_at(origin)
    }

    /// Follows the exit whose rectangle contains `origin`, if it stays in the
    /// slice.
    fn transition_at(&mut self, origin: (u16, u16)) -> Option<Step> {
        let record = self.exits.select(origin.0, origin.1)?;
        let destination = record.direct_destination().ok()?;
        // An exit out of the slice is refused as movement. The player does not
        // pass, rather than arriving in a map that was never loaded.
        if !MAPS.contains(&destination) {
            return None;
        }
        let (arrival_x, arrival_y) = record.destination_position();
        let entered = Self::enter_with_events(
            self.image,
            destination,
            arrival_x,
            arrival_y,
            self.events.clone(),
        )
        .ok()?;
        let from = self.map;
        self.map = entered.map;
        self.room = entered.room;
        self.base = entered.base;
        self.exits = entered.exits;
        self.walking = entered.walking;
        self.residents = entered.residents;
        self.actors = entered.actors;
        self.blocked = entered.blocked;
        // Arriving stands the player facing the way they came in, as the
        // qualified slice does on its own transitions.
        self.animation = AnimationState::standing(self.facing);
        self.armed = false;
        Some(Step::Entered {
            from,
            to: destination,
        })
    }
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
    let rebuilt = Room::new(built.width, built.height, cells)
        .map_err(|source| RoomError::Refused { map, source })?
        .with_material_policy(crate::qualified_policy(map, built.width, built.height))
        .map_err(|source| RoomError::Policy { map, source })?;
    Ok(MapRoom {
        room: rebuilt,
        ..built
    })
}
