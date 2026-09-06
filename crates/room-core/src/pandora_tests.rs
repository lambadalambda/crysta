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
                anchor: anchor(
                    pose,
                    if map == 0xc {
                        Direction::Up
                    } else {
                        Direction::Down
                    },
                ),
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
                anchor: anchor(pose, Direction::Up),
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
    assert_eq!(&s.snapshot()[..8], b"RSLC\x03\x0b\x00\x01");
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
    let mut d = data();
    d.pandora.as_mut().unwrap().motions.push(MotionSpec {
        key: MotionKey::Travel(Travel::TownToResident),
        trigger: Some(anchor((136, 208), Direction::Down)),
        frames: vec![
            MotionFrame {
                map_id: 0xa,
                anchor: anchor((136, 208), Direction::Down),
                reload: false,
                scene: ScenePhase::TownSource,
            },
            MotionFrame {
                map_id: 0x13,
                anchor: anchor((360, 144), Direction::Up),
                reload: true,
                scene: ScenePhase::Resident13,
            },
            MotionFrame {
                map_id: 0x13,
                anchor: anchor((360, 144), Direction::Up),
                reload: false,
                scene: ScenePhase::Resident13,
            },
        ],
    });
    let mut s = at(&d, 0xa, (136, 205), Direction::Down, &[0x26]);
    frames(&mut s, &d, Some(Direction::Down), 4);
    assert_eq!(
        s.pandora_output(&d).unwrap().owner,
        ControlOwner::Transition
    );
    frames(&mut s, &d, Some(Direction::Left), 3);
    assert_eq!(s.output().position, (360, 144));
    assert_eq!(s.output().map_id, 0x13);
    assert_eq!(s.walking.last_activation_direction(), None);
    replay(&mut s, &d, GameState::interact);
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
    bad(|p| p.motions[0].frames[0].map_id = 0x41);
    bad(|p| p.motions[0].frames[0].scene = ScenePhase::Tour410);
    bad(|p| p.motions[0].frames[0].anchor.position = (512, 352));
    bad(|p| p.motions[0].key = MotionKey::Cue(Cue::Returned(Invocation::CChoice)));
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
                anchor: anchor((136, 208), Direction::Down),
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
            let spec = d.pandora.as_mut().unwrap();
            let id = u8::try_from(spec.motions.len()).unwrap();
            // Synthetic pacing, authenticated route identity; exercise the runtime
            // motion load seam and snapshot cursors, not a native movement claim.
            spec.motions.push(MotionSpec {
                key: MotionKey::Travel(Travel::TownToHouse),
                trigger: Some(anchor((136, 208), Direction::Up)),
                frames: [(0xa, false), (0xd, true), (0xd, false)]
                    .into_iter()
                    .map(|(map_id, reload)| MotionFrame {
                        map_id,
                        reload,
                        anchor: anchor(
                            if map_id == 0xa {
                                (136, 208)
                            } else {
                                (136, 600)
                            },
                            Direction::Up,
                        ),
                        scene: if map_id == 0xa {
                            ScenePhase::TownSource
                        } else {
                            ScenePhase::House
                        },
                    })
                    .collect(),
            });
            returned.pandora.as_mut().unwrap().motion = Some((id, 0));
            for _ in 0..3 {
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
