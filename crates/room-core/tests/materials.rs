//! Source-table material pairs for ordinary, action-free house collision.
use room_core::{Direction, FrameInput, Room, WalkingState};

const DIRECTIONS: [Direction; 4] = [
    Direction::Left,
    Direction::Right,
    Direction::Up,
    Direction::Down,
];

fn geometry(direction: Direction, q: u16) -> (u16, u16, usize, usize) {
    let horizontal = matches!(direction, Direction::Left | Direction::Right);
    let (x, y) = if horizontal {
        (104, 112 + q)
    } else {
        (104 + q, 112)
    };
    let (col, row) = match direction {
        Direction::Left => (5, 6),
        Direction::Right => (7, 6),
        Direction::Up => (6, 5),
        Direction::Down => (6, 7),
    };
    let first = row * 32 + col;
    (x, y, first, first + if horizontal { 32 } else { 1 })
}

fn attempt(room: &Room, x: u16, y: u16, direction: Direction) -> room_core::MovementOutput {
    let mut state = WalkingState::new(x, y);
    let input = FrameInput {
        direction: Some(direction),
    };
    for _ in 0..2 {
        state.step(room, input).unwrap();
    }
    state.step(room, input).unwrap()
}

#[test]
fn open_solid_partial_pairs_match_all_four_tables_at_every_remainder() {
    // Ordered O/S/P rows and columns; aligned q=0 ignores the second sample.
    let low = [[0, -1, -1], [0, 0, 0], [0, -1, 0]];
    let high = [[0, 0, 0], [1, 0, 1], [1, 0, 0]];
    for direction in DIRECTIONS {
        for q in 0..16 {
            for (a, class_a) in [(0, 0), (2, 0), (22, 0), (12, 1), (14, 1), (16, 2)] {
                for (b, class_b) in [(0, 0), (2, 0), (22, 0), (12, 1), (14, 1), (16, 2)] {
                    let (x, y, first, second) = geometry(direction, q);
                    let mut cells = vec![0; 2048];
                    cells[first] = a << 9;
                    cells[second] = b << 9;
                    let room = Room::new(32, 64, cells).unwrap();
                    let out = attempt(&room, x, y, direction);
                    let blocked = class_a != 0 || (q != 0 && class_b != 0);
                    let nudge = if q == 0 {
                        0
                    } else if q < 8 {
                        low[class_a][class_b]
                    } else {
                        high[class_a][class_b]
                    };
                    let sign = if matches!(direction, Direction::Left | Direction::Up) {
                        -1
                    } else {
                        1
                    };
                    let primary = if blocked { 0 } else { sign };
                    let expected = if matches!(direction, Direction::Left | Direction::Right) {
                        (primary, nudge)
                    } else {
                        (nudge, primary)
                    };
                    assert_eq!(
                        (out.dx, out.dy, out.blocked),
                        (expected.0, expected.1, blocked),
                        "{direction:?} q={q} {a}/{b}"
                    );
                }
            }
        }
    }
}

#[test]
fn passive_flags_override_every_new_stored_type_but_preserve_raw_cells() {
    for direction in DIRECTIONS {
        for q in 0..16 {
            for stored in 0..32 {
                for other in [0, 2, 22, 12, 14, 16] {
                    for flagged_first in [false, true] {
                        let (x, y, first, second) = geometry(direction, q);
                        let mut cells = vec![0; 2048];
                        let raw = 0x8000 | (stored << 9) | 0x45;
                        cells[first] = if flagged_first { raw } else { other << 9 };
                        cells[second] = if flagged_first { other << 9 } else { raw };
                        let flagged = if flagged_first { first } else { second };
                        let passive = Room::new_passive(32, 64, cells.clone()).unwrap();
                        assert_eq!(passive.cells(), cells);
                        cells[flagged] = 12 << 9;
                        let solid = Room::new(32, 64, cells).unwrap();
                        assert_eq!(
                            attempt(&passive, x, y, direction),
                            attempt(&solid, x, y, direction),
                            "{direction:?} q={q} raw={raw:x}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn default_policy_rejects_flags_and_passive_policy_still_rejects_old_slopes() {
    use room_core::Unqualified;
    for direction in DIRECTIONS {
        let (x, y, first, _) = geometry(direction, 1);
        let mut cells = vec![0; 2048];
        cells[first] = 0x9845;
        let room = Room::new(32, 64, cells).unwrap();
        let mut state = WalkingState::new(x, y);
        let input = FrameInput {
            direction: Some(direction),
        };
        for _ in 0..2 {
            state.step(&room, input).unwrap();
        }
        let before = state;
        assert_eq!(
            state.step(&room, input),
            Err(Unqualified::FlaggedCell(0x9845))
        );
        assert_eq!(state, before);

        for q in 0..16 {
            for kind in [6, 7] {
                for flag in [0, 0x8000] {
                    for second in [false, true] {
                        if q == 0 && second {
                            continue;
                        }
                        let (x, y, _, _) = geometry(direction, q);
                        let mut cells = vec![0; 2048];
                        let old = 6 * 32
                            + 6
                            + if second {
                                if matches!(direction, Direction::Left | Direction::Right) {
                                    32
                                } else {
                                    1
                                }
                            } else {
                                0
                            };
                        cells[old] = flag | (kind << 9);
                        let room = Room::new_passive(32, 64, cells).unwrap();
                        let mut state = WalkingState::new(x, y);
                        for _ in 0..2 {
                            state.step(&room, input).unwrap();
                        }
                        let before = state;
                        assert_eq!(
                            state.step(&room, input),
                            Err(Unqualified::UnsupportedType(u8::try_from(kind).unwrap())),
                            "{direction:?} q={q} old={kind} flag={flag:x}"
                        );
                        assert_eq!(state, before);
                    }
                }
            }
        }
    }
}

#[test]
fn partial_solid_nudges_align_then_block_without_freezing_cadence() {
    for direction in DIRECTIONS {
        for (q, first_type, second_type, nudge) in [(1, 16, 12, -1), (15, 14, 16, 1)] {
            let (x, y, first, second) = geometry(direction, q);
            let mut cells = vec![0; 2048];
            cells[first] = first_type << 9;
            cells[second] = second_type << 9;
            let room = Room::new(32, 64, cells).unwrap();
            let mut state = WalkingState::new(x, y);
            let input = FrameInput {
                direction: Some(direction),
            };
            for _ in 0..2 {
                state.step(&room, input).unwrap();
            }
            let one = state.step(&room, input).unwrap();
            let expected = if matches!(direction, Direction::Left | Direction::Right) {
                (0, nudge)
            } else {
                (nudge, 0)
            };
            assert_eq!((one.dx, one.dy), expected);
            let mut restored =
                WalkingState::decode_snapshot(&room, &state.encode_snapshot()).unwrap();
            let two = state.step(&room, input).unwrap();
            assert_eq!(restored.step(&room, input).unwrap(), two);
            assert_eq!((two.dx, two.dy, two.blocked), (0, 0, true));
            assert_eq!(two.attempted_dx.abs() + two.attempted_dy.abs(), 2);
        }
    }
}
