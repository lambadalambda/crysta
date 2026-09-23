//! A bounded interpreter for a resident's ordinary loop.
//!
//! The script walker reads a script once and reports what it contains. A
//! resident who wanders needs it *run*: the loop re-selects poses, draws a
//! random direction, steps, and waits, for as long as they stand there. This
//! executes the handful of services those loops use and the two native
//! opcodes that close them, and freezes a resident at anything else rather
//! than guessing.
//!
//! What is modelled is what `$80:8E56` decides -- the rectangle, the random
//! draw, the probe -- and what `$80:8F32` selects. What is approximated is
//! timing outside a source-admitted COP26 action: other tiles use
//! [`STEP_FRAMES`] and step waits use [`WAIT_FRAMES`]. An admitted action
//! lasts as long as the resident's own walking or idle list; see `cadence`.

mod cadence;

use assets::maps::actor_script::{
    self, chained_condition_holds, chained_condition_length, flag_branch_taken, BRANCH_ON_FLAG,
    CHAINED_BRANCH, CHAINED_DESPAWN,
};
use assets::maps::scripts::EventFlags;
use room_core::Direction;

/// Selects an animation sequence; one operand byte.
const SELECT_POSE: u8 = 0x80;
/// Clears the horizontal mirror.
const CLEAR_HFLIP: u8 = 0xB6;
/// Sets the horizontal mirror.
const SET_HFLIP: u8 = 0xB7;
/// Resolves the pose and yields until it is done.
const WAIT: u8 = 0x8E;
/// Waits for a step to finish, or for one record when there was no step.
const WAIT_STEP: u8 = 0x8F;
/// Random walk inside a tile rectangle; `$80:8E56`.
const RANDOM_STEP: u8 = 0x26;
/// Branches on whether the player stands at a map position; `$80:888A`.
///
/// Operands: a selector byte, a tile X and Y as signed bytes, and a target.
/// A selector of `$7F` always qualifies; any other is compared with `$0956`,
/// which `docs/house-conversation.md` records as the player's facing, and a
/// mismatch counts as not near. The position, scaled through `$80:BC2F`, is
/// near when it is no more than sixteen pixels beyond the player's on both
/// axes. Bit 7 of the selector inverts the sense: clear branches when near,
/// set when not.
const BRANCH_ON_PLAYER_NEAR: u8 = 0x0F;
/// Branches on a bit of `$0454`; `$80:90AC`.
///
/// `$0454` is the held-button word: `COP 2B`, `COP 2D` and `COP 61` compare
/// it against masks, and a trace shows `$0100` while Right is held and
/// `$0400` while Down is. The runtime has no button state to offer an
/// actor, so it reads the word as zero and never takes the branch.
const BRANCH_ON_GLOBAL: u8 = 0x2E;
/// Stops for a player who stands beside the actor and faces them;
/// `$80:8D1E`. Operand: a two-byte target.
///
/// When the player is nine to sixteen pixels away on one axis and within
/// eight on the other, and `$0956` says they face the actor, the actor turns
/// to face the player, selects the standing sequence for that facing, sets
/// the script pointer to the target and yields. Otherwise the loop continues.
/// The handler skips the test while bit 14 of the slot word at +4 is set,
/// which the runtime never sets, and while `$0999` is nonzero, which a
/// traced conversation never made it.
const STOP_FOR_PLAYER: u8 = 0x23;
/// Holds the actor while the scene pauses them; `$80:9AC7`. One operand
/// byte.
///
/// The handler tests bit 14 of the slot word at +4. When it is set the
/// script pointer is rewound onto the service, the operand becomes the
/// scheduler's countdown at +$0E, and the actor yields. The game sets that
/// bit on every actor for a few frames around a map transition and never
/// during a conversation; the runtime never sets it, so the service
/// continues.
const HOLD_WHILE_PAUSED: u8 = 0x59;
/// Starts a counted loop; `$80:85DF`. Operand: a two-byte count. The count
/// and the address after the operand are the slot's single loop level.
const LOOP_START: u8 = 0x02;
/// Ends a pass of the counted loop; `$80:85F8`. Decrements the count; while
/// it is nonzero, jumps to the loop start and yields one frame.
const LOOP_END: u8 = 0x03;
/// Branches on the map; `$80:8720`. Operands: a word and a two-byte target.
/// Taken when the word's low fifteen bits are the map; bit 15 inverts.
const BRANCH_ON_MAP: u8 = 0x0A;
/// Suspends the script for n frames and resumes on the frame after;
/// `$80:AB17`. Operand: two-byte n. An action under way keeps moving.
const TIMED_WAIT: u8 = 0xC1;

/// Frames a one-tile step takes.
pub const STEP_FRAMES: u16 = 8;
/// Frames a wait lasts.
pub const WAIT_FRAMES: u16 = 8;
/// Commands one frame may execute before the actor is treated as spinning.
const BUDGET: usize = 64;
/// Attributes the step probe accepts; `$80:C0E2`. Bit 15 of the cell counts,
/// so an occupied cell never qualifies.
const PASSABLE: [u16; 3] = [0, 1, 22];

/// What an actor is doing this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Executing commands.
    Running,
    /// Standing for a number of frames.
    Waiting(u16),
    /// Walking one tile.
    Moving {
        /// Direction of travel.
        direction: Direction,
        /// Pixels still to cover.
        remaining: u16,
    },
    /// Source-qualified COP26 plus its following COP8F, including this tick's
    /// final display frame. A zero remainder resumes commands next tick.
    Ordinary {
        direction: Option<Direction>,
        ticks: u16,
        ticks_left: u16,
        destination: Option<(u16, u16)>,
    },
    /// Stopped at something the interpreter does not model.
    Frozen,
    /// Removed by a despawn.
    Gone,
}

/// The map an actor moves through, for one frame.
pub struct Surroundings<'a> {
    /// The ROM image.
    pub image: &'a [u8],
    /// The event-flag bitmap.
    pub events: &'a [u8],
    /// Collision cells, `width * height` of them.
    pub cells: &'a [u16],
    /// Grid width in cells.
    pub width: u16,
    /// Grid height in cells.
    pub height: u16,
    /// Cells other bodies stand on or are stepping into, and the player's.
    pub occupied: &'a [(u16, u16)],
    /// The player's pixel position.
    pub player: (u16, u16),
    /// The way the player faces; `$0956`.
    pub facing: Direction,
}

/// One resident's running script and where it has put them.
#[derive(Debug, Clone)]
pub struct Actor {
    /// Pixel position, the record's origin until a step moves it.
    pub position: (u16, u16),
    /// Way the actor faces; the record's until a step turns them.
    pub facing: Direction,
    /// Sequence selected.
    pub selector: u8,
    /// Horizontal mirror in force.
    pub hflip: bool,
    /// Frames since the pose changed or a qualified action restarted it.
    pub pose_age: u32,
    /// Whether a step is under way.
    pub walking: bool,
    /// Normalized offset of the next command.
    pc: usize,
    state: State,
    rng: u32,
    /// Source-derived COP26 timing, until a skipped service revokes it.
    cadence: Option<cadence::Cadence>,
    /// The map the actor was spawned in, which `COP 0A` compares.
    map: u16,
    /// `COP 02`'s loop start and remaining count; one level per actor.
    counted_loop: (usize, u16),
    /// Derived operand lengths by service, since deriving one explores a
    /// handler's control flow and the loop runs every few frames. The outer
    /// option is whether it has been derived, the inner whether it could be.
    #[allow(clippy::option_option)]
    lengths: Vec<Option<Option<usize>>>,
}

impl Actor {
    /// An actor at its record's origin, about to run its script.
    ///
    /// Without a script the actor stands on its initial selector forever.
    #[must_use]
    pub fn new(position: (u16, u16), script: Option<u32>, initial: u8, seed: u32) -> Self {
        let (pc, state) = match script {
            Some(script) if (0x80..=0xBF).contains(&(script >> 16)) => {
                ((script & 0x3F_FFFF) as usize, State::Running)
            }
            _ => (0, State::Frozen),
        };
        Self {
            position,
            facing: Direction::Down,
            selector: initial,
            hflip: false,
            pose_age: 0,
            walking: false,
            pc,
            state,
            // A zero seed would stay zero.
            rng: seed | 1,
            cadence: None,
            map: 0,
            counted_loop: (0, 0),
            lengths: vec![None; 256],
        }
    }

    /// Builds a resident with source-bound timing admission, not a global
    /// slowdown inferred from shared graphics or initial selector.
    pub(crate) fn for_resident(
        image: &[u8],
        map: u16,
        resident: &crate::residents::Resident,
        seed: u32,
    ) -> Self {
        let mut actor = Self::new(resident.position, resident.script, resident.initial, seed);
        actor.map = map;
        actor.cadence = resident
            .body
            .then(|| cadence::derive(image, map, resident.record).ok())
            .flatten();
        actor
    }

    /// Whether a despawn removed the actor.
    #[must_use]
    pub const fn is_gone(&self) -> bool {
        matches!(self.state, State::Gone)
    }

    /// Cell the actor occupies in the collision grid: movement samples at
    /// `(x - 8, y - 16)`.
    #[must_use]
    pub const fn collision_cell(&self) -> (u16, u16) {
        (
            self.position.0.saturating_sub(8) / 16,
            self.position.1.saturating_sub(16) / 16,
        )
    }

    /// The cell a step under way leads to, if one is.
    #[must_use]
    pub fn destination(&self) -> Option<(u16, u16)> {
        match self.state {
            State::Ordinary { destination, .. } => destination,
            State::Moving { direction, .. } => {
                let (column, row) = self.collision_cell();
                let (dx, dy) = delta(direction);
                Some((column.wrapping_add_signed(dx), row.wrapping_add_signed(dy)))
            }
            _ => None,
        }
    }

    /// Runs one frame.
    pub fn tick(&mut self, around: &Surroundings<'_>) {
        self.pose_age = self.pose_age.saturating_add(1);
        if matches!(self.state, State::Ordinary { ticks_left: 0, .. }) {
            self.walking = false;
            self.state = State::Running;
        }
        match self.state {
            State::Ordinary { .. } => self.tick_ordinary(),
            State::Frozen | State::Gone => {}
            State::Waiting(frames) => {
                self.state = if frames <= 1 {
                    State::Running
                } else {
                    State::Waiting(frames - 1)
                };
            }
            State::Moving {
                direction,
                remaining,
            } => {
                let pixels = (16 / STEP_FRAMES).min(remaining);
                let (dx, dy) = delta(direction);
                let travelled = i16::try_from(pixels).unwrap_or(0);
                self.position = (
                    self.position.0.wrapping_add_signed(dx * travelled),
                    self.position.1.wrapping_add_signed(dy * travelled),
                );
                let remaining = remaining - pixels;
                self.state = if remaining == 0 {
                    self.walking = false;
                    State::Running
                } else {
                    State::Moving {
                        direction,
                        remaining,
                    }
                };
            }
            State::Running => self.run(around),
        }
    }

    /// Apply this action's frame before presenting it. Streams restart with
    /// one pixel, then zero, including a final zero-displacement frame.
    fn tick_ordinary(&mut self) {
        let State::Ordinary {
            direction,
            ticks,
            ticks_left,
            destination,
        } = self.state
        else {
            return;
        };
        if let Some(direction) = direction.filter(|_| (ticks - ticks_left) % 2 == 0) {
            let (dx, dy) = delta(direction);
            self.position = (
                self.position.0.wrapping_add_signed(dx),
                self.position.1.wrapping_add_signed(dy),
            );
        }
        self.state = State::Ordinary {
            direction,
            ticks,
            ticks_left: ticks_left - 1,
            destination,
        };
    }

    fn set_pose(&mut self, selector: u8, hflip: bool) {
        if self.selector != selector || self.hflip != hflip {
            self.selector = selector;
            self.hflip = hflip;
            self.pose_age = 0;
        }
    }

    fn run(&mut self, around: &Surroundings<'_>) {
        let image = around.image;
        let bank = self.pc & 0xFF_0000;
        for _ in 0..BUDGET {
            let Some(window) = image.get(self.pc..self.pc + 2) else {
                self.state = State::Frozen;
                return;
            };
            match window[0] {
                0x02 => {}
                // BRA: the loop's back edge.
                0x80 => {
                    let displacement = i8::from_ne_bytes([window[1]]);
                    self.pc = (self.pc + 2).wrapping_add_signed(displacement as isize);
                    continue;
                }
                0x4C => {
                    let Some(target) = image.get(self.pc + 1..self.pc + 3) else {
                        self.state = State::Frozen;
                        return;
                    };
                    let target = u16::from_le_bytes([target[0], target[1]]);
                    if target < 0x8000 {
                        self.state = State::Frozen;
                        return;
                    }
                    self.pc = bank | usize::from(target);
                    continue;
                }
                _ => {
                    self.state = State::Frozen;
                    return;
                }
            }
            if !self.service(window[1], self.pc + 2, bank, around) {
                return;
            }
        }
        // Spinning without yielding: a loop with no wait in it.
        self.state = State::Frozen;
    }

    /// Executes one service. Returns whether execution continues this
    /// frame; a yield or a freeze ends it.
    fn service(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        match service {
            SELECT_POSE => {
                let Some(selector) = image.get(operands).copied() else {
                    self.state = State::Frozen;
                    return false;
                };
                let hflip = self.hflip;
                self.set_pose(selector, hflip);
                self.pc = operands + 1;
            }
            CLEAR_HFLIP | SET_HFLIP => {
                let selector = self.selector;
                self.set_pose(selector, service == SET_HFLIP);
                self.pc = operands;
            }
            WAIT => {
                self.pc = operands;
                self.state = State::Waiting(1);
                return false;
            }
            WAIT_STEP => {
                self.pc = operands;
                self.state = State::Waiting(WAIT_FRAMES);
                return false;
            }
            RANDOM_STEP => {
                let Some(rect) = image.get(operands..operands + 4) else {
                    self.state = State::Frozen;
                    return false;
                };
                let qualified = self
                    .cadence
                    .filter(|_| image.get(operands + 4..operands + 6) == Some(&[2, WAIT_STEP]));
                self.pc = operands + 4;
                let moving = self.random_step([rect[0], rect[1], rect[2], rect[3]], around);
                if let Some(cadence) = qualified {
                    // COP8F resolves this very action; it is not an extra wait.
                    self.pc += 2;
                    self.pose_age = 0;
                    self.walking = moving;
                    let ticks = if moving {
                        cadence.walk(self.facing)
                    } else {
                        cadence.idle
                    };
                    self.state = State::Ordinary {
                        direction: moving.then_some(self.facing),
                        ticks,
                        ticks_left: ticks,
                        destination: self.destination(),
                    };
                    self.tick_ordinary();
                    return false;
                }
                return !moving;
            }
            BRANCH_ON_PLAYER_NEAR => return self.branch_near_player(operands, bank, around),
            BRANCH_ON_GLOBAL => self.pc = operands + 4,
            LOOP_START => return self.loop_start(operands, image),
            LOOP_END => return self.loop_end(operands),
            BRANCH_ON_MAP => return self.branch_on_map(operands, bank, image),
            TIMED_WAIT => return self.timed_wait(operands, image),
            HOLD_WHILE_PAUSED => self.pc = operands + 1,
            STOP_FOR_PLAYER => return self.stop_for_player(operands, bank, around),
            BRANCH_ON_FLAG => return self.branch_on_flag(operands, bank, around),
            CHAINED_BRANCH | CHAINED_DESPAWN => {
                return self.branch_on_chain(service, operands, bank, around)
            }
            // Anything else is stepped over by its derived length. That
            // includes text and flag writes: the loop's ambient effects
            // are not the runtime's to apply from here.
            other => {
                if !cadence::benign_skipped_service(image, self.pc) {
                    self.cadence = None;
                }
                let length = *self.lengths[usize::from(other)]
                    .get_or_insert_with(|| actor_script::operand_length(image, other));
                let Some(length) = length else {
                    self.state = State::Frozen;
                    return false;
                };
                self.pc = operands + length;
            }
        }
        true
    }

    /// `COP 02`. Returns whether execution continues this frame.
    fn loop_start(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(count) = cadence::word(image, operands) else {
            self.state = State::Frozen;
            return false;
        };
        self.pc = operands + 2;
        self.counted_loop = (self.pc, count);
        true
    }

    /// `COP 03`. Without a `COP 02` the count wraps and the start is zero,
    /// which is no command, so the actor freezes there.
    fn loop_end(&mut self, operands: usize) -> bool {
        let (start, count) = self.counted_loop;
        let count = count.wrapping_sub(1);
        self.counted_loop.1 = count;
        if count == 0 {
            self.pc = operands;
            return true;
        }
        self.pc = start;
        false
    }

    /// `COP 0A`. Returns whether execution continues this frame.
    fn branch_on_map(&mut self, operands: usize, bank: usize, image: &[u8]) -> bool {
        let (Some(word), Some(target)) = (
            cadence::word(image, operands),
            cadence::word(image, operands + 2),
        ) else {
            self.state = State::Frozen;
            return false;
        };
        if (word & 0x7FFF == self.map) != (word & 0x8000 != 0) {
            return self.jump(bank, target);
        }
        self.pc = operands + 4;
        true
    }

    /// `COP C1`: always yields.
    fn timed_wait(&mut self, operands: usize, image: &[u8]) -> bool {
        let Some(frames) = cadence::word(image, operands) else {
            self.state = State::Frozen;
            return false;
        };
        self.pc = operands + 2;
        if frames != 0 {
            self.state = State::Waiting(frames);
        }
        false
    }

    /// Resumes at a bank-relative target, or freezes on one below `$8000`,
    /// which is RAM and not a command address.
    fn jump(&mut self, bank: usize, target: u16) -> bool {
        if target < 0x8000 {
            self.state = State::Frozen;
            return false;
        }
        self.pc = bank | usize::from(target);
        true
    }

    /// `COP 23`: stops for a player who stands beside the actor and faces
    /// them. Returns whether execution continues this frame.
    fn stop_for_player(&mut self, operands: usize, bank: usize, around: &Surroundings<'_>) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 2) else {
            self.state = State::Frozen;
            return false;
        };
        let faced = approach(self.position, around.player)
            .into_iter()
            .flatten()
            .any(|toward| toward == around.facing);
        if !faced {
            self.pc = operands + 2;
            return true;
        }
        // `$8DC2`: the actor's facing is the player's, reversed.
        let facing = opposite(around.facing);
        self.facing = facing;
        let (selector, hflip) = standing_pose(facing);
        self.set_pose(selector, hflip);
        let target = u16::from_le_bytes([bytes[0], bytes[1]]);
        // A taken branch yields whether or not the target was sound.
        self.jump(bank, target);
        false
    }

    /// `COP 0F`: branches on the player standing at a map position. Returns
    /// whether execution continues this frame.
    fn branch_near_player(
        &mut self,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 5) else {
            self.state = State::Frozen;
            return false;
        };
        let id = bytes[0];
        // `$88A3`: `SEC; SBC $0966; CMP #$11; BCS` -- an unsigned, one-sided
        // test: the position minus the player's must be 0 to 16. `$0968`
        // holds the player's Y less eight.
        let near = id & 0x7F == 0x7F && {
            let tile = |byte: u8| i32::from(i8::from_ne_bytes([byte])) * 16;
            let (px, py) = (i32::from(around.player.0), i32::from(around.player.1) - 8);
            (0..=16).contains(&(tile(bytes[1]) - px)) && (0..=16).contains(&(tile(bytes[2]) - py))
        };
        let target = u16::from_le_bytes([bytes[3], bytes[4]]);
        if near != (id & 0x80 != 0) {
            return self.jump(bank, target);
        }
        self.pc = operands + 5;
        true
    }

    /// `COP 08`: branches on one event flag. Returns whether execution
    /// continues this frame.
    fn branch_on_flag(&mut self, operands: usize, bank: usize, around: &Surroundings<'_>) -> bool {
        let Some(bytes) = around.image.get(operands..operands + 4) else {
            self.state = State::Frozen;
            return false;
        };
        let condition = u16::from_le_bytes([bytes[0], bytes[1]]);
        let target = u16::from_le_bytes([bytes[2], bytes[3]]);
        let Some(set) = EventFlags::Bitmap(around.events).get(condition) else {
            self.state = State::Frozen;
            return false;
        };
        if flag_branch_taken(condition, set) {
            return self.jump(bank, target);
        }
        self.pc = operands + 4;
        true
    }

    /// A chained condition: despawns, branches or falls through. Returns
    /// whether execution continues this frame.
    fn branch_on_chain(
        &mut self,
        service: u8,
        operands: usize,
        bank: usize,
        around: &Surroundings<'_>,
    ) -> bool {
        let image = around.image;
        let flags = EventFlags::Bitmap(around.events);
        let (Some(chain), Some(holds)) = (
            chained_condition_length(image, operands),
            chained_condition_holds(image, operands, &flags),
        ) else {
            self.state = State::Frozen;
            return false;
        };
        if service == CHAINED_DESPAWN {
            if holds {
                self.state = State::Gone;
                return false;
            }
            self.pc = operands + chain;
            return true;
        }
        let Some(bytes) = image.get(operands + chain..operands + chain + 2) else {
            self.state = State::Frozen;
            return false;
        };
        let target = u16::from_le_bytes([bytes[0], bytes[1]]);
        if holds {
            return self.jump(bank, target);
        }
        self.pc = operands + chain + 2;
        true
    }

    /// One `COP 26`: draws, bounds-checks, probes, and starts a step or
    /// stands. Returns whether the frame's execution ends here.
    fn random_step(&mut self, rect: [u8; 4], around: &Surroundings<'_>) -> bool {
        let draw = self.next_random();
        let (column, row) = self.collision_cell();
        if draw & 4 != 0 {
            let (selector, hflip) = standing_pose(self.facing);
            self.set_pose(selector, hflip);
            return false;
        }
        let direction = match draw & 3 {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            _ => Direction::Right,
        };
        let [min_column, max_column, min_row, max_row] = rect.map(u16::from);
        // As the handler compares: `$8EA5` stands when the maximum is below
        // the current column, `$8EBE` when the minimum is not below it. The
        // far side is therefore one cell wider than the operands read.
        let inside = match direction {
            Direction::Down => row <= max_row,
            Direction::Up => row > min_row,
            Direction::Left => column > min_column,
            Direction::Right => column <= max_column,
        };
        let (dx, dy) = delta(direction);
        let destination = (column.wrapping_add_signed(dx), row.wrapping_add_signed(dy));
        let passable = destination.0 < around.width
            && destination.1 < around.height
            && around
                .cells
                .get(
                    usize::from(destination.1) * usize::from(around.width)
                        + usize::from(destination.0),
                )
                .is_some_and(|cell| PASSABLE.contains(&(cell >> 9)))
            && !around.occupied.contains(&destination);
        if inside && passable {
            self.facing = direction;
            let (selector, hflip) = match direction {
                Direction::Down => (3, false),
                Direction::Up => (4, false),
                Direction::Right => (5, false),
                Direction::Left => (5, true),
            };
            self.set_pose(selector, hflip);
            self.walking = true;
            self.state = State::Moving {
                direction,
                remaining: 16,
            };
            true
        } else {
            let (selector, hflip) = standing_pose(self.facing);
            self.set_pose(selector, hflip);
            false
        }
    }

    /// The runtime's own generator, not `$86:8236`. An xorshift, seeded per
    /// actor, so a run is repeatable.
    fn next_random(&mut self) -> u8 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 24) as u8
    }
}

/// The standing sequence for a facing: 0, 1 and 2, with 2 mirrored for
/// left, as `$80:8DC8` selects it.
const fn standing_pose(facing: Direction) -> (u8, bool) {
    match facing {
        Direction::Down => (0, false),
        Direction::Up => (1, false),
        Direction::Right => (2, false),
        Direction::Left => (2, true),
    }
}

/// The facing that looks back at `facing`; `$8DC2`'s `EOR #1` on the game's
/// 0 down, 1 up, 2 left, 3 right.
const fn opposite(facing: Direction) -> Direction {
    match facing {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

/// The facings a player at `player` may hold to be looking at an actor at
/// `actor`, as `$80:8D2B`..`$8DB5` classifies their offset; both `None`
/// when the player is not beside them.
///
/// Along one axis the actor must be nine to sixteen pixels away; across it
/// within eight. The handler measures `actor.x - $0966` and
/// `(actor.y - 8) - $0968`, and `$0968` is the player's y less eight, so
/// both are plain differences. Within eight on both axes it tries the
/// vertical facing it settled on and then the horizontal one it had noted.
fn approach(actor: (u16, u16), player: (u16, u16)) -> [Option<Direction>; 2] {
    let dx = i32::from(actor.0) - i32::from(player.0);
    let dy = i32::from(actor.1) - i32::from(player.1);
    let across = |offset: i32| (-8..=8).contains(&offset);
    // `$8D46`..`$8D59`: beside on the horizontal axis.
    if (9..=16).contains(&dx) {
        return [across(dy).then_some(Direction::Right), None];
    }
    if (-16..=-9).contains(&dx) {
        return [across(dy).then_some(Direction::Left), None];
    }
    if !across(dx) {
        return [None, None];
    }
    // `$8D83`..`$8DA3`: aligned horizontally, so the vertical axis decides.
    let horizontal = if dx >= 0 {
        Direction::Right
    } else {
        Direction::Left
    };
    if (9..=16).contains(&dy) {
        return [Some(Direction::Down), None];
    }
    if (-16..=-9).contains(&dy) {
        return [Some(Direction::Up), None];
    }
    if (0..=8).contains(&dy) {
        return [Some(Direction::Down), Some(horizontal)];
    }
    if (-8..=-1).contains(&dy) {
        return [Some(Direction::Up), Some(horizontal)];
    }
    [None, None]
}

/// Cell offset one step in a direction.
const fn delta(direction: Direction) -> (i16, i16) {
    match direction {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank-`$88` image with a script at `$88:8000`.
    fn image_with(script: &[u8]) -> Vec<u8> {
        let mut image = vec![0u8; 0x09_0000];
        image[0x08_8000..0x08_8000 + script.len()].copy_from_slice(script);
        image
    }

    fn open(width: u16, height: u16) -> Vec<u16> {
        vec![0; usize::from(width) * usize::from(height)]
    }

    #[test]
    fn a_static_loop_selects_its_pose_and_never_moves() {
        // COP B6, COP 80 02, COP 8E, BRA -8.
        let image = image_with(&[0x02, 0xB6, 0x02, 0x80, 0x02, 0x02, 0x8E, 0x80, 0xF7]);
        let cells = open(8, 8);
        let around = Surroundings {
            image: &image,
            events: &[0; 512],
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((40, 48), Some(0x88_8000), 7, 1);
        for _ in 0..200 {
            actor.tick(&around);
        }
        assert_eq!(actor.position, (40, 48));
        assert_eq!((actor.selector, actor.hflip), (2, false));
        assert!(!actor.is_gone());
        assert!(matches!(actor.state, State::Waiting(1) | State::Running));
    }

    #[test]
    fn a_random_walk_stays_inside_its_rectangle_and_off_blocked_cells() {
        // COP 26 2 5 2 4, COP 8F, BRA -10: the handler lets the actor reach
        // one cell past the far operands, so columns 2..=6 and rows 2..=5.
        let image = image_with(&[0x02, 0x26, 2, 5, 2, 4, 0x02, 0x8F, 0x80, 0xF6]);
        let mut cells = open(8, 8);
        // Column 4, row 3 is a wall.
        cells[3 * 8 + 4] = 14 << 9;
        let around = Surroundings {
            image: &image,
            events: &[0; 512],
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[(3, 4)],
            player: (0, 0),
            facing: Direction::Down,
        };
        // Origin (56, 64): collision cell (3, 3).
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 12345);
        let mut visited = std::collections::BTreeSet::new();
        for _ in 0..5000 {
            actor.tick(&around);
            let (column, row) = actor.collision_cell();
            assert!(
                (2..=6).contains(&column) && (2..=5).contains(&row),
                "{:?}",
                actor.position
            );
            assert_ne!((column, row), (4, 3), "walked into a wall");
            assert_ne!((column, row), (3, 4), "walked into someone");
            visited.insert((column, row));
        }
        assert!(visited.len() > 3, "never went anywhere: {visited:?}");
        assert!(
            visited.contains(&(6, 5)),
            "the far corner is reachable: {visited:?}"
        );
        // Between steps, positions stay on the grid the record put them on.
        while actor.walking {
            actor.tick(&around);
        }
        assert_eq!(actor.position.0 % 16, 8);
        assert_eq!(actor.position.1 % 16, 0);
    }

    #[test]
    fn walking_selects_the_walking_sequence_and_standing_the_standing_one() {
        // A rectangle whose maxima lie below the actor's cell refuses every
        // direction, so every draw stands.
        let image = image_with(&[0x02, 0x26, 3, 2, 3, 2, 0x02, 0x8F, 0x80, 0xF6]);
        let cells = open(8, 8);
        let around = Surroundings {
            image: &image,
            events: &[0; 512],
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 9, 5);
        for _ in 0..50 {
            actor.tick(&around);
        }
        assert_eq!(actor.selector, 0, "standing, facing down");
        assert!(!actor.walking);
        // Open rectangle: some draw walks.
        let image = image_with(&[0x02, 0x26, 0, 7, 0, 7, 0x02, 0x8F, 0x80, 0xF6]);
        let around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 9, 5);
        let mut walked = false;
        for _ in 0..200 {
            actor.tick(&around);
            if actor.walking {
                walked = true;
                assert!((3..=5).contains(&actor.selector));
                assert_eq!(actor.hflip, actor.facing == Direction::Left);
            }
        }
        assert!(walked);
    }

    #[test]
    fn the_near_player_branch_is_one_sided_and_bit_seven_inverts_it() {
        // COP 0F 7F 03 04 <$8010>: near when the player is at most sixteen
        // pixels short of tile (3, 4) on both axes, Y taken less eight.
        let mut script = vec![0x02, 0x0F, 0x7F, 3, 4, 0x10, 0x80];
        script.extend([0x02, 0x8E, 0x80, 0xFC]); // not taken: wait, loop
        script.resize(0x10, 0xEA);
        script.extend([0x02, 0x80, 0x09, 0x02, 0x8E, 0x80, 0xFB]); // taken: pose 9
        let image = image_with(&script);
        let cells = open(8, 8);
        let run = |player: (u16, u16), id: u8| {
            let mut image = image.clone();
            image[0x08_8002] = id;
            let around = Surroundings {
                image: &image,
                events: &[0; 512],
                cells: &cells,
                width: 8,
                height: 8,
                occupied: &[],
                player,
                facing: Direction::Down,
            };
            let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 1);
            actor.tick(&around);
            actor.selector
        };
        // Tile (3,4) is (48, 64); with Y less eight, a player at (48, 72)
        // sits exactly on it, and sixteen short still qualifies.
        assert_eq!(run((48, 72), 0x7F), 9);
        assert_eq!(run((32, 56), 0x7F), 9);
        // Seventeen short does not, and neither does being past it.
        assert_eq!(run((31, 72), 0x7F), 0);
        assert_eq!(run((49, 72), 0x7F), 0);
        // Bit 7 inverts; another selector never matches.
        assert_eq!(run((48, 72), 0xFF), 0);
        assert_eq!(run((31, 72), 0xFF), 9);
        assert_eq!(run((48, 72), 0x02), 0);
    }

    #[test]
    fn a_jump_into_ram_freezes_and_a_jump_into_rom_is_followed() {
        let image = image_with(&[0x4C, 0x00, 0x40]);
        let cells = open(4, 4);
        let around = Surroundings {
            image: &image,
            events: &[0; 512],
            cells: &cells,
            width: 4,
            height: 4,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&around);
        assert_eq!(actor.state, State::Frozen);
        let mut script = vec![0x4C, 0x10, 0x80];
        script.resize(0x10, 0xEA);
        script.extend([0x02, 0x80, 0x05, 0x02, 0x8E, 0x80, 0xFB]);
        let image = image_with(&script);
        let around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&around);
        assert_eq!(actor.selector, 5);
    }

    #[test]
    fn a_despawn_that_holds_removes_the_actor_and_an_unknown_opcode_freezes() {
        // COP 47 <0x0010>, then a static loop.
        let image = image_with(&[0x02, 0x47, 0x10, 0x00, 0x02, 0x8E, 0x80, 0xFC]);
        let cells = open(4, 4);
        let mut set = vec![0u8; 512];
        set[2] = 1;
        let around = Surroundings {
            image: &image,
            events: &set,
            cells: &cells,
            width: 4,
            height: 4,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&around);
        assert!(actor.is_gone());
        let clear = vec![0u8; 512];
        let around = Surroundings {
            events: &clear,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&around);
        assert!(!actor.is_gone());
        // RTL where a command should be.
        let image = image_with(&[0x6B]);
        let around = Surroundings {
            image: &image,
            ..around
        };
        let mut actor = Actor::new((8, 16), Some(0x88_8000), 0, 3);
        actor.tick(&around);
        assert_eq!(actor.state, State::Frozen);
    }
}

#[cfg(test)]
mod stop_for_player_tests {
    use super::*;

    fn image_with(script: &[u8]) -> Vec<u8> {
        let mut image = vec![0u8; 0x09_0000];
        image[0x08_8000..0x08_8000 + script.len()].copy_from_slice(script);
        image
    }

    /// `COP 23 <$8000>` at the head, then pose 9, wait, and back to the head.
    /// A taken branch lands on the `COP 23` again and never reaches pose 9.
    fn loop_with_stop() -> Vec<u8> {
        vec![
            0x02, 0x23, 0x00, 0x80, 0x02, 0x80, 0x09, 0x02, 0x8E, 0x80, 0xF5,
        ]
    }

    fn run(script: &[u8], player: (u16, u16), facing: Direction, frames: usize) -> Actor {
        let image = image_with(script);
        let cells = vec![0u16; 64];
        let around = Surroundings {
            image: &image,
            events: &[0; 512],
            cells: &cells,
            width: 8,
            height: 8,
            occupied: &[],
            player,
            facing,
        };
        // Origin (56, 64): the actor's own reference for the near test.
        let mut actor = Actor::new((56, 64), Some(0x88_8000), 0, 1);
        for _ in 0..frames {
            actor.tick(&around);
        }
        actor
    }

    #[test]
    fn a_player_one_cell_away_and_facing_the_actor_stops_them_and_is_faced() {
        // Right of the actor, facing left: the actor turns right.
        let actor = run(&loop_with_stop(), (72, 64), Direction::Left, 20);
        assert_eq!((actor.selector, actor.hflip), (2, false));
        assert_eq!(actor.facing, Direction::Right);
        assert_eq!(actor.state, State::Running, "one branch per frame, no spin");
        // Left, facing right: mirrored.
        let actor = run(&loop_with_stop(), (40, 64), Direction::Right, 20);
        assert_eq!((actor.selector, actor.hflip), (2, true));
        assert_eq!(actor.facing, Direction::Left);
        // Above, facing down: the actor faces up.
        let actor = run(&loop_with_stop(), (56, 48), Direction::Down, 20);
        assert_eq!((actor.selector, actor.hflip), (1, false));
        // Below, facing up: the actor faces down.
        let actor = run(&loop_with_stop(), (56, 80), Direction::Up, 20);
        assert_eq!((actor.selector, actor.hflip), (0, false));
    }

    #[test]
    fn a_player_who_is_near_but_faces_away_does_not_stop_the_actor() {
        let actor = run(&loop_with_stop(), (72, 64), Direction::Right, 20);
        assert_eq!(actor.selector, 9);
        let actor = run(&loop_with_stop(), (72, 64), Direction::Up, 20);
        assert_eq!(actor.selector, 9);
    }

    #[test]
    fn the_window_is_nine_to_sixteen_on_the_axis_and_eight_across_it() {
        // `$8D46`..`$8D59`: dx of 9..=16 is beside; 17 is not, and -8 or 8
        // is "aligned", which sends the test to the vertical axis instead.
        assert_eq!(
            run(&loop_with_stop(), (64, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (48, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (65, 64), Direction::Down, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (65, 64), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (47, 64), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (73, 64), Direction::Left, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (40, 64), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (39, 64), Direction::Right, 5).selector,
            9
        );
        // `$8D5B`..`$8D6B`: across the axis, eight either way still counts.
        assert_eq!(
            run(&loop_with_stop(), (72, 72), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 56), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 73), Direction::Left, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (72, 55), Direction::Left, 5).selector,
            9
        );
        // Vertically: 9..=16 below or above, at most eight across.
        assert_eq!(
            run(&loop_with_stop(), (56, 73), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 81), Direction::Up, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (64, 80), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (48, 48), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 47), Direction::Down, 5).selector,
            9
        );
    }

    #[test]
    fn the_overlap_case_tries_the_vertical_facing_then_the_horizontal_one() {
        // `$8D79`: within eight on both axes the handler tests Down (or Up
        // when the actor is above), then the horizontal side it noted.
        assert_eq!(
            run(&loop_with_stop(), (56, 64), Direction::Down, 5).selector,
            1
        );
        assert_eq!(
            run(&loop_with_stop(), (60, 64), Direction::Left, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (60, 64), Direction::Right, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (56, 60), Direction::Up, 5).selector,
            9
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Up, 5).selector,
            0
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Right, 5).selector,
            2
        );
        assert_eq!(
            run(&loop_with_stop(), (52, 66), Direction::Left, 5).selector,
            9
        );
    }

    #[test]
    fn hold_while_paused_is_a_one_byte_service_that_continues() {
        // COP 59 08, COP 80 05, COP 8E, BRA back.
        let actor = run(
            &[0x02, 0x59, 0x08, 0x02, 0x80, 0x05, 0x02, 0x8E, 0x80, 0xF6],
            (0, 0),
            Direction::Down,
            3,
        );
        assert_eq!(actor.selector, 5);
        assert_eq!(actor.state, State::Waiting(1));
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::*;

    const SITE: usize = 0x08_A868;

    fn seed_for(choice: u8) -> u32 {
        (1..100_000)
            .find(|seed| {
                let mut actor = Actor::new((0, 0), None, 0, *seed);
                actor.next_random() & 7 == choice
            })
            .unwrap()
            | 1
    }

    fn surroundings<'a>(image: &'a [u8], cells: &'a [u16]) -> Surroundings<'a> {
        Surroundings {
            image,
            events: &[0; 512],
            cells,
            width: 8,
            height: 8,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        }
    }

    fn run_action(choice: u8, refused: bool, cadence: cadence::Cadence) {
        let mut image = vec![0; SITE + 10];
        image[SITE..].copy_from_slice(&[2, 0x26, 0, 7, 0, 7, 2, 0x8F, 0x80, 0xF6]);
        let cells = vec![if refused { 14 << 9 } else { 0 }; 64];
        let around = surroundings(&image, &cells);
        let seed = seed_for(choice);
        let mut actor = Actor::new((56, 64), Some(0x88_A868), 0, seed);
        actor.cadence = Some(cadence);
        let moving = choice < 4 && !refused;
        let direction = match choice {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::Left,
            _ => Direction::Right,
        };
        let (dx, dy) = if moving { delta(direction) } else { (0, 0) };
        let duration = if moving {
            cadence.walk(direction)
        } else {
            cadence.idle
        };
        for action in 0..2 {
            let start = actor.position;
            let cell = actor.collision_cell();
            let destination = moving.then_some((
                cell.0.wrapping_add_signed(dx),
                cell.1.wrapping_add_signed(dy),
            ));
            actor.rng = seed;
            for tick in 0..duration {
                actor.tick(&around);
                let pixels = i16::try_from(tick / 2 + 1).unwrap();
                assert_eq!(
                    actor.position,
                    (
                        start.0.wrapping_add_signed(dx * pixels),
                        start.1.wrapping_add_signed(dy * pixels)
                    )
                );
                assert_eq!(actor.pose_age, u32::from(tick), "action {action}");
                assert_eq!(actor.walking, moving);
                assert_eq!(
                    actor.destination(),
                    destination,
                    "reservation must not drift mid-step"
                );
            }
            let travelled = (
                actor.position.0.abs_diff(start.0),
                actor.position.1.abs_diff(start.1),
            );
            assert_eq!(travelled.0 + travelled.1, if moving { 16 } else { 0 });
        }
    }

    const CLASS_ZERO: cadence::Cadence = cadence::Cadence {
        walk: [32; 4],
        idle: 16,
    };

    #[test]
    fn qualified_actions_start_immediately_and_restart_all_four_directions_without_gaps() {
        for choice in 0..4 {
            run_action(choice, false, CLASS_ZERO);
        }
    }

    #[test]
    fn qualified_random_idle_and_all_refusals_take_the_idle_length() {
        run_action(4, false, CLASS_ZERO);
        for choice in 0..4 {
            run_action(choice, true, CLASS_ZERO);
        }
        let short = cadence::Cadence {
            walk: [32; 4],
            idle: 9,
        };
        run_action(4, false, short);
    }

    #[test]
    fn a_thirty_one_tick_list_still_walks_sixteen_pixels() {
        let cadence = cadence::Cadence {
            walk: [32, 31, 32, 32],
            idle: 16,
        };
        run_action(1, false, cadence);
    }

    #[test]
    fn a_skipped_service_that_could_change_movement_revokes_admission() {
        for (bytes, kept) in [
            ([2, 0xBB, 0x0C], true),
            ([2, 0xBB, 0x40], false),
            ([2, 0x28, 4], false),
        ] {
            let mut image = vec![0; SITE + 3];
            image[SITE..].copy_from_slice(&bytes);
            let mut actor = Actor::new((0, 0), Some(0x88_A868), 0, 1);
            actor.cadence = Some(CLASS_ZERO);
            actor.tick(&surroundings(&image, &[]));
            assert_eq!(actor.cadence.is_some(), kept, "{bytes:02X?}");
        }
    }
}

#[cfg(test)]
mod script_service_tests {
    use super::*;

    const AT: usize = 0x08_8000;

    fn actor_running(code: &[u8]) -> (Vec<u8>, Actor) {
        let mut image = vec![0; AT + code.len() + 2];
        image[AT..AT + code.len()].copy_from_slice(code);
        (image, Actor::new((56, 64), Some(0x88_8000), 0, 1))
    }

    fn tick(actor: &mut Actor, image: &[u8]) {
        actor.tick(&Surroundings {
            image,
            events: &[0; 512],
            cells: &[],
            width: 0,
            height: 0,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        });
    }

    #[test]
    fn a_counted_loop_yields_one_frame_each_time_it_loops_back() {
        // COP02 3; { pose 7; COP03 }; pose 9; wait.
        let (image, mut actor) =
            actor_running(&[2, 0x02, 3, 0, 2, 0x80, 7, 2, 0x03, 2, 0x80, 9, 2, 0x8E]);
        for frame in 0..2 {
            tick(&mut actor, &image);
            assert_eq!(actor.selector, 7, "frame {frame} loops back and yields");
        }
        tick(&mut actor, &image);
        assert_eq!(actor.selector, 9, "the third pass falls through");
        assert_eq!(actor.state, State::Waiting(1));
    }

    #[test]
    fn a_loop_end_without_a_start_freezes_rather_than_running_on() {
        let (image, mut actor) = actor_running(&[2, 0x03, 2, 0x80, 9, 2, 0x8E]);
        tick(&mut actor, &image);
        tick(&mut actor, &image);
        assert_eq!(actor.state, State::Frozen);
        assert_eq!(actor.selector, 0);
    }

    #[test]
    fn a_timed_wait_resumes_n_plus_one_frames_later() {
        for (frames, resumes) in [(2u8, 4), (1, 3), (0, 2)] {
            let (image, mut actor) = actor_running(&[2, 0xC1, frames, 0, 2, 0x80, 5, 2, 0x8E]);
            for tick_number in 1..=resumes {
                tick(&mut actor, &image);
                assert_eq!(
                    actor.selector == 5,
                    tick_number == resumes,
                    "C1 {frames} tick {tick_number}"
                );
            }
        }
    }

    #[test]
    fn the_map_branch_compares_the_low_fifteen_bits_and_inverts_on_bit_fifteen() {
        // COP0A word target; pose 1; wait; target: pose 2; wait.
        for (word, map, taken) in [
            (0x00D5, 0xD5, true),
            (0x00D5, 0x0A, false),
            (0x80D5, 0x0A, true),
            (0x80D5, 0xD5, false),
        ] {
            let [low, high] = u16::to_le_bytes(word);
            let (image, mut actor) = actor_running(&[
                2, 0x0A, low, high, 0x0B, 0x80, 2, 0x80, 1, 2, 0x8E, 2, 0x80, 2, 2, 0x8E,
            ]);
            actor.map = map;
            tick(&mut actor, &image);
            assert_eq!(
                actor.selector,
                if taken { 2 } else { 1 },
                "{word:04X} on {map:#x}"
            );
        }
    }

    #[test]
    fn the_town_walker_loops_with_one_frame_gaps_and_poses_after_sixteen_actions() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let image = rom::Rom::load(&bytes).unwrap().image().to_vec();
        let flags = crate::world::new_game_flags();
        let events = assets::maps::scripts::EventFlags::Bitmap(&flags);
        let resident = crate::residents::residents(&image, 0xA, events)
            .unwrap()
            .into_iter()
            .find(|resident| resident.record == 0x03_8A2D)
            .unwrap();
        let mut actor = Actor::for_resident(&image, 0xA, &resident, 7);
        assert!(actor.cadence.is_some());
        let cells = vec![0u16; 64 * 64];
        let around = Surroundings {
            image: &image,
            events: &flags,
            cells: &cells,
            width: 64,
            height: 64,
            occupied: &[],
            player: (0, 0),
            facing: Direction::Down,
        };
        // Tick of each action's start, and whether it walked.
        let mut starts = Vec::new();
        for tick in 0..700u32 {
            let before = matches!(actor.state, State::Ordinary { .. });
            actor.tick(&around);
            if let State::Ordinary {
                ticks, ticks_left, ..
            } = actor.state
            {
                if !before || ticks_left + 1 == ticks {
                    starts.push((tick, actor.walking));
                }
            }
            if starts.len() == 16 {
                break;
            }
        }
        assert_eq!(starts.len(), 16);
        for pair in starts.windows(2) {
            let period = pair[1].0 - pair[0].0;
            assert_eq!(period, if pair[0].1 { 33 } else { 17 }, "{pair:?}");
        }
        // After the sixteenth action the first counted loop ends: pose 6.
        for _ in 0..40 {
            actor.tick(&around);
        }
        assert_eq!(actor.selector, 6);
    }
}
