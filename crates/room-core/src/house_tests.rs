use super::*;

fn data() -> GameData {
    let profiles = crate::house::MAPS.map(|map_id| {
        let mut cells = vec![0; 2048];
        cells[19 * 32 + 8] = 0x1cf2;
        cells[20 * 32 + 8] = 0x1cf3;
        HouseRoom {
            map_id,
            collision: Room::new_passive(32, 64, cells).unwrap(),
            exits: crate::house::exits(map_id).unwrap().to_vec(),
        }
    });
    GameData::new_house(
        profiles,
        DataIdentity {
            rom_sha256: [1; 32],
            content_sha256: [2; 32],
        },
        NewGameData {
            bedroom: Room::new(32, 64, vec![0; 2048]).unwrap(),
            position: (304, 112),
        },
    )
    .unwrap()
}

fn at_door(data: &GameData) -> GameState {
    let mut state = GameState::new_game(data, Policy::SemanticPreview);
    state.map_id = 12;
    state.fresh_bedroom = false;
    state.walking = WalkingState::new(136, 352);
    state.animation = AnimationState::standing(Direction::Up);
    state
}

#[test]
fn action_is_one_atomic_tick_with_final_collision_and_persistent_sheet_state() {
    let data = data();
    let mut state = at_door(&data);
    state
        .step(
            &data,
            FrameInput {
                direction: Some(Direction::Up),
            },
        )
        .unwrap();
    // Successful action consumes delayed Up; the host need not drain it.
    let tick = state.output().tick;
    state.interact(&data).unwrap();
    assert_eq!(state.output().tick, tick + 1);
    assert_eq!(state.walking, WalkingState::new(136, 352));
    assert!(state.wooden_door_open());
    assert_eq!(
        data.room(12, false, true).unwrap().cells()[20 * 32 + 8],
        0xf7
    );
    assert_eq!(
        data.room(12, false, true).unwrap().cells()[19 * 32 + 8],
        0x1cf6
    );
    assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
    let before = state.clone();
    assert_eq!(state.interact(&data), Err(SliceError::Interaction));
    assert_eq!(state, before);
}

#[test]
fn unsupported_interactions_and_corrupt_patch_snapshots_fail_closed() {
    let data = data();
    for (map, x, y, facing) in [
        (15, 136, 352, Direction::Up),
        (12, 135, 352, Direction::Up),
        (12, 136, 351, Direction::Up),
        (12, 136, 352, Direction::Down),
    ] {
        let mut state = at_door(&data);
        state.map_id = map;
        state.walking = WalkingState::new(x, y);
        state.animation = AnimationState::standing(facing);
        let before = state.clone();
        assert_eq!(state.interact(&data), Err(SliceError::Interaction));
        assert_eq!(state, before);
    }
    let state = at_door(&data);
    let mut bytes = state.snapshot();
    *bytes.last_mut().unwrap() = 2;
    assert!(GameState::restore(&data, &bytes).is_err());
}

#[test]
fn every_internal_source_exit_has_atomic_transition_and_restore() {
    let data = data();
    for spec in crate::house::DOORWAYS {
        let mut state = at_door(&data);
        state.map_id = spec.source;
        state.wooden_door_open = true;
        state.walking = WalkingState::new(spec.handoff.0, spec.handoff.1);
        state.transition = Some(Transition::select(spec.source, spec.index, spec.handoff).unwrap());
        state.animation = AnimationState::standing(spec.direction);
        for _ in 0..35 {
            let restored = GameState::restore(&data, &state.snapshot()).unwrap();
            assert_eq!(state, restored);
            state.step(&data, FrameInput::default()).unwrap();
        }
        assert_eq!(state.output().map_id, spec.destination);
        assert_eq!(state.output().position, spec.endpoint);
        assert_eq!(state.animation.facing(), spec.direction);
    }
}

#[test]
fn b_return_cannot_erase_the_sheet_mutation_during_any_owned_tick() {
    let data = data();
    let mut state = at_door(&data);
    state.wooden_door_open = true;
    state.map_id = 11;
    state.walking = WalkingState::new(120, 208);
    state.transition = Some(Transition::select(11, 0, (120, 208)).unwrap());
    state.animation = AnimationState::standing(Direction::Down);
    for _ in 0..35 {
        assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
        let mut corrupt = state.snapshot();
        corrupt[108] = 0;
        assert!(GameState::restore(&data, &corrupt).is_err());
        state.step(&data, FrameInput::default()).unwrap();
    }
}

#[test]
fn walking_selects_every_internal_exit_and_rejects_wrong_direction_atomically() {
    let data = data();
    for spec in crate::house::DOORWAYS {
        let (x, y) = spec.handoff;
        let start = match spec.direction {
            Direction::Down => (x, y - 1),
            Direction::Up => (x, y + 1),
            Direction::Left => (x + 1, y),
            Direction::Right => (x - 1, y),
        };
        let mut state = at_door(&data);
        state.map_id = spec.source;
        state.wooden_door_open = true;
        state.walking = WalkingState::new(start.0, start.1);
        for _ in 0..3 {
            state
                .step(
                    &data,
                    FrameInput {
                        direction: Some(spec.direction),
                    },
                )
                .unwrap();
        }
        assert_eq!(state.output().phase, Phase::Departing);
        assert_eq!(state.output().position, spec.handoff);
        state.transition = None;
        state.walking = WalkingState::new(x, y);
        let before = state.clone();
        assert_eq!(
            state.step(&data, FrameInput::default()),
            Err(SliceError::Exit)
        );
        assert_eq!(state, before);
    }
}

#[test]
fn source_boundaries_are_not_silent_walls_or_free_exits() {
    let data = data();
    for (map, index) in [(12, 3), (13, 0), (15, 1)] {
        let exit = crate::house::exits(map).unwrap()[index];
        let mut state = at_door(&data);
        state.map_id = map;
        state.walking = WalkingState::new(
            u16::from(exit.0[0]) * 16 + 8,
            u16::from(exit.0[1]) * 16 + 16,
        );
        let before = state.clone();
        assert_eq!(
            state.step(&data, FrameInput::default()),
            Err(SliceError::Exit)
        );
        assert_eq!(state, before);
    }
}

#[test]
fn door_errors_do_not_change_tick_history_or_patch_and_snapshots_bind_identity() {
    let data = data();
    let mut state = at_door(&data);
    state.tick = u64::MAX;
    let before = state.clone();
    assert_eq!(state.interact(&data), Err(SliceError::TickOverflow));
    assert_eq!(state, before);
    let mut other = self::data();
    other.identity.content_sha256[0] ^= 1;
    assert_eq!(state.interact(&other), Err(SliceError::Data));
    assert_eq!(state, before);
    assert!(GameState::restore(&other, &state.snapshot()).is_err());
}
