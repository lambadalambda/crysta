//! Whole-state animation ownership, snapshot coherence, and semantic doorway replay.
use room_core::slice::{
    DataIdentity, Exit, FrameOutput, GameData, GameState, NewGameData, Phase, Policy, SliceError,
    PROFILE_VERSION,
};
use room_core::{
    AnimationFrame, AnimationSet, AnimationState, Direction, FrameInput, Room, WalkingState,
    SNAPSHOT_SIZE,
};
use Direction::{Down, Left, Right, Up};

fn data(blocked: bool, extra_exit: Option<Exit>) -> GameData {
    let open = || Room::new(32, 64, vec![0; 2048]).unwrap();
    let mut exits = vec![Exit([24, 12, 1, 2, 16, 0, 0, 5, 128, 1, 80, 1])];
    exits.extend(extra_exit);
    let mut hall = vec![0; 2048];
    hall[19 * 32 + 24] = 14 << 9;
    GameData::new(
        [open(), Room::new(32, 64, hall).unwrap()],
        [
            exits,
            vec![Exit([24, 20, 1, 1, 15, 0, 0, 6, 128, 1, 176, 0])],
        ],
        DataIdentity {
            rom_sha256: [1; 32],
            content_sha256: [2; 32],
        },
        NewGameData {
            bedroom: if blocked {
                Room::new(32, 64, vec![14 << 9; 2048]).unwrap()
            } else {
                open()
            },
            position: (304, 112),
        },
    )
    .unwrap()
}
fn step(state: &mut GameState, data: &GameData, direction: Option<Direction>) -> FrameOutput {
    state.step(data, FrameInput { direction }).unwrap()
}
fn standing(direction: Direction) -> AnimationFrame {
    AnimationState::standing(direction).frame()
}
fn walk(direction: Direction, record: u8) -> AnimationFrame {
    AnimationFrame {
        set: AnimationSet::Walking,
        record,
        ..standing(direction)
    }
}

#[test]
fn output_owns_delayed_setup_turn_and_release() {
    let data = data(false, None);
    let mut state = GameState::new_game(&data, Policy::SemanticPreview);
    assert_eq!(state.output().animation, standing(Down));
    assert_eq!(
        GameState::new(&data, Policy::SemanticPreview)
            .output()
            .animation,
        standing(Down)
    );
    assert_eq!(
        step(&mut state, &data, Some(Right)).animation,
        standing(Down)
    );
    for age in 0..20 {
        assert_eq!(
            step(&mut state, &data, Some(Right)).animation,
            walk(Right, age / 9)
        );
    }
    assert_eq!(
        step(&mut state, &data, Some(Left)).animation,
        walk(Right, 2)
    );
    assert_eq!(step(&mut state, &data, Some(Left)).animation, walk(Left, 0));
    assert_eq!(step(&mut state, &data, None).animation, walk(Left, 0));
    assert_eq!(step(&mut state, &data, None).animation, standing(Left));
    for _ in 0..600 {
        assert_eq!(step(&mut state, &data, None).animation, standing(Left));
    }
}

#[test]
fn blocked_hold_keeps_six_record_clock_and_snapshot_phase() {
    let data = data(true, None);
    for direction in [Down, Up, Left, Right] {
        let mut state = GameState::new_game(&data, Policy::SemanticPreview);
        step(&mut state, &data, Some(direction));
        for age in 0..120 {
            let mut restored = GameState::restore(&data, &state.snapshot()).unwrap();
            let out = step(&mut state, &data, Some(direction));
            assert_eq!(out, step(&mut restored, &data, Some(direction)));
            assert_eq!(state, restored);
            assert_eq!(out.position, (304, 112));
            assert_eq!(out.animation, walk(direction, (age % 54) / 9));
            assert_eq!(&state.snapshot()[100..], &[direction as u8, 1, age % 54]);
        }
    }
}

#[test]
fn snapshot_profile_append_and_coherence_are_fail_closed() {
    let data = data(false, None);
    let mut state = GameState::new_game(&data, Policy::SemanticPreview);
    let initial = state.snapshot();
    assert_eq!(PROFILE_VERSION, 7);
    assert_eq!(SNAPSHOT_SIZE, 16);
    assert_eq!(initial.len(), 103);
    assert_eq!(initial[5], 7);
    assert_eq!(
        &initial[83..99],
        &WalkingState::new(304, 112).encode_snapshot()
    );
    assert_eq!(initial[99], 1);
    assert_eq!(&initial[100..], &[0, 0, 0]);
    for facing in 0..4 {
        let mut bytes = initial.clone();
        bytes[100] = facing;
        assert!(
            GameState::restore(&data, &bytes).is_ok(),
            "idle facing is free"
        );
    }
    for (at, value) in [(5, 6), (100, 4), (101, 2), (101, 1), (102, 1), (102, 54)] {
        let mut bytes = initial.clone();
        bytes[at] = value;
        assert!(
            GameState::restore(&data, &bytes).is_err(),
            "field {at}={value}"
        );
    }
    assert!(GameState::restore(&data, &initial[..100]).is_err());
    for _ in 0..30 {
        step(&mut state, &data, Some(Right));
    }
    let active = state.snapshot();
    for (at, value) in [(100, 0), (101, 0), (102, 54), (102, 0)] {
        let mut bytes = active.clone();
        bytes[at] = value;
        assert!(
            GameState::restore(&data, &bytes).is_err(),
            "active field {at}={value}"
        );
    }
    let mut falsely_standing = active.clone();
    falsely_standing[101..103].copy_from_slice(&[0, 0]);
    assert!(GameState::restore(&data, &falsely_standing).is_err());
    // Turning input may differ from active facing until the next successful tick.
    step(&mut state, &data, Some(Up));
    assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
    step(&mut state, &data, Some(Up));
    let mut wrong_setup = state.snapshot();
    assert_eq!(wrong_setup[102], 0);
    wrong_setup[102] = 2; // Same parity, but setup must still be animation phase zero.
    assert!(GameState::restore(&data, &wrong_setup).is_err());
    step(&mut state, &data, Some(Up));
    let mut wrong_parity = state.snapshot();
    assert_eq!(wrong_parity[102], 1);
    wrong_parity[102] = 2;
    assert!(GameState::restore(&data, &wrong_parity).is_err());
}

#[test]
fn early_and_late_errors_preserve_animation_and_entire_snapshot() {
    let data = data(false, None);
    let mut state = GameState::new_game(&data, Policy::SemanticPreview);
    for direction in [Some(Left), None] {
        step(&mut state, &data, direction);
    }
    let before = state.clone();
    assert!(state
        .step(
            &data,
            FrameInput {
                direction: Some(Left)
            }
        )
        .is_err());
    assert_eq!(state, before);
    assert_eq!(state.snapshot(), before.snapshot());
    // Extra unsupported exit catches the *successful* walking setup tick. Its
    // prospective animation changes Down standing -> Right walking before error.
    let bad_data = self::data(false, Some(Exit([18, 6, 2, 1, 99, 0, 0, 0, 0, 0, 0, 0])));
    let mut state = GameState::new_game(&data, Policy::SemanticPreview);
    step(&mut state, &data, Some(Right));
    let before = state.clone();
    assert_eq!(
        state.step(
            &bad_data,
            FrameInput {
                direction: Some(Right)
            }
        ),
        Err(SliceError::Exit)
    );
    assert_eq!(state, before);
    assert_eq!(state.snapshot(), before.snapshot());
}

#[test]
fn doorway_pose_is_directional_standing_from_handoff_through_completion() {
    let data = data(false, None);
    let mut state = GameState::new(&data, Policy::SemanticPreview);
    for direction in (0..56).map(|_| Left).chain((0..24).map(|_| Down)) {
        step(&mut state, &data, Some(direction));
    }
    for (direction, expected_map, expected_position) in
        [(Down, 16, (392, 353)), (Up, 15, (392, 191))]
    {
        assert_eq!(state.output().phase, Phase::Departing);
        let mut ticks = 0;
        loop {
            assert_eq!(state.output().animation, standing(direction));
            let bytes = state.snapshot();
            assert_eq!(&bytes[100..], &[direction as u8, 0, 0]);
            let mut restored = GameState::restore(&data, &bytes).unwrap();
            if state.output().phase == Phase::Walking {
                break;
            }
            for (at, value) in [(100, Right as u8), (101, 1), (102, 1)] {
                let mut bad = bytes.clone();
                bad[at] = value;
                assert!(GameState::restore(&data, &bad).is_err());
            }
            let out = step(&mut state, &data, Some(Left));
            assert_eq!(out, step(&mut restored, &data, None));
            assert_eq!(state, restored);
            ticks += 1;
            assert!(ticks <= 35);
        }
        assert_eq!(ticks, 35);
        assert_eq!(
            (state.output().map_id, state.output().position),
            (expected_map, expected_position)
        );
        assert_eq!(step(&mut state, &data, None).animation, standing(direction));
        if direction == Down {
            for _ in 0..14 {
                step(&mut state, &data, Some(Up));
            }
        }
    }
}
