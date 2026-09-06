//! Admission bounds qualify actual collision samples, not the player's anchor.
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState};

fn grid() -> Room {
    Room::new(8, 8, vec![0; 64]).unwrap()
}

fn admitted() -> Room {
    grid().with_sample_halo([2, 2, 5, 5]).unwrap()
}

fn ready(room: &Room, x: u16, y: u16, direction: Direction) -> WalkingState {
    let mut state = WalkingState::new(x, y);
    let input = FrameInput {
        direction: Some(direction),
    };
    for _ in 0..2 {
        state.step(room, input).unwrap();
    }
    state
}

fn assert_atomic_rejection(room: &Room, mut state: WalkingState) {
    let before = state;
    let snapshot = state.encode_snapshot();
    // Changing input would also change activation history if the step committed.
    let direction = if state.active_direction() == Some(Direction::Up) {
        Direction::Right
    } else {
        Direction::Up
    };
    let input = FrameInput {
        direction: Some(direction),
    };
    for _ in 0..2 {
        assert_eq!(
            state.step(room, input),
            Err(Unqualified::SampleOutsideAdmission)
        );
        assert_eq!(state, before);
        assert_eq!(state.encode_snapshot(), snapshot);
    }
    let mut restored = WalkingState::decode_snapshot(room, &snapshot).unwrap();
    assert_eq!(
        restored.step(room, input),
        Err(Unqualified::SampleOutsideAdmission)
    );
    assert_eq!(restored, before);
    // The same pending movement is ordinary in a full-grid room.
    assert!(state.step(&grid(), input).is_ok());
}

#[test]
fn halos_are_optional_immutable_and_validated_as_half_open_cells() {
    let full = grid();
    assert_eq!(full.sample_halo(), None);
    assert_eq!(
        Room::new_passive(8, 8, vec![0; 64]).unwrap().sample_halo(),
        None
    );
    let halo = full.clone().with_sample_halo([2, 2, 5, 5]).unwrap();
    assert_eq!(halo.sample_halo(), Some([2, 2, 5, 5]));
    assert_eq!(halo.clone(), halo);
    assert_ne!(halo, full);
    assert_eq!(halo.cells(), full.cells());
    assert_eq!((halo.width(), halo.height()), (8, 8));
    assert_eq!(
        halo.with_sample_halo([0, 0, 8, 8]).unwrap().sample_halo(),
        Some([0, 0, 8, 8])
    );
    for bounds in [
        [2, 2, 2, 5],
        [2, 2, 5, 2],
        [5, 2, 2, 5],
        [2, 5, 5, 2],
        [0, 0, 9, 8],
        [0, 0, 8, 9],
        [8, 0, 9, 8],
        [0, 8, 8, 9],
        [0, 0, u16::MAX, u16::MAX],
    ] {
        assert_eq!(
            full.clone().with_sample_halo(bounds),
            Err(Unqualified::RoomDimensions),
            "{bounds:?}"
        );
    }
}

#[test]
fn old_edges_outside_are_rejected_even_when_new_edges_are_inside() {
    let room = admitted();
    for (direction, x, y) in [
        (Direction::Right, 24, 48), // old column 1, new column 2
        (Direction::Left, 88, 48),  // old column 5, new column 4
        (Direction::Down, 40, 32),  // old row 1, new row 2
        (Direction::Up, 40, 96),    // old row 5, new row 4
    ] {
        assert_atomic_rejection(&room, ready(&room, x, y, direction));
    }
}

#[test]
fn new_edges_outside_are_rejected_while_the_anchor_is_inside() {
    let room = admitted();
    for (direction, x, y) in [
        (Direction::Right, 72, 48),
        (Direction::Left, 40, 48),
        (Direction::Up, 40, 48),
    ] {
        assert!((2..5).contains(&(x / 16)) && (2..5).contains(&(y / 16)));
        assert_atomic_rejection(&room, ready(&room, x, y, direction));
    }
    let mut down = ready(&room, 40, 78, Direction::Down);
    down.step(
        &room,
        FrameInput {
            direction: Some(Direction::Down),
        },
    )
    .unwrap();
    assert_eq!(down.position(), (40, 79)); // next two-pixel attempt samples row 5
    assert_atomic_rejection(&room, down);
}

#[test]
fn perpendicular_neighbor_samples_obey_the_exclusive_boundary() {
    let room = admitted();
    for (direction, x, y) in [
        (Direction::Down, 73, 64),  // first column 4, neighbor column 5
        (Direction::Right, 56, 81), // first row 4, neighbor row 5
    ] {
        assert_atomic_rejection(&room, ready(&room, x, y, direction));
    }
    // Alignment samples only the first cell, even if the anchor is outside.
    for (direction, x, y) in [(Direction::Down, 72, 64), (Direction::Right, 56, 80)] {
        let mut state = ready(&room, x, y, direction);
        assert!(
            !state
                .step(
                    &room,
                    FrameInput {
                        direction: Some(direction)
                    }
                )
                .unwrap()
                .blocked
        );
    }
}

#[test]
fn ordinary_down_and_right_match_full_rooms_inside_the_house_halo() {
    for passive in [false, true] {
        let full = if passive {
            Room::new_passive(64, 80, vec![0; 64 * 80]).unwrap()
        } else {
            Room::new(64, 80, vec![0; 64 * 80]).unwrap()
        };
        let room = full.clone().with_sample_halo([29, 47, 36, 53]).unwrap();
        for direction in [Direction::Down, Direction::Right] {
            let mut bounded = WalkingState::new(512, 800);
            let mut unbounded = bounded;
            let input = FrameInput {
                direction: Some(direction),
            };
            for _ in 0..12 {
                let output = bounded.step(&room, input).unwrap();
                assert_eq!(output, unbounded.step(&full, input).unwrap());
                assert!(!output.blocked);
                assert_eq!(bounded, unbounded);
            }
            assert_ne!(bounded.position(), (512, 800));
        }
    }
}

#[test]
fn admission_rejects_before_material_dispatch_instead_of_making_a_wall() {
    for raw in [0, 12 << 9, 6 << 9, 0x8000] {
        let mut cells = vec![0; 64];
        cells[2 * 8 + 5] = raw;
        let room = Room::new_passive(8, 8, cells)
            .unwrap()
            .with_sample_halo([2, 2, 5, 5])
            .unwrap();
        let mut state = ready(&room, 72, 48, Direction::Right);
        let before = state;
        assert_eq!(
            state.step(
                &room,
                FrameInput {
                    direction: Some(Direction::Right)
                }
            ),
            Err(Unqualified::SampleOutsideAdmission)
        );
        assert_eq!(state, before);
    }
}
