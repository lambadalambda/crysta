//! Synthetic controls for the opt-in source-derived directional resolver.
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState};

fn room(kind: u16, col: usize, row: usize) -> Room {
    let mut cells = vec![0; 8 * 8];
    cells[row * 8 + col] = kind << 9;
    Room::new(8, 8, cells)
        .unwrap()
        .with_passive_directional_collision()
}

fn step(room: &Room, x: u16, y: u16, direction: Direction) -> (u16, u16) {
    let mut state = WalkingState::new(x, y);
    let input = FrameInput {
        direction: Some(direction),
    };
    state.step(room, input).unwrap();
    state.step(room, input).unwrap();
    state.step(room, input).unwrap();
    state.position()
}

#[test]
fn old_edge_slopes_slide_in_all_directions_and_mirrors() {
    // $D447/$D4C4, $D82E/$D8A6, $DBD0/$DC26, $DF4C/$DFA2.
    for (kind, direction, x, y, col, row, expected) in [
        (6, Direction::Up, 63, 72, 3, 3, (65, 71)),
        (7, Direction::Up, 65, 72, 4, 3, (63, 71)),
        (7, Direction::Down, 63, 56, 3, 3, (65, 57)),
        (6, Direction::Down, 65, 56, 4, 3, (63, 57)),
        (6, Direction::Left, 64, 71, 3, 3, (63, 73)),
        (7, Direction::Left, 64, 73, 3, 4, (63, 71)),
        (7, Direction::Right, 48, 71, 3, 3, (49, 73)),
        (6, Direction::Right, 48, 73, 3, 4, (49, 71)),
    ] {
        assert_eq!(
            step(&room(kind, col, row), x, y, direction),
            expected,
            "{kind} {direction:?}"
        );
    }
}

#[test]
fn opt_in_preserves_raw_cells_and_default_rejection() {
    let candidate = room(6, 3, 4);
    let default = Room::new(8, 8, candidate.cells().to_vec()).unwrap();
    let mut state = WalkingState::new(48, 73);
    let input = FrameInput {
        direction: Some(Direction::Right),
    };
    state.step(&default, input).unwrap();
    state.step(&default, input).unwrap();
    let before = state;
    assert_eq!(
        state.step(&default, input),
        Err(Unqualified::UnsupportedType(6))
    );
    assert_eq!(state, before);
    assert_eq!(candidate.cells(), default.cells());
}

#[test]
fn unknown_and_flag_input_dependent8_fail_atomically() {
    for kind in [1, 8, 9, 17, 26, 30, 31] {
        let candidate = room(kind, 3, 4);
        let mut state = WalkingState::new(48, 73);
        let input = FrameInput {
            direction: Some(Direction::Right),
        };
        state.step(&candidate, input).unwrap();
        state.step(&candidate, input).unwrap();
        let before = state.encode_snapshot();
        assert_eq!(
            state.step(&candidate, input),
            Err(Unqualified::UnsupportedType(u8::try_from(kind).unwrap()))
        );
        assert_eq!(state.encode_snapshot(), before);
    }
}

#[test]
fn halo_error_during_diagonal_slope_probe_is_atomic() {
    let candidate = room(6, 3, 4).with_sample_halo([3, 3, 4, 5]).unwrap();
    let mut state = WalkingState::new(48, 73);
    let input = FrameInput {
        direction: Some(Direction::Right),
    };
    state.step(&candidate, input).unwrap();
    state.step(&candidate, input).unwrap();
    let before = state.encode_snapshot();
    assert_eq!(
        state.step(&candidate, input),
        Err(Unqualified::SampleOutsideAdmission)
    );
    assert_eq!(state.encode_snapshot(), before);
}

#[test]
fn snapshots_replay_slope_cadence_with_immutable_room_policy() {
    for (kind, x, d) in [(6, 48, Direction::Right), (7, 64, Direction::Left)] {
        let candidate = room(kind, 3, 4);
        let original = candidate.clone();
        let mut state = WalkingState::new(x, 73);
        let input = FrameInput { direction: Some(d) };
        for _ in 0..7 {
            state.step(&candidate, input).unwrap();
        }
        let bytes = state.encode_snapshot();
        let mut restored = WalkingState::decode_snapshot(&candidate, &bytes).unwrap();
        for _ in 0..20 {
            assert_eq!(
                state.step(&candidate, input).unwrap(),
                restored.step(&candidate, input).unwrap()
            );
            assert_eq!(state.encode_snapshot(), restored.encode_snapshot());
        }
        assert_eq!(candidate, original);
        assert!(candidate.passive_directional_collision());
        assert!(!Room::new_passive(8, 8, candidate.cells().to_vec())
            .unwrap()
            .passive_directional_collision());
    }
}
