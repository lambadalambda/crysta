//! Ark's run: the dash a double tap starts, and the brake that ends it,
//! as measured on the native probe (`docs/input-admission.md`).
//!
//! A second press of the same direction inside the onset window, which
//! [`WalkingState::step`] refuses as an accelerated trigger, starts a dash
//! (`$84:AE12`): a still setup frame, then 3, 2, 2 pixels a frame for as long
//! as the direction is held. Released, it runs on for 8 frames of grace;
//! the same direction again resumes it, another one turns it, and the
//! opposite one brakes at once. The brake slides 15 pixels over 16 frames
//! (sound `$0D`), and a direction from its fifth frame walks off. A wall
//! ends the dash: a walk if the direction is held, else a stand.
//! Diagonals, the dash attack and the dash jump are not modelled.

use crate::{Direction, FrameInput, MovementOutput, Room, Unqualified, WalkingState};

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
    /// Dashing: `frame` 0 is the still setup frame; `grace` counts down
    /// once the direction is released.
    Dash {
        /// The way it runs.
        direction: Direction,
        /// 0 on the setup frame, then 1..=3 around the stream.
        frame: u8,
        /// Frames left to run once released.
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

/// A frame of walking or running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunStep {
    /// The movement, as [`WalkingState::step`] reports it.
    pub output: MovementOutput,
    /// Whether a brake started this frame (port 3 `$0D`).
    pub braked: bool,
}

const fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
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
    input: FrameInput,
) -> Result<RunStep, Unqualified> {
    let (x, y) = walking.position();
    let (next, direction, distance) = match *run {
        None => match walking.step(room, input) {
            Err(Unqualified::AcceleratedTrigger(direction)) => {
                (Some(dash(direction, 0, GRACE)), direction, 0)
            }
            other => {
                return other.map(|output| RunStep {
                    output,
                    braked: false,
                })
            }
        },
        Some(Run::Dash {
            direction,
            frame,
            grace,
        }) => {
            let stride = DASH[usize::from(frame % 3)];
            match input.direction {
                Some(held) if held == opposite(direction) => {
                    (Some(brake(direction, 0)), direction, BRAKE[0])
                }
                Some(held) if held == direction => (
                    Some(dash(direction, frame % 3 + 1, GRACE)),
                    direction,
                    stride,
                ),
                // Another direction turns the dash, from a setup frame.
                Some(turn) => (Some(dash(turn, 0, GRACE)), turn, 0),
                None if grace <= 1 => (Some(brake(direction, 0)), direction, BRAKE[0]),
                None => (
                    Some(dash(direction, frame % 3 + 1, grace - 1)),
                    direction,
                    stride,
                ),
            }
        }
        Some(Run::Brake { direction, frame }) => {
            if frame + 1 >= BRAKE_CANCEL && input.direction.is_some() {
                let mut standing = WalkingState::new(x, y);
                let output = standing.step(room, input)?;
                *walking = standing;
                *run = None;
                return Ok(RunStep {
                    output,
                    braked: false,
                });
            }
            match BRAKE.get(usize::from(frame) + 1) {
                Some(&slide) => (Some(brake(direction, frame + 1)), direction, slide),
                None => (None, direction, 0),
            }
        }
    };
    let (dx, dy) = delta(direction, distance);
    let (nx, ny, blocked) = room.resolve(x, y, Some(direction), dx, dy)?;
    let output = MovementOutput {
        x: nx,
        y: ny,
        dx: i16::try_from(i32::from(nx) - i32::from(x))
            .map_err(|_| Unqualified::ArithmeticOverflow)?,
        dy: i16::try_from(i32::from(ny) - i32::from(y))
            .map_err(|_| Unqualified::ArithmeticOverflow)?,
        attempted_dx: dx,
        attempted_dy: dy,
        blocked,
    };
    *walking = WalkingState::new(nx, ny);
    *run = next;
    if blocked && matches!(next, Some(Run::Dash { .. })) {
        // A wall ends the dash (`$80:E70A`): a walk if the direction is held.
        *run = None;
        if input.direction == Some(direction) {
            *walking = WalkingState::walking(nx, ny, direction, ONSET);
        }
    }
    Ok(RunStep {
        output,
        braked: matches!(next, Some(Run::Brake { frame: 0, .. })),
    })
}

const fn dash(direction: Direction, frame: u8, grace: u8) -> Run {
    Run::Dash {
        direction,
        frame,
        grace,
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
    fn a_double_tap_dashes_three_two_two_after_a_still_frame() {
        let mut inputs = tap();
        inputs.extend([R; 7]);
        let (moves, brakes, state) = run(&floor(), &inputs);
        assert_eq!(moves[3..], [0, 3, 2, 2, 3, 2, 2, 3]);
        assert!(brakes.is_empty());
        assert!(matches!(state, Some(Run::Dash { grace: GRACE, .. })));
    }

    #[test]
    fn released_it_runs_eight_frames_then_brakes_fifteen_pixels() {
        let mut inputs = tap();
        inputs.extend([R, R]);
        inputs.extend([None; 8 + 16]);
        let (moves, brakes, state) = run(&floor(), &inputs);
        assert_eq!(brakes, [6 + 7]);
        assert_eq!(moves[6..13], [2, 3, 2, 2, 3, 2, 2], "the grace");
        assert_eq!(moves[13..29].iter().sum::<i16>(), 15, "the slide");
        assert_eq!(state, None, "standing after 16 frames");
    }

    #[test]
    fn the_opposite_brakes_at_once_and_a_press_walks_off_from_the_fifth_frame() {
        let mut inputs = tap();
        inputs.extend([R, Some(Direction::Left), None, None, None]);
        let (_, brakes, state) = run(&floor(), &inputs);
        assert_eq!(brakes, [5]);
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
}
