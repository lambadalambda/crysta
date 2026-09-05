//! Optional private-reference equality tests. No ROM, WRAM or trajectory data is embedded.
//!
//! Set `ROOM_CORE_FIXTURES` to a complete local/movement directory to require it.
//! Otherwise absent local fixtures skip clearly. SHA-256 uses the host shasum or
//! sha256sum executable only in this native integration test, keeping the crate
//! entirely free of runtime AND dev dependencies.

use room_core::{Direction, FrameInput, Room, WalkingState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Fixture {
    name: &'static str,
    start: u16,
    end: u16,
    grid_frame: u16,
    csv_hash: &'static str,
    wram_hash: &'static str,
}

const FIXTURES: [Fixture; 12] = [
    Fixture {
        name: "wall-Left",
        start: 1601,
        end: 1790,
        grid_frame: 1790,
        csv_hash: "fa59efee34115ac1b9f4d5098132aa1e4a1053749211b350f6ebc3408cbfc2b0",
        wram_hash: "8165e5ccf782c1d42efb1a05555a07d599a9cb8b8070a559584bfce9550c2c03",
    },
    Fixture {
        name: "wall-Right",
        start: 1601,
        end: 1790,
        grid_frame: 1790,
        csv_hash: "02965f1caafe8b3ac5f7999e67785151a4069371662095ee368b4dc7d83222b2",
        wram_hash: "e8d0b02551a3b80178530c273c0c06c98e1b6be4866151fe2ed93083f3b93eaa",
    },
    Fixture {
        name: "wall-Down",
        start: 1601,
        end: 1790,
        grid_frame: 1790,
        csv_hash: "1e9fd523ccf035b34a7115503b1eaf33592ff20aa04c90ead09cf473f0caf697",
        wram_hash: "dbc0405213556df2e1ef57aa4377f927794fdae16b330ce7c48b480451be424b",
    },
    Fixture {
        name: "up-central",
        start: 1601,
        end: 1780,
        grid_frame: 1780,
        csv_hash: "3f5697cc8c28b6c2c0cc1e55c2ebd406f59a256c0910b3271253258f989ebbce",
        wram_hash: "5230f9f179f80b13ee20363ee62b421919af0fba69f618ff138e572d68b4f073",
    },
    Fixture {
        name: "up-type12",
        start: 1601,
        end: 1780,
        grid_frame: 1780,
        csv_hash: "ce8c1461e38955e1b804ec73a1427bbbcd464e1576b05eedc82accf7134048d2",
        wram_hash: "d11c817a2062109b48d91711faa4f14c90f108ca7760c08b5bdb2275a2143760",
    },
    Fixture {
        name: "map10-Down",
        start: 1801,
        end: 2000,
        grid_frame: 2000,
        csv_hash: "6781770d3e6d98e2822657e663d8f28626220c778ba23aad426ae69aaccea112",
        wram_hash: "88a16a629c6e8eaa4865e5f2e1eeb8260651b7011346820d461fbff041749ca5",
    },
    Fixture {
        name: "doorway-approach",
        start: 1601,
        end: 1681,
        grid_frame: 1601,
        csv_hash: "11df3bd5fe315e41f9f894c7937a93dd23c7b60b3c4e78baaddbf10cffc25614",
        wram_hash: "655ea43073c4a4f7f0f1b8d736125c1a7b62ddbe07af912ee958c3eb162f0dd6",
    },
    Fixture {
        name: "map10-Left",
        start: 1801,
        end: 2000,
        grid_frame: 2000,
        csv_hash: "e28d45c62f6d909aea34dca67b6acb7014af0aaa28ac3696664873a7860f0cac",
        wram_hash: "8a9f4b469de87a31cfc3f5e7f904e40555ef5a676f3cb0e3f588c92cb09d3dfb",
    },
    Fixture {
        name: "map10-Right",
        start: 1801,
        end: 2000,
        grid_frame: 2000,
        csv_hash: "0d4fb4d4abfb076cbcdea41b55445508efcaeb0f3437ee5ec22680ebd3284b81",
        wram_hash: "94718533060f61fe50e73634dcb5a4857b9c5bdfa66473337c2e5c5431115c01",
    },
    Fixture {
        name: "map10-up-wall",
        start: 1801,
        end: 2050,
        grid_frame: 2050,
        csv_hash: "9b8b9ff057a72b11302051f298c50b1c1b50e1f95bc7bafb8d6fc2ad8db28d7b",
        wram_hash: "27a10a4a274aa00db6bba6ab9814e5567f475ac779a128403a211ed99198f319",
    },
    Fixture {
        name: "corner-positive",
        start: 1801,
        end: 1861,
        grid_frame: 1861,
        csv_hash: "109df44c44689ac15c266b19852f103a3fb40f17d7f9d9df4e7671ea7bee15f8",
        wram_hash: "112a0602bb001230c6748cd9bdc5ced1becbb0ce0cf71f6d998b3cf53ac5ff0d",
    },
    Fixture {
        name: "corner-negative",
        start: 1801,
        end: 1861,
        grid_frame: 1861,
        csv_hash: "718e668a4154f594eadd83273966423688362632a0f6d205d081db10973b1665",
        wram_hash: "613c1b81fbdbbfe81f46f53c99c4f21cfec253d932caf0c3e4821a54c72d1d55",
    },
];
const F_GRID_HASH: &str = "c5d86aec915b09ec3481d48e903bd1d94a824303f4b4eee24da19acfbf8028e1";
const TEN_GRID_HASH: &str = "261e3b4637587b69465178667f70cddb5eb6d996650f7008c37aefbec5325eed";

fn sha256(bytes: &[u8]) -> String {
    for (executable, args) in [("shasum", &["-a", "256"][..]), ("sha256sum", &[][..])] {
        let process = Command::new(executable)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match process {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => panic!("cannot launch {executable}: {e}"),
        };
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{executable}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned();
    }
    panic!("local fixture verification requires host shasum or sha256sum");
}

fn read_authenticated(path: &Path, digest: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        sha256(&bytes),
        digest,
        "authentication failed: {}",
        path.display()
    );
    bytes
}

fn direction(value: &str) -> Option<Direction> {
    match value {
        "" => None,
        "Left" => Some(Direction::Left),
        "Right" => Some(Direction::Right),
        "Up" => Some(Direction::Up),
        "Down" => Some(Direction::Down),
        _ => panic!("unqualified fixture input: {value}"),
    }
}

fn replay(directory: &Path, fixture: &Fixture) -> usize {
    let directory = directory.join(fixture.name);
    let csv = String::from_utf8(read_authenticated(
        &directory.join("frames.csv"),
        fixture.csv_hash,
    ))
    .unwrap();
    let wram = read_authenticated(
        &directory.join(format!("f{}.wram", fixture.grid_frame)),
        fixture.wram_hash,
    );
    assert_eq!(wram.len(), 0x20000);
    // These are the authenticated 32×64 runtime grids, not a generic WRAM decoder.
    assert_eq!(wram[0x827], 2);
    assert_eq!(&wram[0x862..0x864], &[255, 7]);
    let grid = &wram[0xa000..0xb000];
    assert_eq!(
        sha256(grid),
        if fixture.start == 1801 {
            TEN_GRID_HASH
        } else {
            F_GRID_HASH
        }
    );
    let cells = grid
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    let room = Room::new(32, 64, cells).unwrap();
    let rows: Vec<Vec<&str>> = csv
        .lines()
        .skip(1)
        .map(|line| line.split(',').collect::<Vec<_>>())
        .filter(|r| {
            let frame = r[0].parse::<u16>().unwrap();
            frame >= fixture.start && frame <= fixture.end
        })
        .collect();
    assert_eq!(rows.len(), usize::from(fixture.end - fixture.start) + 1);
    let number = |r: &Vec<&str>, i: usize| r[i].parse::<u16>().unwrap();
    let mut state = WalkingState::new(number(&rows[0], 3), number(&rows[0], 4));
    let mut restored = WalkingState::decode_snapshot(&room, &state.encode_snapshot()).unwrap();
    for (index, r) in rows.iter().enumerate().skip(1) {
        assert_eq!(
            usize::from(number(r, 0)),
            usize::from(fixture.start) + index
        );
        assert_eq!(
            r[2], rows[0][2],
            "transition frames must not enter walking tests"
        );
        assert_eq!(u16::from_str_radix(r[5], 16).unwrap() & 0x0406, 0x0404);
        let input = FrameInput {
            direction: direction(r[1]),
        };
        let before = state.position();
        let out = state
            .step(&room, input)
            .unwrap_or_else(|e| panic!("{} frame {}: {e}", fixture.name, r[0]));
        assert_eq!(
            (out.x, out.y),
            (number(r, 3), number(r, 4)),
            "{} frame {}",
            fixture.name,
            r[0]
        );
        assert_eq!(
            (out.attempted_dx, out.attempted_dy),
            (r[14].parse::<i16>().unwrap(), r[15].parse::<i16>().unwrap())
        );
        assert_eq!(
            (i32::from(out.dx), i32::from(out.dy)),
            (
                i32::from(out.x) - i32::from(before.0),
                i32::from(out.y) - i32::from(before.1)
            )
        );
        // Fixture-specific property, not an independent native carry/dispatch
        // oracle: in general solid correction can equal the attempted result.
        assert_eq!(
            out.blocked,
            (out.dx, out.dy) != (out.attempted_dx, out.attempted_dy)
        );
        assert_eq!(restored.step(&room, input).unwrap(), out);
        assert_eq!(restored.encode_snapshot(), state.encode_snapshot());
        restored = WalkingState::decode_snapshot(&room, &restored.encode_snapshot()).unwrap();
        assert_eq!(restored, state);
    }
    if fixture.name == "doorway-approach" {
        assert_eq!(state.position(), (392, 209));
        assert_eq!(state.phase(), 2);
        assert_eq!(state.active_direction(), Some(Direction::Down));
        assert_eq!(state.delayed_direction(), Some(Direction::Down));
        assert_eq!(
            state.used_direction_mask(),
            Direction::Left.mask() | Direction::Down.mask()
        );
    }
    rows.len() - 1
}

#[test]
fn authenticated_reference_positions_streams_and_snapshots() {
    let explicit = std::env::var_os("ROOM_CORE_FIXTURES");
    let root = explicit.as_ref().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/movement"),
        PathBuf::from,
    );
    if !root.exists() && explicit.is_none() {
        eprintln!("SKIP: optional room-core fixtures absent at {}; set ROOM_CORE_FIXTURES to require them",root.display());
        return;
    }
    assert!(
        root.is_dir(),
        "required fixture directory missing: {}",
        root.display()
    );
    // Also check the host digest command against a standard known answer.
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let total: usize = FIXTURES.iter().map(|fixture| replay(&root, fixture)).sum();
    assert_eq!(total, 1971);
    eprintln!("Matched all {total} reference position/stream steps and restored-snapshot replays");
}
