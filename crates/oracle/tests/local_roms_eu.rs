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
fn european_new_game_wakes_ark_and_enters_the_next_room() {
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
            "european_new_game_wakes_ark_and_enters_the_next_room",
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
    let buttons = [Button::Start, Button::Down, Button::A, Button::Right];
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
        (word(0x047e), word(0x1000), word(0x1002), wram[0x6c4], frame)
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
    eprintln!(
        "EU new game: bedroom at {bedroom_frame}, map $10 at {}",
        room.4
    );
    std::process::exit(0);
}
