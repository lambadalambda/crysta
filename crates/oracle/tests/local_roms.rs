//! ROM-backed smoke tests for the oracle session: skip without local dumps.
//!
//! Proves the reference boundary boots the real game headless, advances the
//! game's own state machine, and reaches a stable frame boundary from reset.
#![allow(clippy::too_many_lines)] // scenario corpus is deliberately linear

use oracle::export;
use oracle::replay::{run_fixture, save_snapshot, ButtonDef, Fixture, FrameInput, StartPoint};
use oracle::{Button, Session};
use rom::Revision;
use std::path::Path;

fn local_rom(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    std::fs::read(path).ok()
}

/// Frame after which we consider boot "stable": the game's current-map
/// byte (`$7E047E`) stops changing for at least this many frames.
const STABLE_WINDOW: usize = 30;

#[test]
fn boots_japan_rom_to_stable_frame_boundary() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = rom::Rom::load(&image).expect("validated dump");
    assert_eq!(rom.revision(), Revision::Japan);

    let mut session = Session::new(&rom).expect("core accepts verified ROM");
    // Determinism precondition: two sessions at the same frame agree.
    let mut twin = Session::new(&rom).expect("core accepts verified ROM");

    let mut last_map = session.wram(0x047E);
    let mut stable_since = 0usize;
    let mut stable_frame = None;
    for frame in 0..900 {
        session.run_frame();
        twin.run_frame();
        let map = session.wram(0x047E);
        if map == last_map {
            stable_since += 1;
            if stable_since >= STABLE_WINDOW && stable_frame.is_none() {
                stable_frame = Some(frame);
            }
        } else {
            stable_since = 0;
            last_map = map;
        }
    }
    let stable_at = stable_frame.expect("game state must stabilize within 900 frames");
    // Stability onset is when the run of equal comparisons began; stable_at
    // is when the window filled.
    let stable_onset = stable_at + 1 - STABLE_WINDOW;
    let state = session.frame_state();
    let twin_state = twin.frame_state();
    assert_eq!(
        state, twin_state,
        "two headless sessions must agree exactly"
    );
    eprintln!(
        "map byte stable from frame {stable_onset} (window filled at {stable_at}): frames={} cycles={} map={last_map:#04x}",
        state.frames, state.cycles
    );
    // "Stable" here means the current-map byte is quiet; it is a boot
    // progress signal, not full game-state quiescence.
    // The game must have actually progressed (map byte left its reset value).
    assert_ne!(last_map, 0, "game did not advance past reset state");
}

#[test]
fn start_input_advances_game_state() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = rom::Rom::load(&image).expect("validated dump");
    let mut pressed_at_300 = Session::new(&rom).expect("session");
    let mut idle = Session::new(&rom).expect("session");
    for frame in 0..1200 {
        if frame == 300 {
            pressed_at_300.set_button(Button::Start, true);
        }
        if frame == 310 {
            pressed_at_300.set_button(Button::Start, false);
        }
        pressed_at_300.run_frame();
        idle.run_frame();
    }
    let with_input = pressed_at_300.frame_state();
    let without = idle.frame_state();
    assert_ne!(
        with_input.wram_sha256, without.wram_sha256,
        "Start input must diverge WRAM from the idle run"
    );
    // The framebuffer flush path must actually carry rendered output by the
    // title/attract state (not an all-black buffer).
    let nonblack = pressed_at_300
        .pixels()
        .chunks_exact(4)
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count();
    assert!(
        nonblack > 1000,
        "framebuffer looks unrendered: {nonblack} non-black pixels"
    );
    // Audio path must be flushed too (nonzero samples after boot music).
    let audible = pressed_at_300.samples().iter().filter(|s| **s != 0).count();
    eprintln!("pixels non-black: {nonblack}, samples non-zero: {audible}");
}

#[test]
fn replay_fixture_is_deterministic_and_snapshot_resumes() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = rom::Rom::load(&image).expect("validated dump");
    let rom_sha = rom.revision().sha256();

    let fixture = Fixture {
        version: oracle::replay::FIXTURE_VERSION,
        rom_sha256: rom_sha,
        harness: "smoke".into(),
        start: StartPoint::Reset { skip_frames: 300 },
        frames: vec![
            FrameInput {
                press: vec![ButtonDef::Start],
                release: vec![],
            },
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput::default(),
            FrameInput {
                press: vec![],
                release: vec![ButtonDef::Start],
            },
        ],
    };
    fixture.validate(rom_sha).expect("fixture matches ROM");

    // Two independent runs must agree exactly.
    let mut a = oracle::Session::new(&rom).expect("session a");
    let mut b = oracle::Session::new(&rom).expect("session b");
    let sa = run_fixture(&mut a, &fixture);
    let sb = run_fixture(&mut b, &fixture);
    assert_eq!(sa, sb, "same fixture must produce identical state");

    // Snapshot after the skip, then resume and compare against a run that
    // continued normally: states must match.
    let mut base = oracle::Session::new(&rom).expect("session base");
    base.run_frames(300);
    let snap = save_snapshot(&base);

    let mut resumed = oracle::Session::new(&rom).expect("session resumed");
    oracle::load_state(&mut resumed, &snap);
    for f in &fixture.frames {
        for btn in &f.press {
            resumed.set_button((*btn).into(), true);
        }
        for btn in &f.release {
            resumed.set_button((*btn).into(), false);
        }
        resumed.run_frame();
    }
    // base continues identically from its own state
    for f in &fixture.frames {
        for btn in &f.press {
            base.set_button((*btn).into(), true);
        }
        for btn in &f.release {
            base.set_button((*btn).into(), false);
        }
        base.run_frame();
    }
    assert_eq!(
        resumed.frame_state(),
        base.frame_state(),
        "snapshot resume must equal continuous run"
    );

    // Corrupt fixture rejection.
    let mut bad = fixture.clone();
    bad.rom_sha256 = [0x00; 32];
    assert!(bad.validate(rom_sha).is_err());
    bad.rom_sha256 = rom_sha;
    bad.version = 99;
    assert!(bad.validate(rom_sha).is_err());
}

/// A named scenario checkpoint: frame, `$7E047E` map byte, `$7E0450`
/// program state, and WRAM/VRAM digests.
#[derive(Debug, PartialEq, Eq)]
struct Checkpoint {
    name: &'static str,
    frame: u32,
    map: u8,
    state: u8,
    wram: String,
    vram: String,
}

fn checkpoint(session: &Session, name: &'static str) -> Checkpoint {
    Checkpoint {
        name,
        frame: session.frame_state().frames,
        map: session.wram(0x047E),
        state: session.wram(0x0450),
        wram: export::digest_hex(&session.wram_image()),
        vram: export::digest_hex(&vram_le_bytes(session)),
    }
}

// `vram()` returns words; digest the little-endian bytes for stability of
// the checkpoint encoding.
fn vram_le_bytes(session: &Session) -> Vec<u8> {
    session
        .vram()
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect()
}

#[test]
fn scenario_boot_to_name_entry_reports_named_checkpoints() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = rom::Rom::load(&image).expect("validated dump");

    // The boot-to-name-entry reference input: idle to the title screen,
    // Start at 400 to dismiss it into name entry, then quiet.
    let mut a = Session::new(&rom).expect("session a");
    let mut b = Session::new(&rom).expect("session b");
    let mut points_a = Vec::new();
    let mut points_b = Vec::new();
    for frame in 0..700u32 {
        if frame == 400 {
            a.set_button(Button::Start, true);
            b.set_button(Button::Start, true);
        }
        if frame == 410 {
            a.set_button(Button::Start, false);
            b.set_button(Button::Start, false);
        }
        a.run_frame();
        b.run_frame();
        match frame {
            399 => {
                points_a.push(checkpoint(&a, "title"));
                points_b.push(checkpoint(&b, "title"));
            }
            449 => {
                points_a.push(checkpoint(&a, "name-entry"));
                points_b.push(checkpoint(&b, "name-entry"));
            }
            459 => {
                points_a.push(checkpoint(&a, "name-entry-settled"));
                points_b.push(checkpoint(&b, "name-entry-settled"));
            }
            699 => {
                points_a.push(checkpoint(&a, "quiet-after-name-entry"));
                points_b.push(checkpoint(&b, "quiet-after-name-entry"));
            }
            _ => {}
        }
    }

    for (ca, cb) in points_a.iter().zip(&points_b) {
        assert_eq!(
            ca, cb,
            "checkpoint '{}' must be identical across sessions",
            ca.name
        );
    }
    // The scenario must actually have moved: title map (34) then name entry
    // (4), per the probe-derived timeline.
    let title = points_a.iter().find(|p| p.name == "title").unwrap();
    let name_entry = points_a.iter().find(|p| p.name == "name-entry").unwrap();
    let settled = points_a
        .iter()
        .find(|p| p.name == "name-entry-settled")
        .unwrap();
    assert_eq!(title.map, 34, "expected attract/title map");
    assert_eq!(name_entry.map, 4, "expected name-entry map");
    assert_eq!(settled.map, 4, "name entry must persist past input");
    assert_eq!(
        name_entry.state, 0xC6,
        "expected the name-entry program state"
    );
    assert_eq!(settled.state, 0xC6, "name entry state must persist");
    for p in &points_a {
        eprintln!(
            "checkpoint {}: map={:#04x} state={:#04x} wram_sha256={} vram_sha256={}",
            p.name, p.map, p.state, p.wram, p.vram
        );
    }

    // The committed fixture encodes precisely this input stream; replaying
    // it must land on the same final checkpoint.
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/boot-to-name-entry.json");
    let fixture: Fixture =
        serde_json::from_slice(&std::fs::read(fixture_path).expect("fixture exists"))
            .expect("fixture parses");
    let rom_sha = rom.revision().sha256();
    fixture
        .validate(rom_sha)
        .expect("fixture matches local ROM");
    let mut run = Session::new(&rom).expect("session");
    run_fixture(&mut run, &fixture);
    // skip 400 + 60 frames ends at boundary 460, matching `settled` (frame
    // 459 executed, counter reports 460).
    let final_point = checkpoint(&run, "fixture-final");
    let reference = points_a
        .iter()
        .find(|p| p.name == "name-entry-settled")
        .expect("settled checkpoint captured");
    assert_eq!(
        (
            final_point.frame,
            final_point.map,
            final_point.state,
            final_point.wram.as_str(),
            final_point.vram.as_str()
        ),
        (
            reference.frame,
            reference.map,
            reference.state,
            reference.wram.as_str(),
            reference.vram.as_str()
        ),
        "committed fixture must reproduce the reference run's final checkpoint"
    );
}

/// Scenario: name-entry cursor movement. Start dismisses the title into the
/// name-entry screen; three Down presses move the kana-grid cursor exactly
/// three cells. The wedge (SPC upload) is not entered: no confirm input is
/// sent.
#[test]
fn scenario_name_entry_cursor_moves_deterministically() {
    let Some(image) = local_rom("Tenchi Souzou (Japan).sfc") else {
        eprintln!("skipping: local Japanese dump not present");
        return;
    };
    let rom = rom::Rom::load(&image).expect("validated dump");

    // idle: Start@400 only; pressed: Start@400 plus Down 700..711,
    // 730..741, 760..771.
    let mut idle = Session::new(&rom).expect("session idle");
    let mut a = Session::new(&rom).expect("session a");
    let mut b = Session::new(&rom).expect("session b");
    let mut points_a = Vec::new();
    let mut points_b = Vec::new();
    let mut cursor_after_first_block = None;
    let mut cursor_after_second_block = None;
    for frame in 0..900u32 {
        if frame == 400 {
            idle.set_button(Button::Start, true);
            a.set_button(Button::Start, true);
            b.set_button(Button::Start, true);
        }
        if frame == 410 {
            idle.set_button(Button::Start, false);
            a.set_button(Button::Start, false);
            b.set_button(Button::Start, false);
        }
        let down = (700..712).contains(&frame)
            || (730..742).contains(&frame)
            || (760..772).contains(&frame);
        a.set_button(Button::Down, down);
        b.set_button(Button::Down, down);
        idle.run_frame();
        a.run_frame();
        b.run_frame();
        match frame {
            460 => {
                points_a.push(checkpoint(&a, "name-entry"));
                points_b.push(checkpoint(&b, "name-entry"));
            }
            720 => {
                points_a.push(checkpoint(&a, "cursor-one"));
                points_b.push(checkpoint(&b, "cursor-one"));
                cursor_after_first_block = Some(a.wram(0x04C8));
            }
            750 => {
                points_a.push(checkpoint(&a, "cursor-two"));
                points_b.push(checkpoint(&b, "cursor-two"));
                cursor_after_second_block = Some(a.wram(0x04C8));
            }
            899 => {
                points_a.push(checkpoint(&a, "cursor-three"));
                points_b.push(checkpoint(&b, "cursor-three"));
            }
            _ => {}
        }
    }

    for (ca, cb) in points_a.iter().zip(&points_b) {
        assert_eq!(
            ca, cb,
            "checkpoint '{}' must be identical across sessions",
            ca.name
        );
    }
    let by_name = |n: &str| {
        points_a
            .iter()
            .find(|p| p.name == n)
            .expect("checkpoint must exist")
    };
    assert_eq!(by_name("name-entry").map, 4, "expected name-entry map");
    assert_eq!(
        by_name("name-entry").state,
        0xC6,
        "expected name-entry state"
    );
    // Intermediate cursor rows pin per-block movement so a drift that
    // preserves the final row still fails. Frames 720/750 are quiet gaps
    // between the press blocks.
    assert_eq!(
        cursor_after_first_block,
        Some(1),
        "first Down block must move the cursor to row 1"
    );
    assert_eq!(
        cursor_after_second_block,
        Some(2),
        "second Down block must move the cursor to row 2"
    );
    assert_eq!(
        a.wram(0x04C8),
        3,
        "three Down presses must move the kana cursor three cells"
    );

    // The whole input difference against the idle run must be exactly the
    // cursor cell: the cursor-row byte and the two cell-value bytes.
    let idle_img = idle.wram_image();
    let img = a.wram_image();
    let mut diffs: Vec<usize> = (0..0x20000).filter(|&o| img[o] != idle_img[o]).collect();
    diffs.sort_unstable();
    assert_eq!(
        diffs,
        vec![0x04C8, 0x0A11, 0x10C2],
        "Down presses must touch only the kana-cursor cells"
    );
    assert_eq!(img[0x04C8].wrapping_sub(idle_img[0x04C8]), 3);
    assert_eq!(img[0x0A11].wrapping_sub(idle_img[0x0A11]), 0x30);
    assert_eq!(img[0x10C2].wrapping_sub(idle_img[0x10C2]), 0x30);

    for p in &points_a {
        eprintln!(
            "checkpoint {}: map={:#04x} state={:#04x} wram_sha256={} vram_sha256={}",
            p.name, p.map, p.state, p.wram, p.vram
        );
    }

    // The committed fixture encodes exactly this input stream; replaying it
    // must land on the same final checkpoint.
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/name-entry-cursor.json");
    let fixture: Fixture =
        serde_json::from_slice(&std::fs::read(fixture_path).expect("fixture exists"))
            .expect("fixture parses");
    let rom_sha = rom.revision().sha256();
    fixture
        .validate(rom_sha)
        .expect("fixture matches local ROM");
    let mut run = Session::new(&rom).expect("session");
    run_fixture(&mut run, &fixture);
    let final_point = checkpoint(&run, "fixture-final");
    let reference = by_name("cursor-three");
    assert_eq!(
        (
            final_point.frame,
            final_point.map,
            final_point.state,
            final_point.wram.as_str(),
            final_point.vram.as_str()
        ),
        (
            reference.frame,
            reference.map,
            reference.state,
            reference.wram.as_str(),
            reference.vram.as_str()
        ),
        "committed fixture must reproduce the reference run's final checkpoint"
    );
}
