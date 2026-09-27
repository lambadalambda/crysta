//! ROM-backed scenario suite for the European dump: skips without it.
//!
//! One boot per process: the vendored ares engine is a process singleton,
//! so all checks in this binary share a single session. Cross-boot
//! determinism is validated by snapshot-restart replay (see
//! `scenario_replay_from_snapshot`).

use oracle::replay::Fixture;
use oracle::{Button, Session};
use std::path::Path;

fn local_rom(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    std::fs::read(path).ok()
}

#[test]
fn scenario_eu_boot_to_name_entry_reports_named_checkpoints() {
    if local_rom("Terranigma (E) [!].smc").is_none() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    // The vendored ares engine never tears down cleanly (threads + statics),
    // so the scenario executes in a forked child of this test binary.
    if std::env::var("ORACLE_EXECUTE_CHILD").is_ok() {
        run_scenario_child();
    }
    let exe = std::env::current_exe().expect("current exe");
    let out = std::process::Command::new(&exe)
        .args([
            "--exact",
            "scenario_eu_boot_to_name_entry_reports_named_checkpoints",
            "--nocapture",
        ])
        .env("ORACLE_EXECUTE_CHILD", "1")
        .output()
        .expect("spawn child");
    assert!(
        out.status.success(),
        "child scenario run must exit cleanly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("EU title=(0x22,0xa5)"),
        "child must report the EU checkpoints"
    );
    println!("child EU scenario passed");
}

fn run_scenario_child() -> ! {
    let Some(image) = local_rom("Terranigma (E) [!].smc") else {
        eprintln!("skipping: local European dump not present");
        std::process::exit(0);
    };
    let rom = rom::Rom::load(&image).expect("validated dump");

    let mut a = Session::new(&rom).expect("session");
    let mut title = (0u8, 0u8);
    let mut name_entry = (0u8, 0u8);
    let mut settled = (0u8, 0u8);
    for frame in 0..1960u32 {
        if (1800..1810).contains(&frame) {
            a.set_button(Button::Start, true);
        } else {
            a.set_button(Button::Start, false);
        }
        a.run_frame();
        match frame {
            503 => title = (a.wram(0x047E), a.wram(0x0450)),
            1857 => name_entry = (a.wram(0x047E), a.wram(0x0450)),
            1959 => settled = (a.wram(0x047E), a.wram(0x0450)),
            _ => {}
        }
    }
    assert_eq!(title, (0x22, 0xA5), "expected title/attract map and state");
    assert_eq!(name_entry, (0x04, 0xC8), "expected Europe name entry");
    assert_eq!(settled.0, 0x04, "name entry must persist");
    assert_eq!(settled.1, 0xB0, "Europe name-entry state must persist");
    eprintln!(
        "EU title=({:#04x},{:#04x}) name-entry=({:#04x},{:#04x}) settled=({:#04x},{:#04x})",
        title.0, title.1, name_entry.0, name_entry.1, settled.0, settled.1
    );

    // NOTE: deep-state snapshot restarts are unreliable in the vendored
    // ares build; snapshot-restart determinism is covered by the JP suite at
    // the name-entry depth. Here the fixture is validated and the
    // checkpoints are pinned.
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eu-boot-to-name-entry.json");
    let fixture: Fixture =
        serde_json::from_slice(&std::fs::read(fixture_path).expect("fixture exists"))
            .expect("fixture parses");
    let rom_sha = rom.revision().sha256();
    fixture
        .validate(rom_sha)
        .expect("fixture matches local ROM");

    eprintln!("fixture validates; EU checkpoints pinned");
    // One-boot-per-process is asserted by the lib unit tests (fresh depth);
    // here the engine is too deep for a safe second attempt.
    std::process::exit(0);
}

#[test]
fn european_new_game_reaches_the_weaver_in_crysta() {
    if local_rom("Terranigma (E) [!].smc").is_none() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    // ares can only boot once per process. Keep this independent of the
    // name-entry fixture's child, and never initialize native game state.
    if std::env::var("ORACLE_EU_STORY_CHILD").is_ok() {
        run_story_child();
    }
    let out = std::process::Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "european_new_game_reaches_the_weaver_in_crysta",
            "--nocapture",
        ])
        .env("ORACLE_EU_STORY_CHILD", "1")
        .output()
        .expect("spawn child");
    assert!(
        out.status.success(),
        "European story witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_story_child() -> ! {
    let image = local_rom("Terranigma (E) [!].smc").expect("owned European ROM");
    let rom = rom::Rom::load(&image).expect("validated European ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    let buttons = [
        Button::Start,
        Button::Down,
        Button::A,
        Button::Right,
        Button::Left,
        Button::Up,
    ];
    let mut frame = 0;
    let mut advance = |count: u32, held: Option<Button>| {
        for button in buttons {
            session.set_button(button, held == Some(button));
        }
        for _ in 0..count {
            session.run_frame();
        }
        frame += count;
        let wram = session.wram_image();
        let word = |offset: usize| u16::from_le_bytes([wram[offset], wram[offset + 1]]);
        (
            word(0x047e),
            word(0x1000),
            word(0x1002),
            u16::from_le_bytes([wram[0x6c4], wram[0x6c5]]),
            frame,
            word(0x0dc2),
        )
    };
    // Real inputs from an empty-SRAM boot: load menu -> New Game (fourth
    // entry) -> accept the default name. Europe reaches its menu later than JP.
    advance(1800, None);
    advance(10, Some(Button::Start));
    advance(150, None);
    for _ in 0..3 {
        advance(12, Some(Button::Down));
        advance(18, None);
    }
    advance(100, None);
    advance(12, Some(Button::A));
    advance(300, None);
    let name = advance(12, Some(Button::Start));
    assert_eq!(
        name.0, 0x000f,
        "default-name confirmation starts a new game"
    );
    advance(800, None);
    advance(12, Some(Button::A));
    let bedroom = advance(600, None);
    let bedroom_frame = bedroom.4;
    assert_eq!(bedroom.0, 0x000f);
    assert_eq!((bedroom.1, bedroom.2), (304, 112));
    assert_eq!(bedroom.3 & 1, 0, "Elle has not finished waking Ark");

    // European dialogue has more, shorter pages than JP. Only real A presses
    // acknowledge them; no flags, positions, or state are injected.
    for _ in 0..9 {
        advance(12, Some(Button::A));
        advance(600, None);
    }
    let after = advance(0, None);
    assert_eq!(after.0, 0x000f);
    assert_eq!((after.1, after.2), (304, 112));
    assert_ne!(after.3 & 1, 0, "Elle must set event $20");
    advance(62, Some(Button::Right));
    advance(38, None);
    advance(67, Some(Button::Down));
    let room = advance(83, None);
    assert_eq!(room.0, 0x0010, "ordinary movement enters the next room");
    assert_eq!((room.1, room.2), (392, 353));
    assert_ne!(room.3 & 1, 0, "the story flag persists across the exit");

    // Ordinary movement through C into B. The Japanese route supplies
    // candidate walking legs; these endpoints are checked on the EU CPU.
    for (leg, (frames, held)) in [
        (42, Some(Button::Down)),
        (78, Some(Button::Left)),
        (90, None),
        (12, Some(Button::Left)),
        (100, None),
        (55, Some(Button::Left)),
        (70, Some(Button::Up)),
        (100, None),
        (1, Some(Button::A)), // facing Up: open B's door
        (100, None),
        (50, Some(Button::Up)),
        (100, None),
        (180, Some(Button::Up)),
    ]
    .into_iter()
    .enumerate()
    {
        let state = advance(frames, held);
        if leg == 4 {
            assert_eq!(state.0, 0x000c, "bedroom route crosses C");
        }
    }
    let elder = advance(0, None);
    assert_eq!(elder.0, 0x000b);
    assert_eq!((elder.1, elder.2), (120, 128));
    // The first request completes and grants $26 *before* the choice is
    // answered; the fourth A selects an answer, the rest finish its response.
    assert_eq!(elder.3 & (1 << (0x26 % 8)), 0);
    for _ in 0..3 {
        advance(1, Some(Button::A));
        advance(240, None);
    }
    let choice = advance(0, None);
    assert_ne!(choice.3 & (1 << (0x26 % 8)), 0);
    assert_eq!(choice.5, 0xffff, "the choice is still unanswered");
    advance(1, Some(Button::A));
    let answered = advance(240, None);
    assert_ne!(answered.5, 0xffff, "A answered the choice");
    for _ in 0..4 {
        advance(1, Some(Button::A));
        advance(240, None);
    }
    let spoken = advance(0, None);
    assert_ne!(spoken.3 & (1 << (0x26 % 8)), 0, "Elder grants event $26");
    for (leg, (frames, held)) in [
        (60, Some(Button::Down)),
        (100, None),
        (10, Some(Button::Left)),
        (82, Some(Button::Down)),
        (100, None),
        (45, Some(Button::Down)),
        (140, None),
        (22, Some(Button::Down)),
        (25, None),
        (15, None),
        (140, None),
    ]
    .into_iter()
    .enumerate()
    {
        let state = advance(frames, held);
        if leg == 1 {
            assert_eq!(state.0, 0x000c, "leaving B returns through C");
        } else if leg == 4 {
            assert_eq!(state.0, 0x000d, "the route crosses D");
        }
    }
    let exterior = advance(0, None);
    assert_eq!(exterior.0, 0x000a, "walk out of Ark's house");
    assert_eq!((exterior.1, exterior.2), (504, 769));

    // Cross the town, open the weaver's door, and approach her on foot.
    for (frames, held) in [
        (32, Some(Button::Down)),
        (60, None),
        (24, Some(Button::Right)),
        (90, None),
        (120, Some(Button::Left)),
        (60, None),
        (300, Some(Button::Up)),
        (60, None),
        (132, Some(Button::Right)),
        (60, None),
        (65, Some(Button::Up)),
        (60, None),
        (56, Some(Button::Left)),
        (60, None),
        (25, Some(Button::Up)),
        (100, None),
        (1, Some(Button::A)),
        (100, None),
        (40, Some(Button::Up)),
        (180, None),
        (43, Some(Button::Up)),
        (60, None),
        (22, Some(Button::Left)),
        (60, None),
        (20, Some(Button::Up)),
        (60, None),
    ] {
        advance(frames, held);
    }
    let weaver = advance(0, None);
    assert_eq!(weaver.0, 0x0013);
    assert_eq!((weaver.1, weaver.2), (360, 144));
    advance(1, Some(Button::A));
    advance(180, None);
    assert_eq!(advance(0, None).3 & (1 << 8), 0, "$28 not yet granted");
    // Eleven further English page/choice acknowledgements grant $28.
    for _ in 0..11 {
        advance(1, Some(Button::A));
        advance(240, None);
    }
    let told = advance(0, None);
    assert_ne!(told.3 & (1 << 8), 0, "the weaver grants event $28");
    assert_eq!(told.0, 0x0013);
    eprintln!(
        "EU new game: bedroom at {bedroom_frame}, room $10 at {}, exterior at {}, weaver at {}",
        room.4, exterior.4, told.4
    );
    std::process::exit(0);
}

/// Exact headless inputs observed on the European executable, starting after
/// the 1960-frame empty-SRAM title/name-entry bootstrap. No state injection.
#[test]
fn european_new_game_opens_pandora_and_completes_the_tour() {
    if local_rom("Terranigma (E) [!].smc").is_none() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    if std::env::var("ORACLE_EU_TOUR_CHILD").is_ok() {
        run_tour_child(false);
    }
    let out = std::process::Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "european_new_game_opens_pandora_and_completes_the_tour",
            "--nocapture",
        ])
        .env("ORACLE_EU_TOUR_CHILD", "1")
        .output()
        .expect("spawn child");
    assert!(
        out.status.success(),
        "European tour witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A second fresh child replays the accepted prefix and the separately
/// observed spear, frozen return, Elder mission and south-gate continuation.
#[test]
fn european_new_game_reaches_the_underworld_on_foot() {
    if local_rom("Terranigma (E) [!].smc").is_none() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    if std::env::var("ORACLE_EU_WORLD_CHILD").is_ok() {
        run_tour_child(true);
    }
    let out = std::process::Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "european_new_game_reaches_the_underworld_on_foot",
            "--nocapture",
        ])
        .env("ORACLE_EU_WORLD_CHILD", "1")
        .output()
        .expect("spawn child");
    assert!(
        out.status.success(),
        "European world-map witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The refusal and retry-acceptance branch is not the direct `$2E` route.
/// The native script keeps the pad through later pages, then grants `$0B`
/// and gives Ark control; never infer release from `$2F` alone.
#[test]
fn european_friend_refusal_then_retry_acceptance_releases_ark() {
    if local_rom("Terranigma (E) [!].smc").is_none() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    if std::env::var("ORACLE_EU_RETRY_CHILD").is_ok() {
        run_friend_retry_child();
    }
    let out = std::process::Command::new(std::env::current_exe().expect("current exe"))
        .args([
            "--exact",
            "european_friend_refusal_then_retry_acceptance_releases_ark",
            "--nocapture",
        ])
        .env("ORACLE_EU_RETRY_CHILD", "1")
        .output()
        .expect("spawn child");
    assert!(
        out.status.success(),
        "European retry witness failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_friend_retry_child() -> ! {
    let image = local_rom("Terranigma (E) [!].smc").expect("owned European ROM");
    let rom = rom::Rom::load(&image).expect("validated European ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    for _ in 0..1800 {
        session.run_frame();
    }
    session.set_button(Button::Start, true);
    for _ in 0..10 {
        session.run_frame();
    }
    session.set_button(Button::Start, false);
    for _ in 0..150 {
        session.run_frame();
    }
    let fixture = include_str!("fixtures/eu-pandora-tour.inputs");
    assert_eq!(
        fixture.lines().count(),
        453,
        "the recorded route is complete"
    );
    // Stop at the *first* friend's answer, before the direct `$2E` choice.
    for line in fixture.lines().take(166) {
        replay_european_input(&mut session, line);
    }
    let snapshot = |session: &Session| {
        let w = session.wram_image();
        let word = |at: usize| u16::from_le_bytes([w[at], w[at + 1]]);
        let flag = |id: usize| w[0x6c0 + id / 8] & (1 << (id % 8)) != 0;
        (
            word(0x47e),
            word(0x1000),
            word(0x1002),
            word(0x0dc2),
            [0x2e, 0x2f, 0x3f, 0x42, 0x0b].map(flag),
        )
    };
    assert_eq!(snapshot(&session), (0x0c, 120, 447, 0xffff, [false; 5]));
    for line in ["1 Down", "60", "1 A"] {
        replay_european_input(&mut session, line);
    }
    // Later English requests require separate acknowledgements after typing.
    for i in 0..23 {
        replay_european_input(&mut session, "240");
        replay_european_input(&mut session, "1 A");
        if i == 13 {
            let (map, x, y, _, flags) = snapshot(&session);
            assert_eq!((map, x, y), (0x0c, 120, 448));
            assert_eq!(
                flags[..4],
                [false, true, true, true],
                "retry is `$2F`, not `$2E`"
            );
            assert!(!flags[4], "$0B follows the player script's later pages");
        }
    }
    assert!(
        snapshot(&session).4[4],
        "the native player script grants `$0B`"
    );
    for line in ["600", "44 Right", "100"] {
        replay_european_input(&mut session, line);
    }
    assert_eq!(
        snapshot(&session),
        (0x0c, 184, 448, 0, [false, true, true, true, true]),
        "Ark walks after the alternate scene releases the pad"
    );
    std::process::exit(0);
}

fn replay_european_input(session: &mut Session, line: &str) {
    let mut parts = line.split_whitespace();
    let frames: usize = parts.next().expect("frame count").parse().expect("frames");
    let held: Vec<_> = parts.collect();
    for (name, button) in [
        ("Start", Button::Start),
        ("A", Button::A),
        ("Up", Button::Up),
        ("Down", Button::Down),
        ("Left", Button::Left),
        ("Right", Button::Right),
    ] {
        session.set_button(button, held.contains(&name));
    }
    assert!(held
        .iter()
        .all(|name| matches!(*name, "Start" | "A" | "Up" | "Down" | "Left" | "Right")));
    for _ in 0..frames {
        session.run_frame();
    }
}

fn run_tour_child(to_world: bool) -> ! {
    let image = local_rom("Terranigma (E) [!].smc").expect("owned European ROM");
    let rom = rom::Rom::load(&image).expect("validated European ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    for _ in 0..1800 {
        session.run_frame();
    }
    session.set_button(Button::Start, true);
    for _ in 0..10 {
        session.run_frame();
    }
    session.set_button(Button::Start, false);
    for _ in 0..150 {
        session.run_frame();
    }
    let commands = include_str!("fixtures/eu-pandora-tour.inputs");
    let extension = if to_world {
        include_str!("fixtures/eu-world-map.inputs")
    } else {
        ""
    };
    assert_eq!(commands.lines().count(), 453, "full tour is required");
    if to_world {
        assert_eq!(
            extension.lines().count(),
            212,
            "full continuation is required"
        );
    }
    for (index, line) in commands.lines().chain(extension.lines()).enumerate() {
        replay_european_input(&mut session, line);
        let w = session.wram_image();
        let word = |offset: usize| u16::from_le_bytes([w[offset], w[offset + 1]]);
        let event = |id: usize| w[0x6c0 + id / 8] & (1 << (id % 8)) != 0;
        if (index + 1) % 50 == 0 {
            eprintln!(
                "command {} frame {} map {:x}",
                index + 1,
                session.frame_state().frames,
                word(0x47e)
            );
        }
        match index + 1 {
            339 => {
                assert_eq!((word(0x47e), word(0x1000), word(0x1002)), (0x21, 136, 368));
                assert!(event(0x22), "Pandora's Box must open on foot");
                assert!(event(0x292), "pot throw must break the blue door");
            }
            372 => assert_eq!(word(0x47e), 0x41, "tour begins in Pandora's room"),
            406 => assert_eq!(word(0x47e), 0x44, "tour visits the first side room"),
            434 => assert_eq!(word(0x47e), 0x43, "tour visits the third side room"),
            448 => {
                assert_eq!((word(0x47e), word(0x1000), word(0x1002)), (0x41, 136, 208));
                for id in [0x20, 0x22, 0x26, 0x27, 0x28, 0x2e, 0x243, 0x244, 0x292] {
                    assert!(event(id), "tour must retain event {id:#x}");
                }
                eprintln!(
                    "EU Pandora tour complete at frame {}",
                    session.frame_state().frames
                );
            }
            453 => assert_eq!(
                (word(0x47e), word(0x1000), word(0x1002)),
                (0x41, 120, 192),
                "Ark can move on both axes after the completed tour"
            ),
            467 => assert_eq!(word(0x47e), 0x42, "spear room entered on foot"),
            // The fixture's neutral 120 frames end before either manual direction.
            // The 96px descent belongs to the frozen-return script, not Left/Up.
            573 => {
                assert_eq!(
                    (word(0x47e), word(0x1000), word(0x1002)),
                    (0x21, 136, 464),
                    "native scripted descent completes before manual route input"
                );
                assert!(event(0xfe) && event(0x23), "frozen return releases control");
                assert_eq!(
                    word(0x45e),
                    0,
                    "native pad is unlocked before manual Left/Up"
                );
            }
            // Separate retained manual Left and Up legs, including their rests.
            575 => assert_eq!(
                (word(0x47e), word(0x1000), word(0x1002)),
                (0x21, 120, 464),
                "manual Left changes X only after scripted release"
            ),
            577 => {
                assert_eq!(
                    (word(0x47e), word(0x1000), word(0x1002)),
                    (0x21, 120, 448),
                    "manual Up changes Y after manual Left; not a scripted landing"
                );
                for id in [0x240, 0x241, 0x242, 0x23, 0xfe] {
                    assert!(event(id), "frozen return must retain event {id:#x}");
                }
                assert_eq!(&w[0x18048..0x1804a], &[0x81, 1], "spear entered inventory");
                assert!(!event(0x296), "Elder has not assigned the mission yet");
            }
            612 => {
                assert_eq!((word(0x47e), word(0x1000), word(0x1002)), (0x0d, 120, 704));
                assert!(event(0x21), "Elder learns of the frozen town");
                assert!(!event(0x296), "his answer has not completed yet");
            }
            648 => {
                assert!(event(0x296), "Elder sends Ark on his mission");
                assert!(!event(0x3c), "town has not finished its own scene");
            }
            663 => {
                assert_eq!((word(0x47e), word(0x1000), word(0x1002)), (0x0a, 504, 868));
                assert!(event(0x3c), "frozen Crysta scene finishes");
            }
            665 => {
                assert_eq!((word(0x47e), word(0x1000), word(0x1002)), (0x03, 536, 544));
                assert_eq!(session.frame_state().frames, 73937);
                for id in [
                    0x20, 0x21, 0x22, 0x23, 0x26, 0x27, 0x28, 0x2e, 0x3c, 0xfe, 0x240, 0x241,
                    0x242, 0x243, 0x244, 0x292, 0x296,
                ] {
                    assert!(event(id), "world map must retain event {id:#x}");
                }
                eprintln!(
                    "EU world map reached at frame {}",
                    session.frame_state().frames
                );
            }
            _ => {}
        }
    }
    std::process::exit(0);
}
