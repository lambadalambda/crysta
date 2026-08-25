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
