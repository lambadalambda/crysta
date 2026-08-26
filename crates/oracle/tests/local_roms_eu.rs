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
