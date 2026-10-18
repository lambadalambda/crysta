//! Ark's run: the dash a double tap starts, and the brake that ends it,
//! as measured on the native probe (`docs/input-admission.md`, "Dash").
//!
//! A second press of the same direction inside the onset window, which
//! [`WalkingState::step`] refuses as an accelerated trigger, starts a dash
//! (`$84:AE12`): the press's frame and a setup frame stand still, then it
//! runs 3, 2, 2 pixels a frame for as long as the direction is held.
//! Released, it runs on for 8 frames of grace; the same direction again
//! resumes it, the opposite one brakes the frame after, and another one
//! turns it the next frame: it faces and brakes the new way, but moves on
//! the old way until the new one is held, which starts it at 3. The brake
//! slides 15 pixels over 16 frames (sound `$0D`), and a
//! direction from its fifth frame walks off. A wall ends the dash: a walk
//! if the direction is held, else a stand.
//!
//! A diagonal dashes along its lead with the cross 1 pixel a frame; a
//! change of the cross restarts the stream the next frame, and the grace
//! keeps the last one. The brake slides on the lead alone, and only a wall
//! across the lead stops the dash. The dash attack and the dash jump are
//! the runtime's (`docs/combat.md`, `docs/jump.md`).

use crate::{Direction, MovementOutput, PadInput, Room, Unqualified, WalkingState};

/// The dash's repeating stream after its setup frame.
const DASH: [i16; 3] = [3, 2, 2];
/// Frames the dash runs on after the direction is released.
const GRACE: u8 = 8;
/// The brake's slide, a frame each (`$84:A223`).
const BRAKE: [i16; 16] = [2, 2, 1, 2, 1, 2, 1, 2, 1, 0, 0, 1, 0, 0, 0, 0];
/// The brake frame, from 0, on which a direction first walks off: after
/// the 4 frames of `COP C1 04`.
const BRAKE_CANCEL: u8 = 4;
/// The onset window a wall stop re-arms.
const ONSET: u8 = 11;

/// A run under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// The frame of a double tap: the dash's setup frame follows.
    Tap {
        /// The way it will run.
        direction: Direction,
        /// The diagonal's other direction.
        cross: Option<Direction>,
    },
    /// Dashing; `grace` counts down once the direction is released.
    Dash {
        /// The way it faces, is held and brakes.
        direction: Direction,
        /// The way its stream moves: the old one after a turn until the new
        /// way's stream starts (native `e_turn_off`, `e_turn_back`).
        way: Direction,
        /// The diagonal's other direction, 1 pixel a frame.
        cross: Option<Direction>,
        /// Where in the 3, 2, 2 stream the next stride is, 0..=2.
        frame: u8,
        /// Frames left to run once released.
        grace: u8,
        /// A turn that takes effect the next frame, or a restart in its own
        /// way.
        turn: Option<(Direction, Option<Direction>)>,
    },
    /// The dash after a dash-throw (`$84:9051`): it stands; held, it
    /// runs at once from 3, else it brakes when `grace` is out.
    Rest {
        /// The way it will run.
        direction: Direction,
        /// Frames left to stand.
        grace: u8,
    },
    /// Braking.
    Brake {
        /// The way it slides.
        direction: Direction,
        /// Frames into the slide.
        frame: u8,
    },
}

impl Run {
    /// Whether the dash runs with its direction held, before the grace a
    /// release starts: the loop that takes A (`$84:9078`..`916B`).
    #[must_use]
    pub const fn held(self) -> bool {
        matches!(self, Self::Dash { grace: GRACE, .. })
    }

    /// The dash Ark's control returns to after a dash-throw, from the frame
    /// after its still one (native traces `t_*`).
    #[must_use]
    pub const fn after_throw(direction: Direction) -> Self {
        Self::Rest {
            direction,
            grace: GRACE - 1,
        }
    }

    /// A dash that runs this frame's stride, then starts its stream over
    /// at 3, as when a dash jump's pose hands back to the dash (native
    /// trace `d_jp_right_held`).
    #[must_use]
    pub const fn restarted(self) -> Self {
        match self {
            Self::Dash {
                direction,
                way,
                cross,
                frame,
                grace,
                ..
            } => Self::Dash {
                direction,
                way,
                cross,
                frame,
                grace,
                turn: Some((direction, cross)),
            },
            other => other,
        }
    }
}

/// A frame of walking or running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunStep {
    /// The movement, as [`WalkingState::step`] reports it.
    pub output: MovementOutput,
    /// Whether a brake started this frame (port 3 `$0D`).
    pub braked: bool,
    /// Whether a wall stopped the dash this frame (port 3 `$4F`).
    pub bumped: bool,
}

impl RunStep {
    /// A frame that neither brakes nor bumps.
    const fn plain(output: MovementOutput) -> Self {
        Self {
            output,
            braked: false,
            bumped: false,
        }
    }
}

/// The lead and the cross of `input` ([`PadInput::lead`]): a dash keeps
/// its way, else as the walk ([`WalkingState::lead`]).
#[must_use]
pub fn lead(
    walking: &WalkingState,
    run: Option<Run>,
    input: PadInput,
) -> (Option<Direction>, Option<Direction>) {
    match run {
        Some(Run::Dash { direction, .. } | Run::Rest { direction, .. }) => {
            input.lead(Some(direction))
        }
        _ => walking.lead(input),
    }
}

/// A frame of a move the player's state forces, `distance` pixels along
/// `direction` against the walls, as the dash moves (the dash attack's
/// stream, `$84:9549`). Returns whether a wall stopped it.
///
/// # Errors
/// As the dash's step; on an error the place is unchanged.
pub fn push(
    walking: &mut WalkingState,
    room: &Room,
    direction: Direction,
    distance: i16,
) -> Result<bool, Unqualified> {
    let (x, y) = walking.position();
    let (nx, ny, blocked) = resolve(room, (x, y), direction, None, distance)?;
    *walking = WalkingState::new(nx, ny);
    Ok(blocked)
}

/// Whether Ark runs free or with a pot in hand.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Free,
    /// The carry dash (`$84:B5A7`, native traces `c_*`): the carry walk's
    /// cadence; a release runs one frame more, then ends without a brake;
    /// the opposite way reverses it; a wall does not end it.
    Carry,
}

/// One frame of walking, or of the run under way.
///
/// # Errors
/// As [`WalkingState::step`], but for the accelerated trigger, which
/// starts a dash. Every error leaves both states unchanged.
pub fn step(
    walking: &mut WalkingState,
    run: &mut Option<Run>,
    room: &Room,
    input: impl Into<PadInput>,
) -> Result<RunStep, Unqualified> {
    step_in(Mode::Free, walking, run, room, input.into())
}

/// As [`step`], with a pot in hand (`crate::pots`).
pub(crate) fn carry_step(
    walking: &mut WalkingState,
    run: &mut Option<Run>,
    room: &Room,
    input: impl Into<PadInput>,
) -> Result<RunStep, Unqualified> {
    step_in(Mode::Carry, walking, run, room, input.into())
}

fn step_in(
    mode: Mode,
    walking: &mut WalkingState,
    run: &mut Option<Run>,
    room: &Room,
    input: PadInput,
) -> Result<RunStep, Unqualified> {
    let carry = mode == Mode::Carry;
    let before = *run;
    let (held, held_cross) = lead(walking, *run, input);
    let (x, y) = walking.position();
    let (next, direction, cross, distance) = match *run {
        None => match walking.step_with_cadence(room, input, carry) {
            Err(Unqualified::AcceleratedTrigger(direction)) => {
                let cross = held_cross;
                (Some(Run::Tap { direction, cross }), direction, None, 0)
            }
            other => return other.map(RunStep::plain),
        },
        Some(Run::Tap { direction, cross }) => {
            (Some(dash(direction, cross, 0, GRACE)), direction, None, 0)
        }
        Some(now @ Run::Dash { .. }) => dash_frame(mode, now, held, held_cross),
        Some(Run::Rest { direction, grace }) => match held {
            Some(held) if held == direction => (
                Some(dash(direction, None, 1, GRACE)),
                direction,
                None,
                DASH[0],
            ),
            _ if grace == 0 => (Some(brake(direction, 0)), direction, None, BRAKE[0]),
            _ => (
                Some(Run::Rest {
                    direction,
                    grace: grace - 1,
                }),
                direction,
                None,
                0,
            ),
        },
        Some(Run::Brake { direction, frame }) => {
            let slide = BRAKE.get(usize::from(frame) + 1).copied();
            let next = slide.map(|_| brake(direction, frame + 1));
            (next, direction, None, slide.unwrap_or(0))
        }
    };
    let (nx, ny, blocked) = resolve(room, (x, y), direction, cross, distance)?;
    let attempted = attempt(direction, cross, distance);
    let output = MovementOutput {
        x: nx,
        y: ny,
        dx: i16::try_from(i32::from(nx) - i32::from(x))
            .map_err(|_| Unqualified::ArithmeticOverflow)?,
        dy: i16::try_from(i32::from(ny) - i32::from(y))
            .map_err(|_| Unqualified::ArithmeticOverflow)?,
        attempted_dx: attempted.0,
        attempted_dy: attempted.1,
        blocked,
    };
    let mut standing = WalkingState::new(nx, ny);
    let walks_off = matches!(*run, Some(Run::Brake { frame, .. }) if frame + 1 >= BRAKE_CANCEL)
        && held.is_some();
    if walks_off {
        // The slide's last frame is the walk's input frame.
        standing.step(room, input)?;
        *walking = standing;
        *run = None;
        return Ok(RunStep::plain(output));
    }
    *walking = standing;
    *run = next;
    if carry && next.is_none() {
        // Its last frame: a direction held walks on at once.
        if let (Some(Run::Dash { .. }), Some(held)) = (before, held) {
            *walking = WalkingState::walking(nx, ny, held, ONSET);
        }
    }
    let bumped = !carry && blocked && matches!(next, Some(Run::Dash { .. }));
    if bumped {
        // A wall across the lead ends the dash (`$80:E70A`): a walk if
        // the direction is held.
        *run = None;
        if held == Some(direction) {
            *walking = WalkingState::walking(nx, ny, direction, ONSET);
        }
    }
    Ok(RunStep {
        output,
        braked: matches!(next, Some(Run::Brake { frame: 0, .. })),
        bumped,
    })
}

/// A frame of `now`, a dash, with `held` and `held_cross` on the pad: the
/// run after it, and this frame's move (the way, the cross, the pixels).
type DashFrame = (Option<Run>, Direction, Option<Direction>, i16);

fn dash_frame(
    mode: Mode,
    now: Run,
    held: Option<Direction>,
    held_cross: Option<Direction>,
) -> DashFrame {
    let Run::Dash {
        direction,
        way,
        cross,
        frame,
        grace,
        turn,
    } = now
    else {
        return (Some(now), Direction::Down, None, 0);
    };
    let carry = mode == Mode::Carry;
    let following = (frame + 1) % 3;
    // Not traced: with a pot, a turn and the opposite press compare with
    // the facing, not the way it moves.
    // The stream runs on; a cross moves only along its own way.
    let crossing = cross.filter(|_| way == direction);
    let keep = |next: Option<Run>| (next, way, crossing, DASH[usize::from(frame)]);
    let with = |direction, way, cross, frame, grace| Run::Dash {
        direction,
        way,
        cross,
        frame,
        grace,
        turn: None,
    };
    // A new cross starts the stream over the next frame.
    let on = |new: Option<Direction>, grace| {
        let frame = if new == cross { following } else { 0 };
        Some(with(direction, way, new, frame, grace))
    };
    match (turn, held) {
        // A restart in its own way (a dash jump's end): this frame's
        // stride, then 3.
        (Some((turn, new)), _) if turn == direction => {
            keep(Some(with(direction, way, new, 0, grace)))
        }
        // A turn takes effect: the new facing; the stream runs on, or
        // starts over at 3 if it already moves that way.
        (Some((turn, new)), _) => {
            let grace = if held.is_some() {
                GRACE
            } else {
                grace.saturating_sub(1)
            };
            if turn == way {
                (Some(with(turn, way, new, 1, grace)), way, new, DASH[0])
            } else {
                keep(Some(with(turn, way, new, following, grace)))
            }
        }
        // Released, a carry dash runs this frame, then ends.
        _ if carry && grace == 0 => keep(None),
        // Held after a turn, the new way's stream starts at 3.
        (None, Some(held)) if held == direction && way != direction => (
            Some(with(direction, direction, held_cross, 1, GRACE)),
            direction,
            held_cross,
            DASH[0],
        ),
        (None, Some(held)) if held == direction => keep(on(held_cross, GRACE)),
        // Out of grace, or the frame after an opposite press.
        _ if grace == 0 => (Some(brake(direction, 0)), direction, None, BRAKE[0]),
        (None, Some(held)) if held == direction.opposite() => {
            if carry {
                keep(Some(dash(held, None, 0, GRACE)))
            } else {
                keep(on(cross, 0))
            }
        }
        // The turn takes effect the next frame.
        (None, Some(held)) => keep(Some(Run::Dash {
            direction,
            way,
            cross,
            frame: following,
            grace,
            turn: Some((held, held_cross)),
        })),
        (None, None) => keep(on(cross, if carry { 0 } else { grace - 1 })),
    }
}

/// The move `distance` along `direction`, with 1 along `cross` when it
/// moves at all.
fn attempt(direction: Direction, cross: Option<Direction>, distance: i16) -> (i16, i16) {
    let (dx, dy) = delta(direction, distance);
    let (cx, cy) = match cross {
        Some(cross) if distance != 0 => delta(cross, 1),
        _ => (0, 0),
    };
    (dx + cx, dy + cy)
}

/// Moves from `at` as [`attempt`] against the walls, x first; returns the
/// place and whether a wall across `direction` stopped it.
fn resolve(
    room: &Room,
    at: (u16, u16),
    direction: Direction,
    cross: Option<Direction>,
    distance: i16,
) -> Result<(u16, u16, bool), Unqualified> {
    let (x, y) = at;
    let Some(cross) = cross.filter(|_| distance != 0) else {
        let (dx, dy) = delta(direction, distance);
        return room.resolve(x, y, Some(direction), dx, dy);
    };
    let (horizontal, vertical) = if direction.horizontal() {
        (direction, cross)
    } else {
        (cross, direction)
    };
    let along = |axis: Direction| if axis == direction { distance } else { 1 };
    let (dx, _) = delta(horizontal, along(horizontal));
    let (x, y, across) = room.resolve(x, y, Some(horizontal), dx, 0)?;
    let (_, dy) = delta(vertical, along(vertical));
    let (x, y, down) = room.resolve(x, y, Some(vertical), 0, dy)?;
    let blocked = if horizontal == direction {
        across
    } else {
        down
    };
    Ok((x, y, blocked))
}

const fn dash(direction: Direction, cross: Option<Direction>, frame: u8, grace: u8) -> Run {
    Run::Dash {
        direction,
        way: direction,
        cross,
        frame,
        grace,
        turn: None,
    }
}

const fn brake(direction: Direction, frame: u8) -> Run {
    Run::Brake { direction, frame }
}

fn delta(direction: Direction, distance: i16) -> (i16, i16) {
    let signed = if direction.negative() {
        -distance
    } else {
        distance
    };
    if direction.horizontal() {
        (signed, 0)
    } else {
        (0, signed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameInput;

    #[test]
    fn a_dash_is_held_until_its_grace_starts() {
        let dash = |grace| dash(Direction::Up, None, 1, grace);
        assert!(dash(GRACE).held());
        assert!(!dash(GRACE - 1).held());
        let brake = Run::Brake {
            direction: Direction::Up,
            frame: 0,
        };
        assert!(!brake.held());
    }
    use alloc::vec;
    use alloc::vec::Vec;

    fn floor() -> Room {
        Room::new(32, 64, vec![0; 2048]).unwrap()
    }

    /// Runs `inputs` from (256, 256), returning each frame's X move and
    /// whether it braked.
    fn run(room: &Room, inputs: &[Option<Direction>]) -> (Vec<i16>, Vec<usize>, Option<Run>) {
        let mut walking = WalkingState::new(256, 256);
        let mut state = None;
        let (mut moves, mut brakes) = (Vec::new(), Vec::new());
        for (frame, &direction) in inputs.iter().enumerate() {
            let step = step(&mut walking, &mut state, room, FrameInput { direction }).unwrap();
            moves.push(step.output.dx);
            if step.braked {
                brakes.push(frame);
            }
        }
        (moves, brakes, state)
    }

    const R: Option<Direction> = Some(Direction::Right);

    /// Right, released two frames, Right again: the double tap.
    fn tap() -> Vec<Option<Direction>> {
        vec![R, None, None, R]
    }

    #[test]
    fn a_double_tap_dashes_three_two_two_after_two_still_frames() {
        let mut inputs = tap();
        inputs.extend([R; 7]);
        let (moves, brakes, state) = run(&floor(), &inputs);
        assert_eq!(moves[3..], [0, 0, 3, 2, 2, 3, 2, 2]);
        assert!(brakes.is_empty());
        assert!(matches!(state, Some(Run::Dash { grace: GRACE, .. })));
    }

    #[test]
    fn released_it_runs_eight_frames_then_brakes_fifteen_pixels() {
        let mut inputs = tap();
        inputs.extend([R, R]);
        inputs.extend([None; 8 + 16 + 1]);
        let (moves, brakes, state) = run(&floor(), &inputs);
        assert_eq!(brakes, [6 + 8]);
        assert_eq!(moves[6..14], [2, 2, 3, 2, 2, 3, 2, 2], "the grace");
        assert_eq!(moves[14..30].iter().sum::<i16>(), 15, "the slide");
        assert_eq!(state, None, "standing after 16 frames");
    }

    #[test]
    fn the_opposite_brakes_at_once_and_a_press_walks_off_from_the_fifth_frame() {
        let mut inputs = tap();
        inputs.extend([R, Some(Direction::Left), None, None, None, None]);
        let (_, brakes, state) = run(&floor(), &inputs);
        assert_eq!(brakes, [6], "the frame after the press");
        assert_eq!(
            state,
            Some(Run::Brake {
                direction: Direction::Right,
                frame: 3
            })
        );
        inputs.push(Some(Direction::Up));
        let (_, _, state) = run(&floor(), &inputs);
        assert_eq!(state, None, "walks off");
    }

    #[test]
    fn a_wall_ends_the_dash_in_a_walk_while_the_direction_is_held() {
        // A wall a few cells to the right of (256, 256).
        let mut cells = vec![0; 2048];
        for row in 0..64 {
            cells[row * 32 + 18] = 14 << 9;
        }
        let room = Room::new(32, 64, cells).unwrap();
        let mut inputs = tap();
        inputs.extend([R; 20]);
        let (moves, _, state) = run(&room, &inputs);
        assert_eq!(state, None);
        assert!(moves.iter().rev().take(3).all(|&dx| dx <= 2), "walking");
    }

    /// As [`run`], with a pot in hand.
    fn carry(room: &Room, inputs: &[Option<Direction>]) -> (Vec<i16>, Option<Run>) {
        let mut walking = WalkingState::new(256, 256);
        let mut state = None;
        let mut moves = Vec::new();
        for &direction in inputs {
            let step =
                carry_step(&mut walking, &mut state, room, FrameInput { direction }).unwrap();
            moves.push(step.output.dx);
            assert!(!step.braked && !step.bumped);
        }
        (moves, state)
    }

    #[test]
    fn a_carry_dash_stops_two_frames_after_a_release_without_a_brake() {
        // Native `c_rel_a`: released at n, it runs n and n + 1, then stands.
        let mut inputs = tap();
        inputs.extend([R; 6]);
        inputs.extend([None; 4]);
        let (moves, state) = carry(&floor(), &inputs);
        assert_eq!(moves[3..], [0, 0, 3, 2, 2, 3, 2, 2, 3, 0, 0]);
        assert_eq!(state, None);
        // `c_release1`: pressed again on the last frame, he walks on at once.
        let mut inputs = tap();
        inputs.extend([R; 6]);
        inputs.extend([None, R, R, R]);
        let (moves, _) = carry(&floor(), &inputs);
        assert_eq!(moves[10..], [2, 3, 1, 2]);
    }

    #[test]
    fn the_opposite_reverses_a_carry_dash_and_a_wall_does_not_end_it() {
        // Native `c_opp`: the old stride, then 3 the other way.
        let mut inputs = tap();
        inputs.extend([R; 6]);
        inputs.extend([Some(Direction::Left); 3]);
        let (moves, _) = carry(&floor(), &inputs);
        assert_eq!(moves[10..], [2, -3, -2]);
        // `c_d_turn`: against a wall he runs on in place.
        let mut cells = vec![0; 2048];
        for row in 0..64 {
            cells[row * 32 + 18] = 14 << 9;
        }
        let room = Room::new(32, 64, cells).unwrap();
        let mut inputs = tap();
        inputs.extend([R; 30]);
        let (moves, state) = carry(&room, &inputs);
        assert_eq!(moves.last(), Some(&0));
        assert!(matches!(state, Some(Run::Dash { grace: GRACE, .. })));
    }

    #[test]
    fn after_a_dash_throw_the_dash_waits_for_its_way() {
        // Native `t_held`/`t_free`: held, it runs at once from 3; else it
        // stands out the grace, then brakes.
        let held = |inputs: &[Option<Direction>]| {
            let mut walking = WalkingState::new(256, 256);
            let mut state = Some(Run::after_throw(Direction::Right));
            let mut moves = Vec::new();
            for &direction in inputs {
                let step =
                    step(&mut walking, &mut state, &floor(), FrameInput { direction }).unwrap();
                moves.push(step.output.dx);
            }
            moves
        };
        assert_eq!(held(&[R; 4]), [3, 2, 2, 3]);
        assert_eq!(held(&[None; 9]), [0, 0, 0, 0, 0, 0, 0, 2, 2]);
    }
}
