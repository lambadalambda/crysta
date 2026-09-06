use super::*;
use crate::conversation::{ConversationPages, ConversationSpec, DialogueWait, FIRST, REPEAT};

fn pages() -> ConversationPages {
    ConversationPages {
        first: 1,
        first_choice: 2,
        repeat_choice: 3,
        first_option1: [4, 5, 6],
        first_option2: [7, 8, 9],
        repeat_option1: [10, 11],
        repeat_option2: [12, 13],
    }
}
fn base_data() -> GameData {
    let rooms = crate::house::MAPS.map(|map_id| {
        let mut cells = vec![0; 2048];
        cells[19 * 32 + 8] = 0x1cf2;
        cells[20 * 32 + 8] = 0x1cf3;
        if map_id == 13 {
            cells[1415] = 0x8592;
        }
        HouseRoom {
            map_id,
            collision: Room::new_passive(32, 64, cells).unwrap(),
            exits: crate::house::exits(map_id).unwrap().to_vec(),
        }
    });
    let identity = DataIdentity {
        rom_sha256: [1; 32],
        content_sha256: [2; 32],
    };
    GameData::new_house(
        rooms,
        identity,
        NewGameData {
            bedroom: Room::new(32, 64, vec![0; 2048]).unwrap(),
            position: (304, 112),
        },
    )
    .unwrap()
}
pub(super) fn data() -> GameData {
    let identity = base_data().identity;
    base_data()
        .with_progression(
            ConversationSpec::new(pages()).unwrap(),
            Room::new_passive(64, 80, vec![0; 5120]).unwrap(),
            DataIdentity {
                content_sha256: [3; 32],
                ..identity
            },
        )
        .unwrap()
}
fn resident(data: &GameData) -> GameState {
    let mut state = GameState::new_game(data, Policy::SemanticPreview);
    state.map_id = 11;
    state.fresh_bedroom = false;
    state.wooden_door_open = true;
    state.walking = WalkingState::new(120, 128);
    state.animation = AnimationState::standing(Direction::Up);
    state
}
fn restored_step(data: &GameData, state: &mut GameState) {
    let mut copy = GameState::restore(data, &state.snapshot()).unwrap();
    let input = FrameInput {
        direction: Some(Direction::Right),
    };
    assert_eq!(state.step(data, input), copy.step(data, input));
    assert_eq!(*state, copy);
}
#[test]
fn first_grant_is_after_ack_before_choice_and_every_branch_repeats() {
    let data = data();
    for first_choice in 0..3 {
        for repeat_choice in 0..3 {
            let mut state = resident(&data);
            state.interact(&data).unwrap();
            assert_eq!(
                state.dialogue(&data).unwrap().unwrap().wait,
                DialogueWait::Page(1)
            );
            assert!(!state.event_flags().contains(0x26).unwrap());
            for _ in 0..3 {
                restored_step(&data, &mut state);
            }
            assert_eq!(state.output().position, (120, 128));
            assert!(!state.event_flags().contains(0x26).unwrap());
            assert_eq!(state.dialogue(&data).unwrap().unwrap().request, FIRST);
            let before = state.clone();
            assert_eq!(
                state.choose(&data, first_choice),
                Err(SliceError::Interaction)
            );
            assert_eq!(state, before);
            state.acknowledge(&data).unwrap();
            assert!(state.event_flags().contains(0x26).unwrap());
            assert_eq!(
                state.dialogue(&data).unwrap().unwrap().wait,
                DialogueWait::Choice { catalog: 0, key: 2 }
            );
            let before = state.clone();
            assert_eq!(state.acknowledge(&data), Err(SliceError::Interaction));
            assert_eq!(state.choose(&data, 3), Err(SliceError::Interaction));
            assert_eq!(state, before);
            restored_step(&data, &mut state);
            state.choose(&data, first_choice).unwrap();
            let keys = if first_choice == 1 {
                [4, 5, 6]
            } else {
                [7, 8, 9]
            };
            let source = if first_choice == 1 {
                0x88_90d9
            } else {
                0x88_905a
            };
            for key in keys {
                let output = state.dialogue(&data).unwrap().unwrap();
                assert_eq!(output.request, source);
                assert_eq!(output.wait, DialogueWait::Page(key));
                restored_step(&data, &mut state);
                state.acknowledge(&data).unwrap();
            }
            assert_eq!(state.dialogue(&data).unwrap(), None);
            assert_eq!(state.walking, WalkingState::new(120, 128));
            state.interact(&data).unwrap();
            assert_eq!(state.dialogue(&data).unwrap().unwrap().request, REPEAT);
            assert_eq!(
                state.dialogue(&data).unwrap().unwrap().wait,
                DialogueWait::Choice { catalog: 1, key: 3 }
            );
            state.choose(&data, repeat_choice).unwrap();
            let keys = if repeat_choice == 1 {
                [10, 11]
            } else {
                [12, 13]
            };
            for key in keys {
                assert_eq!(
                    state.dialogue(&data).unwrap().unwrap().wait,
                    DialogueWait::Page(key)
                );
                state.acknowledge(&data).unwrap();
            }
            assert_eq!(state.dialogue(&data).unwrap(), None);
            assert_eq!(
                state.event_flags(),
                crate::conversation::initial_flags(true)
            );
            assert_eq!(
                GameState::new_game(&data, Policy::SemanticPreview).event_flags(),
                crate::conversation::initial_flags(false)
            );
        }
    }
}
#[test]
fn dialogue_actions_and_target_fail_atomically_and_freeze_pose_history() {
    let data = data();
    for (position, facing) in [
        ((121, 128), Direction::Up),
        ((120, 129), Direction::Up),
        ((120, 128), Direction::Down),
    ] {
        let mut state = resident(&data);
        state.walking = WalkingState::new(position.0, position.1);
        state.animation = AnimationState::standing(facing);
        let before = state.clone();
        assert_eq!(state.interact(&data), Err(SliceError::Interaction));
        assert_eq!(state, before);
    }
    let mut state = resident(&data);
    state.tick = u64::MAX;
    let before = state.clone();
    assert_eq!(state.interact(&data), Err(SliceError::TickOverflow));
    assert_eq!(state, before);
    state.tick = 0;
    state.interact(&data).unwrap();
    assert_eq!(state.output().phase, Phase::Dialogue);
    assert_eq!(state.animation, AnimationState::standing(Direction::Up));
    let before = state.clone();
    assert_eq!(state.interact(&data), Err(SliceError::Interaction));
    assert_eq!(state, before);
    let mut wrong = self::data();
    wrong.identity.content_sha256[0] ^= 1;
    assert_eq!(state.acknowledge(&wrong), Err(SliceError::Data));
    assert_eq!(state, before);
}
#[test]
fn snapshot_rejects_erased_owner_effect_cursor_and_incoherent_flags() {
    let data = data();
    let mut state = resident(&data);
    state.interact(&data).unwrap();
    let bytes = state.snapshot();
    assert_eq!(GameState::restore(&data, &bytes).unwrap(), state);
    for (at, value) in [(109 + 4, 0x41), (178, 1), (174, 0), (180, 0)] {
        let mut corrupt = bytes.clone();
        corrupt[at] = value;
        assert!(GameState::restore(&data, &corrupt).is_err(), "{at}");
    }
    let mut erased = bytes.clone();
    erased[174..180].fill(0);
    assert!(GameState::restore(&data, &erased).is_err());
    state.acknowledge(&data).unwrap();
    let mut corrupt = state.snapshot();
    corrupt[113] = 1;
    assert!(GameState::restore(&data, &corrupt).is_err());
    let mut corrupt = state.snapshot();
    corrupt[174..178].copy_from_slice(&0x88_90d9_u32.to_le_bytes());
    corrupt[178..180].copy_from_slice(&2_u16.to_le_bytes());
    // A genuine different follow-up state cannot be distinguished from a fully
    // replaced valid history; invalid sequence identities/cursors must be rejected.
    corrupt[178..180].copy_from_slice(&99_u16.to_le_bytes());
    assert!(GameState::restore(&data, &corrupt).is_err());
}
#[test]
fn d_reload_gate_exterior_handoff_and_bounded_walking() {
    let data = data();
    let mut state = resident(&data);
    assert_eq!(data.room(13, false, false).unwrap().cells()[1415], 0x8592);
    state.interact(&data).unwrap();
    state.acknowledge(&data).unwrap();
    state.choose(&data, 1).unwrap();
    for _ in 0..3 {
        state.acknowledge(&data).unwrap();
    }
    // Reconstruct a source-qualified C->D transition, preserving story flags.
    state.map_id = 12;
    state.walking = WalkingState::new(120, 464);
    state.animation = AnimationState::standing(Direction::Down);
    state.transition = Some(Transition::select(12, 0, (120, 464)).unwrap());
    for _ in 0..35 {
        let restored = GameState::restore(&data, &state.snapshot()).unwrap();
        assert_eq!(restored, state);
        state.step(&data, FrameInput::default()).unwrap();
    }
    assert!(state.d_open_loaded);
    assert_eq!(state.current_room(&data).unwrap().cells()[1415], 0x592);
    state.walking = WalkingState::new(120, 719);
    for _ in 0..3 {
        state
            .step(
                &data,
                FrameInput {
                    direction: Some(Direction::Down),
                },
            )
            .unwrap();
    }
    assert_eq!(state.output().phase, Phase::Departing);
    for elapsed in 0..35 {
        assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
        if elapsed == 18 {
            assert_eq!(state.output().map_id, 10);
            assert_eq!(state.output().position, (504, 752));
        }
        state.step(&data, FrameInput::default()).unwrap();
    }
    assert_eq!(state.output().position, (504, 769));
    assert_eq!(state.output().map_id, 10);
    for (direction, n) in [
        (Some(Direction::Down), 32),
        (None, 60),
        (Some(Direction::Right), 24),
        (None, 90),
    ] {
        for _ in 0..n {
            let mut copy = GameState::restore(&data, &state.snapshot()).unwrap();
            let input = FrameInput { direction };
            assert_eq!(state.step(&data, input), copy.step(&data, input));
            assert_eq!(state, copy);
        }
    }
    assert!(state.output().position.0 > 504 && state.output().position.1 > 769);
    assert_eq!(state.output().map_id, 10);
}

#[test]
fn constructor_admits_only_fixed_source_shape_and_opaque_distinct_keys() {
    let mut duplicate = pages();
    duplicate.first = duplicate.repeat_choice;
    assert!(ConversationSpec::new(duplicate).is_err());
    let mut zero = pages();
    zero.first = 0;
    assert!(ConversationSpec::new(zero).is_ok());
    for dimensions in [(32, 64), (64, 64), (80, 64)] {
        let base = base_data();
        let identity = base.identity;
        let room = Room::new_passive(
            dimensions.0,
            dimensions.1,
            vec![0; usize::from(dimensions.0) * usize::from(dimensions.1)],
        )
        .unwrap();
        assert!(base
            .with_progression(ConversationSpec::new(pages()).unwrap(), room, identity)
            .is_err());
    }
    for raw in [0x8000, 2 << 9, 6 << 9, 7 << 9, 8 << 9, 25 << 9] {
        let base = base_data();
        let identity = base.identity;
        let mut cells = vec![0; 5120];
        cells[47 * 64 + 29] = raw;
        assert!(base
            .with_progression(
                ConversationSpec::new(pages()).unwrap(),
                Room::new_passive(64, 80, cells).unwrap(),
                identity
            )
            .is_err());
    }
    let mut base = base_data();
    base.rooms[2].collision.replace_cell(1415, 0x592);
    let identity = base.identity;
    assert!(base
        .with_progression(
            ConversationSpec::new(pages()).unwrap(),
            Room::new_passive(64, 80, vec![0; 5120]).unwrap(),
            identity
        )
        .is_err());
    let base = base_data();
    let mut identity = base.identity;
    identity.rom_sha256[0] ^= 1;
    assert!(base
        .with_progression(
            ConversationSpec::new(pages()).unwrap(),
            Room::new_passive(64, 80, vec![0; 5120]).unwrap(),
            identity
        )
        .is_err());
}

#[test]
fn every_flag_bit_and_nonwait_request_cursor_mutation_rejects_before_grant() {
    let data = data();
    let mut state = resident(&data);
    state.interact(&data).unwrap();
    let snapshot = state.snapshot();
    for bit in 0..512 {
        let mut corrupt = snapshot.clone();
        corrupt[109 + bit / 8] ^= 1 << (bit % 8);
        assert!(GameState::restore(&data, &corrupt).is_err(), "event {bit}");
    }
    for request in [
        0,
        REPEAT,
        0x88_90d9,
        0x88_905a,
        0x88_918c,
        0x88_91d6,
        u32::MAX,
    ] {
        let mut corrupt = snapshot.clone();
        corrupt[174..178].copy_from_slice(&request.to_le_bytes());
        assert!(
            GameState::restore(&data, &corrupt).is_err(),
            "request {request:x}"
        );
    }
    for cursor in [1_u16, 2, 3, 256, u16::MAX] {
        let mut corrupt = snapshot.clone();
        corrupt[178..180].copy_from_slice(&cursor.to_le_bytes());
        assert!(
            GameState::restore(&data, &corrupt).is_err(),
            "cursor {cursor}"
        );
    }
    state.acknowledge(&data).unwrap();
    let mut corrupt = state.snapshot();
    corrupt[178..180].fill(0);
    assert!(
        GameState::restore(&data, &corrupt).is_err(),
        "cannot rewind past flag write"
    );
    let mut wrong = self::data();
    wrong.identity.content_sha256[0] ^= 1;
    assert!(GameState::restore(&wrong, &state.snapshot()).is_err());
    let before = state.clone();
    assert_eq!(state.choose(&wrong, 1), Err(SliceError::Data));
    assert_eq!(state, before);
    state.tick = u64::MAX;
    let before = state.clone();
    assert_eq!(state.choose(&data, 1), Err(SliceError::TickOverflow));
    assert_eq!(state, before);
}

#[test]
fn loaded_d_never_subscribes_to_live_flag_and_false_grant_cannot_open_gate() {
    let data = data();
    let mut state = resident(&data);
    state.map_id = 13;
    state.walking = WalkingState::new(120, 625);
    state.animation = AnimationState::standing(Direction::Down);
    assert_eq!(state.current_room(&data).unwrap().cells()[1415], 0x8592);
    let mut corrupt = state.snapshot();
    corrupt[173] = 1;
    assert!(GameState::restore(&data, &corrupt).is_err());
    // Internal-only counterfactual matching native source lifetime. There is no
    // public flag setter or acceptance warp; normal B->D loads after the grant.
    state.flags = conversation::initial_story_flags(true);
    state.step(&data, FrameInput::default()).unwrap();
    assert_eq!(state.current_room(&data).unwrap().cells()[1415], 0x8592);
    assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
    assert!(!state.d_open_loaded);
    assert_eq!(data.room(13, false, false).unwrap().cells()[1415], 0x8592);
}

#[test]
fn exterior_north_return_and_halo_edges_fail_atomically_not_as_walls() {
    let data = data();
    let mut state = resident(&data);
    state.map_id = 10;
    state.flags = conversation::initial_story_flags(true);
    state.walking = WalkingState::new(504, 769);
    state.animation = AnimationState::standing(Direction::Down);
    for direction in [
        Direction::Up,
        Direction::Left,
        Direction::Right,
        Direction::Down,
    ] {
        let mut current = state.clone();
        let mut stopped = false;
        for _ in 0..150 {
            let before = current.clone();
            if let Err(error) = current.step(
                &data,
                FrameInput {
                    direction: Some(direction),
                },
            ) {
                assert_eq!(
                    error,
                    SliceError::Walking(Unqualified::SampleOutsideAdmission)
                );
                assert_eq!(current, before);
                stopped = true;
                break;
            }
            assert_eq!(current.output().map_id, 10);
        }
        assert!(stopped, "no invented town access: {direction:?}");
    }
    // The source A->D trigger is Ark (504,752), beyond the top sample halo.
    // No A exits are supplied to the bounded core profile.
    assert_eq!(data.exit(10, (504, 752)), None);
}

#[test]
fn retained_d_arrival_requires_the_just_selected_gate_profile() {
    let data = data();
    for granted in [false, true] {
        let mut state = resident(&data);
        state.map_id = 12;
        state.flags = conversation::initial_story_flags(granted);
        state.walking = WalkingState::new(120, 464);
        state.animation = AnimationState::standing(Direction::Down);
        state.transition = Some(Transition::select(12, 0, (120, 464)).unwrap());
        for elapsed in 0..35 {
            assert_eq!(GameState::restore(&data, &state.snapshot()).unwrap(), state);
            if elapsed >= 18 {
                assert_eq!(state.d_open_loaded, granted);
                let mut corrupt = state.snapshot();
                corrupt[173] = u8::from(!granted);
                assert!(
                    GameState::restore(&data, &corrupt).is_err(),
                    "grant={granted}, elapsed={elapsed}"
                );
            }
            state.step(&data, FrameInput::default()).unwrap();
        }
    }
}

#[test]
fn widened_b_preserves_unrelated_flags_and_grants_before_choice() {
    use crate::conversation::StoryConversationSpec;
    use crate::events::StoryFlags;

    let spec = StoryConversationSpec::new(pages()).unwrap();
    let mut bytes = *conversation::initial_story_flags(false).bytes();
    bytes[0] = 0x80; // Unrelated low flag is not B's restore policy.
    bytes[4] = 0; // B does not own bootstrap $20/$FB admission either.
    bytes[31] = 0;
    bytes[72] = 0x08; // $243
    bytes[82] = 0x04; // $292
    bytes[127] = 0x80;
    let mut flags = StoryFlags::new(bytes);
    let active = spec.begin(&mut flags);
    assert_eq!(active.request, FIRST);
    assert_eq!(spec.restore(FIRST, 0, &flags).unwrap(), active);
    assert!(!flags.contains(0x26).unwrap());
    assert!(spec.restore(FIRST, 2, &flags).is_err());
    assert!(spec.restore(REPEAT, 0, &flags).is_err());
    let active = spec.acknowledge(active, &mut flags).unwrap().unwrap();
    bytes[4] |= 0x40;
    assert_eq!(flags.bytes(), &bytes);
    assert_eq!(
        spec.output(active).unwrap().wait,
        DialogueWait::Choice { catalog: 0, key: 2 }
    );
    assert_eq!(spec.restore(FIRST, 2, &flags).unwrap(), active);
    assert!(spec.restore(FIRST, 0, &flags).is_err());
    assert!(spec.restore(FIRST, 1, &flags).is_err());
    for selection in 0..3 {
        let mut flags = flags.clone();
        let mut followup = Some(spec.choose(active, selection, &mut flags).unwrap());
        while let Some(active) = followup {
            assert_eq!(
                spec.restore(active.request, active.cursor.position(), &flags)
                    .unwrap(),
                active
            );
            followup = spec.acknowledge(active, &mut flags).unwrap();
        }
        let repeat = spec.begin(&mut flags);
        assert_eq!(repeat.request, REPEAT);
        assert_eq!(spec.restore(REPEAT, 0, &flags).unwrap(), repeat);
        let mut followup = Some(spec.choose(repeat, selection, &mut flags).unwrap());
        while let Some(active) = followup {
            assert_eq!(
                spec.restore(active.request, active.cursor.position(), &flags)
                    .unwrap(),
                active
            );
            followup = spec.acknowledge(active, &mut flags).unwrap();
        }
        assert_eq!(flags.bytes(), &bytes);
    }
}

#[test]
fn authoritative_story_storage_keeps_profile9_low_range_and_admission() {
    use crate::events::{EventFlags, StoryFlags};

    let data = data();
    let mut state = resident(&data);
    let mut flags = *state.story_flags().bytes();
    flags[72] = 0x18;
    flags[82] = 0x04;
    state.flags = StoryFlags::new(flags);
    let low: EventFlags = state.event_flags();
    assert_eq!(low, conversation::initial_flags(false));
    state.interact(&data).unwrap();
    state.acknowledge(&data).unwrap();
    assert_eq!(&state.story_flags().bytes()[64..], &flags[64..]);
    assert!(state.story_flags().contains(0x26).unwrap());
    assert!(!low.contains(0x26).unwrap()); // Owned inspection, not a live borrow.
    let snapshot = state.snapshot();
    assert_eq!(snapshot.len(), 181);
    assert_eq!(&snapshot[..8], b"RSLC\x01\x09\x00\x01");
    assert_eq!(
        &snapshot[109..173],
        conversation::initial_flags(true).bytes()
    );
    let restored = GameState::restore(&data, &snapshot).unwrap();
    assert_eq!(&restored.story_flags().bytes()[64..], &[0; 64]);
    for byte in 109..173 {
        let mut corrupt = snapshot.clone();
        corrupt[byte] ^= 0x80;
        assert!(GameState::restore(&data, &corrupt).is_err(), "{byte}");
    }
}
