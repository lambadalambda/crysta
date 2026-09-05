//! Optional authenticated input-admission corpus; no reference data is embedded.
//! Set `ROOM_CORE_ADMISSION_FIXTURES` to the input-admission root to require it.
use room_core::{Direction, FrameInput, Room, Unqualified, WalkingState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// Name, completed end frame, rejected submitted-input CSV row (0 = none), raw CSV SHA-256.
const FIXTURES: [(&str, u16, u16, &str); 42] = [
    (
        "quick",
        1615,
        1606,
        "5b3164bc2b21ea653879d2078f9d3d35f49ba336caa51e60e1d631b99f20bcb9",
    ),
    (
        "boundary-dash",
        1617,
        1612,
        "db4bf3741449c53f7a5cdbec54b71948cd2a4cff3d3f02b4c4931d10eaba148d",
    ),
    (
        "boundary-walk",
        1618,
        0,
        "3a3f79a02a29f659700ba229894bd384a2db85ccee37d32c29537bfe4da5080b",
    ),
    (
        "longhold",
        1640,
        0,
        "f727168a1546e1915b6fd650ef8e2076982b4f3f7c2e7f8122d7a9bb2d8491a0",
    ),
    (
        "reverse",
        1630,
        0,
        "d0bfc215f9345ec41eea5ca47889d45e2c28f851a674e88fed95005d8097ecb1",
    ),
    (
        "rapid-reverse",
        1630,
        0,
        "afe63c989e36d7790d393acceff6f2e3e4da0dc23f01352a09438c2428ea5ad0",
    ),
    (
        "route",
        1810,
        0,
        "75bd1b5a63366d45e938cc03e0c562116bd22c00bba0b922116c10f127827bdd",
    ),
    (
        "trace-dash",
        1612,
        1612,
        "123c58d6b3e54d3a9ccbdb0692c1ab9f27874d84adda4b1df27e0204ee4ab40b",
    ),
    (
        "trace-walk",
        1613,
        0,
        "3f5a02c0f93c60a38eea4e696f0e37d340779948841f438d87304fbd263817f2",
    ),
    (
        "trace-cop61",
        1604,
        0,
        "d44f9e8abb50721dae42923d1184aab87ae9351e349afcd9521ebb2479810275",
    ),
    (
        "Left-1700-10",
        1715,
        1711,
        "7c0ba0f98e51f35bde324fb0fadf915039543b2f6b22a070c9fe252c509e5fba",
    ),
    (
        "Left-1700-11",
        1716,
        0,
        "060e86e5cfffbde2f5144bd3053e2b47fd743fac311d8d73730befc7eb295665",
    ),
    (
        "Left-1701-10",
        1716,
        1712,
        "2cc3464558d20ce42efbe5962f214ef47d2a0e10eccf96cd0acd6823c408c15c",
    ),
    (
        "Left-1701-11",
        1717,
        0,
        "1d29237ad3c2245f10d50f7a41fd20b8a58e3a5e7b3166a8541ed482a8e0cd74",
    ),
    (
        "Left-1702-10",
        1717,
        1713,
        "71c6c69c6faf38015e584021304dd957fe384a052eedcf99c294ce4acdebca59",
    ),
    (
        "Left-1702-11",
        1718,
        0,
        "7949fd917bf8a4f46b863c02cc239c68b73888f0edfcbd6610c45bdcb966af46",
    ),
    (
        "Left-1703-10",
        1718,
        1714,
        "7221d753ddc2d4f096ddc1d7a91c4c33aa5424f019011a2a2c259b401cd5c01a",
    ),
    (
        "Left-1703-11",
        1719,
        0,
        "555d0f33ebb698094cb76ef5217828a0c68d3ef751c514e051091daf4d97770e",
    ),
    (
        "Right-1700-10",
        1715,
        1711,
        "c9bf5b03e3d28635c09ac633b6914ad9313cc6e8b1d3d268bbabb85407738738",
    ),
    (
        "Right-1700-11",
        1716,
        0,
        "35f5372ed5e13375432e7e3939696f39f3f376421f0d34b0f2958649c3265c7d",
    ),
    (
        "Right-1701-10",
        1716,
        1712,
        "2555e67d994a75cc933a5f33c4c9512c72555555c919ae38245d16a08f1e7b37",
    ),
    (
        "Right-1701-11",
        1717,
        0,
        "23253a8fc7f526cf201caf904e9260578c157bc6afff4fe64d16d2ee46fa9739",
    ),
    (
        "Right-1702-10",
        1717,
        1713,
        "aece28ca329065f481a8ef1dd488cd2436b877991785b357f134003ecb520893",
    ),
    (
        "Right-1702-11",
        1718,
        0,
        "638d0f4266d8e03312b4fe15a3d04434d54447cecb9123741c864400e31051d7",
    ),
    (
        "Right-1703-10",
        1718,
        1714,
        "169046f275d29bf0b2f40daf9f494f84ff371dc6aa059944b813381149a22beb",
    ),
    (
        "Right-1703-11",
        1719,
        0,
        "f844313ce601853555ce54071c5b9d6fda1cbc3c6648ba1d2d2840e99dd5ea9d",
    ),
    (
        "Up-1700-10",
        1715,
        1711,
        "274c10c11dd35b8f2e9c61c757ef80d1625f610d8ae758add0bcfc204e173356",
    ),
    (
        "Up-1700-11",
        1716,
        0,
        "ddfe6cdf3f4b910a01d6d08c273bb49f220eb3305b671c856ce80e245beecc10",
    ),
    (
        "Up-1701-10",
        1716,
        1712,
        "ee5ccbc40938bad92877e0253179fca15daf8ec4e90c526c26a923b379a4874e",
    ),
    (
        "Up-1701-11",
        1717,
        0,
        "8e1f27a381e5540c84e03714fb3024325f52fb0de3971b20151b6961fdf99df0",
    ),
    (
        "Up-1702-10",
        1717,
        1713,
        "1d0250215ed494541c5876812cd0700e660a98449b951d212abcb1605e776488",
    ),
    (
        "Up-1702-11",
        1718,
        0,
        "176973cd0369add6f2cf1ff4e600496dff185325bc38ccd7aab6ba0e5a0d774d",
    ),
    (
        "Up-1703-10",
        1718,
        1714,
        "4fcfacfb2a11c15e6b13c9ff28c2b5ba9dac875e6e7882ece7c7be4abdecae04",
    ),
    (
        "Up-1703-11",
        1719,
        0,
        "a84910238150ae5371c3c4935648776c798a2c9f8297b3a650bfb2dff2cac777",
    ),
    (
        "Down-1700-10",
        1715,
        1711,
        "f37cbf4f2ae5c47746edecab00cb18d11d0ca3d882c04f0e166ca22664ed99e6",
    ),
    (
        "Down-1700-11",
        1716,
        0,
        "ea93916037a61da16aa5025fa6b90410bb2ba987b7ffeb3d3179fc4a4611f96e",
    ),
    (
        "Down-1701-10",
        1716,
        1712,
        "eeb997a4a06749ba9be94cd9dfd8faa74533b5168e73aa5dc0a7e5df07475263",
    ),
    (
        "Down-1701-11",
        1717,
        0,
        "0fa172f3a9cb2edef1ce29600715f123a97f0e1fb0873eaf302679ded195711b",
    ),
    (
        "Down-1702-10",
        1717,
        1713,
        "49c0cf99d4231323dab5ad4bbdf14e621541d60c1f17e4410a84370cd8bb3063",
    ),
    (
        "Down-1702-11",
        1718,
        0,
        "c0afef2f4bba24a7a02f4d8cf76628f26ebaff6a7d4a100af78967b0349e11c2",
    ),
    (
        "Down-1703-10",
        1718,
        1714,
        "ff56f7091080458d80884dfa294288fa8c91736d6e29e4d55a81fb97f5d71ef7",
    ),
    (
        "Down-1703-11",
        1719,
        0,
        "12e49521359eb8e9cfc4e8cb18cfce93e1af48acd431f4a964cf4c4f81c98336",
    ),
];

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
    panic!("private fixtures require host shasum or sha256sum");
}

fn authenticated(path: &Path, hash: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(sha256(&bytes), hash, "{}", path.display());
    bytes
}

fn direction(value: &str) -> Option<Direction> {
    match value {
        "" => None,
        "Left" => Some(Direction::Left),
        "Right" => Some(Direction::Right),
        "Up" => Some(Direction::Up),
        "Down" => Some(Direction::Down),
        _ => panic!("unsupported fixture input {value}"),
    }
}

#[test]
#[allow(clippy::too_many_lines)] // Keep the per-frame evidence assertions together.
fn authenticated_reactivation_streams_positions_rejections_and_snapshots() {
    let explicit = std::env::var_os("ROOM_CORE_ADMISSION_FIXTURES");
    let fixture_root = explicit.as_ref().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/input-admission"),
        PathBuf::from,
    );
    if !fixture_root.exists() && explicit.is_none() {
        eprintln!("SKIP: optional input-admission fixtures absent; set ROOM_CORE_ADMISSION_FIXTURES to require them");
        return;
    }
    assert!(
        fixture_root.is_dir(),
        "required fixture root absent: {}",
        fixture_root.display()
    );
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let mut total = 0;
    let mut rejections = 0;
    for (name, end, reject_row, hash) in FIXTURES {
        let first = fixture_root.join("first").join(name);
        let second = fixture_root.join("second").join(name);
        let csv = authenticated(&first.join("frames.csv"), hash);
        assert_eq!(csv, authenticated(&second.join("frames.csv"), hash));
        let grid_hash = "655ea43073c4a4f7f0f1b8d736125c1a7b62ddbe07af912ee958c3eb162f0dd6";
        let wram = authenticated(&first.join("f1601.wram"), grid_hash);
        assert_eq!(wram, authenticated(&second.join("f1601.wram"), grid_hash));
        let room = Room::new(
            32,
            64,
            wram[0xa000..0xb000]
                .chunks_exact(2)
                .map(|cell| u16::from_le_bytes([cell[0], cell[1]]))
                .collect(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();
        let rows: Vec<Vec<&str>> = csv
            .lines()
            .skip(1)
            .map(|line| line.split(',').collect())
            .collect();
        assert_eq!(rows.len(), usize::from(end - 1601) + 1);
        assert_eq!(&rows[0][..5], &["1601", "", "f", "472", "176"]);
        let mut state = WalkingState::new(472, 176);
        let mut restored = state;
        let mut rejected = false;
        for (index, row) in rows.iter().enumerate().skip(1) {
            let frame = row[0].parse::<u16>().unwrap();
            assert_eq!(usize::from(frame), 1601 + index);
            let input = FrameInput {
                direction: direction(row[1]),
            };
            let before = state;
            let result = state.step(&room, input);
            assert_eq!(result, restored.step(&room, input), "{name} frame {frame}");
            if frame == reject_row {
                assert_eq!(
                    result,
                    Err(Unqualified::AcceleratedTrigger(input.direction.unwrap())),
                    "{name}"
                );
                assert_eq!(state, before);
                assert_eq!(restored, before);
                rejected = true;
                rejections += 1;
                break; // The original continues into dash; production deliberately does not.
            }
            let output = result.unwrap_or_else(|error| panic!("{name} frame {frame}: {error}"));
            assert_eq!(row[2], "f");
            assert_eq!(u16::from_str_radix(row[5], 16).unwrap() & 0x0406, 0x0404);
            assert_eq!(
                (output.x, output.y),
                (row[3].parse().unwrap(), row[4].parse().unwrap()),
                "{name} frame {frame}"
            );
            assert_eq!(
                (output.attempted_dx, output.attempted_dy),
                (row[14].parse().unwrap(), row[15].parse().unwrap()),
                "{name} frame {frame}"
            );
            assert_eq!(
                (i32::from(output.dx), i32::from(output.dy)),
                (
                    i32::from(output.x) - i32::from(before.x()),
                    i32::from(output.y) - i32::from(before.y())
                )
            );
            assert_eq!(state.encode_snapshot(), restored.encode_snapshot());
            restored = WalkingState::decode_snapshot(&room, &restored.encode_snapshot()).unwrap();
            assert_eq!(restored, state);
            total += 1;
        }
        assert_eq!(rejected, reject_row != 0, "{name}");
        if name == "route" {
            assert_eq!(state.position(), (456, 176));
            assert_eq!(state.last_activation_direction(), Some(Direction::Right));
            assert_eq!(state.onset_remaining(), 0);
        }
    }
    assert_eq!(total, 3994);
    assert_eq!(rejections, 19);
    eprintln!("Matched {total} ordinary transitions and {rejections} atomic accelerated-trigger rejections, with snapshot replay");
}
