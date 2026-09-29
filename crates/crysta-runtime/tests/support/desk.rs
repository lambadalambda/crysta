//! Saving at the bedroom desk through the world, and the native game
//! loading the SRAM it wrote; shared by the Records tests.
use crysta_runtime::records::{Page, Text, View};
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{new_game_flags, World};
use oracle::{Button, Session};
use rom::Rom;
use room_core::Direction;
use std::path::Path;

pub fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    Rom::load(&std::fs::read(path).ok()?).ok()
}

const A: Presses = Presses {
    confirm: true,
    ..Presses::NONE
};

fn frame(world: &mut World<'_>, presses: Presses) {
    world.update(None, presses).unwrap();
}

/// Stands at the desk facing up, opens the screen, saves to slot 1 and
/// waits for the room; returns the world.
pub fn save_at_desk(image: &[u8]) -> World<'_> {
    let mut world = World::enter_with_events(image, 0x0F, 472, 176, new_game_flags()).unwrap();
    world.face(Direction::Up);
    frame(&mut world, Presses::NONE);
    frame(&mut world, A);
    assert_eq!(world.records(), Some(View::Room(15)), "opened");
    while !matches!(world.records(), Some(View::Screen(_))) {
        frame(&mut world, Presses::NONE);
    }
    assert!(matches!(
        world.records(),
        Some(View::Screen(Page {
            cursor: 0,
            title: true,
            text: Text::Entry
        }))
    ));
    frame(&mut world, Presses::NONE);
    frame(&mut world, A);
    let mut frames = 0;
    while world.records().is_some() {
        frame(&mut world, Presses::NONE);
        frames += 1;
    }
    assert!(frames > 280, "the jingle's wait");
    world
}

/// Boots the native game with `sram`, presses Start on the title and A on
/// the file select's first slot, and returns the session once it has
/// loaded. One session a process: each ROM runs in its own test binary.
pub fn native_load(rom: &Rom, sram: &[u8]) -> Session {
    let mut session = Session::new_with_sram(rom, sram).unwrap();
    for (button, frames) in [
        (None, 600),
        (Some(Button::Start), 10),
        (None, 290),
        (Some(Button::A), 1),
        (None, 200),
    ] {
        if let Some(button) = button {
            session.set_button(button, true);
        }
        session.run_frames(frames);
        if let Some(button) = button {
            session.set_button(button, false);
        }
    }
    session
}

/// Checks that the native game loaded the desk's slot.
pub fn assert_loaded_at_desk(session: &Session, name: &str) {
    let word = |at: usize| u16::from_le_bytes([session.wram(at), session.wram(at + 1)]);
    assert_eq!(
        (word(0x047E), word(0x1000), word(0x1002)),
        (0x0F, 472, 176),
        "{name}: the native game loads the slot at the desk"
    );
}

/// Checks slot 1 as the desk's save wrote it.
pub fn assert_desk_slot(world: &World<'_>, name: &str) {
    let slot = world.sram().slot(0).expect("slot 1 written");
    assert_eq!(
        (slot.map(), slot.position(), slot.facing()),
        (0x0F, (472, 176), 1),
        "{name}"
    );
    assert_eq!(
        slot.events()[0xFB / 8] & 0x18,
        0,
        "{name}: flags `$FB`, `$FC` clear"
    );
    assert!(world.dialogue().is_none(), "{name}");
}
