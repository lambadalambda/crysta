//! A player walking the slice, moving between maps through real exits.
//!
//! Movement is [`room_core`]'s, unchanged. This adds the two things free roam
//! needs on top: the current map's room, and the exit geometry that carries the
//! player into the next one. Two exact return records additionally own the
//! player through measured initialized-to-free arrival profiles.

use crate::actors::{Actor, Surroundings, Wait};
use crate::audio::{map_selection, Audio, Cue};
use crate::plane::Plane;
use crate::residents::{residents, Resident};
use crate::scene::{Globals, Presses, View, PAD_DIRECTIONS};
use crate::{admitted, room, room_candidate, MapRoom, RoomError, WORLD_MAPS};
use assets::maps::actors::ResolveError;
use assets::maps::exits::{ExitError, ExitList, ExitRecord};
use assets::maps::flag_patches::{self, Patch};
use assets::maps::scripts::EventFlags;
use room_core::arrival::{Arrival, ReturnRoute};
use room_core::{
    AnimationFrame, AnimationState, Direction, FrameInput, Room, Unqualified, WalkingState,
};
use std::fmt;

mod contact;
mod door;
mod pots;
mod transition;
pub use pots::{CarriedPot, Carry};

/// A map the player is standing in, and where they are standing.
#[derive(Clone)]
pub struct World<'a> {
    image: &'a [u8],
    map: u16,
    /// The map's room with the cells scripts marked blocked in.
    room: MapRoom,
    /// The map's room as built, before any cell is marked.
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
    /// Collision cells marked when `room` was last rebuilt.
    blocked: Vec<(u16, u16)>,
    /// The `$7E:06C0` event-flag bitmap, owned so it can be written to.
    globals: Globals,
    /// A script the world waits on: a blocking text or choice service in a
    /// resident's own script, or in a callback running on one.
    scene: Option<Scene>,
    /// Cells scripts have patched since entry (`COP 44`): column, row, tile.
    patched: Vec<(u16, u16, u16)>,
    /// The bitmap at map entry, which decided the spawn stream's branches.
    spawn_events: Vec<u8>,
    /// The cellar's pots, when this map has them.
    pots: Option<pots::Pots>,
    /// The world map's plane the player walks, on a world map.
    plane: Option<Plane>,
    /// The actor whose contact callback runs next frame.
    touched: Option<usize>,
    /// The player's recoil from a contact.
    recoil: Option<contact::Recoil>,
    /// A wooden door the player is opening.
    opening: Option<door::Opening>,
    /// Last direction the player moved in, which is the way they face.
    facing: Direction,
    /// Which of the player's ordinary frames is showing.
    animation: AnimationState,
    /// Leaving through an exit, and arriving after one ([`transition`]).
    leaving: Option<transition::Leaving>,
    arriving: Option<transition::Arriving>,
    /// The brightness since the last load, which the fade-in raises a step
    /// a frame.
    dawn: u8,
    /// Whether exits walk the player out and in (`update`) rather than
    /// load at once (the stepping API the route discovery uses).
    animate: bool,
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
    /// The pot component refused even a neutral frame.
    Pot(room_core::pots::Error),
    /// The flag-gated load patch table did not decode.
    LoadPatches {
        /// Map being loaded.
        map: u16,
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
            Self::Pot(source) => write!(f, "{source}"),
            Self::LoadPatches { map } => write!(f, "map {map:#06x}: load patch table"),
        }
    }
}

impl std::error::Error for WorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Arrival { .. } | Self::LoadPatches { .. } => None,
            Self::Room(source) => Some(source),
            Self::Residents { source, .. } => Some(source),
            Self::Exits { source, .. } => Some(source),
            Self::Pot(source) => Some(source),
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
        let mut world = Self::enter_with_policy(image, map, x, y, events, false)?;
        world.finish_load()?;
        Ok(world)
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
        let mut world = Self::enter_with_policy(image, map, x, y, events, true)?;
        world.finish_load()?;
        Ok(world)
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
        let actors: Vec<Actor> = present
            .iter()
            .enumerate()
            .map(|(index, resident)| {
                let seed = u32::from(map)
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add(u32::try_from(index).unwrap_or(0).wrapping_mul(0x85EB_CA6B));
                Actor::for_resident(image, map, resident, seed)
            })
            .collect();
        let base = if candidate {
            room_candidate(image, map)?
        } else {
            room(image, map)?
        };
        let blocked = blocking_cells(&present, &actors);
        let built = occupy_cells(base.clone(), &blocked)?;
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
            spawn_events: events.clone(),
            globals: Globals::with_events(events),
            scene: None,
            patched: Vec::new(),
            pots: None,
            plane: None,
            touched: None,
            recoil: None,
            opening: None,
            facing: Direction::Down,
            animation: AnimationState::standing(Direction::Down),
            armed: false,
            arrival: None,
            leaving: None,
            arriving: None,
            dawn: 15,
            animate: false,
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
        // A script holds the world until [`Self::update`] answers it.
        if self.scene.is_some() {
            return Ok(Step::Stayed);
        }
        let before = self.position();
        if let Some(plane) = &mut self.plane {
            if let Some(direction) = direction.filter(|_| !plane.busy()) {
                self.facing = direction;
            }
            let (x, y) = plane.tick(before, direction);
            let settled = !plane.busy();
            self.walking = WalkingState::new(x, y);
            self.animation
                .advance((before != (x, y)).then_some(self.facing));
            // Exits fire from a settled cell, as on the room walker; arriving
            // on one disarms it until the player steps clear.
            if settled {
                if let Some(step) = self.take_exit()? {
                    return Ok(step);
                }
            }
            self.run_actors()?;
            return Ok(if (x, y) == before {
                Step::Stayed
            } else {
                Step::Walked
            });
        }
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
    /// destination as occupied, so nobody steps onto anybody. When a marked
    /// cell changes, the room is rebuilt from the base with the new cells
    /// blocked.
    fn run_actors(&mut self) -> Result<(), WorldError> {
        let (x, y) = self.position();
        for index in 0..self.actors.len() {
            let occupied = occupied_by_others(&self.actors, &self.residents, index, (x, y));
            let mut around = surroundings(
                self.image,
                &mut self.globals,
                &self.base,
                &occupied,
                (x, y),
                self.facing,
            );
            self.actors[index].tick(&mut around);
            if self.actors[index].blocked().is_some() {
                // `$80:8C4A`/`8B85` wait inside the handler: nobody after
                // this actor runs until the window is answered.
                break;
            }
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
            resident.hidden = actor.hidden;
        }
        for index in gone.into_iter().rev() {
            self.residents.remove(index);
            self.actors.remove(index);
        }
        // Found after removals, which move indices.
        if let Some(index) = self
            .actors
            .iter()
            .position(|actor| actor.blocked().is_some())
        {
            self.scene = Some(Scene::Own(index));
        }
        self.apply_patches()?;
        self.spawn_actors();
        let cells = blocking_cells(&self.residents, &self.actors);
        if cells != self.blocked {
            self.room = occupy_cells(self.base.clone(), &cells)?;
            self.blocked = cells;
        }
        Ok(())
    }

    /// Writes the tile patches scripts queued into the base room, and the
    /// walkable room over it.
    fn apply_patches(&mut self) -> Result<(), WorldError> {
        if self.globals.patches.is_empty() {
            return Ok(());
        }
        let mut cells = self.base.room.cells().to_vec();
        for (column, row, tile) in std::mem::take(&mut self.globals.patches) {
            if column >= self.base.width || row >= self.base.height {
                continue;
            }
            cells[usize::from(row) * usize::from(self.base.width) + usize::from(column)] =
                self.base.patch_word(tile);
            self.patched.retain(|&(c, r, _)| (c, r) != (column, row));
            self.patched.push((column, row, tile & 0x1FF));
        }
        self.base = self.base.with_cells(cells)?;
        self.room = occupy_cells(self.base.clone(), &self.blocked)?;
        Ok(())
    }

    /// The end of a map load (`$8D:8C0C`): the flag-gated tile patches
    /// ([`assets::maps::flag_patches`]) go on the grid as it stands, the
    /// loading script's music is selected, then the pots are found.
    fn finish_load(&mut self) -> Result<(), WorldError> {
        self.apply_load_patches()?;
        let selection = map_selection(self.image, self.map, &self.globals.events);
        self.globals.audio.load_map(selection);
        self.pots = pots::Pots::at_entry(self.map, self.base.room.cells());
        self.plane = WORLD_MAPS.contains(&self.map).then(|| {
            let cells = self
                .base
                .room
                .cells()
                .iter()
                .map(|&word| (word & 0xFF) as u8)
                .collect();
            Plane::new(cells, self.base.width, self.base.height)
        });
        Ok(())
    }

    /// Applies the load's flag-gated patches on the first layer, in order: a
    /// block copy reads the grid as the entries before it left it, word by
    /// word. The second layer's are left out: only the first layer's
    /// collision is modelled.
    fn apply_load_patches(&mut self) -> Result<(), WorldError> {
        let events = &self.globals.events;
        let flag = |flag: u16| EventFlags::Bitmap(events).get(flag) == Some(true);
        let patches = flag_patches::for_map(self.image, self.map, flag)
            .ok_or(WorldError::LoadPatches { map: self.map })?;
        let (width, height) = (self.base.width, self.base.height);
        for patch in patches.iter().filter(|patch| !patch.second_layer) {
            match patch.patch {
                Patch::Tile { cell, tile } => {
                    self.globals
                        .patches
                        .push((cell.0.into(), cell.1.into(), tile));
                }
                Patch::Copy { from, size, to } => {
                    for dy in 0..u16::from(size.1) {
                        for dx in 0..u16::from(size.0) {
                            let (column, row) = (u16::from(from.0) + dx, u16::from(from.1) + dy);
                            if column >= width || row >= height {
                                continue;
                            }
                            let word = self.base.room.cells()
                                [usize::from(row) * usize::from(width) + usize::from(column)];
                            self.globals.patches.push((
                                u16::from(to.0) + dx,
                                u16::from(to.1) + dy,
                                word & 0x1FF,
                            ));
                            // Each word lands before the next is read.
                            self.apply_patches()?;
                        }
                    }
                }
            }
        }
        self.apply_patches()
    }

    /// Adds the actors scripts spawned (`COP A2`); they run from next frame.
    fn spawn_actors(&mut self) {
        for (script, flags, position) in std::mem::take(&mut self.globals.spawns) {
            let runtime = u32::try_from(script).map_or(0, |script| 0x80_0000 | script);
            let mut actor = Actor::new(position, Some(runtime), 0, 1);
            actor.set_map(self.map);
            actor.hidden = flags & 0x8000 != 0;
            self.residents.push(Resident {
                position,
                record: 0,
                script: Some(runtime),
                body: false,
                initial: 0,
                selector: 0,
                hflip: false,
                pose_age: 0,
                walking: false,
                descriptor: None,
                hidden: actor.hidden,
            });
            self.actors.push(actor);
        }
    }

    /// Cells patched in this map, by scripts and by the load's flag table,
    /// with those carried from the maps before on the same layer: column,
    /// row and tile.
    #[must_use]
    pub fn patched_cells(&self) -> &[(u16, u16, u16)] {
        &self.patched
    }

    /// Residents present in the current map.
    #[must_use]
    pub fn residents(&self) -> &[Resident] {
        &self.residents
    }

    /// The event-flag bitmap in force.
    #[must_use]
    pub fn events(&self) -> &[u8] {
        &self.globals.events
    }

    /// The event-flag bitmap at map entry, which decided who spawned and in
    /// what order; see [`crate::art::residents_art`].
    #[must_use]
    pub fn spawn_events(&self) -> &[u8] {
        &self.spawn_events
    }

    /// One frame of play from the player's pad: the dialogue window takes
    /// presses first, then walking, then interaction.
    ///
    /// While a script waits in a blocking service (`COP 1F`, `COP 1A`) the
    /// world stands still and presses only answer it; once answered, the
    /// script goes on in the same frame. Otherwise a press acknowledges a
    /// cooperative page (`COP 20`), the pad moves the player unless a script
    /// locked it (`COP 2A`), and a confirm press on nothing else talks to the
    /// faced resident's callback or opens a doorway.
    ///
    /// Returns the walking step and, when the confirm press opened a doorway
    /// rather than a conversation, that step too.
    ///
    /// # Errors
    /// As [`Self::step_interactive`] and [`Self::interact_checked`].
    pub fn update(
        &mut self,
        direction: Option<Direction>,
        presses: Presses,
    ) -> Result<(Step, Option<Step>), WorldError> {
        self.animate = true;
        let steps = self.frame(direction, presses);
        self.animate = false;
        let steps = steps?;
        // A transfer a script queued this frame loads at its end.
        let steps = match self.follow_transfer()? {
            Some(entered) => (entered, None),
            None => steps,
        };
        self.globals.audio.end_frame();
        Ok(steps)
    }

    fn frame(
        &mut self,
        direction: Option<Direction>,
        presses: Presses,
    ) -> Result<(Step, Option<Step>), WorldError> {
        self.globals.pad = direction.map_or(0, |direction| match direction {
            Direction::Right => 0x0100,
            Direction::Left => 0x0200,
            Direction::Down => 0x0400,
            Direction::Up => 0x0800,
        });
        self.dawn = (self.dawn + 1).min(15);
        if let Some(step) = self.transition_frame()? {
            self.apply_patches()?;
            return Ok((step, None));
        }
        if self.scene.is_some() {
            self.answer_scene(presses);
            self.apply_patches()?;
            return Ok((Step::Stayed, None));
        }
        if let Some(step) = self.contact_frame()? {
            self.apply_patches()?;
            return Ok((step, None));
        }
        if self.opening.is_some() {
            self.door_frame()?;
            self.apply_patches()?;
            return Ok((Step::Stayed, None));
        }
        let busy = self.globals.dialogue.busy();
        self.globals.dialogue.press(presses);
        let locked = self.globals.input_mask & PAD_DIRECTIONS != 0;
        let direction = direction.filter(|_| !locked);
        let lift = presses.confirm && !locked && !busy && !self.globals.dialogue.busy();
        if let Some(step) = self.pot_frame(direction, lift)? {
            self.apply_patches()?;
            return Ok((step, None));
        }
        let step = self.step_interactive(direction)?;
        // On the plane, not while arriving or mid-step.
        let free = !busy
            && self.scene.is_none()
            && !self.globals.dialogue.busy()
            && !self.plane.as_ref().is_some_and(Plane::busy);
        // A talks while a script holds the pad (`$FF50` leaves A free), but
        // doors and the host doorway action wait: a reaction's lock keeps
        // Ark where he is, as natively.
        let opened = if presses.confirm && free && !self.talk() && !locked && !self.open_door() {
            Some(self.interact_checked()?)
        } else {
            None
        };
        let entered =
            matches!(step, Step::Entered { .. }) || matches!(opened, Some(Step::Entered { .. }));
        if !entered && !self.in_transition() {
            self.touch();
        }
        self.apply_patches()?;
        Ok((step, opened))
    }

    /// The music and sound effect requests made since the last call.
    pub fn take_cues(&mut self) -> Vec<Cue> {
        self.globals.audio.take()
    }

    /// Residents whose scripts stopped at something the interpreter does not
    /// model: record and the normalized offset where each stopped.
    #[must_use]
    pub fn frozen_scripts(&self) -> Vec<(usize, usize)> {
        self.residents
            .iter()
            .zip(&self.actors)
            .filter_map(|(resident, actor)| Some((resident.record, actor.frozen_at()?)))
            .collect()
    }

    /// Hits the resident spawned from `record`, as a thrown object would;
    /// for hosts and tests. Returns whether it could be hit.
    pub fn strike(&mut self, record: usize) -> bool {
        self.residents
            .iter()
            .position(|resident| resident.record == record)
            .is_some_and(|index| self.actors[index].strike())
    }

    /// Puts the player at a position in this map, standing; for hosts and
    /// tests, like a debug warp. Scripts and patches stay as they are.
    pub fn place(&mut self, x: u16, y: u16) {
        self.opening = None;
        self.walking = WalkingState::new(x, y);
        self.arrival = None;
        self.leaving = None;
        self.arriving = None;
    }

    /// Sets an event flag as `COP 07` would; for hosts and tests.
    pub fn set_flag(&mut self, flag: u16) {
        self.globals.write_flag(0x8000 | flag);
    }

    /// Whether a script has locked the pad's directions (`COP 2A`).
    #[must_use]
    pub const fn pad_locked(&self) -> bool {
        self.globals.input_mask & PAD_DIRECTIONS != 0
    }

    /// Items scripts have given, in order.
    #[must_use]
    pub fn items(&self) -> &[u8] {
        &self.globals.items
    }

    /// The dialogue window, if a script has something on it.
    #[must_use]
    pub fn dialogue(&self) -> Option<View<'_, assets::text::DialoguePage>> {
        self.globals.dialogue.view()
    }

    /// Whether a script holds the world still.
    #[must_use]
    pub const fn in_scene(&self) -> bool {
        self.scene.is_some()
    }

    /// The interaction dispatcher (`$87:923F`): runs the callback of the
    /// resident the player faces, when that resident takes interaction.
    /// Returns whether one did.
    fn talk(&mut self) -> bool {
        if self.arrival.is_some() {
            return false;
        }
        let (x, y) = self.position();
        let (dx, dy) = facing_delta(self.facing);
        let faced = (
            (x / 16).wrapping_add_signed(dx),
            (y / 16).wrapping_add_signed(dy),
        );
        // Natively `$87:C783` takes the first actor hit whose `+$04` has bit 8
        // and checks only that one (`$87:93B9`); `+$04` is not modelled, so
        // the first interactable actor on the cell stands in. It differs only
        // where a targetable actor without interaction stands in front of an
        // interactable one; the spear's display lacks bit 8.
        let Some(index) = self
            .residents
            .iter()
            .zip(&self.actors)
            .position(|(resident, actor)| {
                resident.cell() == faced && actor.interactable(self.facing)
            })
        else {
            return false;
        };
        let player = self.position();
        let occupied = occupied_by_others(&self.actors, &self.residents, index, player);
        let mut around = surroundings(
            self.image,
            &mut self.globals,
            &self.base,
            &occupied,
            player,
            self.facing,
        );
        let blocked = self.actors[index].run_callback(&mut around);
        self.scene = blocked.map(|(pc, wait)| Scene::Callback {
            actor: index,
            pc,
            wait,
        });
        true
    }

    /// Feeds presses to the window a script waits on, and resumes it once
    /// answered.
    fn answer_scene(&mut self, presses: Presses) {
        let Some(scene) = self.scene else {
            return;
        };
        let answer = self.globals.dialogue.press(presses);
        let wait = match scene {
            Scene::Own(index) => self.actors[index].blocked(),
            Scene::Callback { wait, .. } => Some(wait),
        };
        let answer = match wait {
            Some(Wait::Text) if !self.globals.dialogue.busy() => 0,
            Some(Wait::Choice(_)) => match answer {
                Some(answer) => answer,
                None => return,
            },
            Some(Wait::Text) => return,
            None => {
                self.scene = None;
                return;
            }
        };
        self.scene = None;
        match scene {
            Scene::Own(index) => {
                let player = self.position();
                let occupied = occupied_by_others(&self.actors, &self.residents, index, player);
                let mut around = surroundings(
                    self.image,
                    &mut self.globals,
                    &self.base,
                    &occupied,
                    player,
                    self.facing,
                );
                self.actors[index].resume(answer, &mut around);
                if self.actors[index].blocked().is_some() {
                    self.scene = Some(Scene::Own(index));
                }
            }
            Scene::Callback { actor, pc, wait } => {
                let player = self.position();
                let occupied = occupied_by_others(&self.actors, &self.residents, actor, player);
                let mut around = surroundings(
                    self.image,
                    &mut self.globals,
                    &self.base,
                    &occupied,
                    player,
                    self.facing,
                );
                let blocked = self.actors[actor].resume_callback(pc, wait, answer, &mut around);
                self.scene = blocked.map(|(pc, wait)| Scene::Callback { actor, pc, wait });
            }
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
        // An occupied doorway stays shut: D's hidden gate stamps the house
        // exit until `$26`. Only a blocked cell of the doorway itself counts.
        let (left, top) = (u16::from(record.x()), u16::from(record.y()));
        let in_doorway = (left..left + u16::from(record.width())).contains(&faced.0)
            && (top..top + u16::from(record.height())).contains(&faced.1);
        if in_doorway && self.blocked.contains(&faced) {
            return Ok(Step::Stayed);
        }
        // Nor does a closed door open by hand.
        let cell = usize::from(top) * usize::from(self.base.width) + usize::from(left);
        if self
            .base
            .room
            .cells()
            .get(cell)
            .is_some_and(|word| word >> 9 & 31 == CLOSED_DOOR)
        {
            return Ok(Step::Stayed);
        }
        if self.animate {
            self.leave(&record.clone());
            return Ok(Step::Stayed);
        }
        let Some(entered) = self.enter_exit(record, false)? else {
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
        if self.animate {
            // Leaving starts; this frame walks on as any other.
            if let Some(record) = self.exits.select(origin.0, origin.1).cloned() {
                self.leave(&record);
            }
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
        let Some(mut entered) = self.enter_exit(record, false)? else {
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
    fn enter_exit(&self, record: &ExitRecord, departed: bool) -> Result<Option<Self>, WorldError> {
        // Validate pinned source identities before interpreting mutable operands.
        let arrival = qualified_arrival(self.map, record)?;
        let Ok(destination) = record.direct_destination() else {
            return Ok(None);
        };
        if !admitted(destination) {
            return Ok(None);
        }
        let (x, y) = match arrival {
            Some(arrival) => arrival.position(),
            // Stairs settle at the raw anchor plus (8,16): the adjustment
            // (`$8D:8985`, down (-14,-23), up (8,-6)) and the stair walk
            // back by the same; [`transition`] plays the walk.
            None if matches!(record.selector(), STAIRS | STAIRS_UP) => {
                let (x, y) = record.destination_position();
                (x + 8, y + 16)
            }
            // A world map spawns the player 8 to the right of the anchor;
            // the plane's arrival walk takes it the rest of the way.
            None if WORLD_MAPS.contains(&destination) => {
                let (x, y) = record.destination_position();
                (x + 8, y)
            }
            None => record.destination_position(),
        };
        // Leaving plays the exit's sound before the load (`$8D:8872`);
        // landing on stairs plays their step (`COP 36 17`, `$84:BA19`).
        let mut audio = self.globals.audio.clone();
        if !departed {
            audio.sound_port3(EXIT_SOUND);
            audio.flush();
        }
        if matches!(record.selector(), STAIRS | STAIRS_UP) {
            audio.sound_port3(STAIR_SOUND);
        }
        let mut entered = self.enter_destination(destination, x, y, audio)?;
        entered.arrival = arrival;
        Ok(Some(entered))
    }

    /// Loads the next map. `$8D:8AED` clears the map-local flags (0..31)
    /// and counters first; the other flags and the items carry over.
    fn enter_destination(
        &self,
        map: u16,
        x: u16,
        y: u16,
        audio: Audio,
    ) -> Result<Self, WorldError> {
        let mut events = self.globals.events.clone();
        events[..4].fill(0);
        let mut entered = Self::enter_with_policy(
            self.image,
            map,
            x,
            y,
            events,
            self.base.room.passive_directional_type8_special_bit_clear(),
        )?;
        entered.globals.items.clone_from(&self.globals.items);
        entered.globals.scratch.clone_from(&self.globals.scratch);
        entered.globals.audio = audio;
        // The same first layer is not reloaded: its patches stay, and the
        // load's own patches go on top.
        if entered.base.layer_source == self.base.layer_source {
            entered.globals.patches.clone_from(&self.patched);
            entered.apply_patches()?;
        }
        entered.finish_load()?;
        Ok(entered)
    }
}

/// Port 3's sounds for leaving a map, and for a stair step.
const EXIT_SOUND: u8 = 0x4D;
const STAIR_SOUND: u8 = 0x17;
/// The stair transfer selectors (decimal): 14 down, 13 back up.
pub(super) const STAIRS: u8 = 14;
pub(super) const STAIRS_UP: u8 = 13;
/// A closed door's collision type: C's blue door stands on its stairs with
/// it (`$0B81`) until it breaks.
const CLOSED_DOOR: u16 = 5;

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

/// Cells that block the player: those visible actors' scripts marked
/// (`COP 3B`, `COP 3D`), as natively (`$80:BE8E`), and the cells of bodies
/// that [hold them](Actor::holds_its_cell) unmarked. C's blue door stands on
/// its closed-door cell unmarked, which the pots' throw lane needs
/// (`docs/pandora-pots.md`).
fn blocking_cells(residents: &[Resident], actors: &[Actor]) -> Vec<(u16, u16)> {
    residents
        .iter()
        .zip(actors)
        .filter(|(resident, _)| !resident.hidden)
        .flat_map(|(resident, actor)| {
            let own = if resident.body && actor.holds_its_cell() {
                Some(resident.collision_cell())
            } else {
                actor.stamp()
            };
            own.into_iter().chain(actor.stamps().iter().copied())
        })
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

/// A script the world waits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scene {
    /// A resident's own script, blocked in it.
    Own(usize),
    /// A callback running on a resident, blocked at `pc`.
    Callback {
        /// The resident it runs on.
        actor: usize,
        /// Where it goes on.
        pc: usize,
        /// What it waits for.
        wait: Wait,
    },
}

/// What one actor sees this frame.
fn surroundings<'s>(
    image: &'s [u8],
    globals: &'s mut Globals,
    base: &'s MapRoom,
    occupied: &'s [(u16, u16)],
    player: (u16, u16),
    facing: Direction,
) -> Surroundings<'s> {
    Surroundings {
        image,
        globals,
        cells: base.room.cells(),
        width: base.width,
        height: base.height,
        occupied,
        player,
        facing,
    }
}

/// Cells one actor must not step into: the player's and every other body's,
/// where it stands and where it is stepping. Wider than what blocks the
/// player ([`blocking_cells`]): walkers keep clear of every body.
fn occupied_by_others(
    actors: &[Actor],
    residents: &[Resident],
    index: usize,
    (x, y): (u16, u16),
) -> Vec<(u16, u16)> {
    let mut occupied = vec![(x.saturating_sub(8) / 16, y.saturating_sub(16) / 16)];
    for (other, actor) in actors.iter().enumerate() {
        if other == index || actor.hidden {
            continue;
        }
        if residents[other].body {
            occupied.push(actor.collision_cell());
            occupied.extend(actor.destination());
        } else {
            occupied.extend(actor.stamp());
        }
    }
    occupied
}

/// The flags a new game starts the bedroom with, before the wake-up: only
/// `$FB`, which name entry sets (`$87:80EC`). Elle's script sets `$20`.
#[must_use]
pub fn fresh_game_flags() -> Vec<u8> {
    let mut flags = new_game_flags();
    flags[0x20 / 8] &= !(1 << (0x20 % 8));
    flags
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
/// The world blocks the cells scripts mark instead ([`blocking_cells`]).
///
/// The cell blocked is [`Resident::collision_cell`], not the visual one:
/// movement samples at `(x - 8, y - 16)`.
///
/// # Errors
/// As [`World::enter`].
#[cfg(test)]
fn occupy(built: MapRoom, present: &[Resident]) -> Result<MapRoom, WorldError> {
    let cells: Vec<_> = present.iter().map(Resident::collision_cell).collect();
    occupy_cells(built, &cells)
}

/// [`occupy`] for any set of cells.
fn occupy_cells(built: MapRoom, blocked: &[(u16, u16)]) -> Result<MapRoom, WorldError> {
    if blocked.is_empty() {
        return Ok(built);
    }
    let mut cells = built.room.cells().to_vec();
    for &(column, row) in blocked {
        if column >= built.width || row >= built.height {
            continue;
        }
        cells[usize::from(row) * usize::from(built.width) + usize::from(column)] = 14 << 9;
    }
    Ok(built.with_cells(cells)?)
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
            descriptor: None,
            hidden: false,
        }
    }

    fn synthetic_world<'a>() -> World<'a> {
        let base = MapRoom {
            room: Room::new(8, 8, vec![0; 64]).unwrap(),
            map: 0xB,
            width: 8,
            height: 8,
            attributes: vec![0; 512],
            layer_source: 0,
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
            globals: Globals::with_events(new_game_flags()),
            scene: None,
            patched: Vec::new(),
            pots: None,
            plane: None,
            touched: None,
            recoil: None,
            opening: None,
            spawn_events: new_game_flags(),
            facing: Direction::Down,
            animation: AnimationState::standing(Direction::Down),
            armed: true,
            arrival: None,
            leaving: None,
            arriving: None,
            dawn: 15,
            animate: false,
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
        world.actors = vec![Actor::new((24, 32), None, 0, 0).marked()];
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
        world.actors = vec![Actor::new((24, 32), None, 0, 0).marked()];
        world.blocked = blocking_cells(&world.residents, &world.actors);
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
            world.actors = vec![Actor::new((24, 32), None, 0, 0).marked()];
            world.blocked = blocking_cells(&world.residents, &world.actors);
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

    #[test]
    fn a_patch_writes_the_tile_under_its_attribute_and_is_listed() {
        let mut world = synthetic_world();
        world.base.attributes[0x1A7] = 0x0E | 0x80;
        world.globals.patches.push((3, 2, 0x1A7));
        world.globals.patches.push((3, 2, 0x5A7));
        world.apply_patches().unwrap();
        // The attribute's low seven bits above the tile; the last patch of a
        // cell is the one listed.
        assert_eq!(world.base.room.cells()[2 * 8 + 3], 0x0E << 9 | 0x1A7);
        assert_eq!(world.room.room.cells()[2 * 8 + 3], 0x0E << 9 | 0x1A7);
        assert_eq!(world.patched_cells(), [(3, 2, 0x1A7)]);
        assert!(world.globals.patches.is_empty());
    }
}
