//! Owned-ROM witness for the European map-$21 frozen-return player descent.
//!
//! This stays separate from the portable runtime: a fresh native session replays
//! the retained input fixture, then samples only bounded WRAM fields and exact
//! source-PC stops through the scripted movement and control-release boundary.

use oracle::{export::digest_hex, Button, CpuTraceStop, Session};
use std::collections::VecDeque;
use std::path::Path;

const TEST: &str = "european_frozen_return_executes_owned_player_descent";
const BUTTONS: [(&str, Button); 6] = [
    ("Start", Button::Start),
    ("A", Button::A),
    ("Up", Button::Up),
    ("Down", Button::Down),
    ("Left", Button::Left),
    ("Right", Button::Right),
];

const GUIDE_COP_DF: u32 = 0x88_b7b4;
const PLAYER_PCS: [u32; 14] = [
    0x88_b7f6, 0x88_b7fb, 0x88_b7fd, 0x88_b802, 0x88_b804, 0x88_b809, 0x88_b80b, 0x88_b80f,
    0x88_b814, 0x88_b816, 0x88_b81a, 0x88_b81f, 0x88_b821, 0x88_b827,
];
const PLAYER_SOURCE: &[(u32, u8, &[u8])] = &[
    (0x88_b7f6, 0x84, &[0x17, 0x0f, 0x01]),
    (0x88_b7fb, 0x8e, &[]),
    (0x88_b7fd, 0x84, &[0x09, 0x1b, 0x00]),
    (0x88_b802, 0x8e, &[]),
    (0x88_b804, 0x84, &[0x00, 0x00, 0x00]),
    (0x88_b809, 0x8e, &[]),
    (0x88_b80b, 0xc1, &[0x3c, 0x00]),
    (0x88_b80f, 0x84, &[0x01, 0x00, 0x00]),
    (0x88_b814, 0x8e, &[]),
    (0x88_b816, 0xc1, &[0x3c, 0x00]),
    (0x88_b81a, 0x84, &[0x00, 0x00, 0x00]),
    (0x88_b81f, 0x8e, &[]),
    (0x88_b821, 0xcb, &[0x01, 0xc1, 0x87, 0x84]),
    (0x88_b827, 0xbc, &[0x6b]),
];

type MovementDeltas = Vec<(i16, i16)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Census {
    frame: u32,
    y: u16,
    player_resume: u32,
    input_mask: u16,
    frozen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MotionFrame {
    frame: u32,
    before_y: u16,
    pending_y: i16,
    resolved_y: u16,
}

#[derive(Debug, Clone, Copy)]
struct Axis {
    pointer: u16,
    counter: i32,
}

fn rom_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join("Terranigma (E) [!].smc")
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn source_at(image: &[u8], address: u32, length: usize) -> &[u8] {
    let start = rom::RuntimeRomAddress::new(address)
        .expect("runtime ROM address")
        .normalized()
        .value() as usize;
    image
        .get(start..start + length)
        .expect("bounded ROM source")
}

fn source_word(image: &[u8], address: u32) -> u16 {
    word(source_at(image, address, 2), 0)
}

fn source_long(image: &[u8], address: u32) -> u32 {
    let bytes = source_at(image, address, 3);
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}

fn assert_cop(image: &[u8], address: u32, selector: u8, operands: &[u8]) {
    let bytes = source_at(image, address, operands.len() + 2);
    assert_eq!(&bytes[..2], &[0x02, selector], "COP at ${address:06X}");
    assert_eq!(&bytes[2..], operands, "operands at ${address:06X}");
}

fn parse_line(line: &str) -> (usize, Vec<&str>) {
    let mut fields = line.split_whitespace();
    let frames = fields.next().expect("frame count").parse().expect("frames");
    let held: Vec<_> = fields.collect();
    assert!(held
        .iter()
        .all(|name| BUTTONS.iter().any(|(known, _)| name == known)));
    (frames, held)
}

fn hold(session: &mut Session, held: &[&str]) {
    for (name, button) in BUTTONS {
        session.set_button(button, held.contains(&name));
    }
}

fn replay_line(session: &mut Session, line: &str) {
    let (frames, held) = parse_line(line);
    hold(session, &held);
    session.run_frames(frames);
}

fn resume(wram: &[u8], slot: usize) -> u32 {
    u32::from(word(wram, slot + 10)) | (u32::from(wram[slot + 12]) << 16)
}

fn sample(session: &Session) -> Census {
    let wram = session.wram_image();
    assert_eq!(word(&wram, 0x047e), 0x21);
    assert_eq!(word(&wram, 0x1000), 136);
    let event = |id: usize| wram[0x6c0 + id / 8] & (1 << (id % 8)) != 0;
    Census {
        frame: session.frame_state().frames,
        y: word(&wram, 0x1002),
        player_resume: resume(&wram, 0x1000),
        input_mask: word(&wram, 0x045e),
        frozen: event(0xfe) && event(0x23),
    }
}

fn player_motion_frame(session: &mut Session) -> (MotionFrame, Vec<u32>) {
    let frame = session.frame_state().frames;
    let mut source_hits = Vec::new();
    let mut remaining = 100_000;
    loop {
        let trace = session
            .trace_until_pc(0x80_d107, remaining, 1)
            .expect("bounded player-motion trace");
        source_hits.extend(
            trace
                .entries
                .iter()
                .filter_map(|entry| PLAYER_PCS.contains(&entry.address).then_some(entry.address)),
        );
        assert_eq!(
            trace.stop,
            CpuTraceStop::TargetReached,
            "player resolver absent in completed-frame interval {frame}"
        );
        assert_eq!(session.frame_state().frames, frame);
        remaining = remaining
            .checked_sub(trace.entries.len())
            .expect("player-motion trace budget");
        if session.cpu_registers().x == 0x1000 {
            break;
        }
        assert!(remaining > 0, "player-motion trace budget exhausted");
    }

    let pending = session.wram_image();
    let before_y = word(&pending, 0x1002);
    let pending_y = word(&pending, 0x1_101a).cast_signed();
    let rest = session
        .trace_until_pc(0xff_ffff, remaining, 1)
        .expect("finish player-motion frame");
    source_hits.extend(
        rest.entries
            .iter()
            .filter_map(|entry| PLAYER_PCS.contains(&entry.address).then_some(entry.address)),
    );
    assert_eq!(rest.stop, CpuTraceStop::FrameLimit);
    assert_eq!(session.frame_state().frames, frame + 1);
    let resolved_y = word(&session.wram_image(), 0x1002);
    (
        MotionFrame {
            frame: frame + 1,
            before_y,
            pending_y,
            resolved_y,
        },
        source_hits,
    )
}

fn axis_delta(packet: &[u8], axis: &mut Axis) -> i16 {
    if axis.pointer == 0 {
        return 0;
    }
    let at = |pointer: u16| {
        usize::from(
            pointer
                .checked_sub(0x6000)
                .expect("movement pointer below base"),
        )
    };
    axis.counter -= 1;
    if axis.counter < 0 {
        axis.pointer = axis.pointer.checked_add(2).expect("movement pointer");
        let mut counter = word(packet, at(axis.pointer));
        if counter & 0x8000 != 0 {
            axis.pointer = word(packet, at(axis.pointer) + 2);
            counter = word(packet, at(axis.pointer));
        }
        axis.counter = i32::from(counter);
        axis.pointer = axis.pointer.checked_add(2).expect("movement pointer");
    }
    word(packet, at(axis.pointer)).cast_signed()
}

fn movement_deltas(packet: &[u8], selector: u8, ticks: usize) -> MovementDeltas {
    let table = usize::from(selector) * 4;
    let mut x = Axis {
        pointer: word(packet, table),
        counter: 0,
    };
    let mut y = Axis {
        pointer: word(packet, table + 2),
        counter: 0,
    };
    (0..ticks)
        .map(|_| (axis_delta(packet, &mut x), axis_delta(packet, &mut y)))
        .collect()
}

#[test]
fn european_frozen_return_executes_owned_player_descent() {
    let path = rom_path();
    if !path.is_file() {
        eprintln!("skipping: owned European ROM absent");
        return;
    }
    if std::env::var_os("ORACLE_EU_FROZEN_CHILD").is_some() {
        run_child(&path);
    }
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("ORACLE_EU_FROZEN_CHILD", "1")
        .output()
        .expect("spawn owned-ROM child");
    assert!(
        output.status.success(),
        "European frozen-return witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
}

fn authenticate_source(rom: &rom::Rom) -> (MovementDeltas, MovementDeltas) {
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let image = rom.image();

    // Map $21's scene table selects $83:9272. Its record at $83:9285 places
    // the guide at (136,384) and owns header $88:B73C / entry $88:B741.
    assert_eq!(source_word(image, 0x83_8042), 0x9272);
    let record = source_at(image, 0x83_9285, 10);
    assert_eq!(
        digest_hex(record),
        "77dbe5d21ac204c359d83238494f98cd68e3b01f1c4bb29416d9b66976aa76f5"
    );
    assert_eq!((record[1], record[2]), (8, 24));
    assert_eq!(source_long(image, 0x83_9289), 0x88_b73c);
    assert_eq!(
        digest_hex(source_at(image, 0x88_b73c, 0xee)),
        "15de2227b54716d9e39504c0a117851723fbd526e84172c0e1bc2086cb1d7c32"
    );

    // The owned guide starts the exact player entry; typed fragments below
    // describe every straight-line COP through its terminating COP BC.
    assert_cop(image, GUIDE_COP_DF, 0xdf, &[0xf6, 0xb7, 0x88]);
    for &(address, selector, operands) in PLAYER_SOURCE {
        assert_cop(image, address, selector, operands);
    }
    assert_eq!(
        digest_hex(source_at(image, 0x88_b7f6, 0x34)),
        "5975cadbcc4ed4d14d76beeaf77337f1b305ca4519fdf0b0120a86b2f19ab6ab"
    );

    // COP 84's motion selectors read the common resource loaded at $AE:8000.
    // Animation-list duration bounds how many signed deltas COP 8E consumes.
    let packet = assets::compression::decode(&image[0x2e_8000..], 0x1a0c)
        .expect("European common movement packet");
    assert_eq!((packet.consumed, packet.data.len()), (0x8c0, 0x1a0c));
    assert_eq!(
        digest_hex(&image[0x2e_8000..0x2e_8000 + packet.consumed]),
        "fe60c07cee1af24765b438254227d3bbb0f92658e639f72e6a6a4ccdfff33f2d"
    );
    let art = assets::sprites::PandoraSprites::run_art(image).expect("European player art");
    let ticks = |resource: usize, pose: u8| {
        art[resource]
            .list(pose)
            .expect("COP 84 animation list")
            .frames()
            .iter()
            .map(|frame| usize::from(frame.duration()) + 1)
            .sum()
    };
    let first = movement_deltas(&packet.data, 0x0f, ticks(1, 0x17));
    let second = movement_deltas(&packet.data, 0x1b, ticks(0, 0x09));
    assert_eq!((first.len(), second.len()), (36, 16));
    assert!(first.iter().chain(&second).all(|&(x, y)| x == 0 && y >= 0));
    assert_eq!(first.iter().map(|&(_, y)| y).sum::<i16>(), 84);
    assert_eq!(second.iter().map(|&(_, y)| y).sum::<i16>(), 15);
    (first, second)
}

#[allow(clippy::too_many_lines)] // One linear, input-only native witness is easier to audit.
fn run_child(path: &Path) -> ! {
    let source = std::fs::read(path).expect("owned European ROM");
    let rom = rom::Rom::load(&source).expect("authenticated European ROM");
    let (first_source, second_source) = authenticate_source(&rom);

    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    session.run_frames(1_800);
    session.set_button(Button::Start, true);
    session.run_frames(10);
    session.set_button(Button::Start, false);
    session.run_frames(150);

    let lines: Vec<_> = include_str!("fixtures/eu-pandora-tour.inputs")
        .lines()
        .chain(include_str!("fixtures/eu-world-map.inputs").lines())
        .collect();
    assert_eq!(lines.len(), 665);
    for line in &lines[..565] {
        replay_line(&mut session, line);
    }
    let before = sample(&session);
    assert_eq!(
        before,
        Census {
            frame: 63_752,
            y: 368,
            player_resume: 0x84_a258,
            input_mask: 0xff50,
            frozen: false,
        }
    );

    // Commands 566..573 contain only neutral frames and three isolated A
    // acknowledgements. Manual Left is command 574 and is deliberately absent.
    let mut targets = VecDeque::from(
        std::iter::once(GUIDE_COP_DF)
            .chain(PLAYER_PCS)
            .collect::<Vec<_>>(),
    );
    let mut reached: Vec<(u32, u32, u16, u16)> = Vec::new();
    let mut motion = Vec::new();
    let mut census = Vec::new();
    for line in &lines[565..573] {
        let (frames, held) = parse_line(line);
        assert!(held.iter().all(|button| *button == "A"));
        hold(&mut session, &held);
        for _ in 0..frames {
            let frame = session.frame_state().frames;

            // Once the first COP 8E is parked, stop at the player's actual
            // collision resolver on every tick of both animation-bounded legs.
            // This reads raw pending Y before resolution and samples resolved Y
            // only after that same completed frame.
            if motion.len() < first_source.len() + second_source.len()
                && (reached.last().is_some_and(|&(pc, ..)| pc == 0x88_b7fb) || !motion.is_empty())
            {
                let (row, hits) = player_motion_frame(&mut session);
                for address in hits {
                    if matches!(targets.front(), Some(&target) if target == address) {
                        reached.push((address, frame, row.before_y, 0x1000));
                        targets.pop_front();
                    }
                }
                motion.push(row);
                census.push(sample(&session));
                continue;
            }

            let mut completed = false;
            while let Some(&target) = targets.front() {
                let trace = session
                    .trace_until_pc(target, 100_000, 1)
                    .expect("bounded source-PC trace");
                match trace.stop {
                    CpuTraceStop::TargetReached => {
                        let registers = session.cpu_registers();
                        let state = sample(&session);
                        reached.push((target, state.frame, state.y, registers.x));
                        targets.pop_front();
                        if target == 0x88_b7fb {
                            break;
                        }
                    }
                    CpuTraceStop::FrameLimit => {
                        assert_eq!(session.frame_state().frames, frame + 1);
                        completed = true;
                        break;
                    }
                    CpuTraceStop::InstructionLimit => panic!("one-frame source trace overflow"),
                }
            }
            if !completed
                && motion.is_empty()
                && reached.last().is_some_and(|&(pc, ..)| pc == 0x88_b7fb)
            {
                let (row, hits) = player_motion_frame(&mut session);
                for address in hits {
                    if matches!(targets.front(), Some(&target) if target == address) {
                        reached.push((address, frame, row.before_y, 0x1000));
                        targets.pop_front();
                    }
                }
                motion.push(row);
                census.push(sample(&session));
                continue;
            }
            if !completed {
                session.run_frame();
                assert_eq!(session.frame_state().frames, frame + 1);
            }
            census.push(sample(&session));
        }
    }
    assert!(targets.is_empty(), "all source instructions must execute");
    assert_eq!(
        reached,
        [
            (0x88_b7b4, 63_839, 368, 0x1080),
            (0x88_b7f6, 63_840, 368, 0x1000),
            (0x88_b7fb, 63_840, 368, 0x1000),
            (0x88_b7fd, 63_876, 452, 0x1000),
            (0x88_b802, 63_876, 452, 0x1000),
            (0x88_b804, 63_892, 464, 0x1000),
            (0x88_b809, 63_892, 464, 0x1000),
            (0x88_b80b, 63_893, 464, 0x1000),
            (0x88_b80f, 63_954, 464, 0x1000),
            (0x88_b814, 63_954, 464, 0x1000),
            (0x88_b816, 63_955, 464, 0x1000),
            (0x88_b81a, 64_016, 464, 0x1000),
            (0x88_b81f, 64_016, 464, 0x1000),
            (0x88_b821, 64_017, 464, 0x1000),
            (0x88_b827, 64_017, 464, 0x1000),
        ]
    );

    // The 1,083-row census pins Y for every completed frame through neutral
    // command 573 without retaining WRAM. The first COP 8E applies all +84 source
    // pixels. The second requests +15, but native collision applies +12 over
    // eight frames (including a final +1 at y463) and then stays at y464. This
    // is an observed per-frame boundary, not a coordinate snap.
    assert_eq!(
        (census.first().unwrap().frame, census.last().unwrap().frame),
        (63_753, 64_835)
    );
    let mut encoded_y = Vec::with_capacity(census.len() * 2);
    for state in &census {
        encoded_y.extend_from_slice(&state.y.to_le_bytes());
    }
    assert_eq!(
        digest_hex(&encoded_y),
        "7e2dbd401863f1d16484f46fd1e504b105dce7cf785026d9adc06710e8d000dd"
    );
    assert_eq!(
        (motion.first().unwrap().frame, motion.last().unwrap().frame),
        (63_841, 63_892)
    );
    let source_y: Vec<_> = first_source
        .iter()
        .chain(&second_source)
        .map(|&(_, y)| y)
        .collect();
    assert_eq!(motion.len(), source_y.len());
    for (index, (row, &pending_y)) in motion.iter().zip(&source_y).enumerate() {
        assert_eq!(row.frame, 63_841 + u32::try_from(index).unwrap());
        assert_eq!(row.pending_y, pending_y, "raw pending Y at tick {index}");
        if index > 0 {
            assert_eq!(row.before_y, motion[index - 1].resolved_y);
        }
    }
    let resolved_delta =
        |row: &MotionFrame| row.resolved_y.cast_signed() - row.before_y.cast_signed();
    assert!(motion[..first_source.len()]
        .iter()
        .all(|row| resolved_delta(row) == row.pending_y));
    let second_native = &motion[first_source.len()..];
    assert_eq!(second_native.iter().map(resolved_delta).sum::<i16>(), 12);
    assert_eq!(second_source.iter().map(|&(_, y)| y).sum::<i16>(), 15);
    assert!(second_native
        .iter()
        .all(|row| (0..=row.pending_y).contains(&resolved_delta(row))));
    let first_clipped = second_native
        .iter()
        .position(|row| resolved_delta(row) != row.pending_y)
        .expect("second leg must meet the lower boundary");
    assert_eq!(
        (
            first_clipped,
            second_native[first_clipped].before_y,
            second_native[first_clipped].pending_y,
            second_native[first_clipped].resolved_y,
        ),
        (7, 463, 2, 464)
    );
    assert!(second_native[first_clipped + 1..]
        .iter()
        .all(|row| row.resolved_y == 464));
    assert_eq!(
        motion.last().unwrap().resolved_y - motion.first().unwrap().before_y,
        96
    );

    let release = census
        .iter()
        .position(|state| state.frozen && state.input_mask == 0)
        .expect("FE/23 input release");
    assert_eq!(
        census[release],
        Census {
            frame: 64_236,
            y: 464,
            player_resume: 0x84_a258,
            input_mask: 0,
            frozen: true,
        }
    );
    assert!(census[release..].iter().all(|state| state.y == 464));
    assert_eq!(
        census.last().copied(),
        Some(Census {
            frame: 64_835,
            y: 464,
            player_resume: 0x84_a2a3,
            input_mask: 0,
            frozen: true,
        })
    );
    eprintln!(
        "EU frozen return: guide DF frame 63839; player +84 then clipped +12; release frame 64236 at (136,464); neutral through 64835"
    );
    std::process::exit(0);
}
