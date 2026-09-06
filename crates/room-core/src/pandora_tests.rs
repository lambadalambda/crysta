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
            trigger: anchor((136, 359), Direction::Down),
            result: anchor((136, 359), Direction::Down),
        },
        ContactSpec {
            kind: ContactKind::BoxOpen,
            trigger: anchor((136, 368), Direction::Down),
            result: anchor((136, 368), Direction::Down),
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
    let spec = PandoraData::new(text(), rooms, motions, contacts, objects, true).unwrap();
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
    s.pandora_load();
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
    assert_eq!(&s.snapshot()[..8], b"RSLC\x02\x0a\x00\x01");
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
    s.pandora_load();
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
    assert_eq!(valid.len(), 300);
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
    for len in [0, 8, 180, 181, 245, 299, 301] {
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
    bad(|p| p.contacts[2].result.position = (256, 368));
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
