//! Finite raw-material policy admission and delayed collision integration.
use room_core::{
    Direction, FrameInput, MaterialAlias, MaterialPolicyError, MaterialRule, Room, Unqualified,
    WalkingState,
};

const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

fn rule(alias: MaterialAlias, bounds: [u16; 4], direction: Option<Direction>) -> MaterialRule {
    MaterialRule {
        alias,
        bounds,
        direction,
    }
}

fn cell_rule(alias: MaterialAlias, col: u16, row: u16) -> MaterialRule {
    rule(alias, [col, row, col + 1, row + 1], Some(Direction::Up))
}

fn grid(col: u16, row: u16, raw: u16, passive: bool) -> Room {
    let mut cells = vec![0; 64 * 64];
    cells[usize::from(row) * 64 + usize::from(col)] = raw;
    if passive {
        Room::new_passive(64, 64, cells)
    } else {
        Room::new(64, 64, cells)
    }
    .unwrap()
}

fn anchor(col: u16, row: u16, direction: Direction) -> (u16, u16) {
    match direction {
        Direction::Up => (col * 16 + 8, row * 16 + 24),
        Direction::Down => (col * 16 + 8, row * 16 + 8),
        Direction::Left => (col * 16 + 16, row * 16 + 16),
        Direction::Right => (col * 16, row * 16 + 16),
    }
}

fn ready(room: &Room, x: u16, y: u16, direction: Direction) -> WalkingState {
    let mut state = WalkingState::new(x, y);
    for _ in 0..2 {
        state
            .step(
                room,
                FrameInput {
                    direction: Some(direction),
                },
            )
            .unwrap();
    }
    state
}

fn rejected(room: &Room, mut state: WalkingState, error: Unqualified) {
    let before = state;
    let snapshot = state.encode_snapshot();
    // Even new activation/history must roll back on rejected delayed movement.
    let input = FrameInput {
        direction: Some(Direction::Right),
    };
    for _ in 0..2 {
        assert_eq!(state.step(room, input), Err(error));
        assert_eq!(state, before);
        assert_eq!(state.encode_snapshot(), snapshot);
    }
    let mut restored = WalkingState::decode_snapshot(room, &snapshot).unwrap();
    assert_eq!(restored.step(room, input), Err(error));
    assert_eq!(restored, before);
}

#[test]
fn policy_is_opt_in_raw_immutable_and_replaceable() {
    let raw = grid(11, 21, (29 << 9) | 0xcb, false);
    assert!(raw.material_policy().is_empty());
    let rules = vec![
        cell_rule(MaterialAlias::ClosedDoorPartial5, 11, 21),
        cell_rule(MaterialAlias::StairOpen29, 11, 21),
    ];
    let room = raw.clone().with_material_policy(rules.clone()).unwrap();
    assert_eq!(room.material_policy(), rules);
    assert_eq!(room.cells(), raw.cells());
    assert_eq!(room.clone(), room);
    assert_ne!(room, raw);
    assert_eq!(room.with_material_policy(vec![]).unwrap(), raw);
    rejected(
        &raw,
        ready(&raw, 184, 360, Direction::Up),
        Unqualified::UnsupportedType(29),
    );
}

#[test]
fn policy_validates_extent_halo_shape_direction_and_overlap() {
    use MaterialAlias::{ClosedDoorPartial5, StairOpen29, TownSolid25};
    let raw = grid(11, 21, 0, false);
    for bounds in [
        [1, 1, 1, 2],
        [2, 1, 1, 2],
        [1, 2, 2, 1],
        [0, 0, 65, 64],
        [0, 0, 64, 65],
        [0, 0, u16::MAX, u16::MAX],
    ] {
        assert_eq!(
            raw.clone()
                .with_material_policy(vec![rule(TownSolid25, bounds, None)]),
            Err(MaterialPolicyError::Bounds)
        );
    }
    for alias in [ClosedDoorPartial5, StairOpen29] {
        for direction in [
            None,
            Some(Direction::Down),
            Some(Direction::Left),
            Some(Direction::Right),
        ] {
            assert_eq!(
                raw.clone()
                    .with_material_policy(vec![rule(alias, [11, 21, 12, 22], direction)]),
                Err(MaterialPolicyError::Scope)
            );
        }
        for bounds in [
            [10, 21, 11, 22],
            [11, 20, 12, 21],
            [11, 21, 13, 22],
            [11, 21, 12, 23],
        ] {
            assert_eq!(
                raw.clone()
                    .with_material_policy(vec![rule(alias, bounds, Some(Direction::Up))]),
                Err(MaterialPolicyError::Scope)
            );
        }
    }
    for (col, row) in [(6, 53), (22, 53)] {
        assert_eq!(
            raw.clone()
                .with_material_policy(vec![cell_rule(ClosedDoorPartial5, col, row)]),
            Err(MaterialPolicyError::Scope)
        );
        assert!(raw
            .clone()
            .with_material_policy(vec![cell_rule(StairOpen29, col, row)])
            .is_ok());
    }
    let town = rule(TownSolid25, [21, 16, 36, 53], None);
    let bounded = raw.clone().with_sample_halo([21, 16, 36, 53]).unwrap();
    assert!(bounded.clone().with_material_policy(vec![town]).is_ok());
    assert_eq!(
        bounded.with_material_policy(vec![rule(TownSolid25, [20, 16, 36, 53], None)]),
        Err(MaterialPolicyError::Bounds)
    );
    let room = raw.clone().with_material_policy(vec![town]).unwrap();
    assert_eq!(
        room.clone().with_sample_halo([22, 16, 36, 53]),
        Err(Unqualified::RoomDimensions)
    );
    assert!(room.with_sample_halo([20, 15, 37, 54]).is_ok());
    for other in [
        town,
        rule(TownSolid25, [35, 52, 37, 54], Some(Direction::Up)),
    ] {
        assert_eq!(
            raw.clone().with_material_policy(vec![town, other]),
            Err(MaterialPolicyError::Overlap)
        );
    }
    let up = rule(TownSolid25, town.bounds, Some(Direction::Up));
    let down = rule(TownSolid25, town.bounds, Some(Direction::Down));
    assert!(raw.clone().with_material_policy(vec![up, down]).is_ok());
    assert!(raw
        .with_material_policy(vec![town, rule(TownSolid25, [36, 16, 37, 53], None)])
        .is_ok());
}

#[test]
fn town25_is_solid_only_in_its_bounded_direction_scope() {
    for direction in DIRECTIONS {
        let raw = grid(28, 25, (25 << 9) | 0x123, false);
        let room = raw
            .clone()
            .with_material_policy(vec![rule(
                MaterialAlias::TownSolid25,
                [21, 16, 36, 53],
                None,
            )])
            .unwrap();
        let solid = grid(28, 25, 12 << 9, false);
        let (x, y) = anchor(28, 25, direction);
        let mut state = ready(&room, x, y, direction);
        let mut reference = ready(&solid, x, y, direction);
        let input = FrameInput {
            direction: Some(direction),
        };
        assert_eq!(state.step(&room, input), reference.step(&solid, input));
        assert_eq!(state, reference);
        rejected(
            &raw,
            ready(&raw, x, y, direction),
            Unqualified::UnsupportedType(25),
        );
    }
    for bounds in [[29, 25, 30, 26], [28, 26, 29, 27]] {
        let room = grid(28, 25, 25 << 9, false)
            .with_material_policy(vec![rule(MaterialAlias::TownSolid25, bounds, None)])
            .unwrap();
        rejected(
            &room,
            ready(&room, 456, 424, Direction::Up),
            Unqualified::UnsupportedType(25),
        );
    }
    let room = grid(28, 25, 25 << 9, false)
        .with_material_policy(vec![rule(
            MaterialAlias::TownSolid25,
            [28, 25, 29, 26],
            Some(Direction::Down),
        )])
        .unwrap();
    rejected(
        &room,
        ready(&room, 456, 424, Direction::Up),
        Unqualified::UnsupportedType(25),
    );
}

#[test]
fn exact_up_aliases_use_delayed_not_submitted_direction() {
    for (alias, kind, reference_kind, col, row) in [
        (MaterialAlias::ClosedDoorPartial5, 5, 16, 11, 21),
        (MaterialAlias::StairOpen29, 29, 0, 11, 21),
        (MaterialAlias::StairOpen29, 29, 0, 6, 53),
        (MaterialAlias::StairOpen29, 29, 0, 22, 53),
    ] {
        let room = grid(col, row, (kind << 9) | 0x181, false)
            .with_material_policy(vec![cell_rule(alias, col, row)])
            .unwrap();
        let reference = grid(col, row, reference_kind << 9, false);
        let (x, y) = (col * 16 + 8, row * 16 + 24);
        for submitted in [None, Some(Direction::Right), Some(Direction::Up)] {
            let mut state = ready(&room, x, y, Direction::Up);
            let mut expected = ready(&reference, x, y, Direction::Up);
            let input = FrameInput {
                direction: submitted,
            };
            let output = state.step(&room, input).unwrap();
            assert_eq!(output, expected.step(&reference, input).unwrap());
            assert_eq!(output.attempted_dy, -1);
            assert_eq!(output.blocked, kind == 5);
            assert_eq!(state, expected);
        }
        for direction in [Direction::Down, Direction::Left, Direction::Right] {
            let (x, y) = anchor(col, row, direction);
            let mut state = ready(&room, x, y, direction);
            let before = state;
            assert_eq!(
                state.step(
                    &room,
                    FrameInput {
                        direction: Some(Direction::Up)
                    }
                ),
                Err(Unqualified::UnsupportedType(u8::try_from(kind).unwrap()))
            );
            assert_eq!(state, before);
        }
        // A permitted rule cannot admit the same material in a neighboring cell.
        let elsewhere = grid(col + 1, row, kind << 9, false)
            .with_material_policy(vec![cell_rule(alias, col, row)])
            .unwrap();
        rejected(
            &elsewhere,
            ready(&elsewhere, x + 16, y, Direction::Up),
            Unqualified::UnsupportedType(u8::try_from(kind).unwrap()),
        );
    }
}

#[test]
fn unknown_second_sample_and_halo_fail_atomically_after_admitted_first() {
    let rules = vec![cell_rule(MaterialAlias::StairOpen29, 11, 21)];
    let mut cells = vec![0; 64 * 64];
    cells[21 * 64 + 11] = 29 << 9;
    cells[21 * 64 + 12] = 29 << 9;
    let room = Room::new(64, 64, cells)
        .unwrap()
        .with_material_policy(rules.clone())
        .unwrap();
    rejected(
        &room,
        ready(&room, 185, 360, Direction::Up),
        Unqualified::UnsupportedType(29),
    );
    let bounded = grid(11, 21, 29 << 9, false)
        .with_sample_halo([11, 21, 12, 22])
        .unwrap()
        .with_material_policy(rules)
        .unwrap();
    rejected(
        &bounded,
        ready(&bounded, 185, 360, Direction::Up),
        Unqualified::SampleOutsideAdmission,
    );
}

#[test]
fn passive_bit15_overrides_aliases_but_not_old_slopes() {
    for kind in [5, 29, 6, 7, 31] {
        let rules = vec![
            cell_rule(MaterialAlias::ClosedDoorPartial5, 11, 21),
            cell_rule(MaterialAlias::StairOpen29, 11, 21),
        ];
        let raw = 0x8000 | (kind << 9) | 0x181;
        let room = grid(11, 21, raw, true)
            .with_material_policy(rules.clone())
            .unwrap();
        if matches!(kind, 6 | 7) {
            rejected(
                &room,
                ready(&room, 184, 360, Direction::Up),
                Unqualified::UnsupportedType(u8::try_from(kind).unwrap()),
            );
        } else {
            let mut state = ready(&room, 184, 360, Direction::Up);
            assert!(state.step(&room, FrameInput::default()).unwrap().blocked);
        }
        let strict = grid(11, 21, raw, false)
            .with_material_policy(rules)
            .unwrap();
        let error = if matches!(kind, 6 | 7) {
            Unqualified::UnsupportedType(u8::try_from(kind).unwrap())
        } else {
            Unqualified::FlaggedCell(raw)
        };
        rejected(&strict, ready(&strict, 184, 360, Direction::Up), error);
        // New-edge-only flagged slopes are solid, not a stored-slope diversion.
        let mut state = ready(&room, 184, 368, Direction::Up);
        assert!(state.step(&room, FrameInput::default()).unwrap().blocked);
        assert_eq!(room.cells()[21 * 64 + 11], raw);
    }
}

#[test]
fn closed5_retains_partial_solid_corner_nudges() {
    // P/S nudges -1; S/P nudges +1. A solid alias would incorrectly stay still.
    for (col, x, nudge) in [(12, 185, -1), (10, 183, 1)] {
        let mut cells = vec![0; 64 * 64];
        cells[21 * 64 + 11] = 0x0b81;
        cells[21 * 64 + col] = 12 << 9;
        let room = Room::new(64, 64, cells)
            .unwrap()
            .with_material_policy(vec![cell_rule(MaterialAlias::ClosedDoorPartial5, 11, 21)])
            .unwrap();
        let mut state = ready(&room, x, 360, Direction::Up);
        let out = state.step(&room, FrameInput::default()).unwrap();
        assert_eq!((out.dx, out.dy, out.blocked), (nudge, 0, true));
    }
}

#[test]
fn unflagged_new_edge_and_unrelated_types_still_fail_closed() {
    let rules = vec![
        cell_rule(MaterialAlias::ClosedDoorPartial5, 11, 21),
        cell_rule(MaterialAlias::StairOpen29, 11, 21),
    ];
    for kind in [1, 6, 7, 25, 31] {
        let room = grid(11, 21, kind << 9, false)
            .with_material_policy(rules.clone())
            .unwrap();
        for y in [360, 368] {
            rejected(
                &room,
                ready(&room, 184, y, Direction::Up),
                Unqualified::UnsupportedType(u8::try_from(kind).unwrap()),
            );
        }
    }
    // Alias scope is checked on tentative edges too, not merely old edges.
    let room = grid(11, 21, 29 << 9, false)
        .with_material_policy(rules)
        .unwrap();
    let mut state = ready(&room, 168, 352, Direction::Right); // old175 open, tentative176 type29
    let before = state;
    assert_eq!(
        state.step(
            &room,
            FrameInput {
                direction: Some(Direction::Up)
            }
        ),
        Err(Unqualified::UnsupportedType(29))
    );
    assert_eq!(state, before);
}
