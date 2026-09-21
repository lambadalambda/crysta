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
//! timing: a tile is walked in [`STEP_FRAMES`] frames and a wait lasts
//! [`WAIT_FRAMES`], where the game consults a velocity table the runtime has
//! not decoded and resolves animation records.

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
/// What `$0454` holds is not established. The runtime reads it as zero, so
/// the branch is never taken.
const BRANCH_ON_GLOBAL: u8 = 0x2E;

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
    /// Frames since the selector or mirror last changed.
    pub pose_age: u32,
    /// Whether a step is under way.
    pub walking: bool,
    /// Normalized offset of the next command.
    pc: usize,
    state: State,
    rng: u32,
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
            lengths: vec![None; 256],
        }
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
        match self.state {
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
            let service = window[1];
            let operands = self.pc + 2;
            match service {
                SELECT_POSE => {
                    let Some(selector) = image.get(operands).copied() else {
                        self.state = State::Frozen;
                        return;
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
                    return;
                }
                WAIT_STEP => {
                    self.pc = operands;
                    self.state = State::Waiting(WAIT_FRAMES);
                    return;
                }
                RANDOM_STEP => {
                    let Some(rect) = image.get(operands..operands + 4) else {
                        self.state = State::Frozen;
                        return;
                    };
                    self.pc = operands + 4;
                    if self.random_step([rect[0], rect[1], rect[2], rect[3]], around) {
                        return;
                    }
                }
                BRANCH_ON_PLAYER_NEAR => {
                    if !self.branch_near_player(operands, bank, around) {
                        return;
                    }
                }
                BRANCH_ON_GLOBAL => self.pc = operands + 4,
                BRANCH_ON_FLAG => {
                    if !self.branch_on_flag(operands, bank, around) {
                        return;
                    }
                }
                CHAINED_BRANCH | CHAINED_DESPAWN => {
                    if !self.branch_on_chain(service, operands, bank, around) {
                        return;
                    }
                }
                // Anything else is stepped over by its derived length. That
                // includes text and flag writes: the loop's ambient effects
                // are not the runtime's to apply from here.
                other => {
                    let length = *self.lengths[usize::from(other)]
                        .get_or_insert_with(|| actor_script::operand_length(image, other));
                    let Some(length) = length else {
                        self.state = State::Frozen;
                        return;
                    };
                    self.pc = operands + length;
                }
            }
        }
        // Spinning without yielding: a loop with no wait in it.
        self.state = State::Frozen;
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
        let standing = |facing: Direction| match facing {
            Direction::Down => (0, false),
            Direction::Up => (1, false),
            Direction::Right => (2, false),
            Direction::Left => (2, true),
        };
        if draw & 4 != 0 {
            let (selector, hflip) = standing(self.facing);
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
            let (selector, hflip) = standing(self.facing);
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
