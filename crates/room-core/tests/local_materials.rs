//! Optional passive-material evidence; no reference data is embedded.
//! Set `ROOM_CORE_MATERIAL_FIXTURES` to the fresh house-materials capture root.
use room_core::{Direction, FrameInput, Room, WalkingState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn sha256(bytes: &[u8]) -> String {
    for (command, args) in [("shasum", &["-a", "256"][..]), ("sha256sum", &[][..])] {
        let process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();
        let mut child = match process {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("cannot launch {command}: {error}"),
        };
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        return String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned();
    }
    panic!("private fixtures require shasum or sha256sum");
}

fn authenticated_pair(root: &Path, name: &str, file: &str, hash: &str) -> Vec<u8> {
    let first = std::fs::read(root.join("first").join(name).join(file)).unwrap();
    assert_eq!(sha256(&first), hash, "{name}/{file}");
    let second = std::fs::read(root.join("second").join(name).join(file)).unwrap();
    assert_eq!(first, second, "{name}/{file}");
    first
}

#[test]
#[allow(clippy::too_many_lines)] // Keep each authenticated replay's checks together.
fn passive_material_routes_have_no_excluded_frames() {
    let explicit = std::env::var_os("ROOM_CORE_MATERIAL_FIXTURES");
    let fixture_root = explicit.as_ref().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/house-materials"),
        PathBuf::from,
    );
    if !fixture_root.exists() && explicit.is_none() {
        eprintln!("SKIP: optional house-material fixtures absent; set ROOM_CORE_MATERIAL_FIXTURES to require them");
        return;
    }
    assert!(fixture_root.is_dir());
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let mut matched = 0;
    let mut partial_nudges = 0;
    // name, full capture end, qualification end, CSV, final WRAM, per-frame hook CSV.
    for (name, capture_end, end, csv_hash, wram_hash, hooks_hash) in [
        (
            "wall-Up",
            1790,
            1790,
            "12cdc28567f70e47fe5f76036a0658bd5700898a3f3ee5341a142a516e8bbb9b",
            "8290e49fcf5e7d2dbd7d5121df6e02ea4d2005dffdb8f9141358e33eab7ce47f",
            "c2f176bfaeaffe91a562af0131e7618072f07882bdd857442819416e9f347a13",
        ),
        (
            "cadence",
            1670,
            1650,
            "f892cb3dd8c29412ad2d21f43707583121fbfc87c07c9cd16798ccbedb6d39d5",
            "19f30fa29e888570bcacb199716c7f21287c5ce403366ba40ba3d99d13f912c4",
            "f74236f8602a5102a3e692de0476da51a262689b44c3fea1bb5ed3859c618dfa",
        ),
    ] {
        let csv = String::from_utf8(authenticated_pair(
            &fixture_root,
            name,
            "frames.csv",
            csv_hash,
        ))
        .unwrap();
        let wram = authenticated_pair(
            &fixture_root,
            name,
            &format!("f{capture_end}.wram"),
            wram_hash,
        );
        let hooks = String::from_utf8(authenticated_pair(
            &fixture_root,
            name,
            "hooks.csv",
            hooks_hash,
        ))
        .unwrap();
        let room = Room::new_passive(
            32,
            64,
            wram[0xa000..0xb000]
                .chunks_exact(2)
                .map(|cell| u16::from_le_bytes([cell[0], cell[1]]))
                .collect(),
        )
        .unwrap();
        let directional_room = room.clone().with_passive_directional_collision();
        let rows: Vec<Vec<&str>> = csv
            .lines()
            .skip(1)
            .map(|line| line.split(',').collect())
            .collect();
        let hook_rows: Vec<Vec<&str>> = hooks
            .lines()
            .skip(1)
            .map(|line| line.split(',').collect())
            .collect();
        assert_eq!(rows.len(), capture_end - 1601 + 1);
        assert_eq!(hook_rows.len(), rows.len());
        assert_eq!(&rows[0][..5], &["1601", "", "f", "472", "176"]);
        let mut state = WalkingState::new(472, 176);
        let mut restored = state;
        // Advance independently through exactly the same admitted rows; do not
        // reset candidate history or position from the baseline/native outputs.
        let mut directional = WalkingState::new(472, 176);
        let mut directional_restored = directional;
        for (index, row) in rows.iter().enumerate().take(end - 1601 + 1).skip(1) {
            let frame = row[0].parse::<usize>().unwrap();
            assert_eq!(frame, 1601 + index);
            assert_eq!(hook_rows[index][0], row[0]);
            assert_eq!(u16::from_str_radix(hook_rows[index][1], 16).unwrap(), 0);
            assert_eq!(
                u16::from_str_radix(hook_rows[index][2], 16).unwrap() & 0x50,
                0
            );
            assert_eq!(row[2], "f");
            assert_eq!(u16::from_str_radix(row[5], 16).unwrap() & 0x0406, 0x0404);
            let input = FrameInput {
                direction: match row[1] {
                    "" => None,
                    "Up" => Some(Direction::Up),
                    "Down" => Some(Direction::Down),
                    "Left" => Some(Direction::Left),
                    "Right" => Some(Direction::Right),
                    other => panic!("unsupported fixture input {other}"),
                },
            };
            let out = state
                .step(&room, input)
                .unwrap_or_else(|error| panic!("{name} frame {frame}: {error}"));
            assert_eq!(
                (out.x, out.y),
                (row[3].parse().unwrap(), row[4].parse().unwrap()),
                "{name} frame {frame}"
            );
            assert_eq!(
                (out.attempted_dx, out.attempted_dy),
                (row[14].parse().unwrap(), row[15].parse().unwrap())
            );
            assert_eq!(restored.step(&room, input).unwrap(), out);
            assert_eq!(restored.encode_snapshot(), state.encode_snapshot());
            restored = WalkingState::decode_snapshot(&room, &restored.encode_snapshot()).unwrap();
            let directional_out = directional
                .step(&directional_room, input)
                .unwrap_or_else(|error| panic!("{name} directional frame {frame}: {error}"));
            // Includes native-checked XY/attempts and the baseline's P/S nudges.
            assert_eq!(directional_out, out, "{name} directional frame {frame}");
            assert_eq!(directional.encode_snapshot(), state.encode_snapshot());
            assert_eq!(
                directional_restored.step(&directional_room, input).unwrap(),
                directional_out
            );
            directional_restored = WalkingState::decode_snapshot(
                &directional_room,
                &directional_restored.encode_snapshot(),
            )
            .unwrap();
            assert_eq!(directional_restored, directional);
            if name == "wall-Up" {
                assert_eq!((out.x, out.y), (472, 176));
            }
            if name == "cadence" && (1624..=1626).contains(&frame) {
                assert_eq!((out.dx, out.dy, out.blocked), (-1, 0, true));
                partial_nudges += 1;
            }
            matched += 1;
        }
    }
    assert_eq!((matched, partial_nudges), (238, 3));
    eprintln!("Matched {matched} material transitions, including {partial_nudges} P/S nudges, for both default and directional rooms; zero exclusions");
}
