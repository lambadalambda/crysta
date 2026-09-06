//! Optional exact-original contiguous native pot segments; private grids are TEST
//! collision oracles only, never a production initializer. Set `PANDORA_POT_FIXTURES`.
use room_core::pots::{Admission, Input, Phase, PotState, SourceObject};
use room_core::{Direction, Room, WalkingState};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn read_pin(path: &Path, pin: &str) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let mut child = Command::new("shasum")
        .args(["-a", "256"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("local native test requires host shasum");
    child.stdin.take().unwrap().write_all(&data).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap(),
        pin
    );
    data
}
fn direction(d: u32) -> Direction {
    match d {
        0 => Direction::Down,
        1 => Direction::Up,
        2 => Direction::Left,
        3 => Direction::Right,
        _ => panic!("bad direction"),
    }
}
#[test]
#[allow(clippy::too_many_lines)] // Keep each authenticated replay's per-frame assertions together.
fn original_lift_carry_release_flight_and_two_contacts_without_teleports() {
    let Ok(fixture_dir) = std::env::var("PANDORA_POT_FIXTURES") else {
        eprintln!("SKIP private native pots: set PANDORA_POT_FIXTURES");
        return;
    };
    let fixture_dir = Path::new(&fixture_dir);
    let mut total_hits = 0;
    for (name, csv_pin, grid_pin, expected_hit, expected_slot) in [
        (
            "miss",
            "56a830ecf961a9204c234d9d07bffe4bfb56fc0f82e4ce736601201ec1a767d7",
            "f674e74afa472a9e8c5049b6bf9ada2b7b487e921d3ff0dbffa12cfed0240fe7",
            0,
            0x98a,
        ),
        (
            "fa-hit",
            "10f134d96e5422dd8d7b954454cca477938f7056a9a7d890ffd90e1c51ba5dea",
            "0cc8b84a1b3a83680683d478ee3b24dfc2d2fcba57b72ec79a3c9b6c2cb73c89",
            20778,
            0x98a,
        ),
        (
            "fb-hit",
            "6b3e9d255e9d6d24478b56dfc8fb5c8ed3ab56bac9c20caa034c646a1b0c5f07",
            "3db2b37535c4124413b74ccf6b0672d1e1d51c7ec9df2b300fc8c542be200357",
            21985,
            0x98f,
        ),
    ] {
        let grid = read_pin(&fixture_dir.join(format!("{name}.grid")), grid_pin);
        let cells: Vec<u16> = grid
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let objects: Vec<_> = cells
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(**r, 0x18fa | 0x18fb))
            .map(|(cell, &raw)| SourceObject {
                cell: u16::try_from(cell).unwrap(),
                raw,
                replacement: 0xf8,
            })
            .collect();
        let room = Room::new_passive(32, 64, cells)
            .unwrap()
            .with_sample_halo([1, 19, 15, 30])
            .unwrap();
        let admission = Admission {
            room: &room,
            objects: &objects,
            cellar_up_lanes: true,
            door_hit_enabled: true,
        };
        let csv =
            String::from_utf8(read_pin(&fixture_dir.join(format!("{name}.csv")), csv_pin)).unwrap();
        let mut rows = csv.lines().map(|line| {
            line.split(',')
                .map(|s| s.parse::<u32>().unwrap())
                .collect::<Vec<_>>()
        });
        let start = rows.next().unwrap();
        let mut state = PotState::new(
            &admission,
            WalkingState::new(
                u16::try_from(start[0]).unwrap(),
                u16::try_from(start[1]).unwrap(),
            ),
            direction(start[2]),
        )
        .unwrap();
        let (mut hits, mut consumes, mut releases, mut flights, mut recovery) = (0, 0, 0, 0, 0);
        for row in rows {
            let frame = row[0];
            let input = Input {
                direction: (row[1] < 4).then(|| direction(row[1])),
                action: row[1] == 4,
            };
            // Continuation from canonical snapshots at EVERY native frame, not
            // native emulator restores or checkpoint position corrections.
            let mut restored =
                PotState::decode_snapshot(&admission, &state.encode_snapshot()).unwrap();
            let out = state
                .step(&admission, input)
                .unwrap_or_else(|e| panic!("{name} frame{frame}: {e}"));
            assert_eq!(restored.step(&admission, input).unwrap(), out);
            assert_eq!(restored.encode_snapshot(), state.encode_snapshot());
            assert_eq!(
                state.position(),
                (
                    u16::try_from(row[2]).unwrap(),
                    u16::try_from(row[3]).unwrap()
                ),
                "{name} frame{frame}"
            );
            assert_eq!(state.facing() as u32, row[4], "{name} frame{frame} facing");
            assert_eq!(
                u32::from(out.control),
                row[5],
                "{name} frame{frame} control"
            );
            let script = match state.phase() {
                Phase::Empty => 0x0084_a258,
                Phase::Lifting => {
                    if state.phase_tick() < 22 {
                        0x0084_be9d
                    } else {
                        0x0084_bea1
                    }
                }
                Phase::Held => {
                    if out.control == 0 {
                        [0x0084_b4cd, 0x0084_b4df, 0x0084_b4fc, 0x0084_b4fc]
                            [state.facing() as usize]
                    } else {
                        [0x0084_b50d, 0x0084_b51e, 0x0084_b533, 0x0084_b533]
                            [state.facing() as usize]
                    }
                }
                Phase::Throwing => match state.phase_tick() {
                    0..=29 => 0x0084_b558,
                    30 => 0x0084_b55c,
                    31 => 0x0084_a318,
                    _ => unreachable!(),
                },
            };
            assert_eq!(row[6], script, "{name} frame{frame} presentation boundary");
            if state.phase() == Phase::Held {
                assert_eq!(state.held_slot_in(&admission), Some(expected_slot));
            }
            if state.phase() == Phase::Throwing && state.phase_tick() >= 18 {
                // Source STZ0988 executes in the native break script, not release.
                assert_eq!(
                    state.reserved_slot_in(&admission).is_some(),
                    row[9] == 0x0084_c721
                );
                assert_eq!(state.held_slot_in(&admission), None);
            }
            if let Some(flight) = out.flight {
                assert_eq!(
                    (u32::from(flight.x), u32::from(flight.y)),
                    (row[7], row[8]),
                    "{name} frame{frame} flight"
                );
                assert_eq!(row[9], 0x0084_c721);
                flights += 1;
            }
            assert_eq!(
                out.flight.is_some(),
                row[9] == 0x0084_c721,
                "{name} frame{frame} flight admission"
            );
            if out.door_hit {
                assert_eq!(frame, expected_hit);
                assert_eq!(
                    row[11],
                    if name == "fa-hit" {
                        0x0088_abac
                    } else {
                        0x0088_abde
                    }
                );
                hits += 1;
            }
            if out.consumed_cell.is_some() {
                consumes += 1;
            }
            if out.held_changed == Some(None) {
                releases += 1;
            }
            if out.control_restored {
                recovery += 1;
                assert_eq!(row[6], 0x0084_a258);
            }
        }
        assert_eq!((consumes, releases, recovery), (1, 1, 1));
        assert_eq!(flights, if expected_hit == 0 { 9 } else { 4 });
        assert_eq!(hits, usize::from(expected_hit != 0));
        total_hits += hits;
    }
    assert_eq!(total_hits, 2);
}
