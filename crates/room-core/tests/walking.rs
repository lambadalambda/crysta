//! ROM-free walking and snapshot regressions.
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState};

fn floor() -> Room {
    Room::new(32, 64, vec![0; 2048]).unwrap()
}
fn input(direction: Direction) -> FrameInput {
    FrameInput {
        direction: Some(direction),
    }
}

#[test]
fn latency_horizontal_gap_and_release() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    for frame in 1..=60 {
        let out = state.step(&room, input(Direction::Right)).unwrap();
        let age = frame - 2;
        let expected = if frame <= 2 || age % 54 == 0 {
            0
        } else if age % 2 == 1 {
            1
        } else {
            2
        };
        assert_eq!(out.attempted_dx, expected, "frame {frame}");
        assert_eq!(out.dx, expected);
    }
    assert_eq!(state.step(&room, FrameInput::default()).unwrap().dx, 1);
    assert_eq!(state.step(&room, FrameInput::default()).unwrap().dx, 0);
}

#[test]
fn vertical_has_no_horizontal_restart_and_turn_resets() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    for frame in 1..=58 {
        let out = state.step(&room, input(Direction::Down)).unwrap();
        let expected = if frame <= 2 {
            0
        } else if frame % 2 == 1 {
            1
        } else {
            2
        };
        assert_eq!(out.dy, expected);
    }
    assert_eq!(state.step(&room, input(Direction::Left)).unwrap().dy, 1);
    assert_eq!(state.step(&room, input(Direction::Left)).unwrap().dx, 0);
    assert_eq!(state.step(&room, input(Direction::Left)).unwrap().dx, -1);
    assert_eq!(state.active_direction(), Some(Direction::Left));
    assert_eq!(state.phase(), 1);
}

#[test]
fn four_full_walls_block_without_stopping_cadence() {
    for (direction, col, row) in [
        (Direction::Right, 7, 6),
        (Direction::Left, 5, 6),
        (Direction::Down, 6, 7),
        (Direction::Up, 6, 5),
    ] {
        let mut cells = vec![0; 2048];
        cells[row * 32 + col] = 14 << 9;
        let room = Room::new(32, 64, cells).unwrap();
        let mut state = WalkingState::new(104, 112);
        for frame in 1..=7 {
            let out = state.step(&room, input(direction)).unwrap();
            assert_eq!((out.x, out.y), (104, 112));
            assert_eq!(out.blocked, frame > 2);
            assert_eq!((out.dx, out.dy), (0, 0));
        }
    }
}

#[test]
fn errors_are_atomic_including_input_history() {
    for raw in [16 << 9, 6 << 9, (12 << 9) | 0x8000] {
        let mut cells = vec![0; 2048];
        cells[6 * 32 + 7] = raw;
        let room = Room::new(32, 64, cells).unwrap();
        let mut state = WalkingState::new(104, 112);
        state.step(&room, input(Direction::Right)).unwrap();
        state.step(&room, input(Direction::Right)).unwrap();
        let before = state;
        assert!(state.step(&room, input(Direction::Left)).is_err());
        assert_eq!(state, before); // Failed attempt must not consume Left admission.
        assert!(state.step(&floor(), input(Direction::Left)).is_ok());
    }
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Direction::Left)).unwrap();
    state.step(&room, FrameInput::default()).unwrap();
    let before = state;
    assert_eq!(
        state.step(&room, input(Direction::Left)),
        Err(Unqualified::ReactivatedDirection(Direction::Left))
    );
    assert_eq!(state, before);
}

#[test]
fn mixed_pairs_and_old_special_edges_are_rejected() {
    let mut cells = vec![0; 2048];
    cells[7 * 32 + 7] = 14 << 9;
    let room = Room::new(32, 64, cells).unwrap();
    let mut state = WalkingState::new(104, 113);
    state.step(&room, input(Direction::Right)).unwrap();
    state.step(&room, input(Direction::Right)).unwrap();
    assert_eq!(
        state.step(&room, input(Direction::Right)),
        Err(Unqualified::MixedPair)
    );
    let mut cells = vec![0; 2048];
    cells[6 * 32 + 6] = 6 << 9;
    let room = Room::new(32, 64, cells).unwrap();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Direction::Right)).unwrap();
    state.step(&room, input(Direction::Right)).unwrap();
    assert_eq!(
        state.step(&room, input(Direction::Right)),
        Err(Unqualified::UnsupportedType(6))
    );
}

#[test]
fn dimensions_bounds_and_snapshot_roundtrip() {
    assert!(Room::new(0, 64, vec![]).is_err());
    assert!(Room::new(4096, 1, vec![]).is_err());
    assert!(Room::new(32, 64, vec![0]).is_err());
    let room = floor();
    let mut invalid = WalkingState::new(0, 0);
    let before = invalid;
    assert!(invalid.step(&room, FrameInput::default()).is_err());
    assert_eq!(invalid, before);
    let mut state = WalkingState::new(104, 112);
    for _ in 0..21 {
        state.step(&room, input(Direction::Down)).unwrap();
    }
    let encoded = state.encode_snapshot();
    let mut restored = WalkingState::decode_snapshot(&room, &encoded).unwrap();
    assert_eq!(state, restored);
    for _ in 0..90 {
        assert_eq!(
            state.step(&room, input(Direction::Left)),
            restored.step(&room, input(Direction::Left))
        );
        assert_eq!(state.encode_snapshot(), restored.encode_snapshot());
    }
    assert!(WalkingState::decode_snapshot(&room, &encoded[..encoded.len() - 1]).is_err());
    let mut bad = encoded;
    bad[0] = 255;
    assert!(WalkingState::decode_snapshot(&room, &bad).is_err());
}

#[test]
fn boundary_attempts_and_reversals_are_atomic() {
    let room = floor();
    for (x, y, d) in [
        (8, 112, Direction::Left),
        (504, 112, Direction::Right),
        (104, 16, Direction::Up),
        (104, 1024, Direction::Down),
    ] {
        let mut state = WalkingState::new(x, y);
        state.step(&room, input(d)).unwrap();
        state.step(&room, input(d)).unwrap();
        let before = state;
        assert!(state.step(&room, input(d)).is_err());
        assert_eq!(state, before);
    }
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Direction::Left)).unwrap();
    state.step(&room, input(Direction::Right)).unwrap();
    let before = state;
    assert_eq!(
        state.step(&room, input(Direction::Left)),
        Err(Unqualified::ReactivatedDirection(Direction::Left))
    );
    assert_eq!(state, before);
}

#[test]
fn edge_bit_three_controls_snap_or_rollback() {
    let room = Room::new(32, 64, vec![14 << 9; 2048]).unwrap();
    for (x, y, d, expected) in [
        (102, 112, Direction::Left, (104, 112)),
        (102, 112, Direction::Right, (102, 112)),
        (106, 112, Direction::Right, (104, 112)),
        (104, 110, Direction::Up, (104, 112)),
        (104, 110, Direction::Down, (104, 110)),
        (104, 114, Direction::Down, (104, 112)),
    ] {
        let mut state = WalkingState::new(x, y);
        state.step(&room, input(d)).unwrap();
        state.step(&room, input(d)).unwrap();
        let out = state.step(&room, input(d)).unwrap();
        assert!(out.blocked);
        assert_eq!(state.position(), expected);
    }
}

#[test]
fn snapshots_validate_encoding_phase_history_and_room_bounds() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Direction::Right)).unwrap();
    state.step(&room, input(Direction::Right)).unwrap();
    let bytes = state.encode_snapshot();
    assert_eq!(
        bytes,
        [b'R', b'W', b'K', 0, 1, 0, 104, 0, 112, 0, 4, 4, 0, 8, 0, 0]
    );
    for (offset, value) in [
        (4, 2),
        (5, 1),
        (10, 5),
        (11, 255),
        (12, 54),
        (13, 0),
        (13, 16),
        (14, 1),
        (15, 1),
    ] {
        let mut invalid = bytes;
        invalid[offset] = value;
        assert!(
            WalkingState::decode_snapshot(&room, &invalid).is_err(),
            "byte {offset}"
        );
    }
    let mut idle = WalkingState::new(104, 112).encode_snapshot();
    idle[12] = 1;
    assert!(WalkingState::decode_snapshot(&room, &idle).is_err());
    let mut vertical = WalkingState::new(104, 112);
    vertical.step(&room, input(Direction::Up)).unwrap();
    vertical.step(&room, input(Direction::Up)).unwrap();
    let mut invalid = vertical.encode_snapshot();
    invalid[12] = 3;
    assert!(WalkingState::decode_snapshot(&room, &invalid).is_err());
    for (x, y) in [(0, 112), (104, 0), (511, 112), (104, 1025)] {
        let bytes = WalkingState::new(x, y).encode_snapshot();
        assert_eq!(
            WalkingState::decode_snapshot(&room, &bytes),
            Err(Unqualified::PositionOutOfBounds)
        );
    }
    for length in 0..16 {
        assert!(WalkingState::decode_snapshot(&room, &bytes[..length]).is_err());
    }
}

#[test]
fn single_frame_tap_never_moves_and_snapshot_replay_is_repeatable() {
    let room = floor();
    let mut one = WalkingState::new(104, 112);
    let mut two = one;
    let inputs = [
        input(Direction::Left),
        FrameInput::default(),
        FrameInput::default(),
        input(Direction::Down),
        input(Direction::Down),
        input(Direction::Down),
    ];
    for (index, frame) in inputs.into_iter().enumerate() {
        let a = one.step(&room, frame).unwrap();
        let b = two.step(&room, frame).unwrap();
        assert_eq!(a, b);
        assert_eq!(one.encode_snapshot(), two.encode_snapshot());
        if index < 3 {
            assert_eq!((a.dx, a.dy), (0, 0));
        }
        two = WalkingState::decode_snapshot(&room, &two.encode_snapshot()).unwrap();
    }
}
