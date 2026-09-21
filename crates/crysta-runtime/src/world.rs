//! A player walking the slice, moving between maps through real exits.
//!
//! Movement is [`room_core`]'s, unchanged. This adds the two things free roam
//! needs on top: the current map's room, and the exit geometry that carries the
//! player into the next one.

use crate::{room, MapRoom, RoomError, MAPS};
use assets::maps::exits::{ExitError, ExitList};
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState};
use std::fmt;

/// A map the player is standing in, and where they are standing.
#[derive(Clone)]
pub struct World<'a> {
    image: &'a [u8],
    map: u16,
    room: MapRoom,
    exits: ExitList,
    walking: WalkingState,
    /// Last direction the player moved in, which is the way they face.
    facing: Direction,
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
        let built = room(image, map)?;
        // Every map in the slice has a list that decodes; a malformed one is a
        // refusal rather than a map the player silently cannot leave.
        let exits =
            ExitList::from_rom(image, map).map_err(|source| WorldError::Exits { map, source })?;
        Ok(Self {
            image,
            map,
            room: built,
            exits,
            walking: WalkingState::new(x, y),
            facing: Direction::Down,
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
        match self.take_exit() {
            Some(step) => step,
            None if self.position() == before => Step::Stayed,
            None => Step::Walked,
        }
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
        let (dx, dy) = match self.facing {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        };
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
            Ok(entered) => {
                let from = self.map;
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
        let entered = Self::enter(self.image, destination, arrival_x, arrival_y).ok()?;
        let from = self.map;
        self.map = entered.map;
        self.room = entered.room;
        self.exits = entered.exits;
        self.walking = entered.walking;
        self.armed = false;
        Some(Step::Entered {
            from,
            to: destination,
        })
    }
}
