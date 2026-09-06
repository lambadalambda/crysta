//! Optional ordinary-owner comparison against two fresh native input-only runs.
//! The checker authenticates source, paired traces, WRAM and this generated CSV
//! before this test runs. Captures are qualification fixtures, never `GameData`.
use room_core::{Direction, FrameInput, Room, WalkingState};
use std::path::PathBuf;

#[test]
fn selected_native_ordinary_segments_match_every_owned_step() {
    let Some(fixtures) = std::env::var_os("HOUSE_NAVIGATION_FIXTURES") else {
        eprintln!("SKIP: run tools/house-navigation-qualification/replay.sh for private evidence");
        return;
    };
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(fixtures);
    let text = std::fs::read_to_string(fixtures.join("walking.csv")).unwrap();
    let mut current = None;
    let mut count = 0;
    let mut segments = 0;
    for line in text.lines() {
        let fields: Vec<_> = line.split(',').collect();
        match fields[0] {
            "S" => {
                assert_eq!(fields.len(), 6);
                let wram =
                    std::fs::read(fixtures.join(fields[1]).join(format!("{}.wram", fields[2])))
                        .unwrap();
                assert_eq!(wram.len(), 131_072);
                let cells = wram[0xa000..0xb000]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect();
                let room = Room::new_passive(32, 64, cells).unwrap();
                let mut state =
                    WalkingState::new(fields[3].parse().unwrap(), fields[4].parse().unwrap());
                let direction = match fields[5] {
                    "0" => Direction::Down,
                    "1" => Direction::Up,
                    "2" => Direction::Left,
                    "3" => Direction::Right,
                    _ => panic!("invalid direction"),
                };
                let input = FrameInput {
                    direction: Some(direction),
                };
                // Prime the measured setup owner, not native dialogue scheduling.
                state.step(&room, input).unwrap();
                current = Some((room, state, input));
                segments += 1;
            }
            "F" => {
                assert_eq!(fields.len(), 4);
                let (room, state, input) = current.as_mut().unwrap();
                let output = state
                    .step(room, *input)
                    .unwrap_or_else(|e| panic!("frame {}: {e}", fields[1]));
                assert_eq!(
                    (output.x, output.y),
                    (fields[2].parse().unwrap(), fields[3].parse().unwrap()),
                    "frame {}",
                    fields[1]
                );
                count += 1;
            }
            _ => panic!("invalid row"),
        }
    }
    assert_eq!(segments, 11);
    assert_eq!(count, 374);
}
