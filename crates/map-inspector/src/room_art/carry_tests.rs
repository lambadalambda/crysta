use super::*;
use room_core::Direction::{Down, Left, Right, Up};

fn input(phase: Phase, facing: Direction, slot: u16) -> CarryInput {
    CarryInput {
        phase,
        phase_tick: 0,
        facing,
        motion: None,
        position: (184, 368),
        held_slot: Some(slot),
        reserved_slot: Some(slot),
        flight: None,
    }
}

#[test]
fn every_phase_facing_kind_uses_exact_record_zero_pair() {
    for (phase, walking, art, ark_lists, pot_lists) in [
        (
            Phase::Lifting,
            false,
            0x80_a261,
            [24, 25, 26, 26],
            [43, 44, 45, 45],
        ),
        (
            Phase::Held,
            false,
            0x80_a24f,
            [3, 4, 5, 5],
            [25, 26, 26, 26],
        ),
        (
            Phase::Held,
            true,
            0x80_a255,
            [9, 10, 11, 11],
            [29, 29, 30, 30],
        ),
        (
            Phase::Throwing,
            false,
            0x80_a261,
            [15, 16, 17, 17],
            [46, 47, 48, 48],
        ),
    ] {
        for (i, facing) in [Down, Up, Left, Right].into_iter().enumerate() {
            for (slot, pot_art) in [(0x98a, 0x96_e1a6), (0x98f, 0x96_e1ab)] {
                let mut state = input(phase, facing, slot);
                state.motion = walking.then_some(facing);
                let result = select(12, state).unwrap().unwrap();
                assert_eq!(result.actor_key, frame_key(art, ark_lists[i], 0, i == 2));
                let pose = pose(result.overlay["pose"].as_str().unwrap()).unwrap();
                assert_eq!(
                    pose["pots"][slot.to_string()],
                    frame_key(pot_art, pot_lists[i], 0, i == 2)
                );
                assert_eq!(result.overlay["flight"], Value::Null);
            }
        }
    }
}

#[test]
fn every_tick_preserves_hand_reservation_and_break_lifetimes() {
    for x in [136, 184] {
        for slot in [0x98a, 0x98f] {
            for tick in 0..32 {
                let end = if x == 184 { 22 } else { 27 };
                let mut state = input(Phase::Throwing, Up, slot);
                state.position = (x, 368);
                state.phase_tick = tick;
                state.held_slot = (tick < 18).then_some(slot);
                state.reserved_slot = (tick < end).then_some(slot);
                state.flight = (18..end).contains(&tick).then(|| Flight {
                    x,
                    y: 357 - 3 * u16::from(tick - 18),
                });
                let out = select(12, state).unwrap().unwrap();
                assert_eq!(out.actor_key, "pandora:80a261:16:0:0");
                assert_eq!(out.overlay["held_slot"], json!(state.held_slot));
                assert_eq!(out.overlay["reserved_slot"], json!(state.reserved_slot));
                assert_eq!(
                    out.overlay["flight"],
                    json!(state.flight.map(|p| [p.x, p.y]))
                );
                // Neither missing flight nor lingering reservation may invent a sprite.
                let mut bad = state;
                if state.flight.is_some() {
                    bad.flight = None;
                } else {
                    bad.flight = Some(Flight { x, y: 357 });
                }
                assert!(select(12, bad).is_err());
                let mut bad = state;
                bad.held_slot = if tick < 18 { None } else { Some(slot) };
                assert!(select(12, bad).is_err());
                let mut bad = state;
                bad.reserved_slot = if tick < end { None } else { Some(slot) };
                assert!(select(12, bad).is_err());
            }
        }
    }
    for phase in [Phase::Lifting, Phase::Held] {
        let mut state = input(phase, Left, 0x98a);
        for tick in 0..if phase == Phase::Lifting { 23 } else { 1 } {
            state.phase_tick = tick;
            assert!(select(12, state).unwrap().is_some());
        }
        state.phase_tick = if phase == Phase::Lifting { 23 } else { 1 };
        assert!(select(12, state).is_err());
    }
    let mut empty = input(Phase::Empty, Down, 0x98a);
    empty.held_slot = None;
    empty.reserved_slot = None;
    assert!(select(12, empty).unwrap().is_none());
    empty.reserved_slot = Some(0x98a);
    assert!(select(12, empty).is_err());
}

#[test]
fn malformed_slots_and_unqualified_flight_fail_closed() {
    let state = input(Phase::Held, Down, 0x98a);
    assert!(select(14, state).is_err());
    for slot in [None, Some(0x98b), Some(0x98f)] {
        let mut bad = state;
        bad.reserved_slot = slot;
        assert!(select(12, bad).is_err());
    }
    let mut state = input(Phase::Throwing, Up, 0x98f);
    state.phase_tick = 18;
    state.held_slot = None;
    state.flight = Some(Flight { x: 184, y: 357 });
    for position in [(183, 368), (184, 367), (136, 368)] {
        let mut bad = state;
        bad.position = position;
        assert!(select(12, bad).is_err());
    }
    for facing in [Down, Left, Right] {
        let mut bad = state;
        bad.facing = facing;
        assert!(select(12, bad).is_err());
    }
    for p in [Flight { x: 183, y: 357 }, Flight { x: 184, y: 356 }] {
        let mut bad = state;
        bad.flight = Some(p);
        assert!(select(12, bad).is_err());
    }
    state.phase_tick = 32;
    assert!(select(12, state).is_err());
}

#[test]
fn public_getters_project_explicit_motion_not_stale_active_direction() {
    use room_core::{
        pots::{Admission, Input, PotState, SourceObject},
        Room, WalkingState,
    };
    let mut cells = vec![0; 32 * 32];
    cells[677] = 0x18fa;
    let room = Room::new_passive(32, 32, cells).unwrap();
    let objects = [SourceObject {
        cell: 677,
        raw: 0x18fa,
        replacement: 0xf8,
    }];
    let a = Admission {
        room: &room,
        objects: &objects,
        cellar_up_lanes: false,
        door_hit_enabled: false,
    };
    let mut p = PotState::new(&a, WalkingState::new(104, 352), Left).unwrap();
    let project =
        |p: &PotState| CarryInput::from_state(p, (p.held_slot_in(&a), p.reserved_slot_in(&a)));
    assert!(select(12, project(&p)).unwrap().is_none());
    p.step(
        &a,
        Input {
            action: true,
            direction: None,
        },
    )
    .unwrap();
    for _ in 0..24 {
        p.step(&a, Input::default()).unwrap();
    }
    assert_eq!(p.phase(), Phase::Held);
    p.step(
        &a,
        Input {
            direction: Some(Down),
            action: false,
        },
    )
    .unwrap();
    assert!(select(12, project(&p)).unwrap().unwrap().overlay["pose"]
        .as_str()
        .unwrap()
        .starts_with("walking:"));
    p.step(&a, Input::default()).unwrap();
    assert_eq!(p.walking().active_direction(), Some(Down));
    assert!(select(12, project(&p)).unwrap().unwrap().overlay["pose"]
        .as_str()
        .unwrap()
        .starts_with("standing:"));
}

#[test]
fn authenticated_catalog_is_opt_in_complete_and_requires_source_frames() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    if !path.try_exists().unwrap() {
        eprintln!("SKIP: local Japanese ROM absent");
        return;
    }
    let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
    let old = super::super::compile(&rom).unwrap();
    let art = super::super::compile_profile(&rom, true).unwrap();
    let bundle: Value = serde_json::from_slice(&art.bytes).unwrap();
    let old_bundle: Value = serde_json::from_slice(&old.bytes).unwrap();
    assert!(old_bundle.get("pandora_carry").is_none());
    assert!(old.carry(12, input(Phase::Held, Left, 0x98a)).is_err());
    assert!(art
        .carry(12, input(Phase::Held, Left, 0x98a))
        .unwrap()
        .is_some());
    let catalog = &bundle["pandora_carry"];
    assert_eq!(catalog["poses"].as_object().unwrap().len(), 16);
    assert_eq!(catalog["flight"]["2442"], "pandora:96e1a6:60:0:0");
    assert_eq!(catalog["flight"]["2447"], "pandora:96e1ab:60:0:0");
    let frames = bundle["frames"].as_object().unwrap();
    assert_eq!(compile(frames).unwrap(), *catalog);
    for key in catalog["poses"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|p| std::iter::once(&p["ark"]).chain(p["pots"].as_object().unwrap().values()))
        .chain(catalog["flight"].as_object().unwrap().values())
    {
        let key = key.as_str().unwrap();
        assert!(
            frames[key]["rgba"]
                .as_array()
                .unwrap()
                .iter()
                .skip(3)
                .step_by(4)
                .any(|a| a == 255),
            "nonempty {key}"
        );
        let mut bad = frames.clone();
        bad.remove(key);
        assert!(compile(&bad).is_err());
    }
    if let Some(path) = std::env::var_os("PANDORA_CARRY_EXPORT") {
        std::fs::write(path, &art.bytes).unwrap();
    }
}

#[test]
fn actual_core_lift_walk_throw_getters_remain_admissible_through_recovery() {
    use room_core::{
        pots::{Admission, Input, PotState, SourceObject},
        Room, WalkingState,
    };
    let mut cells = vec![0; 32 * 32];
    cells[675] = 0x18fa;
    cells[20 * 32 + 11] = 0x1d80;
    cells[21 * 32 + 11] = 0x0b81;
    let room = Room::new_passive(32, 32, cells).unwrap();
    let objects = [SourceObject {
        cell: 675,
        raw: 0x18fa,
        replacement: 0xf8,
    }];
    let a = Admission {
        room: &room,
        objects: &objects,
        cellar_up_lanes: true,
        door_hit_enabled: true,
    };
    let mut p = PotState::new(&a, WalkingState::new(40, 352), Right).unwrap();
    let mut phases = [0; 4];
    let mut flight_samples = 0;
    let mut recovery = 0;
    let mut step = |p: &mut PotState, direction, action| {
        p.step(&a, Input { direction, action }).unwrap();
        phases[p.phase() as usize] += 1;
        let input = CarryInput::from_state(p, (p.held_slot_in(&a), p.reserved_slot_in(&a)));
        let selected = select(12, input).unwrap();
        assert_eq!(selected.is_none(), p.phase() == Phase::Empty);
        if let Some(selected) = selected {
            assert_eq!(selected.overlay["held_slot"], json!(p.held_slot_in(&a)));
            assert_eq!(
                selected.overlay["reserved_slot"],
                json!(p.reserved_slot_in(&a))
            );
        }
        flight_samples += usize::from(p.flight().is_some());
        recovery += usize::from(p.phase() == Phase::Throwing && p.reserved_slot_in(&a).is_none());
    };
    step(&mut p, None, true);
    for _ in 0..24 {
        step(&mut p, None, false);
    }
    for (direction, count) in [
        (Some(Down), 12),
        (None, 20),
        (Some(Right), 97),
        (None, 20),
        (Some(Up), 1),
        (None, 20),
    ] {
        for _ in 0..count {
            step(&mut p, direction, false);
        }
    }
    assert_eq!(p.walking().position(), (184, 368));
    step(&mut p, None, true);
    for _ in 0..34 {
        step(&mut p, None, false);
    }
    assert!(phases.iter().all(|n| *n > 0));
    assert_eq!(flight_samples, 4);
    assert_eq!(recovery, 10);
}
