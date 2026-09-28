//! Owned-ROM witness for European post-return frozen residents.
//!
//! A fresh native session replays the retained tour and frozen-return prefix,
//! then takes a resident-census detour before the doorway Elder is accepted.

use oracle::{export::digest_hex, Button, CpuTraceStop, Session};
use std::path::Path;

const TEST: &str = "european_frozen_residents_match_source_records";
const INIT_HOOK: u32 = 0x80_f5d3;
const INIT_DONE: u32 = 0x80_f5e4;
const INIT_CONTINUE: u32 = 0x80_f5d7;
const TRACED_COMMANDS: [usize; 7] = [7, 11, 13, 17, 25, 27, 33];
const BUTTONS: [Button; 6] = [
    Button::Start,
    Button::A,
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
];

const B_RECORDS: &[(u32, u32)] = &[(0x83_8b9e, 0x83_8ba8)];
const C_RECORDS: &[(u32, u32)] = &[
    (0x83_8c12, 0x83_8c1c),
    (0x83_8c1c, 0x83_8c26),
    (0x83_8c26, 0x83_8c30),
    (0x83_8c30, 0x83_8c3a),
];
const D_RECORDS: &[(u32, u32)] = &[
    (0x83_8cbc, 0x83_8cc6),
    (0x83_8cc6, 0x83_8cd0), // Separate doorway Elder.
];
const TEN_RECORDS: &[(u32, u32)] = &[(0x83_8d84, 0x83_8d8e), (0x83_8d8e, 0x83_8d98)];
const ELEVEN_RECORDS: &[(u32, u32)] = &[(0x83_8dea, 0x83_8df4)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityState {
    position: (u16, u16),
    flags: [u16; 3],
    resume: u32,
    animation_base: u32,
    facing: u16,
    anchors: (u16, u16),
    timer: u16,
    display_cursor: u16,
    spawn_parameter: u16,
    selector: u16,
    composition: u32,
    resource: u16,
    draw_override: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnBinding {
    record: u32,
    cursor_after: u32,
    slot: u16,
    entry: u32,
    creation: EntityState,
}

#[derive(Debug, Default)]
struct Recorder {
    map: Option<u16>,
    bindings: Vec<SpawnBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtState {
    composition_sha256: String,
    palettes: Vec<(u8, [u16; 16])>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResidentObservation {
    record: u32,
    entry: u32,
    creation: EntityState,
    linked: bool,
    settled: EntityState,
    art: Option<ArtState>,
}

const FROZEN_PALETTE_5: [u16; 16] = [
    0x39ce, 0x0c62, 0x1d07, 0x31ac, 0x4651, 0x6338, 0x08c8, 0x094f, 0x0a3a, 0x490a, 0x618e, 0x0931,
    0x09ba, 0x46fe, 0x7c1f, 0x7bde,
];
const FROZEN_PALETTE_4: [u16; 16] = [
    0x39ce, 0x0c62, 0x004a, 0x108f, 0x1916, 0x0d9e, 0x28c7, 0x3d4c, 0x5a12, 0x288b, 0x48d3, 0x0931,
    0x09ba, 0x46fe, 0x7fff, 0x7bde,
];
const FROZEN_ELEVEN_PALETTE: [u16; 16] = [
    0x39aa, 0x0c62, 0x08cc, 0x0992, 0x123d, 0x2f3f, 0x28c7, 0x3d4c, 0x5a12, 0x004e, 0x04d7, 0x0931,
    0x09ba, 0x46fe, 0x7fff, 0x7bde,
];
const DOORWAY_ELDER_PALETTE: [u16; 16] = [
    0x39ce, 0x0c62, 0x2124, 0x31c5, 0x4eab, 0x6fb5, 0x4960, 0x6266, 0x7b4d, 0x098d, 0x1254, 0x0931,
    0x09ba, 0x46fe, 0x7c1f, 0x7bde,
];

#[allow(clippy::too_many_arguments)] // Compact constructor for literal native tuples.
fn entity(
    position: (u16, u16),
    flags: [u16; 3],
    resume: u32,
    animation_base: u32,
    facing: u16,
    anchors: (u16, u16),
    selector: u16,
    composition: u32,
) -> EntityState {
    EntityState {
        position,
        flags,
        resume,
        animation_base,
        facing,
        anchors,
        timer: 0,
        display_cursor: 1,
        spawn_parameter: 0,
        selector,
        composition,
        resource: 0,
        draw_override: 0,
    }
}

#[allow(clippy::unnecessary_wraps)] // Expected linked art mirrors the observation's Option.
fn expected_art(hash: &str, palette: u8, colors: [u16; 16]) -> Option<ArtState> {
    Some(ArtState {
        composition_sha256: hash.to_owned(),
        palettes: vec![(palette, colors)],
    })
}

#[allow(clippy::too_many_lines)] // Literal native tuples are evidence, not generated fixtures.
fn expected_observations(map: u16) -> Vec<ResidentObservation> {
    let created = |position, entry, animation, facing, anchors, selector, composition| {
        entity(
            position,
            [0x5100, 0, 0x0100],
            entry,
            animation,
            facing,
            anchors,
            selector,
            composition,
        )
    };
    let frozen = |position, animation, facing, anchors, selector, composition, flags| {
        entity(
            position,
            [0x1100, 0x0200, flags],
            0x88_e0f0,
            animation,
            facing,
            anchors,
            selector,
            composition,
        )
    };
    match map {
        0x0b => vec![ResidentObservation {
            record: 0x83_8b9e,
            entry: 0x88_9027,
            creation: created(
                (0x78, 0x70),
                0x88_9027,
                0x7e_7000,
                0,
                (0x10, 0x18),
                6,
                0x7e_7477,
            ),
            linked: false,
            settled: created(
                (0x78, 0x70),
                0x88_9027,
                0x7e_7000,
                0,
                (0x10, 0x18),
                6,
                0x7e_7477,
            ),
            art: None,
        }],
        0x0c => vec![
            ResidentObservation {
                record: 0x83_8c12,
                entry: 0x88_a9ab,
                creation: created(
                    (0x58, 0x1a0),
                    0x88_a9ab,
                    0x7e_7000,
                    1,
                    (0x10, 0x20),
                    1,
                    0x7e_7103,
                ),
                linked: true,
                settled: frozen(
                    (0x58, 0x1a0),
                    0x7e_7000,
                    1,
                    (0x10, 0x20),
                    1,
                    0x7e_7103,
                    0x0d00,
                ),
                art: expected_art(
                    "77c0c97719bdafdc1163b7e0e50b989f140d237b2c76399680b81aa58349e4dd",
                    5,
                    FROZEN_PALETTE_5,
                ),
            },
            ResidentObservation {
                record: 0x83_8c1c,
                entry: 0x88_abbe,
                creation: created(
                    (0x38, 0x180),
                    0x88_abbe,
                    0x7e_7000,
                    3,
                    (8, 0x21),
                    2,
                    0x7e_7168,
                ),
                linked: true,
                settled: frozen((0x38, 0x180), 0x7e_7000, 3, (8, 0x21), 2, 0x7e_7168, 0x0d00),
                art: expected_art(
                    "89404525240eb55cb001ae32162149b46b936f7977ce3c68073dc1bd71120c70",
                    5,
                    FROZEN_PALETTE_5,
                ),
            },
            ResidentObservation {
                record: 0x83_8c26,
                entry: 0x88_9f52,
                creation: created(
                    (0x48, 0x170),
                    0x88_9f52,
                    0x7e_735a,
                    0,
                    (0x10, 0x20),
                    0,
                    0x7e_73f8,
                ),
                linked: true,
                settled: frozen(
                    (0x48, 0x170),
                    0x7e_735a,
                    0,
                    (0x10, 0x20),
                    0,
                    0x7e_73f8,
                    0x0f00,
                ),
                art: expected_art(
                    "4c299b74076284286f3ba1cb2103bd92305d5031a3aff7ac1d4cb1aafaa1cdf1",
                    4,
                    FROZEN_PALETTE_4,
                ),
            },
            ResidentObservation {
                record: 0x83_8c30,
                entry: 0x88_a7ed,
                creation: created(
                    (0x68, 0x170),
                    0x88_a7ed,
                    0x7e_7636,
                    0,
                    (0x10, 0x20),
                    0,
                    0x7e_76cc,
                ),
                linked: true,
                settled: frozen(
                    (0x68, 0x170),
                    0x7e_7636,
                    0,
                    (0x10, 0x20),
                    0,
                    0x7e_76cc,
                    0x0f00,
                ),
                art: expected_art(
                    "6f4efde96f1e4abca84812d530bd5e5702c248febd709b6f55c18a274d7d1c3f",
                    4,
                    FROZEN_PALETTE_4,
                ),
            },
        ],
        0x0d => vec![
            ResidentObservation {
                record: 0x83_8cbc,
                entry: 0x88_afc3,
                creation: created(
                    (0x48, 0x2a0),
                    0x88_afc3,
                    0x7e_7000,
                    0,
                    (0x10, 0x1f),
                    3,
                    0x7e_7163,
                ),
                linked: true,
                settled: frozen(
                    (0x48, 0x2a0),
                    0x7e_7000,
                    0,
                    (0x10, 0x1f),
                    3,
                    0x7e_7163,
                    0x0f00,
                ),
                art: expected_art(
                    "2e15eedc7f46538f036904942be8455ffd3a4818e3b3b77ba1fd594e836f95d6",
                    4,
                    FROZEN_PALETTE_4,
                ),
            },
            ResidentObservation {
                record: 0x83_8cc6,
                entry: 0x88_8d0b,
                creation: created(
                    (0x78, 0x2d0),
                    0x88_8d0b,
                    0x7e_728b,
                    1,
                    (8, 0x20),
                    1,
                    0x7e_7404,
                ),
                linked: true,
                settled: entity(
                    (0x78, 0x2d0),
                    [0x1100, 0, 0x0100],
                    0x88_8d22,
                    0x7e_728b,
                    1,
                    (8, 0x20),
                    1,
                    0x7e_7404,
                ),
                art: expected_art(
                    "f453489a711d08344f565bedc0c206c51f06d75026d76a3b9b029823ef47d863",
                    5,
                    DOORWAY_ELDER_PALETTE,
                ),
            },
        ],
        0x10 => vec![
            ResidentObservation {
                record: 0x83_8d84,
                entry: 0x88_9d56,
                creation: created(
                    (0x1a8, 0x1a0),
                    0x88_9d56,
                    0x7e_7000,
                    3,
                    (8, 0x21),
                    2,
                    0x7e_7168,
                ),
                linked: true,
                settled: frozen(
                    (0x1a8, 0x1a0),
                    0x7e_7000,
                    3,
                    (8, 0x21),
                    2,
                    0x7e_7168,
                    0x0d00,
                ),
                art: expected_art(
                    "89404525240eb55cb001ae32162149b46b936f7977ce3c68073dc1bd71120c70",
                    5,
                    FROZEN_PALETTE_5,
                ),
            },
            ResidentObservation {
                record: 0x83_8d8e,
                entry: 0x88_9e5f,
                creation: created(
                    (0x1b8, 0x1a0),
                    0x88_9e5f,
                    0x7e_735a,
                    3,
                    (8, 0x20),
                    2,
                    0x7e_7498,
                ),
                linked: true,
                settled: frozen(
                    (0x1b8, 0x1a0),
                    0x7e_735a,
                    3,
                    (8, 0x20),
                    2,
                    0x7e_7498,
                    0x0f00,
                ),
                art: expected_art(
                    "97a6e629c9faa57ef7cb1b47d8e13c9faee95687f14bddada4fe350ccef63967",
                    4,
                    FROZEN_PALETTE_4,
                ),
            },
        ],
        0x11 => vec![ResidentObservation {
            record: 0x83_8dea,
            entry: 0x88_b1b5,
            creation: created(
                (0x1b8, 0x280),
                0x88_b1b5,
                0x7e_7000,
                1,
                (8, 0x1f),
                4,
                0x7e_71d1,
            ),
            linked: true,
            settled: frozen(
                (0x1b8, 0x280),
                0x7e_7000,
                1,
                (8, 0x1f),
                4,
                0x7e_71d1,
                0x0f00,
            ),
            art: expected_art(
                "6c94b2f80bd82f9288aaddada2df0bcac29c639e5ac8d35d5411b01089c2fda5",
                4,
                FROZEN_ELEVEN_PALETTE,
            ),
        }],
        _ => Vec::new(),
    }
}

fn rom_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join("Terranigma (E) [!].smc")
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn long(bytes: &[u8], at: usize) -> u32 {
    u32::from(bytes[at]) | (u32::from(bytes[at + 1]) << 8) | (u32::from(bytes[at + 2]) << 16)
}

fn event(wram: &[u8], id: usize) -> bool {
    wram[0x6c0 + id / 8] & (1 << (id % 8)) != 0
}

fn target_records(map: u16) -> &'static [(u32, u32)] {
    match map {
        0x0b => B_RECORDS,
        0x0c => C_RECORDS,
        0x0d => D_RECORDS,
        0x10 => TEN_RECORDS,
        0x11 => ELEVEN_RECORDS,
        _ => &[],
    }
}

fn source_entry(image: &[u8], record: u32) -> u32 {
    let at = rom::RuntimeRomAddress::new(record)
        .expect("source record address")
        .normalized()
        .value() as usize;
    long(image, at + 4) + 5
}

fn read_entity(wram: &[u8], slot: u16) -> EntityState {
    let at = usize::from(slot);
    let bank = u32::from(wram[at + 0x12]);
    EntityState {
        position: (word(wram, at), word(wram, at + 2)),
        flags: [word(wram, at + 4), word(wram, at + 6), word(wram, at + 8)],
        resume: long(wram, at + 10),
        animation_base: long(wram, at + 0x10),
        facing: word(wram, at + 0x14),
        anchors: (word(wram, at + 0x18), word(wram, at + 0x1a)),
        timer: word(wram, at + 0x1e),
        display_cursor: word(wram, at + 0x20),
        spawn_parameter: word(wram, at + 0x26),
        selector: word(wram, at + 0x10008),
        composition: (bank << 16) | u32::from(word(wram, at + 0x1000a)),
        resource: word(wram, at + 0x12016),
        draw_override: word(wram, at + 0x11020),
    }
}

fn linked_slots(wram: &[u8]) -> Vec<u16> {
    let mut linked = Vec::new();
    let mut slot = word(wram, 0x0dfc);
    while slot != 0 {
        assert!((0x1000..0x2000).contains(&slot));
        assert_eq!(slot & 0x3f, 0);
        assert!(!linked.contains(&slot), "cyclic native entity list");
        linked.push(slot);
        slot = word(wram, usize::from(slot) + 0x2c);
    }
    linked
}

fn art_state(image: &[u8], session: &Session, entity: EntityState) -> ArtState {
    let wram = session.wram_image();
    let bank = entity.composition >> 16;
    let pointer = if bank == 0x7e {
        usize::try_from(entity.composition & 0xffff).expect("WRAM composition pointer")
    } else {
        assert!(bank >= 0x80, "unsupported composition bank {bank:02X}");
        rom::RuntimeRomAddress::new(entity.composition)
            .expect("composition address")
            .normalized()
            .value() as usize
    };
    let memory = if bank == 0x7e { &wram[..] } else { image };
    let count = usize::from(memory[pointer + 12]);
    assert!((1..=128).contains(&count));
    let bytes = &memory[pointer - 4..pointer + 13 + count * 7];
    let mut palette_ids = Vec::new();
    for component in 0..count {
        let attributes = word(bytes, 17 + component * 7 + 5);
        let palette = ((attributes >> 9) & 7) as u8;
        if !palette_ids.contains(&palette) {
            palette_ids.push(palette);
        }
    }
    palette_ids.sort_unstable();
    let cgram = session.cgram();
    let palettes = palette_ids
        .into_iter()
        .map(|palette| {
            let start = 128 + usize::from(palette) * 16;
            (
                palette,
                cgram[start..start + 16]
                    .try_into()
                    .expect("complete OBJ palette"),
            )
        })
        .collect();
    ArtState {
        composition_sha256: digest_hex(bytes),
        palettes,
    }
}

fn set_held(session: &mut Session, held: &[Button]) {
    for button in BUTTONS {
        session.set_button(button, held.contains(&button));
    }
}

fn parse_line(line: &str) -> (usize, Vec<Button>) {
    let mut fields = line.split_whitespace();
    let frames = fields.next().expect("frame count").parse().expect("frames");
    let held = fields
        .map(|name| match name {
            "Start" => Button::Start,
            "A" => Button::A,
            "Up" => Button::Up,
            "Down" => Button::Down,
            "Left" => Button::Left,
            "Right" => Button::Right,
            _ => panic!("unknown button {name}"),
        })
        .collect();
    (frames, held)
}

fn run_frames(session: &mut Session, frames: usize, held: &[Button]) {
    set_held(session, held);
    session.run_frames(frames);
}

fn replay_line(session: &mut Session, line: &str) {
    let (frames, held) = parse_line(line);
    run_frames(session, frames, &held);
}

fn trace_frames(image: &[u8], session: &mut Session, recorder: &mut Recorder, frames: usize) {
    let end = session
        .frame_state()
        .frames
        .checked_add(u32::try_from(frames).expect("trace frame count"))
        .expect("trace frame endpoint");
    while session.frame_state().frames < end {
        let remaining = end - session.frame_state().frames;
        let trace = session
            .trace_until_pc(INIT_HOOK, 2_000_000, remaining.min(60))
            .expect("bounded initialization trace");
        match trace.stop {
            CpuTraceStop::FrameLimit => {}
            CpuTraceStop::TargetReached => {
                let wram = session.wram_image();
                let map = word(&wram, 0x047e);
                let cursor_after = long(&wram, 0x006e);
                let slot = session.cpu_registers().x;
                if recorder.map != Some(map) {
                    recorder.map = Some(map);
                    recorder.bindings.clear();
                }
                recorder.bindings.retain(|binding| binding.slot != slot);
                let remaining = end - session.frame_state().frames;
                assert!(
                    remaining > 0,
                    "actor initialization crossed the input command"
                );
                let Some(&(record, _)) = target_records(map)
                    .iter()
                    .find(|&&(_, cursor)| cursor == cursor_after)
                else {
                    let continued = session
                        .trace_until_pc(INIT_CONTINUE, 500_000, remaining)
                        .expect("skip unrelated initialized actor");
                    assert_eq!(continued.stop, CpuTraceStop::TargetReached);
                    continue;
                };
                let entry = read_entity(&wram, slot).resume;
                assert_eq!(entry, source_entry(image, record));
                let completed = session
                    .trace_until_pc(INIT_DONE, 500_000, remaining)
                    .expect("complete source-bound actor creation");
                assert_eq!(completed.stop, CpuTraceStop::TargetReached);
                recorder.bindings.push(SpawnBinding {
                    record,
                    cursor_after,
                    slot,
                    entry,
                    creation: read_entity(&session.wram_image(), slot),
                });
            }
            CpuTraceStop::InstructionLimit => panic!("initialization trace budget exhausted"),
        }
    }
}

fn replay_line_traced(image: &[u8], session: &mut Session, recorder: &mut Recorder, line: &str) {
    let (frames, held) = parse_line(line);
    set_held(session, &held);
    trace_frames(image, session, recorder, frames);
}

fn state(session: &Session) -> (u16, u16, u16) {
    let wram = session.wram_image();
    (
        word(&wram, 0x047e),
        word(&wram, 0x1000),
        word(&wram, 0x1002),
    )
}

fn report(
    image: &[u8],
    session: &Session,
    recorder: &Recorder,
    label: &str,
) -> Vec<ResidentObservation> {
    let wram = session.wram_image();
    let (map, _, _) = state(session);
    assert_eq!(recorder.map, Some(map), "missing map initialization trace");
    let linked = linked_slots(&wram);
    let observations = target_records(map)
        .iter()
        .map(|&(record, cursor_after)| {
            let binding = recorder
                .bindings
                .iter()
                .find(|binding| binding.record == record)
                .expect("source resident must reach the initialization hook");
            assert_eq!(binding.cursor_after, cursor_after);
            let is_linked = linked.contains(&binding.slot);
            let settled = read_entity(&wram, binding.slot);
            ResidentObservation {
                record,
                entry: binding.entry,
                creation: binding.creation,
                linked: is_linked,
                settled,
                art: is_linked.then(|| art_state(image, session, settled)),
            }
        })
        .collect::<Vec<_>>();
    eprintln!(
        "{label}: frame={} state={:?} mask={:04X} records={:X?}",
        session.frame_state().frames,
        state(session),
        word(&wram, 0x045e),
        observations
            .iter()
            .map(|observation| observation.record)
            .collect::<Vec<_>>()
    );
    observations
}

#[test]
fn european_frozen_residents_match_source_records() {
    let path = rom_path();
    if !path.is_file() {
        eprintln!("skipping: owned European ROM absent");
        return;
    }
    if std::env::var_os("ORACLE_EU_FROZEN_RESIDENTS_CHILD").is_some() {
        run_child(&path);
    }
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("ORACLE_EU_FROZEN_RESIDENTS_CHILD", "1")
        .output()
        .expect("spawn owned-ROM child");
    assert!(
        output.status.success(),
        "European frozen-resident witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
}

#[allow(clippy::too_many_lines)] // One linear input-only native witness is easiest to audit.
fn run_child(path: &Path) -> ! {
    let rom = rom::Rom::load(&std::fs::read(path).expect("owned European ROM"))
        .expect("validated European ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let image = rom.image();
    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    let mut recorder = Recorder::default();

    run_frames(&mut session, 1800, &[]);
    run_frames(&mut session, 10, &[Button::Start]);
    run_frames(&mut session, 150, &[]);

    let tour: Vec<_> = include_str!("fixtures/eu-pandora-tour.inputs")
        .lines()
        .collect();
    let world: Vec<_> = include_str!("fixtures/eu-world-map.inputs")
        .lines()
        .collect();
    let detour: Vec<_> = include_str!("fixtures/eu-frozen-residents.inputs")
        .lines()
        .collect();
    assert_eq!(tour.len(), 453);
    assert_eq!(world.len(), 212);
    assert_eq!(world[143], "20 Up");
    assert_eq!(world[144], "240");
    assert_eq!(world[145], "34 Down");
    assert_eq!(detour.len(), 34);
    assert_eq!(
        detour[..4],
        world[145..149],
        "detour must fork with the retained route's exact common prefix"
    );

    for line in tour.iter().chain(&world[..144]) {
        replay_line(&mut session, line);
    }
    replay_line_traced(image, &mut session, &mut recorder, world[144]);
    assert_eq!(
        report(image, &session, &recorder, "fork-C"),
        expected_observations(0x0c)
    );
    assert_eq!(
        (session.frame_state().frames, state(&session)),
        (66_333, (0x0c, 184, 368))
    );

    for (index, line) in detour.iter().enumerate() {
        if TRACED_COMMANDS.contains(&index) {
            replay_line_traced(image, &mut session, &mut recorder, line);
        } else {
            replay_line(&mut session, line);
        }
        let checkpoint = match index {
            7 => Some(("room10-outbound", 66_693, (0x10, 297, 432))),
            11 => Some(("room11", 66_940, (0x11, 360, 609))),
            13 => Some(("room10-return", 67_102, (0x10, 360, 463))),
            17 => Some(("C-after-10", 67_349, (0x0c, 215, 432))),
            25 => Some(("roomB", 67_791, (0x0b, 120, 191))),
            27 => Some(("C-after-B", 67_952, (0x0c, 136, 353))),
            33 => Some(("roomD-before-Elder", 68_231, (0x0d, 120, 625))),
            _ => None,
        };
        if let Some((label, frame, expected)) = checkpoint {
            assert_eq!(
                report(image, &session, &recorder, label),
                expected_observations(expected.0)
            );
            assert_eq!(
                (session.frame_state().frames, state(&session)),
                (frame, expected)
            );
        }
    }

    let wram = session.wram_image();
    assert_eq!(word(&wram, 0x045e), 0);
    assert!([0x23, 0x27, 0xfe].into_iter().all(|id| event(&wram, id)));
    assert!([0x21, 0x107, 0x2a, 0x2c, 0x296]
        .into_iter()
        .all(|id| !event(&wram, id)));
    std::process::exit(0);
}
