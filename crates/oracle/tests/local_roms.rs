//! ROM-backed smoke tests for the oracle session: skip without local dumps.
//!
//! Proves the reference boundary boots the real game headless, advances the
//! game's own state machine, and reaches a stable frame boundary from reset.

use oracle::{Button, Session};
use rom::Revision;

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
    let state = session.frame_state();
    let twin_state = twin.frame_state();
    assert_eq!(
        state, twin_state,
        "two headless sessions must agree exactly"
    );
    eprintln!(
        "stable after {stable_at} frames: frames={} cycles={} map={last_map:#04x}",
        state.frames, state.cycles
    );
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
}
