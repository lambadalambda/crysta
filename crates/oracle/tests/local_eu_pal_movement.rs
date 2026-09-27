//! Frame-by-frame European walking witness from an empty-SRAM native boot.
//! ares has one safe boot per process; the owned-ROM run lives in a fresh child.
//! Only the two ordinary walking legs are frame-compared: not the doorway/load
//! duration, animation, host real-time pacing, or full-route equivalence.
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, World};
use oracle::{Button, Session};
use rom::{Revision, Rom};
use room_core::Direction;
use std::{path::Path, process::Command};

const TEST: &str = "european_first_bedroom_walk_matches_native_each_frame";
const FIXTURE: &str = include_str!("fixtures/eu-pandora-tour.inputs");

#[test]
fn european_first_bedroom_walk_matches_native_each_frame() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    let image = match std::fs::read(path) {
        Ok(image) => image,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: owned European ROM absent");
            return;
        }
        Err(error) => panic!("cannot read European ROM: {error}"),
    };
    let rom = Rom::load(&image).expect("authenticated European dump");
    assert_eq!(rom.revision(), Revision::EuropeEnglish);
    if std::env::var("EU_WALK_CHILD").is_ok() {
        inspect_walk(&rom);
        std::process::exit(0);
    }
    let out = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env("EU_WALK_CHILD", "1")
        .output()
        .expect("fresh native child");
    assert!(
        out.status.success()
            && String::from_utf8_lossy(&out.stderr)
                .contains("EU first bedroom walk: 63 frame boundaries match")
            && String::from_utf8_lossy(&out.stderr)
                .contains("EU exterior held-Down walk: 43 frame boundaries match"),
        "native/portable EU walk diverged\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn inspect_walk(rom: &Rom) {
    let mut native = Session::new(rom).expect("empty-SRAM native session");
    run(&mut native, 1800, None);
    run(&mut native, 10, Some(Button::Start));
    run(&mut native, 150, None);
    let mut commands = FIXTURE.lines();
    for _ in 0..30 {
        let line = commands.next().expect("native opening fixture");
        let mut parts = line.split_whitespace();
        let frames: usize = parts.next().unwrap().parse().unwrap();
        let button = match parts.next() {
            None => None,
            Some("A") => Some(Button::A),
            Some("Down") => Some(Button::Down),
            Some("Start") => Some(Button::Start),
            other => panic!("unexpected prefix button {other:?}"),
        };
        assert!(parts.next().is_none(), "single-button prefix");
        run(&mut native, frames, button);
    }
    assert_eq!(commands.next(), Some("62 Right"), "fixture's first walk");
    let mut portable =
        World::enter_with_events(rom.image(), 0x0f, 304, 112, fresh_game_flags()).unwrap();
    finish_wakeup(&mut portable);
    assert!(native.wram(0x06c4) & 1 != 0, "native Elle grants $20");
    assert!(portable.events()[0x20 / 8] & 1 != 0);
    assert!(!portable.pad_locked());
    let position = |session: &Session| {
        let word = |offset| u16::from_le_bytes([session.wram(offset), session.wram(offset + 1)]);
        (word(0x047e), word(0x1000), word(0x1002))
    };
    for frame in 0..=62 {
        assert_eq!(
            (portable.map(), portable.position().0, portable.position().1),
            position(&native),
            "first held-Right walk boundary {frame}"
        );
        if frame < 62 {
            native.set_button(Button::Right, true);
            native.run_frame();
            portable
                .update(Some(Direction::Right), Presses::NONE)
                .unwrap();
        }
    }
    assert!(portable.position().0 > 304, "the held leg actually walks");
    eprintln!("EU first bedroom walk: 63 frame boundaries match");

    // Replay the fixture's natural doorway/load interval, but do not compare
    // its frames: this witness starts the next leg at the shared landing.
    for (expected, frames, button, direction) in [
        ("38", 38, None, None),
        ("67 Down", 67, Some(Button::Down), Some(Direction::Down)),
        ("83", 83, None, None),
    ] {
        assert_eq!(commands.next(), Some(expected), "fixture doorway command");
        run(&mut native, frames, button);
        for _ in 0..frames {
            portable.update(direction, Presses::NONE).unwrap();
        }
    }
    let anchor = (0x10, 392, 353);
    assert_eq!(position(&native), anchor, "native exterior anchor");
    assert_eq!(
        (portable.map(), portable.position().0, portable.position().1),
        anchor,
        "portable exterior anchor"
    );
    assert_eq!(commands.next(), Some("42 Down"), "fixture's exterior walk");
    for frame in 0..=42 {
        assert_eq!(
            (portable.map(), portable.position().0, portable.position().1),
            position(&native),
            "exterior held-Down walk boundary {frame}"
        );
        if frame < 42 {
            native.set_button(Button::Down, true);
            native.run_frame();
            portable
                .update(Some(Direction::Down), Presses::NONE)
                .unwrap();
        }
    }
    assert!(
        portable.position().1 > anchor.2,
        "the exterior leg actually walks"
    );
    eprintln!(
        "EU exterior held-Down walk: 43 frame boundaries match; anchor {anchor:?}, end {:?}",
        position(&native)
    );
}

fn finish_wakeup(world: &mut World<'_>) {
    for _ in 0..400 {
        if world.dialogue().is_some() {
            break;
        }
        world.update(None, Presses::NONE).unwrap();
    }
    assert!(world.dialogue().is_some(), "Elle speaks");
    let mut pages = 0;
    while world.in_scene() {
        for _ in 0..600 {
            if !world.typing() {
                break;
            }
            world.update(None, Presses::NONE).unwrap();
        }
        assert!(!world.typing());
        world
            .update(
                None,
                Presses {
                    confirm: true,
                    ..Presses::NONE
                },
            )
            .unwrap();
        pages += 1;
        assert!(pages < 20, "Elle's pages finish");
    }
    for _ in 0..600 {
        if !world.pad_locked() {
            break;
        }
        world.update(None, Presses::NONE).unwrap();
    }
}

fn run(session: &mut Session, frames: usize, held: Option<Button>) {
    for button in [Button::Start, Button::A, Button::Down, Button::Right] {
        session.set_button(button, held == Some(button));
    }
    for _ in 0..frames {
        session.run_frame();
    }
}
