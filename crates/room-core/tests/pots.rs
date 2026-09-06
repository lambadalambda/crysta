//! Atomic component, action/mode, source-membership and snapshot tests.
use room_core::pots::{Admission, Error, Input, Phase, PotState, SourceObject};
use room_core::{Direction, Room, WalkingState};

fn room() -> Room {
    let mut cells = vec![0; 32 * 32];
    cells[21 * 32 + 5] = 0x18fa;
    cells[21 * 32 + 4] = 0x18fb;
    cells[21 * 32 + 3] = 0x18fa;
    cells[20 * 32 + 11] = 0x1d80;
    cells[21 * 32 + 11] = 0x0b81;
    Room::new_passive(32, 32, cells).unwrap()
}
const OBJECTS: [SourceObject; 3] = [
    SourceObject {
        cell: 675,
        raw: 0x18fa,
        replacement: 0xf8,
    },
    SourceObject {
        cell: 676,
        raw: 0x18fb,
        replacement: 0xf8,
    },
    SourceObject {
        cell: 677,
        raw: 0x18fa,
        replacement: 0xf8,
    },
];
fn admission(room: &Room) -> Admission<'_> {
    Admission {
        room,
        objects: &OBJECTS,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    }
}
fn state(room: &Room, x: u16, y: u16, facing: Direction) -> PotState {
    PotState::new(&admission(room), WalkingState::new(x, y), facing).unwrap()
}
fn lift(s: &mut PotState, r: &Room) {
    s.step(
        &admission(r),
        Input {
            action: true,
            direction: None,
        },
    )
    .unwrap();
    let out = s.step(&admission(r), Input::default()).unwrap();
    assert!(out.consumed_cell.is_some());
    for _ in 0..23 {
        s.step(&admission(r), Input::default()).unwrap();
    }
    assert_eq!(s.phase(), Phase::Held);
}
fn frames(s: &mut PotState, r: &Room, d: Option<Direction>, count: usize) {
    for _ in 0..count {
        s.step(
            &admission(r),
            Input {
                direction: d,
                action: false,
            },
        )
        .unwrap();
    }
}

#[test]
fn lift_consumes_source_once_and_retains_source_slot() {
    let r = room();
    for (x, facing, slot, cell) in [
        (104, Direction::Left, 0x98a, 677),
        (88, Direction::Left, 0x98f, 676),
    ] {
        let mut s = state(&r, x, 352, facing);
        lift(&mut s, &r);
        assert_eq!(s.held_slot_in(&admission(&r)), Some(slot));
        assert!(s.consumed_in(&admission(&r), cell));
        assert_eq!(
            r.cells()[usize::from(cell)],
            if slot == 0x98a { 0x18fa } else { 0x18fb }
        );
    }
}

#[test]
fn carrying_has_no_ordinary_horizontal_restart_and_snapshot_continues() {
    let r = room();
    let mut s = state(&r, 104, 352, Direction::Left);
    lift(&mut s, &r);
    frames(&mut s, &r, Some(Direction::Down), 12);
    frames(&mut s, &r, None, 20); // a clear horizontal row, not through the door
    frames(&mut s, &r, Some(Direction::Right), 54);
    let bytes = s.encode_snapshot();
    let mut restored = PotState::decode_snapshot(&admission(&r), &bytes).unwrap();
    for _ in 0..20 {
        let input = Input {
            direction: Some(Direction::Right),
            action: false,
        };
        let out = s.step(&admission(&r), input).unwrap();
        assert!(out.movement.unwrap().dx > 0);
        assert_eq!(out.control, 0x20);
        assert_eq!(restored.step(&admission(&r), input).unwrap(), out);
        assert_eq!(restored.encode_snapshot(), s.encode_snapshot());
    }
}

#[test]
fn no_pot_no_throw_and_errors_are_atomic() {
    let r = room();
    let mut s = state(&r, 184, 368, Direction::Up);
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &admission(&r),
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::NoSourceObject)
    );
    assert_eq!(s.encode_snapshot(), before);
    // A rejected sample is not left queued to poison every future tick.
    s.step(&admission(&r), Input::default()).unwrap();
}

#[test]
fn repeated_or_combined_actions_are_not_new_throws() {
    let r = room();
    let mut s = state(&r, 104, 352, Direction::Left);
    let action = Input {
        action: true,
        direction: None,
    };
    s.step(&admission(&r), action).unwrap();
    let before = s.encode_snapshot();
    assert_eq!(s.step(&admission(&r), action), Err(Error::Input));
    assert_eq!(s.encode_snapshot(), before);
    assert_eq!(
        s.step(
            &admission(&r),
            Input {
                action: true,
                direction: Some(Direction::Up)
            }
        ),
        Err(Error::Input)
    );
}

#[test]
fn truncated_noncanonical_and_held_bad_phase_snapshots_fail_closed() {
    let r = room();
    let mut s = state(&r, 104, 352, Direction::Left);
    lift(&mut s, &r);
    let bytes = s.encode_snapshot();
    for n in 0..bytes.len() {
        assert!(PotState::decode_snapshot(&admission(&r), &bytes[..n]).is_err());
    }
    let mut bad = bytes;
    bad[0] ^= 1;
    assert!(PotState::decode_snapshot(&admission(&r), &bad).is_err());
}

#[test]
fn exact_stair_word_is_traversable_but_temporary_occupancy_is_not() {
    for (raw, pass) in [(0x3acb, true), (0xbacb, false), (0x0b81, false)] {
        let mut cells = vec![0; 32 * 32];
        cells[21 * 32 + 11] = raw;
        let r = Room::new_passive(32, 32, cells).unwrap();
        let a = Admission {
            room: &r,
            objects: &[],
            cellar_up_lanes: true,
            door_hit_enabled: false,
        };
        let mut s = PotState::new(&a, WalkingState::new(184, 368), Direction::Up).unwrap();
        for _ in 0..8 {
            s.step(
                &a,
                Input {
                    direction: Some(Direction::Up),
                    action: false,
                },
            )
            .unwrap();
        }
        assert_eq!(s.position().1 < 368, pass);
        assert_eq!(r.cells()[21 * 32 + 11], raw);
    }
}

fn ready(r: &Room) -> PotState {
    let mut s = state(r, 40, 352, Direction::Right);
    lift(&mut s, r);
    frames(&mut s, r, Some(Direction::Down), 12);
    frames(&mut s, r, None, 20);
    frames(&mut s, r, Some(Direction::Right), 97);
    frames(&mut s, r, None, 20);
    frames(&mut s, r, Some(Direction::Up), 1);
    frames(&mut s, r, None, 20);
    assert_eq!(s.position(), (184, 368));
    assert_eq!(s.facing(), Direction::Up);
    s
}

#[test]
fn release_contact_reservation_and_recovery_are_distinct_and_restore_safe() {
    let r = room();
    let a = admission(&r);
    let mut s = ready(&r);
    s.step(
        &a,
        Input {
            action: true,
            direction: None,
        },
    )
    .unwrap();
    s.step(&a, Input::default()).unwrap(); // throw age0
    let mut hits = 0;
    for age in 1..=40 {
        let bytes = s.encode_snapshot();
        let mut restored = PotState::decode_snapshot(&a, &bytes).unwrap();
        let out = s.step(&a, Input::default()).unwrap();
        assert_eq!(restored.step(&a, Input::default()).unwrap(), out);
        assert_eq!(restored, s);
        assert_eq!(out.held_changed == Some(None), age == 18);
        assert_eq!(out.door_hit, age == 19);
        assert_eq!(out.control_restored, age == 32);
        assert_eq!(s.held_slot_in(&a).is_some(), age < 18);
        assert_eq!(s.reserved_slot_in(&a).is_some(), age < 22);
        hits += usize::from(out.door_hit);
    }
    assert_eq!(hits, 1);
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &a,
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::NoSourceObject)
    );
    assert_eq!(s.encode_snapshot(), before);
    assert!(s.consumed_in(&a, 675));
}

#[test]
fn lane_direction_position_callback_and_busy_input_fail_closed() {
    let r = room();
    let mut s = ready(&r);
    let a = Admission {
        cellar_up_lanes: false,
        ..admission(&r)
    };
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &a,
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::ThrowLane)
    );
    assert_eq!(s.encode_snapshot(), before);
    frames(&mut s, &r, Some(Direction::Right), 1);
    frames(&mut s, &r, None, 20);
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &admission(&r),
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::ThrowLane)
    );
    assert_eq!(s.encode_snapshot(), before);
    frames(&mut s, &r, Some(Direction::Down), 3);
    frames(&mut s, &r, None, 20);
    frames(&mut s, &r, Some(Direction::Up), 1);
    frames(&mut s, &r, None, 20); // wrong Y, even though facing Up
    assert_eq!(
        s.step(
            &admission(&r),
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::ThrowLane)
    );
    let mut s = ready(&r);
    let a = Admission {
        door_hit_enabled: false,
        ..admission(&r)
    };
    s.step(
        &a,
        Input {
            action: true,
            direction: None,
        },
    )
    .unwrap();
    s.step(&a, Input::default()).unwrap();
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &a,
            Input {
                action: false,
                direction: Some(Direction::Up)
            }
        ),
        Err(Error::Input)
    );
    assert_eq!(s.encode_snapshot(), before);
    for _ in 0..40 {
        assert!(!s.step(&a, Input::default()).unwrap().door_hit);
    }
}

#[test]
fn membership_and_source_validation_are_not_a_pot_counter() {
    let r = room();
    let mut s = state(&r, 40, 352, Direction::Right);
    lift(&mut s, &r);
    assert!(s.consumed_in(&admission(&r), 675));
    assert!(!s.consumed_in(&admission(&r), 677));
    let mut objects = OBJECTS;
    objects[0].replacement = 0x18f8; // retaining old collision class is not removal
    let bad = Admission {
        objects: &objects,
        ..admission(&r)
    };
    let before = s.encode_snapshot();
    assert_eq!(s.step(&bad, Input::default()), Err(Error::Source));
    assert_eq!(s.encode_snapshot(), before);
    let mut bad = before;
    bad[31] = 3; // claim ownership of a different, unconsumed source cell
    assert_eq!(
        PotState::decode_snapshot(&admission(&r), &bad),
        Err(Error::Snapshot)
    );
    frames(&mut s, &r, Some(Direction::Down), 3);
    let mut bad = s.encode_snapshot();
    bad[4 + 12] = 3; // ordinary walker could admit this horizontal phase, held cannot
                     // Set a structurally valid horizontal state to isolate the enclosing check.
    bad[4 + 10] = 4;
    bad[4 + 11] = 4;
    bad[4 + 13] = 4;
    assert_eq!(
        PotState::decode_snapshot(&admission(&r), &bad),
        Err(Error::Snapshot)
    );
}

#[test]
fn classifier_aliases_require_exact_cell_raw_gate_and_up_direction() {
    for (col, raw, gate) in [
        (11, 0x3acb, false),
        (10, 0x3acb, true),
        (11, 0x3acc, true),
        (11, 0x0b82, true),
    ] {
        let mut cells = vec![0; 32 * 32];
        cells[21 * 32 + col] = raw;
        let r = Room::new_passive(32, 32, cells).unwrap();
        let a = Admission {
            room: &r,
            objects: &[],
            cellar_up_lanes: gate,
            door_hit_enabled: false,
        };
        let mut s = PotState::new(
            &a,
            WalkingState::new(u16::try_from(col * 16 + 8).unwrap(), 368),
            Direction::Up,
        )
        .unwrap();
        for _ in 0..2 {
            s.step(
                &a,
                Input {
                    direction: Some(Direction::Up),
                    action: false,
                },
            )
            .unwrap();
        }
        let before = s.encode_snapshot();
        assert!(s
            .step(
                &a,
                Input {
                    direction: Some(Direction::Up),
                    action: false
                }
            )
            .is_err());
        assert_eq!(s.encode_snapshot(), before);
    }
    let mut cells = vec![0; 32 * 32];
    cells[21 * 32 + 11] = 0x3acb;
    let r = Room::new_passive(32, 32, cells).unwrap();
    let a = Admission {
        room: &r,
        objects: &[],
        cellar_up_lanes: true,
        door_hit_enabled: false,
    };
    let mut s = PotState::new(&a, WalkingState::new(184, 368), Direction::Up).unwrap();
    for _ in 0..2 {
        s.step(
            &a,
            Input {
                direction: Some(Direction::Up),
                action: false,
            },
        )
        .unwrap();
    }
    // The old delayed Up still controls collision, not newly submitted Right.
    s.step(
        &a,
        Input {
            direction: Some(Direction::Right),
            action: false,
        },
    )
    .unwrap();
    s.step(
        &a,
        Input {
            direction: Some(Direction::Right),
            action: false,
        },
    )
    .unwrap(); // setup
    let before = s.encode_snapshot();
    // Conversely newly submitted Up cannot authorize the old delayed Right sample.
    assert!(s
        .step(
            &a,
            Input {
                direction: Some(Direction::Up),
                action: false
            }
        )
        .is_err());
    assert_eq!(s.encode_snapshot(), before);
}

#[test]
fn consumed_cell_cannot_be_lifted_again_after_a_real_throw() {
    let r = room();
    let mut s = ready(&r);
    s.step(
        &admission(&r),
        Input {
            action: true,
            direction: None,
        },
    )
    .unwrap();
    frames(&mut s, &r, None, 50);
    // Qualified ordinary one-pixel activations; no test-only position setter.
    for _ in 0..144 {
        frames(&mut s, &r, Some(Direction::Left), 2);
        frames(&mut s, &r, None, 20);
    }
    for _ in 0..16 {
        frames(&mut s, &r, Some(Direction::Up), 2);
        frames(&mut s, &r, None, 20);
    }
    frames(&mut s, &r, Some(Direction::Right), 1);
    frames(&mut s, &r, None, 20);
    assert_eq!(s.position(), (40, 352));
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &admission(&r),
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::NoSourceObject)
    );
    assert_eq!(s.encode_snapshot(), before);
    assert!(s.consumed_in(&admission(&r), 675));
    assert!(!s.consumed_in(&admission(&r), 677));
}

#[test]
fn type5_remains_partial_not_solid_in_mixed_pair() {
    let mut cells = vec![0; 32 * 32];
    cells[21 * 32 + 11] = 0x0b81;
    cells[21 * 32 + 12] = 0x1800;
    let r = Room::new_passive(32, 32, cells).unwrap();
    let a = Admission {
        room: &r,
        objects: &[],
        cellar_up_lanes: true,
        door_hit_enabled: false,
    };
    let mut s = PotState::new(&a, WalkingState::new(185, 368), Direction::Up).unwrap();
    for _ in 0..3 {
        s.step(
            &a,
            Input {
                direction: Some(Direction::Up),
                action: false,
            },
        )
        .unwrap();
    }
    assert_eq!(s.position(), (184, 368)); // P/S q<8 nudges -1; S/S would not.
}

#[test]
fn launch_coordinates_without_source_door_geometry_do_not_hit() {
    let r = room();
    let mut s = ready(&r);
    let mut cells = r.cells().to_vec();
    cells[21 * 32 + 11] = 0; // no actual closed target at the supposed hit lane
    let cleared = Room::new_passive(32, 32, cells).unwrap();
    let before = s.encode_snapshot();
    assert_eq!(
        s.step(
            &admission(&cleared),
            Input {
                action: true,
                direction: None
            }
        ),
        Err(Error::ThrowLane)
    );
    assert_eq!(s.encode_snapshot(), before);
}

#[test]
fn queued_actions_in_snapshots_must_have_an_admitted_source_or_lane() {
    let r = room();
    let a = admission(&r);
    let mut empty = state(&r, 184, 368, Direction::Up).encode_snapshot();
    empty[32] = 1;
    assert_eq!(PotState::decode_snapshot(&a, &empty), Err(Error::Snapshot));
    let mut held = ready(&r).encode_snapshot();
    held[28] = Direction::Down as u8;
    held[32] = 1;
    assert_eq!(PotState::decode_snapshot(&a, &held), Err(Error::Snapshot));

    for mut s in [state(&r, 40, 352, Direction::Right), ready(&r)] {
        s.step(
            &a,
            Input {
                action: true,
                direction: None,
            },
        )
        .unwrap();
        let mut restored = PotState::decode_snapshot(&a, &s.encode_snapshot()).unwrap();
        assert_eq!(
            s.step(&a, Input::default()),
            restored.step(&a, Input::default())
        );
        assert_eq!(s, restored);
    }
}
