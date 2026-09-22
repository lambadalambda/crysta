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
fn type8_requires_its_separate_state_contract_and_preserves_defaults() {
    let cells = vec![8 << 9; 8 * 8];
    for candidate in [
        Room::new(8, 8, cells.clone()).unwrap(),
        Room::new_passive(8, 8, cells.clone()).unwrap(),
        Room::new(8, 8, cells.clone())
            .unwrap()
            .with_passive_directional_collision(),
    ] {
        assert!(!candidate.passive_directional_type8_special_bit_clear());
        // Without the $097C&4-clear assertion (including unknown/set state),
        // neither the legacy constructors nor the ordinary opt-in admit raw8.
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            let mut state = WalkingState::new(56, 64);
            let input = FrameInput {
                direction: Some(direction),
            };
            state.step(&candidate, input).unwrap();
            state.step(&candidate, input).unwrap();
            let before = state.encode_snapshot();
            assert_eq!(
                state.step(&candidate, input),
                Err(Unqualified::UnsupportedType(8))
            );
            assert_eq!(state.encode_snapshot(), before);
        }
    }
}

#[test]
fn type8_aligned_movement_and_down6_corner_use_source_coordinates() {
    for (direction, x, y, expected) in [
        (Direction::Up, 56, 72, (56, 72)),
        (Direction::Down, 56, 56, (56, 57)),
        (Direction::Left, 64, 64, (63, 64)),
        (Direction::Right, 48, 64, (49, 64)),
    ] {
        let candidate = room(8, 3, 3).with_passive_directional_type8_special_bit_clear();
        assert_eq!(step(&candidate, x, y, direction), expected, "{direction:?}");
    }
    for flag in [0, 0x8000] {
        let mut cells = vec![0; 8 * 8];
        cells[3 * 8 + 3] = 6 << 9;
        cells[3 * 8 + 4] = flag | 8 << 9;
        let candidate = Room::new(8, 8, cells)
            .unwrap()
            .with_passive_directional_type8_special_bit_clear();
        for (x, expected_x) in [(60, 59), (64, 65), (68, 69)] {
            assert_eq!(step(&candidate, x, 56, Direction::Down), (expected_x, 56));
        }
        // The corner's neighbor must still be admitted by the sample halo.
        let bounded = candidate.with_sample_halo([3, 3, 4, 4]).unwrap();
        let mut state = WalkingState::new(60, 56);
        let input = FrameInput {
            direction: Some(Direction::Down),
        };
        state.step(&bounded, input).unwrap();
        state.step(&bounded, input).unwrap();
        let before = state.encode_snapshot();
        assert_eq!(
            state.step(&bounded, input),
            Err(Unqualified::SampleOutsideAdmission)
        );
        assert_eq!(state.encode_snapshot(), before);
    }
}

#[test]
fn type8_contract_is_immutable_and_does_not_admit_other_unknown_types() {
    let raw = Room::new(8, 8, vec![8 << 9; 8 * 8]).unwrap();
    let candidate = raw
        .clone()
        .with_passive_directional_type8_special_bit_clear();
    assert_eq!(candidate.cells(), raw.cells());
    assert!(candidate.passive_directional_collision());
    assert!(candidate
        .clone()
        .passive_directional_type8_special_bit_clear());
    let mut state = WalkingState::new(56, 64);
    let input = FrameInput {
        direction: Some(Direction::Down),
    };
    for _ in 0..7 {
        state.step(&candidate, input).unwrap();
    }
    let mut restored = WalkingState::decode_snapshot(&candidate, &state.encode_snapshot()).unwrap();
    for _ in 0..20 {
        assert_eq!(
            state.step(&candidate, input),
            restored.step(&candidate, input)
        );
        assert_eq!(state.encode_snapshot(), restored.encode_snapshot());
    }
    for kind in [1, 9, 17, 26, 30, 31] {
        let unknown = room(kind, 3, 4).with_passive_directional_type8_special_bit_clear();
        let mut state = WalkingState::new(48, 73);
        let input = FrameInput {
            direction: Some(Direction::Right),
        };
        state.step(&unknown, input).unwrap();
        state.step(&unknown, input).unwrap();
        let before = state.encode_snapshot();
        assert_eq!(
            state.step(&unknown, input),
            Err(Unqualified::UnsupportedType(u8::try_from(kind).unwrap()))
        );
        assert_eq!(state.encode_snapshot(), before);
    }
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
