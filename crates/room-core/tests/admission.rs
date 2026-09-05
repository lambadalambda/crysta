//! Exact bounded admission, atomicity, and walking snapshot v3 contracts.
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState, SNAPSHOT_VERSION};

fn floor() -> Room {
    Room::new(32, 64, vec![0; 2048]).unwrap()
}

fn input(direction: Option<Direction>) -> FrameInput {
    FrameInput { direction }
}

#[test]
fn onset_gap_eleven_is_ordinary_for_every_cardinal() {
    let room = floor();
    for direction in [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ] {
        for gap in 2..=12 {
            let mut state = WalkingState::new(256, 256);
            state.step(&room, input(Some(direction))).unwrap();
            for _ in 1..gap {
                state.step(&room, input(None)).unwrap();
            }
            let before = state;
            let result = state.step(&room, input(Some(direction)));
            if gap < 11 {
                assert_eq!(result, Err(Unqualified::AcceleratedTrigger(direction)));
                assert_eq!(state, before);
            } else {
                result.unwrap();
                assert_eq!(state.onset_remaining(), 11);
            }
        }
    }
}

#[test]
fn reversals_replace_history_and_release_does_not_restart_window() {
    let room = floor();
    let mut state = WalkingState::new(256, 256);
    for _ in 0..20 {
        for direction in [Direction::Left, Direction::Right] {
            state.step(&room, input(Some(direction))).unwrap();
            assert_eq!(state.last_activation_direction(), Some(direction));
            assert_eq!(state.onset_remaining(), 11);
        }
    }
    for _ in 0..19 {
        state.step(&room, input(Some(Direction::Right))).unwrap();
    }
    assert_eq!(state.onset_remaining(), 0);
    state.step(&room, input(None)).unwrap();
    state.step(&room, input(Some(Direction::Right))).unwrap();
    for _ in 0..100 {
        state.step(&room, input(None)).unwrap();
    }
    assert_eq!(state.onset_remaining(), 0);
    assert_eq!(state.last_activation_direction(), Some(Direction::Right));
    state.step(&room, input(Some(Direction::Right))).unwrap();
}

#[test]
fn errors_roll_back_history_timer_cadence_and_delayed_input() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Some(Direction::Left))).unwrap();
    state.step(&room, input(None)).unwrap();
    let before = state.encode_snapshot();
    assert_eq!(
        state.step(&room, input(Some(Direction::Left))),
        Err(Unqualified::AcceleratedTrigger(Direction::Left))
    );
    assert_eq!(state.encode_snapshot(), before);
    // Rejected submissions do not secretly consume cooldown ticks.
    assert!(state.step(&room, input(Some(Direction::Left))).is_err());
    assert_eq!(state.encode_snapshot(), before);
    let mut restored = WalkingState::decode_snapshot(&room, &before).unwrap();
    assert_eq!(
        state.step(&room, input(Some(Direction::Right))),
        restored.step(&room, input(Some(Direction::Right)))
    );

    // A collision error must also undo a newly admitted direction and countdown.
    let mut cells = vec![0; 2048];
    cells[6 * 32 + 5] = 17 << 9;
    let unsupported = Room::new(32, 64, cells).unwrap();
    let mut state = WalkingState::new(104, 112);
    for _ in 0..2 {
        state.step(&room, input(Some(Direction::Left))).unwrap();
    }
    let before = state;
    assert_eq!(
        state.step(&unsupported, input(Some(Direction::Right))),
        Err(Unqualified::UnsupportedType(17))
    );
    assert_eq!(state, before);
}

#[test]
fn snapshot_v3_preserves_history_on_both_sides_of_boundary() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Some(Direction::Right))).unwrap();
    state.step(&room, input(Some(Direction::Right))).unwrap();
    assert_eq!(SNAPSHOT_VERSION, 3);
    assert_eq!(
        state.encode_snapshot(),
        [b'R', b'W', b'K', 0, 3, 0, 104, 0, 112, 0, 4, 4, 0, 4, 10, 0]
    );
    for _ in 0..12 {
        let bytes = state.encode_snapshot();
        let mut restored = WalkingState::decode_snapshot(&room, &bytes).unwrap();
        let mut uninterrupted = state;
        assert_eq!(
            restored.step(&room, input(Some(Direction::Right))),
            uninterrupted.step(&room, input(Some(Direction::Right)))
        );
        assert_eq!(restored, uninterrupted);
        state.step(&room, input(None)).unwrap();
    }
    assert_eq!(state.last_activation_direction(), Some(Direction::Right));
    assert_eq!(state.onset_remaining(), 0);
}

#[test]
fn snapshot_rejects_invalid_encodings_and_history_relationships() {
    let room = floor();
    let mut state = WalkingState::new(104, 112);
    state.step(&room, input(Some(Direction::Right))).unwrap();
    state.step(&room, input(Some(Direction::Right))).unwrap();
    let bytes = state.encode_snapshot();
    for (offset, value) in [
        (0, 0),
        (4, 1),
        (4, 2),
        (4, 4),
        (5, 1),
        (10, 5),
        (11, 5),
        (12, 54),
        (13, 0),
        (13, 3),
        (13, 5),
        (14, 11),
        (14, 12),
        (14, 255),
        (15, 1),
    ] {
        let mut invalid = bytes;
        invalid[offset] = value;
        assert_eq!(
            WalkingState::decode_snapshot(&room, &invalid),
            Err(Unqualified::Snapshot),
            "offset {offset} value {value}"
        );
    }
    let mut onset = WalkingState::new(104, 112);
    onset.step(&room, input(Some(Direction::Up))).unwrap();
    let mut invalid = onset.encode_snapshot();
    invalid[14] = 10; // New input unlike active direction requires a freshly armed window.
    assert_eq!(
        WalkingState::decode_snapshot(&room, &invalid),
        Err(Unqualified::Snapshot)
    );
    onset.step(&room, input(None)).unwrap();
    let mut invalid = onset.encode_snapshot();
    invalid[13] = 3; // Residual active direction must match last activation on release.
    assert_eq!(
        WalkingState::decode_snapshot(&room, &invalid),
        Err(Unqualified::Snapshot)
    );
    onset.step(&room, input(None)).unwrap();
    let mut invalid = onset.encode_snapshot();
    invalid[14] = 10; // Fully idle has had at least two ticks since activation.
    assert_eq!(
        WalkingState::decode_snapshot(&room, &invalid),
        Err(Unqualified::Snapshot)
    );
    for (offset, value) in [(12, 1), (14, 1)] {
        let mut invalid = WalkingState::new(104, 112).encode_snapshot();
        invalid[offset] = value;
        assert_eq!(
            WalkingState::decode_snapshot(&room, &invalid),
            Err(Unqualified::Snapshot)
        );
    }
    for length in 0..16 {
        assert_eq!(
            WalkingState::decode_snapshot(&room, &bytes[..length]),
            Err(Unqualified::Snapshot)
        );
    }
    assert_eq!(
        WalkingState::decode_snapshot(&room, &[0; 17]),
        Err(Unqualified::Snapshot)
    );
    assert_eq!(
        WalkingState::decode_snapshot(&room, &WalkingState::new(0, 0).encode_snapshot()),
        Err(Unqualified::PositionOutOfBounds)
    );
}
