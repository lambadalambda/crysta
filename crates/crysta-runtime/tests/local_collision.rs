//! Private input-only reference replays for the directional collision extension.
//! Fixtures contain live layers captured immediately before $80:D107, not ROM
//! data embedded in tests. Missing defaults skip; an explicit root is mandatory.
use room_core::{Direction, FrameInput, Room, WalkingState};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

const CASES: &[(&str, &str, usize)] = &[
    (
        "motion-Right",
        "149503d616e5f6604b5362f2a44b0b1c46f0917a9a3fdd0c3ecf8d6336db3f92",
        40,
    ),
    (
        "motion-Left",
        "4a54726c603c9d5d50fdf27265ab231ba53f9003859073ffdc41e63b3505b84a",
        40,
    ),
    (
        "motion-Right-vertical",
        "a8b5ac3e4a82dcd7c88aa6ee9486ee1d4b2f42b4ff22be665defd03462d86736",
        236,
    ),
    (
        "motion-Left-vertical",
        "4d90d2fce33cf00107be92cfdea4e141349ac07d036ea3ab6cb3975a997f0657",
        236,
    ),
    (
        "motion-tree-east",
        "e15bb355fc996b1b52e3c653d3b29effabf7d83213a0b0cd1ffac1ec8277270b",
        714,
    ),
    (
        "motion-tree-bottom",
        "10b86a42c91f1dde9dba3f5df7e5d0ed96d2cfc10839b1c7198ccd851e533955",
        706,
    ),
    (
        "motion-type8-gap-bounded",
        "6a79fa70e595aceef210081e3e3f5072959273095eddc9c4ec9ab511ac43b1c5",
        2482,
    ),
    (
        "motion-door5-contact",
        "efdf26290581b40230d4af6db74cbe1aa0195a28241c533ea77a0700ff1dcf2f",
        207,
    ),
    (
        "motion-map41",
        "63952a21cdd18e7a9e1c9855f7ada777f7901564fc8ab8fe55487936f2ce6c61",
        880,
    ),
];

fn number(row: &Value, field: &str) -> u16 {
    u16::try_from(row[field].as_u64().expect("unsigned integer")).unwrap()
}

fn pair(value: &Value) -> (u16, u16) {
    assert_eq!(value.as_array().unwrap().len(), 2);
    (
        u16::try_from(value[0].as_u64().unwrap()).unwrap(),
        u16::try_from(value[1].as_u64().unwrap()).unwrap(),
    )
}

fn live_room(row: &Value) -> Room {
    assert_eq!(number(row, "flags") & 0x0406, 0x0404);
    assert_eq!(number(row, "special"), 0);
    assert_eq!(row["bounds"], serde_json::json!([65528, 16, 65520, 16]));
    let control = row["control"].as_array().unwrap();
    assert_eq!(control.len(), 2);
    for value in control {
        assert_eq!(value.as_u64().unwrap() & 0x50, 0, "action hook enabled");
    }
    let cells = row["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| u16::try_from(cell.as_u64().unwrap()).unwrap())
        .collect();
    Room::new_passive(number(row, "width"), number(row, "height"), cells)
        .unwrap()
        .with_material_policy(crysta_runtime::qualified_policy(
            number(row, "map"),
            number(row, "width"),
            number(row, "height"),
        ))
        .unwrap()
        .with_passive_directional_type8_special_bit_clear()
}

fn player_path(row: &Value) -> Vec<u64> {
    // The recorder continues to frame end, including later NPC calls. The
    // pending D107 instruction was already captured by the entry trace; D109
    // starts the resumed trace. D197 terminates the player's collision work.
    let path: Vec<_> = row["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pc| pc.as_u64().unwrap())
        .collect();
    assert_eq!(path.first(), Some(&0x80_D109));
    let end = path
        .iter()
        .position(|&pc| pc == 0x80_D197)
        .expect("player resolver did not finish");
    path[..=end].to_vec()
}

fn type8_witnesses(rows: &[Value]) {
    // Shared Open handlers do not by themselves demonstrate type8 coverage.
    // Pin both the dispatched cells and player-only PCs for the bounded cases.
    for (frame, cells, pcs) in [
        (12915, vec![(22, 11, 8)], vec![0x80_D506, 0x80_D50E]),
        (
            13948,
            vec![(8, 16, 0), (9, 16, 8)],
            vec![0x80_D3B3, 0x80_D3F1],
        ),
        (14328, vec![(22, 11, 8)], vec![0x80_D79D, 0x80_D7A0]),
    ] {
        let row = rows.iter().find(|row| row["frame"] == frame).unwrap();
        let path = player_path(row);
        for pc in pcs {
            assert!(path.contains(&pc), "missing type8 witness {frame}/{pc:06x}");
        }
        for (x, y, kind) in cells {
            let raw = row["cells"][y * usize::from(number(row, "width")) + x]
                .as_u64()
                .unwrap();
            assert_eq!(raw & 0x8000, 0);
            assert_eq!((raw >> 9) & 31, kind);
        }
    }
}

fn replay(bytes: &[u8], name: &str, hash: &str, count: usize) -> BTreeSet<u64> {
    let actual = rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        });
    assert_eq!(actual, hash, "{name}: native fixture identity");
    let rows: Vec<Value> = std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let start = rows.iter().position(|row| row["kind"] == "motion").unwrap();
    let origin = pair(&rows[start]["before"]);
    let map = number(&rows[start], "map");
    let first = rows[start]["frame"].as_u64().unwrap();
    // Initialize once, only after twelve contiguous, settled neutral frames.
    for (index, row) in rows[start - 12..start].iter().enumerate() {
        assert_eq!(row["kind"], "frame");
        assert_eq!(
            row["frame"].as_u64().unwrap(),
            first - 12 + u64::try_from(index).unwrap()
        );
        assert!(row["held"].as_array().unwrap().is_empty());
        assert_eq!(number(row, "map"), map);
        assert_eq!(pair(&row["position"]), origin);
    }
    let mut state = WalkingState::new(origin.0, origin.1);
    let mut restored = state;
    let motion = &rows[start..];
    assert_eq!(motion.len(), count);
    let mut coverage = BTreeSet::new();
    for (index, row) in motion.iter().enumerate() {
        coverage.extend(player_path(row));
        assert_eq!(row["kind"], "motion");
        let frame = row["frame"].as_u64().unwrap();
        assert_eq!(frame, first + u64::try_from(index).unwrap());
        assert_eq!(number(row, "map"), map);
        let room = live_room(row);
        let held = row["held"].as_array().unwrap();
        assert!(held.len() <= 1);
        let direction = held.first().map(|held| match held.as_str().unwrap() {
            "Right" => Direction::Right,
            "Left" => Direction::Left,
            "Up" => Direction::Up,
            "Down" => Direction::Down,
            other => panic!("unsupported fixture input {other}"),
        });
        assert_eq!(
            state.position(),
            pair(&row["before"]),
            "{name} frame {frame}"
        );
        let input = FrameInput { direction };
        let out = state
            .step(&room, input)
            .unwrap_or_else(|error| panic!("{name} frame {frame}: {error}"));
        assert_eq!(
            (i64::from(out.attempted_dx), i64::from(out.attempted_dy)),
            (
                row["attempt"][0].as_i64().unwrap(),
                row["attempt"][1].as_i64().unwrap()
            ),
            "{name} frame {frame}: native velocity before collision"
        );
        assert_eq!(
            state.position(),
            pair(&row["after"]),
            "{name} frame {frame}"
        );
        assert_eq!(restored.step(&room, input).unwrap(), out);
        restored = WalkingState::decode_snapshot(&room, &restored.encode_snapshot()).unwrap();
        assert_eq!(restored.encode_snapshot(), state.encode_snapshot());
    }
    if name == "motion-type8-gap-bounded" {
        type8_witnesses(motion);
    }
    eprintln!("{name}: {count} native motion frames, zero exclusions");
    coverage
}

#[test]
fn native_directional_motion_matches_every_frame_and_attempt() {
    let explicit = std::env::var_os("CRYSTA_COLLISION_FIXTURES");
    let root = explicit.as_ref().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/collision-qualification"),
        PathBuf::from,
    );
    if explicit.is_none() && !root.join("motion-Right.jsonl").exists() {
        eprintln!("SKIP: optional native collision fixtures absent; set CRYSTA_COLLISION_FIXTURES to require them");
        return;
    }
    let mut coverage = BTreeSet::new();
    for &(name, hash, count) in CASES {
        let bytes = std::fs::read(root.join(format!("{name}.jsonl"))).unwrap();
        coverage.extend(replay(&bytes, name, hash, count));
    }
    // Actual sum/neighbor handlers for both slope types in every direction,
    // not merely presence of all four held inputs somewhere in a route.
    for handler in [
        0x80_D447, 0x80_D4C4, 0x80_D8A6, 0x80_D82E, 0x80_DBD0, 0x80_DC26, 0x80_DFA2, 0x80_DF4C,
    ] {
        assert!(
            coverage.contains(&handler),
            "missing player slope handler {handler:06x}"
        );
    }
}

#[test]
fn coverage_stops_before_later_actors_and_rejects_incomplete_paths() {
    assert_eq!(
        player_path(&serde_json::json!({"path": [0x80_D109, 0x80_D197, 0x80_D447]})),
        vec![0x80_D109, 0x80_D197]
    );
    for path in [vec![], vec![0x80_D109], vec![0x80_D447, 0x80_D197]] {
        assert!(
            std::panic::catch_unwind(|| player_path(&serde_json::json!({"path": path}))).is_err()
        );
    }
}
