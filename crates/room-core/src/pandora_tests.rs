//! Synthetic aggregate tests; no production pose/geometry initializer is supplied.
use super::*;
use crate::conversation::DialogueWait;
use crate::pots::SourceObject;
fn text() -> PandoraText {
    let mut requests = Vec::new();
    for i in Invocation::ALL
        .into_iter()
        .chain([Invocation::ResidentRetry, Invocation::ResidentRefusal])
    {
        if requests
            .iter()
            .any(|r: &RequestPages| r.source == i.source())
        {
            continue;
        }
        let (count, choice) = Invocation::request_shape(i.source()).unwrap();
        requests.push(RequestPages {
            source: i.source(),
            pages: (0..count).map(|n| i.source() * 16 + u32::from(n)).collect(),
            choice,
        });
    }
    PandoraText::new(requests).unwrap()
}
fn anchor(position: (u16, u16), facing: Direction) -> Anchor {
    Anchor { position, facing }
}
#[allow(clippy::too_many_lines)] // Keep the explicitly synthetic compiler fixture together.
fn data() -> GameData {
    let rooms = CollisionKey::ALL
        .into_iter()
        .map(|key| {
            let (width, height) = key.dimensions();
            let mut cells = vec![0; usize::from(width) * usize::from(height)];
            if key.map() == 0xc {
                cells[19 * 32 + 8] = 0x1cf2;
                cells[20 * 32 + 8] = 0x1cf3;
                cells[21 * 32 + 11] = 0x0b81;
                cells[20 * 32 + 11] = 0x1d80;
                match key {
                    CollisionKey::CDamaged => cells[20 * 32 + 11] = 0x1da7,
                    CollisionKey::CReaction => {
                        cells[20 * 32 + 11] = 0x9cf6;
                        cells[21 * 32 + 11] = 0xbacb;
                    }
                    CollisionKey::COpen => {
                        cells[20 * 32 + 11] = 0x1cf6;
                        cells[21 * 32 + 11] = 0x3acb;
                    }
                    _ => {}
                }
                cells[21 * 32 + 3] = 0x18fa;
                cells[21 * 32 + 4] = 0x18fb;
            }
            ProfileRoom {
                key,
                room: Room::new_passive(width, height, cells)
                    .unwrap()
                    .with_sample_halo([0, 0, width, height])
                    .unwrap(),
            }
        })
        .collect();
    let mut motions = Vec::new();
    for i in Invocation::ALL {
        if matches!(
            i,
            Invocation::ResidentFirst
                | Invocation::ResidentGrant
                | Invocation::CChoice
                | Invocation::CDirect
                | Invocation::BoxEntry
                | Invocation::TourFinal
        ) {
            continue;
        }
        let key = MotionKey::Cue(Cue::Returned(i));
        let (_, map, reload) = key.maps();
        let pose = match i {
            Invocation::FirstHit
            | Invocation::SecondHit
            | Invocation::ReactionSpeaker
            | Invocation::ReactionRequest
            | Invocation::ReactionRight
            | Invocation::ReactionLeft
            | Invocation::ReactionFinal => (184, 368),
            Invocation::BoxWarning => (136, 359),
            _ => (136, 208),
        };
        let scene = match map {
            0xc => ScenePhase::CEntry,
            0x21 => ScenePhase::BoxOpening,
            0x41 => ScenePhase::Tour410,
            0x44 => ScenePhase::Tour44,
            0x42 => ScenePhase::Tour42,
            0x43 => ScenePhase::Tour43,
            _ => unreachable!(),
        };
        motions.push(MotionSpec {
            key,
            trigger: None,
            frames: vec![MotionFrame {
                map_id: map,
                pose: MotionPose::Absolute(anchor(
                    pose,
                    if map == 0xc {
                        Direction::Up
                    } else {
                        Direction::Down
                    },
                )),
                reload,
                scene,
            }],
        });
    }
    for cue in [
        Cue::BoxReload,
        Cue::BoxAcquireControl,
        Cue::SecondHitPatched,
        Cue::ReactionColorMath,
        Cue::ReactionColorReturn,
    ] {
        let key = MotionKey::Cue(cue);
        let (_, map, reload) = key.maps();
        let pose = if map == 0x21 { (136, 368) } else { (184, 368) };
        motions.push(MotionSpec {
            key,
            trigger: None,
            frames: vec![MotionFrame {
                map_id: map,
                pose: MotionPose::Absolute(anchor(pose, Direction::Up)),
                reload,
                scene: if map == 0x21 {
                    ScenePhase::BoxOpening
                } else {
                    ScenePhase::CColorMath
                },
            }],
        });
    }
    let contacts = [
        ContactSpec {
            kind: ContactKind::Resident,
            trigger: anchor((360, 144), Direction::Up),
            result: anchor((360, 144), Direction::Up),
        },
        ContactSpec {
            kind: ContactKind::BoxWarning,
            trigger: anchor((136, 370), Direction::Down),
            result: anchor((136, 359), Direction::Down),
        },
    ];
    let objects = vec![
        SourceObject {
            cell: 21 * 32 + 3,
            raw: 0x18fa,
            replacement: 0x00f8,
        },
        SourceObject {
            cell: 21 * 32 + 4,
            raw: 0x18fb,
            replacement: 0x00f8,
        },
    ];
    for motion in &mut motions {
        let mut last = motion.frames[0];
        last.reload = false;
        motion.frames.push(last);
    }
    let spec = PandoraData::new(
        text(),
        rooms,
        motions,
        contacts,
        BoxOpeningGate {
            raw_bounds: [120, 368, 152, 400],
        },
        objects,
        true,
    )
    .unwrap();
    let base = progression_tests::data();
    let identity = DataIdentity {
        content_sha256: [9; 32],
        ..base.identity
    };
    base.with_pandora(spec, identity).unwrap()
}
fn flag(state: &mut GameState, bit: u16) {
    let mut bytes = *state.flags.bytes();
    bytes[usize::from(bit / 8)] |= 1 << (bit % 8);
    state.flags = StoryFlags::new(bytes);
}
fn at(
    data: &GameData,
    map: u16,
    position: (u16, u16),
    facing: Direction,
    bits: &[u16],
) -> GameState {
    let mut s = GameState::new_game(data, Policy::SemanticPreview);
    s.fresh_bedroom = false;
    s.wooden_door_open = true;
    for &b in bits {
        flag(&mut s, b);
    }
    s.map_id = map;
    s.walking = WalkingState::new(position.0, position.1);
    s.animation = AnimationState::standing(facing);
    s.pandora_load().unwrap();
    s.ensure_pot(data.pandora.as_ref().unwrap()).unwrap();
    s
}
fn replay(
    s: &mut GameState,
    d: &GameData,
    action: fn(&mut GameState, &GameData) -> Result<FrameOutput, SliceError>,
) {
    let mut restored = GameState::restore(d, &s.snapshot()).unwrap();
    assert_eq!(action(s, d), action(&mut restored, d));
    assert_eq!(*s, restored);
    assert_eq!(GameState::restore(d, &s.snapshot()).unwrap(), *s);
    if s.pandora.is_some_and(|p| p.motion.is_some()) {
        let mut erased = s.snapshot();
        erased[249..252].copy_from_slice(&[255, 0, 0]);
        assert!(GameState::restore(d, &erased).is_err());
    }
}
fn neutral(s: &mut GameState, d: &GameData) -> Result<FrameOutput, SliceError> {
    s.step(d, FrameInput::default())
}
fn ack(s: &mut GameState, d: &GameData) {
    replay(s, d, GameState::acknowledge);
}
fn drain(s: &mut GameState, d: &GameData) {
    loop {
        match s.dialogue(d).unwrap().map(|x| x.wait) {
            Some(DialogueWait::Page(_)) => ack(s, d),
            None if s.pandora.unwrap().owns() => replay(s, d, neutral),
            Some(DialogueWait::Choice { .. }) | None => break,
        }
    }
}
#[test]
fn same_new_game_opt_in_identity_and_old_profile_remain_distinct() {
    let d = data();
    let old = progression_tests::data();
    let mut s = GameState::new_game(&d, Policy::SemanticPreview);
    assert_eq!(s.output().position, (304, 112));
    assert_eq!(s.output().tick, 0);
    assert_eq!(&s.snapshot()[..8], b"RSLC\x05\x0d\x00\x01");
    assert_eq!(
        &GameState::new_game(&old, Policy::SemanticPreview).snapshot()[..8],
        b"RSLC\x01\x09\x00\x01"
    );
    assert!(GameState::restore(&old, &s.snapshot()).is_err());
    for _ in 0..5 {
        replay(&mut s, &d, neutral);
    }
}
#[test]
fn aggregate_resident_c_choice_and_unsupported_errors_restore_per_action() {
    let d = data();
    let mut s = at(&d, 0x13, (360, 144), Direction::Up, &[0x26]);
    replay(&mut s, &d, GameState::interact);
    ack(&mut s, &d);
    assert_dialogue_ready_readonly(&s, &d, true); // Visible graph choice, no motion.
    s.choose(&d, 0).unwrap();
    drain(&mut s, &d);
    assert!(s.flags.contains(1).unwrap());
    replay(&mut s, &d, GameState::interact);
    s.choose(&d, 1).unwrap();
    for _ in 0..3 {
        ack(&mut s, &d);
        assert!(!s.flags.contains(0x28).unwrap());
    }
    ack(&mut s, &d);
    assert!(s.flags.contains(0x28).unwrap());
    s.map_id = 0xc;
    s.walking = WalkingState::new(136, 352);
    s.animation = AnimationState::standing(Direction::Down);
    s.pandora_load().unwrap();
    assert!(s.flags.contains(0x27).unwrap());
    drain(&mut s, &d);
    let before = s.snapshot();
    for choice in [0, 2, 3] {
        assert!(s.choose(&d, choice).is_err());
        assert_eq!(s.snapshot(), before);
    }
    s.choose(&d, 1).unwrap();
    ack(&mut s, &d);
    assert!(!s.flags.contains(0x2e).unwrap());
    ack(&mut s, &d);
    assert!(s.flags.contains(0x2e).unwrap());
    assert!(s.pot_state().is_some());
}
fn frames(s: &mut GameState, d: &GameData, direction: Option<Direction>, count: usize) {
    for _ in 0..count {
        let mut copy = GameState::restore(d, &s.snapshot()).unwrap();
        let input = FrameInput { direction };
        let a = s.step(d, input);
        let b = copy.step(d, input);
        assert_eq!(a, b);
        a.unwrap();
        assert_eq!(*s, copy);
        assert_eq!(GameState::restore(d, &s.snapshot()).unwrap(), *s);
    }
}
#[test]
fn real_pot_hit_preserves_launch_grid_and_ledger_through_story_and_recovery() {
    real_pot_hit_with_pose(false);
}
#[test]
fn preserve_cues_follow_authoritative_pot_pose_through_recovery() {
    real_pot_hit_with_pose(true);
}
fn real_pot_hit_with_pose(preserve: bool) {
    let mut d = data();
    if preserve {
        for motion in &mut d.pandora.as_mut().unwrap().motions {
            if motion.key.maps() == (0xc, 0xc, false) {
                for frame in &mut motion.frames {
                    frame.pose = MotionPose::Preserve { facing: None };
                }
            }
        }
    }
    let mut s = at(
        &d,
        0xc,
        (40, 352),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    assert_eq!(s.pot_slots(&d).unwrap().0, Some(0x98a));
    frames(&mut s, &d, Some(Direction::Down), 12);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Right), 97);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Up), 12);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.output().position, (184, 368));
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 1);
    assert_eq!(s.pandora.unwrap().frozen, Some(CollisionKey::CClosed));
    assert_eq!(
        s.pot_state().unwrap().flight(),
        Some(crate::pots::Flight { x: 184, y: 354 })
    );
    assert_eq!(s.current_room(&d).unwrap().cells()[20 * 32 + 11], 0x1da7);
    assert!(!s.flags.contains(0x292).unwrap());
    // Readiness follows the dialogue guard, not the overlapping PotRecovery owner.
    assert_eq!(
        s.pandora_output(&d).unwrap().owner,
        ControlOwner::PotRecovery
    );
    assert!(s.dialogue(&d).unwrap().is_some());
    assert_dialogue_ready_readonly(&s, &d, true);
    // Acknowledgement during recovery may finish text, not the action or ledger.
    ack(&mut s, &d);
    frames(&mut s, &d, None, 13);
    assert!(s.pandora.unwrap().frozen.is_none());
    assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
    drain(&mut s, &d);
    assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
    frames(&mut s, &d, Some(Direction::Left), 65);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Up), 12);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Left), 4);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.output().position, (88, 352));
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    assert_eq!(s.pot_slots(&d).unwrap().0, Some(0x98f));
    frames(&mut s, &d, Some(Direction::Down), 12);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Right), 65);
    frames(&mut s, &d, None, 20);
    frames(&mut s, &d, Some(Direction::Up), 12);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.output().position, (184, 368));
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 2);
    assert!(!s.flags.contains(0x292).unwrap());
    frames(&mut s, &d, None, 2);
    assert!(s.flags.contains(0x292).unwrap());
    assert_eq!(s.pot_state().unwrap().phase(), crate::pots::Phase::Throwing);
    assert_eq!(s.pandora.unwrap().frozen, Some(CollisionKey::CDamaged));
    assert_eq!(s.current_room(&d).unwrap().cells()[21 * 32 + 11], 0xbacb);
    frames(&mut s, &d, None, 11);
    drain(&mut s, &d);
    assert_eq!(s.consumed_pots(&d).unwrap().len(), 2);
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 2);
    assert_eq!(s.current_room(&d).unwrap().cells()[21 * 32 + 11], 0x3acb);
    for ledger in [0_u64, 1, 2] {
        let mut bad = s.snapshot();
        bad[272..280].copy_from_slice(&ledger.to_le_bytes());
        assert!(GameState::restore(&d, &bad).is_err());
    }
    let before = s.snapshot();
    assert!(s.pot_action(&d).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn box_contact_not_interact_and_forced_aggregate_finishes_in_controllable41() {
    let d = data();
    let mut s = at(
        &d,
        0x21,
        (136, 352),
        Direction::Down,
        &[0x26, 0x28, 0x27, 0x2e, 0x292],
    );
    ack(&mut s, &d);
    let before = s.snapshot();
    assert!(s.interact(&d).is_err());
    assert_eq!(s.snapshot(), before);
    while !s.pandora.unwrap().owns() {
        frames(&mut s, &d, Some(Direction::Down), 1);
    }
    assert_eq!(s.output().position, (136, 359));
    assert_eq!(
        s.dialogue(&d).unwrap().unwrap().request,
        Invocation::BoxWarning.source()
    );
    ack(&mut s, &d);
    frames(&mut s, &d, Some(Direction::Down), 10);
    assert!(!s.flags.contains(2).unwrap());
    ack(&mut s, &d);
    assert!(!s.flags.contains(2).unwrap());
    drain(&mut s, &d);
    assert!(s.flags.contains(2).unwrap());
    frames(&mut s, &d, None, 15);
    assert!(!s.flags.contains(0x22).unwrap());
    while !s.pandora.unwrap().owns() {
        frames(&mut s, &d, Some(Direction::Down), 1);
    }
    assert!(!s.flags.contains(0x22).unwrap());
    replay(&mut s, &d, neutral);
    assert!(!s.flags.contains(0x22).unwrap());
    replay(&mut s, &d, neutral);
    assert!(s.flags.contains(0x22).unwrap());
    assert_eq!(s.pandora_output(&d).unwrap().scene, ScenePhase::BoxContact);
    assert_eq!(
        s.pandora_output(&d).unwrap().owner,
        ControlOwner::Presentation
    );
    replay(&mut s, &d, neutral);
    assert_eq!(&s.flags.bytes()[..4], &[0; 4]);
    assert!(s.pandora.unwrap().owns());
    assert!(core::ptr::eq(
        s.current_room(&d).unwrap(),
        d.pandora.as_ref().unwrap().room(CollisionKey::BoxOpened)
    ));
    let mut erased = s.snapshot();
    erased[249..252].copy_from_slice(&[255, 0, 0]);
    assert!(GameState::restore(&d, &erased).is_err());
    drain(&mut s, &d);
    assert_eq!(s.output().map_id, 0x41);
    assert!(s.flags.contains(0x243).unwrap());
    assert!(s.flags.contains(0x244).unwrap());
    assert_eq!(s.pandora_output(&d).unwrap().scene, ScenePhase::TourControl);
    assert_eq!(s.pandora_output(&d).unwrap().owner, ControlOwner::Player);
    frames(&mut s, &d, Some(Direction::Left), 12);
    frames(&mut s, &d, None, 20);
    assert_eq!(s.output().position, (120, 208));
}
#[test]
fn canonical_aggregate_rejects_erased_ownership_flags_motion_and_identity() {
    let d = data();
    let mut s = at(&d, 0xc, (136, 352), Direction::Down, &[0x26, 0x28]);
    drain(&mut s, &d);
    let valid = s.snapshot();
    assert_eq!(valid.len(), 320);
    for (offset, value) in [
        (4, 1),
        (5, 9),
        (8, 88),
        (40, 88),
        (245, 0),
        (247, 4),
        (249, 0),
        (250, 1),
        (252, 80),
        (292, 0),
        (293, 1),
    ] {
        let mut bad = valid.clone();
        bad[offset] = value;
        assert!(GameState::restore(&d, &bad).is_err(), "offset {offset}");
    }
    for len in [0, 8, 180, 181, 245, 299, 300, 319, 321] {
        let mut bad = valid.clone();
        bad.resize(len, 0);
        assert!(GameState::restore(&d, &bad).is_err());
    }
    let mut bad = valid;
    bad[109] |= 1;
    assert!(GameState::restore(&d, &bad).is_err());
}
#[test]
fn qualified_travel_restores_departure_reload_arrival_and_discards_input() {
    let d = navigation_data();
    for (door, travel, endpoint, destination) in [
        (TownDoor::North, Travel::TownToResident, (392, 207), 0x13),
        (TownDoor::Home, Travel::TownToHouse, (120, 719), 0xd),
    ] {
        let spec = d
            .pandora
            .as_ref()
            .unwrap()
            .navigation
            .as_ref()
            .unwrap()
            .door(door);
        let mut s = at(&d, 0xa, spec.interaction.position, Direction::Up, &[0x26]);
        replay(&mut s, &d, GameState::interact);
        // Isolate the exact post-collision exit witness, not a native itinerary.
        let trigger = d
            .pandora
            .as_ref()
            .unwrap()
            .motion(MotionKey::Travel(travel))
            .unwrap()
            .1
            .trigger
            .unwrap();
        s.walking = WalkingState::new(trigger.position.0, trigger.position.1 + 3);
        frames(&mut s, &d, Some(Direction::Up), 4);
        assert_eq!(
            s.pandora_output(&d).unwrap().motion,
            Some((MotionKey::Travel(travel), 0))
        );
        for elapsed in 1..=35 {
            replay(&mut s, &d, |s, d| {
                s.step(
                    d,
                    FrameInput {
                        direction: Some(Direction::Left),
                    },
                )
            });
            assert_eq!(
                s.pandora_output(&d).unwrap().town_open,
                if elapsed <= 17 { door.mask() } else { 0 }
            );
        }
        assert_eq!(s.output().position, endpoint);
        assert_eq!(s.output().map_id, destination);
        assert_eq!(s.walking.last_activation_direction(), None);
    }
}
#[test]
fn a_synthetic_held_miss_boundary_consumes_a_pot_but_never_counts_an_attempt_as_hit() {
    let d = data();
    let mut s = at(
        &d,
        0xc,
        (40, 352),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    // Seed an admitted held launch for this isolated boundary test. This is NOT a
    // native carry-route witness or an exposed production position initializer.
    let mut bytes = s.pot_state().unwrap().encode_snapshot();
    let walking = WalkingState::new(136, 368);
    bytes[4..20].copy_from_slice(&walking.encode_snapshot());
    bytes[28] = Direction::Up as u8;
    let spec = d.pandora.as_ref().unwrap();
    let admission = crate::pots::Admission {
        room: spec.room(CollisionKey::CClosed),
        objects: &spec.objects,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    };
    s.pandora.as_mut().unwrap().pot =
        Some(crate::pots::PotState::decode_snapshot(&admission, &bytes).unwrap());
    s.walking = walking;
    s.animation = AnimationState::standing(Direction::Up);
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 33);
    assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 0);
    assert!(!s.flags.contains(0x292).unwrap());
    assert_eq!(s.dialogue(&d).unwrap(), None);
}
#[test]
fn missing_qualified_pacing_never_falls_back_or_partially_advances() {
    let mut d = data();
    d.pandora
        .as_mut()
        .unwrap()
        .motions
        .retain(|m| m.key != MotionKey::Cue(Cue::Returned(Invocation::CEntry)));
    let mut s = at(&d, 0xc, (136, 352), Direction::Down, &[0x26, 0x28]);
    ack(&mut s, &d);
    ack(&mut s, &d);
    let before = s.snapshot();
    assert_eq!(neutral(&mut s, &d), Err(SliceError::Exit));
    assert_eq!(s.snapshot(), before);
    assert_eq!(GameState::restore(&d, &before).unwrap(), s);
}
#[test]
fn malformed_immutable_geometry_and_catalogs_are_rejected() {
    fn bad(mutate: fn(&mut PandoraData)) {
        let mut p = data().pandora.take().unwrap();
        mutate(&mut p);
        assert!(PandoraData::new(
            p.text,
            p.rooms,
            p.motions,
            p.contacts,
            p.opening_gate,
            p.objects,
            p.cellar_up_lanes
        )
        .is_err());
    }
    bad(|p| p.rooms.swap(0, 1));
    bad(|p| p.rooms[0].room = Room::new(64, 80, vec![0; 5120]).unwrap());
    bad(|p| {
        p.rooms[0].room = Room::new(32, 64, vec![0; 2048])
            .unwrap()
            .with_sample_halo([0, 0, 32, 64])
            .unwrap();
    });
    bad(|p| p.objects.push(p.objects[0]));
    bad(|p| {
        p.rooms[CollisionKey::COpen as usize]
            .room
            .replace_cell(21 * 32 + 11, 0);
    });
    bad(|p| p.contacts[1].trigger.facing = Direction::Up);
    bad(|p| p.opening_gate.raw_bounds = [120, 368, 256, 400]);
    bad(|p| p.motions[0].frames.clear());
    bad(|p| p.motions[0].frames[0].reload = true);
    bad(|p| {
        let m = p
            .motions
            .iter_mut()
            .find(|m| m.key == MotionKey::Cue(Cue::BoxReload))
            .unwrap();
        m.frames[0].pose = MotionPose::Preserve { facing: None };
    });
    bad(|p| p.motions[0].frames[0].map_id = 0x41);
    bad(|p| p.motions[0].frames[0].scene = ScenePhase::Tour410);
    bad(|p| p.motions[0].frames[0].pose = MotionPose::Absolute(anchor((512, 352), Direction::Up)));
    bad(|p| p.motions[0].key = MotionKey::Cue(Cue::Returned(Invocation::CChoice)));
    bad(|p| {
        p.motions.push(MotionSpec {
            key: MotionKey::Travel(Travel::TownToResident),
            trigger: Some(anchor((136, 208), Direction::Down)),
            frames: vec![
                MotionFrame {
                    map_id: 0xa,
                    pose: MotionPose::Preserve { facing: None },
                    reload: false,
                    scene: ScenePhase::TownSource,
                },
                MotionFrame {
                    map_id: 0x13,
                    pose: MotionPose::Absolute(anchor((360, 144), Direction::Up)),
                    reload: true,
                    scene: ScenePhase::Resident13,
                },
            ],
        });
    });
}
#[test]
fn independent_source_dimensions_and_ambiguous_exits_are_checked() {
    assert_eq!(
        CollisionKey::ALL.map(CollisionKey::dimensions),
        [
            (64, 80),
            (64, 32),
            (32, 64),
            (32, 64),
            (32, 64),
            (32, 64),
            (32, 64),
            (32, 64),
            (16, 32),
            (16, 32),
            (32, 32),
            (32, 32),
            (32, 32),
            (32, 32)
        ]
    );
    let mut p = data().pandora.take().unwrap();
    for (travel, map, scene) in [
        (Travel::TownToResident, 0x13, ScenePhase::Resident13),
        (Travel::TownToHouse, 0xd, ScenePhase::House),
    ] {
        p.motions.push(MotionSpec {
            key: MotionKey::Travel(travel),
            trigger: Some(anchor((136, 208), Direction::Down)),
            frames: vec![MotionFrame {
                map_id: map,
                pose: MotionPose::Absolute(anchor((136, 208), Direction::Down)),
                reload: true,
                scene,
            }],
        });
    }
    assert!(PandoraData::new(
        p.text,
        p.rooms,
        p.motions,
        p.contacts,
        p.opening_gate,
        p.objects,
        p.cellar_up_lanes
    )
    .is_err());
}
#[test]
fn terminal_motion_commits_next_scene_without_inventing_a_trailing_frame() {
    let d = data();
    let mut s = at(&d, 0xc, (136, 352), Direction::Down, &[0x26, 0x28]);
    ack(&mut s, &d);
    ack(&mut s, &d);
    replay(&mut s, &d, neutral);
    assert_eq!(s.pandora_output(&d).unwrap().scene, ScenePhase::CEntry);
    assert!(s.pandora_output(&d).unwrap().motion.is_some());
    replay(&mut s, &d, neutral);
    assert_eq!(s.pandora_output(&d).unwrap().scene, ScenePhase::CChoice);
    assert_eq!(
        s.pandora_output(&d).unwrap().invocation,
        Some(Invocation::CApproach)
    );
    assert!(s.pandora_output(&d).unwrap().motion.is_none());
}
#[test]
fn opening_polls_both_locals_and_any_facing_but_waits_for_script_handoff() {
    let d = data();
    for facing in [
        Direction::Down,
        Direction::Up,
        Direction::Left,
        Direction::Right,
    ] {
        for direction in [None, Some(Direction::Down), Some(Direction::Up)] {
            let mut s = at(
                &d,
                0x21,
                (136, 368),
                facing,
                &[0x26, 0x28, 0x27, 0x2e, 0x292],
            );
            ack(&mut s, &d);
            flag(&mut s, 1);
            flag(&mut s, 2);
            frames(&mut s, &d, direction, 1);
            assert_eq!(
                s.pandora_output(&d).unwrap().cue,
                Some(Cue::BoxAcquireControl)
            );
            assert!(!s.flags.contains(0x22).unwrap());
            replay(&mut s, &d, neutral); // qualified busy sample, not COPDF success
            assert!(!s.flags.contains(0x22).unwrap());
            replay(&mut s, &d, neutral); // qualified completion: COPDF succeeded
            assert!(s.flags.contains(0x22).unwrap());
            assert_eq!(s.pandora_output(&d).unwrap().cue, Some(Cue::BoxReload));
        }
    }
}
#[test]
fn opening_raw_gate_is_inclusive_and_outside_recoil_never_grants() {
    let d = data();
    for (position, inside) in [
        ((120, 368), true),
        ((152, 400), true),
        ((136, 368), true),
        ((119, 368), false),
        ((153, 368), false),
        ((136, 367), false),
        ((136, 401), false),
        ((136, 359), false),
    ] {
        let mut s = at(
            &d,
            0x21,
            position,
            Direction::Left,
            &[0x26, 0x28, 0x27, 0x2e, 0x292],
        );
        ack(&mut s, &d);
        flag(&mut s, 1);
        flag(&mut s, 2);
        frames(&mut s, &d, None, 1);
        assert_eq!(
            s.pandora_output(&d).unwrap().cue,
            inside.then_some(Cue::BoxAcquireControl)
        );
        assert!(!s.flags.contains(0x22).unwrap());
        if !inside {
            frames(&mut s, &d, None, 20);
            assert!(!s.flags.contains(0x22).unwrap());
        }
    }
}
#[test]
fn missing_copdf_readiness_does_not_grant_or_reload() {
    let mut d = data();
    d.pandora
        .as_mut()
        .unwrap()
        .motions
        .retain(|m| m.key != MotionKey::Cue(Cue::BoxAcquireControl));
    let mut s = at(
        &d,
        0x21,
        (136, 368),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e, 0x292],
    );
    ack(&mut s, &d);
    flag(&mut s, 1);
    flag(&mut s, 2);
    replay(&mut s, &d, neutral);
    let before = s.snapshot();
    assert_eq!(neutral(&mut s, &d), Err(SliceError::Exit));
    assert_eq!(s.snapshot(), before);
    assert!(!s.flags.contains(0x22).unwrap());
    assert_eq!(s.output().map_id, 0x21);
    let mut forged = before;
    forged[109 + 0x22 / 8] |= 1 << (0x22 % 8);
    assert!(GameState::restore(&d, &forged).is_err());
}

// Synthetic load boundaries isolate cache lifetime, not a native itinerary.
fn load_at(s: &mut GameState, d: &GameData, map: u16, position: (u16, u16)) {
    s.map_id = map;
    s.walking = WalkingState::new(position.0, position.1);
    s.animation = AnimationState::standing(Direction::Up);
    s.pandora_load().unwrap();
    s.ensure_pot(d.pandora.as_ref().unwrap()).unwrap();
}

#[test]
fn sheet_survives_shared_loads_but_replacement_reconstructs_closed_wood() {
    let d = data();
    let mut s = at(
        &d,
        0xc,
        (40, 352),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    // A consumed empty boundary is seeded transparently; ordinary pot snapshots
    // remain the public component format, not an aggregate initializer.
    let mut pot = s.pot_state().unwrap().encode_snapshot();
    pot[29..32].fill(0);
    let spec = d.pandora.as_ref().unwrap();
    let admission = crate::pots::Admission {
        room: spec.room(CollisionKey::CClosed),
        objects: &spec.objects,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    };
    s.pandora.as_mut().unwrap().pot =
        Some(crate::pots::PotState::decode_snapshot(&admission, &pot).unwrap());
    for map in [0xd, 0xc, 0xb, 0xc] {
        load_at(&mut s, &d, map, (136, 352));
        let out = s.pandora_output(&d).unwrap();
        assert!(out.sheet.resident);
        assert_eq!(out.sheet.consumed, 1);
        assert_eq!(out.door_counter, 0);
        assert_eq!(out.locals, 0);
        assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
        assert_eq!(s.effective_room(&d).unwrap().cells()[21 * 32 + 3], 0xf8);
        assert_eq!(
            s.effective_room(&d).unwrap().cells()[19 * 32 + 8] & 0x7fff,
            0x1cf6
        );
        assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    }
    load_at(&mut s, &d, 0xa, (136, 208));
    assert!(!s.pandora_output(&d).unwrap().sheet.resident);
    assert!(s.consumed_pots(&d).unwrap().is_empty());
    assert!(!s.wooden_door_open());
    load_at(&mut s, &d, 0xd, (136, 600));
    assert!(!s.wooden_door_open());
    assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    load_at(&mut s, &d, 0xc, (136, 352));
    assert!(s.pot_state().is_some());
    // New sheet, then a consumed/empty boundary before reopening the rebuilt door.
    // This deliberately synthetic seam checks that Interact does not replace the ledger.
    s.pandora.as_mut().unwrap().pot =
        Some(crate::pots::PotState::with_ledger(&admission, s.walking, Direction::Up, 1).unwrap());
    replay(&mut s, &d, GameState::interact);
    assert!(s.wooden_door_open());
    assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
    assert_eq!(s.effective_room(&d).unwrap().cells()[19 * 32 + 8], 0x1cf6);
    assert_eq!(s.current_room(&d).unwrap().cells()[19 * 32 + 8], 0x1cf2);
}

// Qualified held-launch seam, not proof of the intervening native carry route.
fn held_launch(s: &mut GameState, d: &GameData) {
    let mut bytes = s.pot_state().unwrap().encode_snapshot();
    let walking = WalkingState::new(184, 368);
    bytes[4..20].copy_from_slice(&walking.encode_snapshot());
    bytes[28] = Direction::Up as u8;
    let spec = d.pandora.as_ref().unwrap();
    let admission = crate::pots::Admission {
        room: s.current_room(d).unwrap(),
        objects: &spec.objects,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    };
    s.pandora.as_mut().unwrap().pot =
        Some(crate::pots::PotState::decode_snapshot(&admission, &bytes).unwrap());
    s.walking = walking;
    s.animation = AnimationState::standing(Direction::Up);
}

#[test]
#[allow(clippy::too_many_lines)] // Keep the cross-visit action/cache regression in order.
fn retained_damage_new_visit_throw_and_five_cell_shared_patch_lifetime() {
    let mut d = data();
    let spec = d.pandora.as_mut().unwrap();
    spec.objects.push(SourceObject {
        cell: 21 * 32 + 5,
        raw: 0x18fa,
        replacement: 0x00f8,
    });
    for profile in &mut spec.rooms {
        if profile.key.map() == 0xc {
            profile.room.replace_cell(21 * 32 + 5, 0x18fa);
        }
    }
    // Distinct source scene rosters: the departed C stamp is not in E/20.
    for key in [
        CollisionKey::COpen,
        CollisionKey::CellarE,
        CollisionKey::Cellar20,
    ] {
        d.pandora.as_mut().unwrap().rooms[key as usize]
            .room
            .replace_cell(
                31 * 32 + 7,
                if key == CollisionKey::COpen {
                    0x9ce8
                } else {
                    0x1ce8
                },
            );
    }
    let mut s = at(
        &d,
        0xc,
        (40, 352),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    held_launch(&mut s, &d);
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 33);
    drain(&mut s, &d);
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 1);
    for map in [0xd, 0xc] {
        load_at(&mut s, &d, map, (88, 352));
        assert_eq!(s.pandora_output(&d).unwrap().door_counter, 0);
        assert_eq!(s.pandora_output(&d).unwrap().locals, 0);
        assert_eq!(
            s.pandora_output(&d).unwrap().sheet.cellar,
            CellarDoorPatch::Damaged
        );
        assert_eq!(s.effective_room(&d).unwrap().cells()[20 * 32 + 11], 0x1da7);
        assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
        assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    }
    s.animation = AnimationState::standing(Direction::Left);
    s.pandora
        .as_mut()
        .unwrap()
        .pot
        .as_mut()
        .unwrap()
        .rebase(s.walking, Direction::Left)
        .unwrap();
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    held_launch(&mut s, &d);
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 1);
    assert_eq!(s.pandora.unwrap().frozen, Some(CollisionKey::CDamaged));
    assert_eq!(s.pandora_output(&d).unwrap().door_counter, 0);
    for (offset, value) in [
        (300, 0),
        (301, 0),
        (302, 0),
        (303, 1),
        (304, 1),
        (312, 3),
        (292, CollisionKey::CClosed as u8),
    ] {
        let mut bad = s.snapshot();
        bad[offset] = value;
        assert!(GameState::restore(&d, &bad).is_err(), "offset {offset}");
    }
    frames(&mut s, &d, None, 32);
    drain(&mut s, &d);
    // Same-map reload still clears locals/counter, but not the cracked sheet.
    let mut same = s.clone();
    load_at(&mut same, &d, 0xc, (184, 368));
    assert_eq!(same.pandora_output(&d).unwrap().door_counter, 0);
    assert_eq!(same.pandora_output(&d).unwrap().locals, 0);
    assert_eq!(
        same.pandora_output(&d).unwrap().sheet.cellar,
        CellarDoorPatch::Damaged
    );
    assert_eq!(GameState::restore(&d, &same.snapshot()).unwrap(), same);

    s.walking = WalkingState::new(104, 352);
    s.animation = AnimationState::standing(Direction::Left);
    s.pandora
        .as_mut()
        .unwrap()
        .pot
        .as_mut()
        .unwrap()
        .rebase(s.walking, Direction::Left)
        .unwrap();
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 24);
    held_launch(&mut s, &d);
    replay(&mut s, &d, GameState::pot_action);
    frames(&mut s, &d, None, 33);
    drain(&mut s, &d);
    assert_eq!(s.effective_room(&d).unwrap().cells()[31 * 32 + 7], 0x9ce8);
    let opened = s.clone();
    for map in [0xb, 0xc, 0xd, 0xe, 0x20] {
        load_at(&mut s, &d, map, (136, 352));
        let out = s.pandora_output(&d).unwrap();
        assert_eq!(out.sheet.cellar, CellarDoorPatch::Open);
        assert_eq!(out.sheet.consumed, 7);
        assert_eq!(out.door_counter, 0);
        assert_eq!(out.locals, 0);
        let room = s.effective_room(&d).unwrap();
        for (cell, word) in [
            (20 * 32 + 11, 0x1cf6),
            (21 * 32 + 11, 0x3acb),
            (21 * 32 + 3, 0xf8),
            (21 * 32 + 4, 0xf8),
            (21 * 32 + 5, 0xf8),
        ] {
            assert_eq!(room.cells()[cell], word);
        }
        if matches!(map, 0xe | 0x20) {
            assert_eq!(room.cells()[31 * 32 + 7], 0x1ce8);
        }
        assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    }
    for map in [0xa, 0x13, 0x21] {
        let mut replaced = opened.clone();
        load_at(&mut replaced, &d, map, (136, 208));
        assert!(!replaced.wooden_door_open());
        assert_eq!(
            replaced.pandora_output(&d).unwrap().sheet,
            SharedSheetOutput {
                resident: false,
                cellar: CellarDoorPatch::Closed,
                consumed: 0
            }
        );
        assert_eq!(
            GameState::restore(&d, &replaced.snapshot()).unwrap(),
            replaced
        );
        if map == 0xa {
            let mut returned = replaced.clone();
            let nav_data = navigation_data();
            let spec = nav_data.pandora.unwrap();
            let id = u8::try_from(d.pandora.as_ref().unwrap().motions.len()).unwrap();
            let motion = spec
                .motions
                .into_iter()
                .find(|m| m.key == MotionKey::Travel(Travel::TownToHouse))
                .unwrap();
            d.pandora.as_mut().unwrap().motions.push(motion);
            d.pandora.as_mut().unwrap().navigation = spec.navigation;
            returned.walking = WalkingState::new(504, 768);
            returned.animation = AnimationState::standing(Direction::Up);
            replay(&mut returned, &d, GameState::interact);
            returned.walking = WalkingState::new(504, 752);
            returned.pandora.as_mut().unwrap().motion = Some((id, 0));
            for _ in 0..35 {
                replay(&mut returned, &d, neutral);
            }
            assert!(returned.flags.contains(0x292).unwrap());
            assert_eq!(
                returned.pandora_output(&d).unwrap().sheet.cellar,
                CellarDoorPatch::Closed
            );
            assert_eq!(
                GameState::restore(&d, &returned.snapshot()).unwrap(),
                returned
            );
            returned.walking = WalkingState::new(120, 608);
            returned.transition = Some(Transition::select(0xd, 1, (120, 608)).unwrap());
            for _ in 0..17 {
                replay(&mut returned, &d, neutral);
            }
            let before = returned.snapshot();
            assert_eq!(neutral(&mut returned, &d), Err(SliceError::Exit));
            assert_eq!(returned.snapshot(), before);
        }
        replaced.map_id = 0xc;
        assert_eq!(replaced.pandora_load(), Err(SliceError::Exit));
    }
}

#[test]
fn persistent_words_keep_current_scene_occupancy_and_original_admission_catalog() {
    let mut d = data();
    let source_cell = 21 * 32 + 3;
    // An E-only actor occupies a persistent pot cell. Never copy a previous
    // room's bit15 and never clear this scene's bit15 when replacing the tile.
    d.pandora.as_mut().unwrap().rooms[CollisionKey::CellarE as usize]
        .room
        .replace_cell(source_cell, 0x98fa);
    let mut s = at(
        &d,
        0xc,
        (136, 352),
        Direction::Up,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    let spec = d.pandora.as_ref().unwrap();
    let admission = crate::pots::Admission {
        room: spec.room(CollisionKey::CClosed),
        objects: &spec.objects,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    };
    s.pandora.as_mut().unwrap().pot =
        Some(crate::pots::PotState::with_ledger(&admission, s.walking, Direction::Up, 3).unwrap());
    // Synthetic settled opened boundary; no native journey claim.
    flag(&mut s, 0x292);
    let state = s.pandora.as_mut().unwrap();
    state.sheet.cellar = CellarDoorPatch::Open;
    state.visit_cellar = CellarDoorPatch::Open;
    state.visit_consumed = 3;
    load_at(&mut s, &d, 0xe, (136, 864));
    assert_eq!(s.effective_room(&d).unwrap().cells()[source_cell], 0x80f8);
    assert_eq!(s.current_room(&d).unwrap().cells()[source_cell], 0x98fa);
    assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    load_at(&mut s, &d, 0xc, (136, 352));
    // Reentry admission still sees raw FA/FB, not the consumed effective room.
    assert!(s.pot_state().is_some());
    assert_eq!(s.effective_room(&d).unwrap().cells()[source_cell], 0xf8);
    assert_eq!(s.current_room(&d).unwrap().cells()[source_cell], 0x18fa);
    frames(&mut s, &d, None, 2);
}

fn settled_first_hit(d: &GameData) -> GameState {
    let mut s = at(
        d,
        0xc,
        (40, 352),
        Direction::Right,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    replay(&mut s, d, GameState::pot_action);
    frames(&mut s, d, None, 24);
    held_launch(&mut s, d);
    replay(&mut s, d, GameState::pot_action);
    frames(&mut s, d, None, 33);
    drain(&mut s, d);
    s
}

#[test]
fn snapshot_cannot_resurrect_an_already_hit_pot() {
    let d = data();
    let s = settled_first_hit(&d);
    let mut bad = s.snapshot();
    bad[281] = 2; // Held
    bad[283] = 1; // The sole object already spent on the first hit.
    assert!(GameState::restore(&d, &bad).is_err());
}

#[test]
fn snapshot_baseline_damage_requires_prior_consumption() {
    let d = data();
    let s = settled_first_hit(&d);
    let mut bad = s.snapshot();
    bad[302] = 1; // Retained damage, but baseline ledger is empty.
    assert!(GameState::restore(&d, &bad).is_err());
}

#[test]
fn snapshot_arrival_cannot_retain_departing_visit_action() {
    let d = data();
    let s = settled_first_hit(&d);
    let mut bad = s.snapshot();
    bad[82] = 6; // D -> C, arrival already reconstructed C.
    bad[103] = 34;
    bad[104..106].copy_from_slice(&120_u16.to_le_bytes());
    bad[106..108].copy_from_slice(&608_u16.to_le_bytes());
    assert!(GameState::restore(&d, &bad).is_err());
}

#[test]
fn retained_damage_house_reload_replays_every_owned_tick() {
    let d = data();
    let mut s = settled_first_hit(&d);
    // Seed only the admitted exit onset; execute both complete doorway schedules.
    for (map, index, handoff, facing) in [
        (0xc, 0, (120, 464), Direction::Down),
        (0xd, 1, (120, 608), Direction::Up),
    ] {
        assert_eq!(s.map_id, map);
        s.walking = WalkingState::new(handoff.0, handoff.1);
        s.animation = AnimationState::standing(facing);
        if let Some(pot) = s.pandora.as_mut().unwrap().pot.as_mut() {
            pot.rebase(s.walking, facing).unwrap();
        }
        s.transition = Some(Transition::select(map, index, handoff).unwrap());
        for _ in 0..35 {
            replay(&mut s, &d, neutral);
        }
        assert_eq!(s.pandora_output(&d).unwrap().door_counter, 0);
        assert_eq!(
            s.pandora_output(&d).unwrap().sheet.cellar,
            CellarDoorPatch::Damaged
        );
        assert_eq!(s.consumed_pots(&d).unwrap().len(), 1);
    }
    assert_eq!(s.walking.position(), (120, 447));
    assert_eq!(s.pot_state().unwrap().walking(), &s.walking);
}

#[test]
fn preserve_copdf_samples_keep_gate_position_and_restore_facing() {
    let mut d = data();
    let motion = d
        .pandora
        .as_mut()
        .unwrap()
        .motions
        .iter_mut()
        .find(|m| m.key == MotionKey::Cue(Cue::BoxAcquireControl))
        .unwrap();
    motion.frames[0].pose = MotionPose::Preserve { facing: None };
    motion.frames[1].pose = MotionPose::Preserve {
        facing: Some(Direction::Down),
    };
    motion.frames.push(motion.frames[1]);
    for position in [(120, 368), (152, 400), (136, 384)] {
        for facing in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            let mut s = at(&d, 0x21, position, facing, &[0x26, 0x28, 0x27, 0x2e, 0x292]);
            ack(&mut s, &d);
            flag(&mut s, 1);
            flag(&mut s, 2);
            replay(&mut s, &d, neutral);
            replay(&mut s, &d, neutral);
            assert_eq!(s.walking.position(), position);
            assert_eq!(s.animation.facing(), facing);
            assert!(!s.flags.contains(0x22).unwrap());
            replay(&mut s, &d, neutral);
            assert_eq!(s.walking.position(), position);
            assert_eq!(s.animation.facing(), Direction::Down);
            assert!(!s.flags.contains(0x22).unwrap());
            for (byte, value) in [(100, Direction::Left as u8), (293, 0)] {
                let mut forged = s.snapshot();
                forged[byte] = value;
                assert!(GameState::restore(&d, &forged).is_err());
            }
            replay(&mut s, &d, neutral);
            assert_eq!(s.walking.position(), position);
            assert!(s.flags.contains(0x22).unwrap());
        }
    }
}

#[test]
fn preserve_after_absolute_checks_canonical_position_and_facing() {
    let mut d = data();
    let motion = d
        .pandora
        .as_mut()
        .unwrap()
        .motions
        .iter_mut()
        .find(|m| m.key == MotionKey::Cue(Cue::BoxAcquireControl))
        .unwrap();
    let absolute = anchor((136, 384), Direction::Left);
    motion.frames[0].pose = MotionPose::Absolute(absolute);
    motion.frames[1].pose = MotionPose::Preserve {
        facing: Some(Direction::Down),
    };
    motion.frames.push(motion.frames[1]);
    let mut s = at(
        &d,
        0x21,
        (136, 368),
        Direction::Up,
        &[0x26, 0x28, 0x27, 0x2e, 0x292],
    );
    ack(&mut s, &d);
    flag(&mut s, 1);
    flag(&mut s, 2);
    for _ in 0..3 {
        replay(&mut s, &d, neutral);
    }
    assert_eq!(s.walking.position(), absolute.position);
    assert_eq!(s.animation.facing(), Direction::Down);
    for (byte, value) in [(296, 137), (100, Direction::Left as u8), (293, 0), (294, 1)] {
        let mut forged = s.snapshot();
        forged[byte] = value;
        assert!(GameState::restore(&d, &forged).is_err());
    }
    replay(&mut s, &d, neutral);
}

#[test]
fn town_requires_real_interaction_and_preserves_pose_flags() {
    let d = navigation_data();
    for door in [TownDoor::North, TownDoor::Home] {
        let spec = d
            .pandora
            .as_ref()
            .unwrap()
            .navigation
            .as_ref()
            .unwrap()
            .door(door);
        let mut s = at(&d, 0xa, spec.interaction.position, Direction::Up, &[0x26]);
        let before = s.effective_room(&d).unwrap();
        let flags = s.flags.clone();
        let pose = s.walking.position();
        replay(&mut s, &d, GameState::interact);
        assert_eq!(s.walking.position(), pose);
        assert_eq!(s.flags, flags);
        assert_eq!(s.pandora_output(&d).unwrap().town_open, door.mask());
        let after = s.effective_room(&d).unwrap();
        assert_eq!(
            before
                .cells()
                .iter()
                .zip(after.cells())
                .filter(|(a, b)| a != b)
                .count(),
            2
        );
        let snapshot = s.snapshot();
        assert_eq!(s.interact(&d), Err(SliceError::Interaction));
        assert_eq!(s.snapshot(), snapshot);
    }
}

fn navigation_spec() -> NavigationSpec {
    let key = |map_id, index| ExitKey { map_id, index };
    NavigationSpec {
        maps: vec![
            MapExits {
                map_id: 0xa,
                records: vec![
                    Exit([31, 45, 1, 2, 13, 0, 0, 6, 112, 0, 192, 2]),
                    Exit([29, 16, 1, 2, 19, 0, 0, 6, 128, 1, 192, 0]),
                ],
            },
            MapExits {
                map_id: 0x13,
                records: vec![Exit([24, 12, 1, 2, 10, 0, 0, 5, 208, 1, 32, 1])],
            },
            MapExits {
                map_id: 0xc,
                records: crate::house::exits(0xc).unwrap().to_vec(),
            },
            MapExits {
                map_id: 0xd,
                records: crate::house::exits(0xd).unwrap().to_vec(),
            },
            MapExits {
                map_id: 0xe,
                records: vec![Exit([6, 53, 1, 1, 32, 0, 0, 14, 144, 1, 96, 3])],
            },
            MapExits {
                map_id: 0x20,
                records: vec![Exit([22, 53, 1, 1, 33, 0, 0, 14, 128, 0, 112, 0])],
            },
        ],
        travels: vec![
            TravelExit {
                travel: Travel::TownToResident,
                exit: key(0xa, 1),
            },
            TravelExit {
                travel: Travel::ResidentToTown,
                exit: key(0x13, 0),
            },
            TravelExit {
                travel: Travel::TownToHouse,
                exit: key(0xa, 0),
            },
            TravelExit {
                travel: Travel::CToCellar,
                exit: key(0xc, 3),
            },
            TravelExit {
                travel: Travel::ETo20,
                exit: key(0xe, 0),
            },
            TravelExit {
                travel: Travel::TwentyToBox,
                exit: key(0x20, 0),
            },
        ],
        doors: [(TownDoor::North, 1, 29, 16), (TownDoor::Home, 0, 31, 45)].map(
            |(door, index, x, y)| TownDoorSpec {
                door,
                exit: key(0xa, index),
                interaction: anchor((x * 16 + 8, (y + 3) * 16), Direction::Up),
                patches: [
                    CellPatch {
                        cell: y * 64 + x,
                        closed: 0x1cf2,
                        open: 0x1cf6,
                    },
                    CellPatch {
                        cell: (y + 1) * 64 + x,
                        closed: 0x1cf3,
                        open: 0x00f7,
                    },
                ],
            },
        ),
    }
}
fn navigation_data() -> GameData {
    let mut d = data();
    let mut p = d.pandora.take().unwrap();
    let nav = navigation_spec();
    for door in nav.doors {
        for patch in door.patches {
            p.rooms[0]
                .room
                .replace_cell(usize::from(patch.cell), patch.closed);
        }
    }
    for (travel, trigger, loaded, scene) in [
        (
            Travel::TownToResident,
            anchor((472, 288), Direction::Up),
            (392, 224),
            ScenePhase::Resident13,
        ),
        (
            Travel::ResidentToTown,
            anchor((392, 208), Direction::Down),
            (472, 288),
            ScenePhase::TownSource,
        ),
        (
            Travel::TownToHouse,
            anchor((504, 752), Direction::Up),
            (120, 736),
            ScenePhase::House,
        ),
        (
            Travel::CToCellar,
            anchor((184, 352), Direction::Up),
            (138, 857),
            ScenePhase::CellarE,
        ),
        (
            Travel::ETo20,
            anchor((104, 864), Direction::Up),
            (394, 857),
            ScenePhase::Cellar20,
        ),
        (
            Travel::TwentyToBox,
            anchor((360, 864), Direction::Up),
            (122, 105),
            ScenePhase::BoxContact,
        ),
    ] {
        let ordinary = matches!(
            travel,
            Travel::TownToResident | Travel::ResidentToTown | Travel::TownToHouse
        );
        let frames = if ordinary {
            (1..=35)
                .map(|elapsed| MotionFrame {
                    map_id: if elapsed <= 17 {
                        travel.maps().0
                    } else {
                        travel.maps().1
                    },
                    pose: MotionPose::Absolute(anchor(
                        crate::transition::doorway_position(
                            trigger.position,
                            loaded,
                            trigger.facing,
                            elapsed,
                        )
                        .unwrap(),
                        trigger.facing,
                    )),
                    reload: elapsed == 18,
                    scene: if elapsed <= 17 {
                        if travel == Travel::ResidentToTown {
                            ScenePhase::Resident13
                        } else {
                            ScenePhase::TownSource
                        }
                    } else {
                        scene
                    },
                })
                .collect()
        } else {
            vec![MotionFrame {
                map_id: travel.maps().1,
                pose: MotionPose::Absolute(anchor(loaded, Direction::Up)),
                reload: true,
                scene,
            }]
        };
        p.motions.push(MotionSpec {
            key: MotionKey::Travel(travel),
            trigger: Some(trigger),
            frames,
        });
    }
    d.pandora = Some(p.with_navigation(nav).unwrap());
    d
}

#[test]
fn navigation_contract_rejects_mutated_order_binding_raw_and_patch() {
    let bad = |change: fn(&mut NavigationSpec)| {
        let mut p = navigation_data().pandora.unwrap();
        let mut nav = p.navigation.take().unwrap();
        change(&mut nav);
        assert!(p.with_navigation(nav).is_err());
    };
    bad(|n| n.maps.swap(0, 1));
    bad(|n| n.maps[0].records.swap(0, 1));
    bad(|n| n.travels[0].exit.index = 0);
    bad(|n| n.travels[0].exit.map_id = 0x13);
    bad(|n| n.travels[1] = n.travels[0]);
    bad(|n| {
        n.maps[0].records[1].0[8] += 1;
    });
    bad(|n| n.maps[0].records[1].0[6] = 1);
    bad(|n| n.maps[0].records[1].0[7] = 14);
    bad(|n| n.maps[0].records[1].0[2] = 0);
    bad(|n| n.maps[0].records[1].0[0] = 255);
    bad(|n| n.doors[0].patches[0].cell += 1);
    bad(|n| n.doors[0].patches[0].open |= 0x8000);
    bad(|n| n.doors[0].patches[1].closed = 0x00f7);
    bad(|n| n.doors[1] = n.doors[0]);
    bad(|n| n.doors[0].interaction.position.1 -= 1);
    bad(|n| {
        let record = n.maps[0].records[1];
        n.maps[0].records.insert(0, record);
    });
    for frame in [0, 16, 17, 18, 34] {
        let mut p = navigation_data().pandora.unwrap();
        let nav = p.navigation.take().unwrap();
        let m = p
            .motions
            .iter_mut()
            .find(|m| m.key == MotionKey::Travel(Travel::TownToResident))
            .unwrap();
        m.frames[frame].pose = MotionPose::Absolute(anchor((472, 288), Direction::Up));
        assert!(p.with_navigation(nav).is_err());
    }
    let mut p = navigation_data().pandora.unwrap();
    let nav = p.navigation.take().unwrap();
    p.motions
        .iter_mut()
        .find(|m| m.key == MotionKey::Travel(Travel::TownToResident))
        .unwrap()
        .frames
        .pop();
    assert!(p.with_navigation(nav).is_err());
    let mut p = navigation_data().pandora.unwrap();
    let mut nav = p.navigation.take().unwrap();
    nav.maps[2].records[0].0[8] += 1; // Shape-valid, but not the aggregate's C list.
    let p = p.with_navigation(nav).unwrap();
    let base = progression_tests::data();
    let identity = DataIdentity {
        content_sha256: [9; 32],
        ..base.identity
    };
    assert!(base.with_pandora(p, identity).is_err());
}

#[test]
fn ordered_travel_fine_failure_unsupported_and_postselection_errors_are_atomic() {
    for first in [
        Exit([29, 16, 1, 1, 255, 0, 0, 0, 0, 0, 0, 0]),
        Exit([29, 16, 1, 2, 255, 0, 0, 0, 0, 0, 0, 0]),
    ] {
        let mut d = navigation_data();
        let mut s = at(&d, 0xa, (472, 304), Direction::Up, &[0x26]);
        s.interact(&d).unwrap();
        // At origin y=271 first record coarsely matches row16, but fails its fine test;
        // second candidate would match. The other case selects unsupported255.
        let p = d.pandora.as_mut().unwrap();
        let n = p.navigation.as_mut().unwrap();
        n.maps[0].records.insert(0, first);
        for binding in &mut n.travels {
            if binding.exit.map_id == 0xa {
                binding.exit.index += 1;
            }
        }
        for door in &mut n.doors {
            door.exit.index += 1;
        }
        s.walking = WalkingState::new(472, 287);
        let before = s.snapshot();
        let result = s.step(&d, FrameInput::default());
        if first.0[3] == 1 {
            assert!(result.is_ok());
            assert_eq!(s.pandora_output(&d).unwrap().motion, None);
        } else {
            assert_eq!(result, Err(SliceError::Exit));
            assert_eq!(s.snapshot(), before);
        }
    }
    // Correct ordinal with wrong mandatory exact witness or delayed direction cannot fall through.
    let d = navigation_data();
    for (position, facing) in [((472, 287), Direction::Up), ((472, 288), Direction::Down)] {
        let mut s = at(&d, 0xa, (472, 304), Direction::Up, &[0x26]);
        s.interact(&d).unwrap();
        s.walking = WalkingState::new(position.0, position.1);
        s.animation = AnimationState::standing(facing);
        let before = s.snapshot();
        assert_eq!(s.step(&d, FrameInput::default()), Err(SliceError::Exit));
        assert_eq!(s.snapshot(), before);
    }
}

#[test]
fn town_interaction_errors_and_reload_are_atomic_and_occupancy_is_preserved() {
    let mut d = navigation_data();
    let cell = 16 * 64 + 29;
    d.pandora.as_mut().unwrap().rooms[0]
        .room
        .replace_cell(cell, 0x9cf2);
    let mut s = at(&d, 0xa, (472, 304), Direction::Up, &[0x26]);
    let original = s.clone();
    for (position, facing, overflow) in [
        ((471, 304), Direction::Up, false),
        ((472, 304), Direction::Down, false),
        ((472, 304), Direction::Up, true),
    ] {
        let mut bad = original.clone();
        bad.walking = WalkingState::new(position.0, position.1);
        bad.animation = AnimationState::standing(facing);
        if overflow {
            bad.tick = u64::MAX;
        }
        let before = bad.snapshot();
        assert_eq!(
            bad.interact(&d),
            Err(if overflow {
                SliceError::TickOverflow
            } else {
                SliceError::Interaction
            })
        );
        assert_eq!(bad.snapshot(), before);
    }
    s.interact(&d).unwrap();
    assert_eq!(s.effective_room(&d).unwrap().cells()[cell], 0x9cf6);
    assert_eq!(s.current_room(&d).unwrap().cells()[cell], 0x9cf2);
    s.walking = WalkingState::new(504, 768);
    s.interact(&d).unwrap();
    assert_eq!(s.pandora_output(&d).unwrap().town_open, 3);
    load_at(&mut s, &d, 0x13, (392, 207));
    load_at(&mut s, &d, 0xa, (472, 305));
    assert_eq!(s.pandora_output(&d).unwrap().town_open, 0);
    assert_eq!(s.effective_room(&d).unwrap().cells()[cell], 0x9cf2);
    let mut bad = s.snapshot();
    bad[303] = 4;
    assert!(GameState::restore(&d, &bad).is_err());
}

#[test]
fn material_aliases_require_their_pandora_profile_membership() {
    use crate::{MaterialAlias, MaterialRule};
    for key in CollisionKey::ALL {
        for (alias, bounds) in [
            (MaterialAlias::TownSolid25, [0, 0, 1, 1]),
            (MaterialAlias::ClosedDoorPartial5, [11, 21, 12, 22]),
            (MaterialAlias::StairOpen29, [11, 21, 12, 22]),
            (MaterialAlias::StairOpen29, [6, 53, 7, 54]),
            (MaterialAlias::StairOpen29, [22, 53, 23, 54]),
        ] {
            let mut p = data().pandora.unwrap();
            let room = &mut p.rooms[key as usize].room;
            if bounds[2] > room.width() || bounds[3] > room.height() {
                continue;
            }
            *room = room
                .clone()
                .with_material_policy(vec![MaterialRule {
                    bounds,
                    direction: if alias == MaterialAlias::TownSolid25 {
                        None
                    } else {
                        Some(Direction::Up)
                    },
                    alias,
                }])
                .unwrap();
            let allowed = if alias == MaterialAlias::TownSolid25 {
                key == CollisionKey::Town
            } else {
                match bounds {
                    [6, 53, 7, 54] => key == CollisionKey::CellarE,
                    [22, 53, 23, 54] => key == CollisionKey::Cellar20,
                    _ => key.map() == 0xc,
                }
            };
            let result = PandoraData::new(
                p.text,
                p.rooms,
                p.motions,
                p.contacts,
                p.opening_gate,
                p.objects,
                p.cellar_up_lanes,
            );
            assert_eq!(result.is_ok(), allowed, "{key:?} {alias:?}");
        }
    }
}

#[test]
fn town_closed_routes_and_departure_snapshot_cannot_bypass_interact() {
    let d = navigation_data();
    for door in [TownDoor::North, TownDoor::Home] {
        let nav = d.pandora.as_ref().unwrap().navigation.as_ref().unwrap();
        let spec = nav.door(door);
        let mut s = at(&d, 0xa, spec.interaction.position, Direction::Up, &[0x26]);
        for _ in 0..24 {
            let before = s.snapshot();
            if s.step(
                &d,
                FrameInput {
                    direction: Some(Direction::Up),
                },
            )
            .is_err()
            {
                assert_eq!(s.snapshot(), before);
                break;
            }
            assert_eq!(s.pandora_output(&d).unwrap().motion, None);
            assert_eq!(s.map_id, 0xa);
        }
        let mut s = at(&d, 0xa, spec.interaction.position, Direction::Up, &[0x26]);
        s.interact(&d).unwrap();
        s.walking = WalkingState::new(
            spec.interaction.position.0,
            spec.interaction.position.1 - 13,
        );
        frames(&mut s, &d, Some(Direction::Up), 4);
        for cursor in 0..18 {
            assert_eq!(s.pandora_output(&d).unwrap().motion.unwrap().1, cursor);
            for mask in [0, 3 ^ door.mask()] {
                let mut bad = s.snapshot();
                bad[303] = mask;
                assert!(GameState::restore(&d, &bad).is_err());
            }
            replay(&mut s, &d, neutral);
        }
        let mut bad = s.snapshot();
        bad[303] = door.mask();
        assert!(GameState::restore(&d, &bad).is_err());
    }
}

#[test]
fn resident_return_uses_all_35_samples_and_reconstructs_closed_town() {
    let d = navigation_data();
    let mut s = at(&d, 0x13, (392, 205), Direction::Down, &[0x26]);
    frames(&mut s, &d, Some(Direction::Down), 4);
    assert_eq!(
        s.pandora_output(&d).unwrap().motion,
        Some((MotionKey::Travel(Travel::ResidentToTown), 0))
    );
    for _ in 0..35 {
        replay(&mut s, &d, neutral);
    }
    assert_eq!(s.map_id, 0xa);
    assert_eq!(s.walking.position(), (472, 305));
    assert_eq!(s.pandora_output(&d).unwrap().town_open, 0);
}

#[test]
fn stair_selection_is_ordered_and_wrong_postselection_witness_is_atomic() {
    let d = navigation_data();
    for (map, position, travel) in [
        (0xe, (104, 864), Travel::ETo20),
        (0x20, (360, 864), Travel::TwentyToBox),
    ] {
        let mut s = at(
            &d,
            map,
            (position.0, position.1 + 3),
            Direction::Up,
            &[0x26, 0x28, 0x27, 0x2e, 0x292],
        );
        frames(&mut s, &d, Some(Direction::Up), 4);
        assert_eq!(
            s.pandora_output(&d).unwrap().motion,
            Some((MotionKey::Travel(travel), 0))
        );
        replay(&mut s, &d, neutral);
        assert_eq!(s.map_id, travel.maps().1);
        let mut s = at(
            &d,
            map,
            position,
            Direction::Up,
            &[0x26, 0x28, 0x27, 0x2e, 0x292],
        );
        let before = s.snapshot();
        assert_eq!(s.step(&d, FrameInput::default()), Err(SliceError::Exit)); // no delayed direction
        assert_eq!(s.snapshot(), before);
    }
}

#[test]
fn missing_navigation_has_no_exact_trigger_fallback() {
    let mut d = navigation_data();
    d.pandora.as_mut().unwrap().navigation = None;
    let mut s = at(&d, 0xa, (472, 304), Direction::Up, &[0x26]);
    let before = s.snapshot();
    assert_eq!(s.interact(&d), Err(SliceError::Interaction));
    assert_eq!(s.snapshot(), before);
    // Even an internal motion-cursor forgery cannot restore without source selection.
    let id = d
        .pandora
        .as_ref()
        .unwrap()
        .motion(MotionKey::Travel(Travel::TownToResident))
        .unwrap()
        .0;
    s.walking = WalkingState::new(472, 288);
    s.pandora.as_mut().unwrap().motion = Some((u8::try_from(id).unwrap(), 0));
    assert!(GameState::restore(&d, &s.snapshot()).is_err());
}

#[test]
fn restored_arrival_cannot_invent_town_interaction_or_change_d_load_gate() {
    let d = navigation_data();
    for travel in [Travel::ResidentToTown, Travel::TownToHouse] {
        let p = d.pandora.as_ref().unwrap();
        let (id, motion) = p.motion(MotionKey::Travel(travel)).unwrap();
        let trigger = motion.trigger.unwrap();
        let mut s = at(
            &d,
            travel.maps().0,
            trigger.position,
            trigger.facing,
            &[0x26],
        );
        if travel == Travel::TownToHouse {
            s.walking = WalkingState::new(504, 768);
            s.interact(&d).unwrap();
            s.walking = WalkingState::new(trigger.position.0, trigger.position.1);
        }
        s.pandora.as_mut().unwrap().motion = Some((u8::try_from(id).unwrap(), 0));
        for _ in 0..18 {
            replay(&mut s, &d, neutral);
        }
        for _ in 18..35 {
            let mut bad = s.snapshot();
            if travel == Travel::ResidentToTown {
                bad[303] = TownDoor::North.mask();
            } else {
                bad[173] = 0;
            }
            assert!(GameState::restore(&d, &bad).is_err(), "{travel:?}");
            replay(&mut s, &d, neutral);
        }
    }
}

#[test]
fn idle_restore_cannot_erase_selected_navigation_ownership() {
    let d = navigation_data();
    let mut s = at(&d, 0xa, (472, 304), Direction::Up, &[0x26]);
    s.interact(&d).unwrap();
    s.walking = WalkingState::new(472, 288);
    assert!(GameState::restore(&d, &s.snapshot()).is_err());
}

#[test]
fn legacy_d_to_town_arrival_cannot_invent_open_doors() {
    let d = navigation_data();
    let exit = crate::house::EXTERIOR;
    let mut s = at(&d, 0xd, exit.handoff, exit.direction, &[0x26]);
    s.d_open_loaded = true;
    s.transition = Some(Transition::select(0xd, 0, exit.handoff).unwrap());
    for _ in 0..18 {
        replay(&mut s, &d, neutral);
    }
    for _ in 18..35 {
        let mut bad = s.snapshot();
        bad[303] = 3;
        assert!(GameState::restore(&d, &bad).is_err());
        replay(&mut s, &d, neutral);
    }
}

#[test]
fn c_stair_keeps_source_selection_and_empty_pot_ownership() {
    let d = navigation_data();
    let mut s = at(
        &d,
        0xc,
        (184, 355),
        Direction::Up,
        &[0x26, 0x28, 0x27, 0x2e],
    );
    // Isolated post-departure resident-sheet fixture, not an action shortcut API.
    let p = s.pandora.as_mut().unwrap();
    p.pot = None;
    p.sheet.cellar = CellarDoorPatch::Open;
    p.sheet.parked_consumed = 3;
    flag(&mut s, 0x292);
    s.pandora_load().unwrap();
    s.ensure_pot(d.pandora.as_ref().unwrap()).unwrap();
    frames(&mut s, &d, Some(Direction::Up), 4);
    assert_eq!(
        s.pandora_output(&d).unwrap().motion,
        Some((MotionKey::Travel(Travel::CToCellar), 0))
    );
    replay(&mut s, &d, neutral);
    assert_eq!(s.map_id, 0xe);
    assert!(s.pot_state().is_none());
}

// Complete Town source list $818D53..$818DBF (terminator excluded), checked
// against the owned JP ROM. Ordinal8 is an exit predicate, not a grid extent.
fn authentic_town_exits() -> Vec<Exit> {
    vec![
        Exit([0x38, 0x2e, 1, 1, 0x1d, 0, 0, 6, 0x70, 1, 0xb0, 1]),
        Exit([0x31, 0x2e, 1, 1, 0x1e, 0, 0, 6, 0x70, 2, 0xb0, 0]),
        Exit([0x1f, 0x2e, 1, 1, 0x0d, 0, 0, 6, 0x70, 0, 0xc0, 2]),
        Exit([0x04, 0x27, 1, 1, 0x1f, 0, 0, 6, 0x70, 2, 0xb0, 1]),
        Exit([0x35, 0x11, 1, 1, 0x17, 0, 0, 6, 0x80, 1, 0xc0, 1]),
        Exit([0x1d, 0x11, 1, 1, 0x13, 0, 0, 6, 0x80, 1, 0xc0, 0]),
        Exit([0x20, 0x0c, 1, 1, 0x15, 0, 0, 6, 0x70, 3, 0xb0, 0]),
        Exit([0x0a, 0x0b, 1, 1, 0x1b, 0, 0, 6, 0x60, 0, 0xb0, 1]),
        Exit([0x00, 0x3e, 0x50, 2, 3, 0, 0, 0x55, 0x10, 2, 0x10, 2]),
    ]
}
fn with_authentic_town_exits() -> GameData {
    let mut d = navigation_data();
    let mut p = d.pandora.take().unwrap();
    let mut nav = p.navigation.take().unwrap();
    nav.maps[0].records = authentic_town_exits();
    for binding in &mut nav.travels {
        match binding.travel {
            Travel::TownToResident => binding.exit.index = 5,
            Travel::TownToHouse => binding.exit.index = 2,
            _ => {}
        }
    }
    nav.doors[0].exit.index = 5;
    nav.doors[1].exit.index = 2;
    d.pandora = Some(p.with_navigation(nav).unwrap());
    d
}
#[test]
fn authentic_overhanging_town_record_is_retained_and_unsupported_exit_is_atomic() {
    let d = with_authentic_town_exits();
    let records = &d
        .pandora
        .as_ref()
        .unwrap()
        .navigation
        .as_ref()
        .unwrap()
        .maps[0]
        .records;
    assert_eq!(*records, authentic_town_exits());
    assert_eq!(select_exit(records, (136, 1024)).unwrap().0, 8);
    let mut s = at(&d, 0xa, (136, 1026), Direction::Up, &[0x26]);
    for _ in 0..8 {
        let before = s.snapshot();
        let result = s.step(
            &d,
            FrameInput {
                direction: Some(Direction::Up),
            },
        );
        if result == Err(SliceError::Exit) {
            assert_eq!(s.snapshot(), before);
            assert_eq!(GameState::restore(&d, &before).unwrap(), s);
            assert_eq!(s.map_id, 0xa);
            return;
        }
        result.unwrap();
        assert_eq!(GameState::restore(&d, &s.snapshot()).unwrap(), s);
    }
    panic!("source unsupported exit was not rejected");
}
#[test]
fn overhanging_predicate_fine_failure_still_blocks_later_record() {
    let mut d = with_authentic_town_exits();
    let mut p = d.pandora.take().unwrap();
    let mut nav = p.navigation.take().unwrap();
    // Synthetic later predicate: fine-matches where authentic ordinal8 fails.
    let later = Exit([8, 63, 1, 2, 3, 0, 0, 0x55, 0x10, 2, 0x10, 2]);
    nav.maps[0].records.push(later);
    let position = (136, 1025);
    assert!(select_exit(&[later], position).is_some());
    assert!(select_exit(&nav.maps[0].records, position).is_none());
    d.pandora = Some(p.with_navigation(nav).unwrap());
    let mut s = at(&d, 0xa, position, Direction::Up, &[0x26]);
    replay(&mut s, &d, neutral);
    assert_eq!(s.output().position, position);
    assert_eq!(s.pandora_output(&d).unwrap().motion, None);
}

#[test]
fn overhanging_exit_does_not_relax_origins_or_nonempty_dimensions() {
    for (field, value) in [(0, 64), (1, 80), (2, 0), (3, 0)] {
        let mut p = with_authentic_town_exits().pandora.unwrap();
        let mut nav = p.navigation.take().unwrap();
        nav.maps[0].records[8].0[field] = value;
        assert!(p.with_navigation(nav).is_err());
    }
    // Maximum byte extents still use the existing bounded u16 fine arithmetic;
    // they cannot overflow into a match across the wrapped coarse boundary.
    let far = Exit([63, 79, 255, 255, 3, 0, 0, 0x55, 0x10, 2, 0x10, 2]);
    assert!(select_exit(&[far], (8, 16)).is_none());
    assert!(select_exit(&[far], (1016, 1280)).is_some());
}

fn assert_dialogue_ready_readonly(s: &GameState, d: &GameData, ready: bool) {
    let before = s.clone();
    let bytes = s.snapshot();
    assert_eq!(s.dialogue_input_ready(d), Ok(ready));
    assert_eq!(*s, before);
    assert_eq!(s.snapshot(), bytes);
}
#[test]
fn dialogue_readiness_is_readonly_and_never_falls_back_on_wrong_data() {
    let d = data();
    let s = GameState::new_game(&d, Policy::SemanticPreview);
    assert_dialogue_ready_readonly(&s, &d, false);
    let mut wrong = data();
    wrong.identity.content_sha256[0] ^= 1;
    assert_eq!(s.dialogue_input_ready(&wrong), Err(SliceError::Data));
    let mut wrong = data();
    wrong.progression = None;
    assert_eq!(s.dialogue_input_ready(&wrong), Err(SliceError::Data));
    let mut wrong = data();
    wrong.pandora = None;
    assert_eq!(s.dialogue_input_ready(&wrong), Err(SliceError::Data));
    let mut s = at(&d, 0xc, (120, 464), Direction::Up, &[0x26, 0x28]);
    s.tick = u64::MAX;
    // Readiness is ownership admission, not a speculative action/tick-overflow probe.
    assert_dialogue_ready_readonly(&s, &d, true);
    assert_eq!(s.acknowledge(&d), Err(SliceError::TickOverflow));
}
#[test]
fn c_entry_is_visible_but_not_ready_until_all_seventeen_arrival_updates() {
    let d = navigation_data();
    let mut s = at(&d, 0xd, (120, 611), Direction::Up, &[0x26, 0x28]);
    frames(&mut s, &d, Some(Direction::Up), 4);
    assert!(s.transition.is_some());
    frames(&mut s, &d, None, 18);
    assert_eq!(s.map_id, 0xc);
    assert_eq!(s.output().position, (120, 464));
    let arrival_tick = s.tick;
    for _ in 0..17 {
        assert!(s.dialogue(&d).unwrap().is_some());
        assert_eq!(
            s.pandora_output(&d).unwrap().invocation,
            Some(Invocation::CEntry)
        );
        assert_dialogue_ready_readonly(&s, &d, false);
        let before = s.snapshot();
        assert_eq!(s.acknowledge(&d), Err(SliceError::Interaction));
        assert_eq!(s.choose(&d, 1), Err(SliceError::Interaction));
        assert_eq!(s.snapshot(), before);
        replay(&mut s, &d, neutral);
    }
    assert_eq!(s.tick, arrival_tick + 17);
    assert_eq!(s.output().position, (120, 447));
    assert_dialogue_ready_readonly(&s, &d, true);
    ack(&mut s, &d);
}
#[test]
fn box_entry_visibility_does_not_make_remaining_travel_samples_ready() {
    let mut d = navigation_data();
    let motion = d
        .pandora
        .as_mut()
        .unwrap()
        .motions
        .iter_mut()
        .find(|m| m.key == MotionKey::Travel(Travel::TwentyToBox))
        .unwrap();
    motion.frames.push(MotionFrame {
        map_id: 0x21,
        pose: MotionPose::Absolute(anchor((136, 128), Direction::Down)),
        reload: false,
        scene: ScenePhase::BoxContact,
    });
    let mut s = at(
        &d,
        0x20,
        (360, 867),
        Direction::Up,
        &[0x26, 0x28, 0x27, 0x2e, 0x292],
    );
    frames(&mut s, &d, Some(Direction::Up), 4);
    assert_dialogue_ready_readonly(&s, &d, false);
    replay(&mut s, &d, neutral); // Actual reload requests BoxEntry; arrival is not done.
    assert_eq!(
        s.pandora_output(&d).unwrap().invocation,
        Some(Invocation::BoxEntry)
    );
    assert!(s.dialogue(&d).unwrap().is_some());
    assert_dialogue_ready_readonly(&s, &d, false);
    let before = s.snapshot();
    assert_eq!(s.acknowledge(&d), Err(SliceError::Interaction));
    assert_eq!(s.snapshot(), before);
    replay(&mut s, &d, neutral); // Preserve the remaining qualified arrival sample.
    assert_eq!(s.output().position, (136, 128));
    assert_dialogue_ready_readonly(&s, &d, true);
    ack(&mut s, &d);
    assert_dialogue_ready_readonly(&s, &d, false);
}
