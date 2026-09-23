//! The Crysta story slice, played through the world with real presses and
//! compared against the native input-only route.
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, World};
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn owned_rom() -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == Revision::Japan).then_some(rom)
}

const A: Presses = Presses {
    confirm: true,
    cancel: false,
    up: false,
    down: false,
};

fn flag(world: &World<'_>, flag: usize) -> bool {
    world.events()[flag / 8] & (1 << (flag % 8)) != 0
}

/// Frames until `done`, each one neutral.
fn frames_until(world: &mut World<'_>, limit: u32, done: impl Fn(&World<'_>) -> bool) -> u32 {
    for frame in 0..limit {
        if done(world) {
            return frame;
        }
        world.update(None, Presses::default()).unwrap();
    }
    panic!("not within {limit} frames");
}

const ELLE: usize = 0x03_8D36;

#[test]
fn elle_wakes_ark_then_walks_out() {
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let mut world = World::enter_with_events(image, 0x000F, 304, 112, fresh_game_flags()).unwrap();
    let elle = |world: &World<'_>| world.residents().iter().find(|r| r.record == ELLE).cloned();
    let at_start = elle(&world).expect("Elle is in the bedroom before the intro");
    assert!(at_start.body, "and drawn");
    assert_eq!(at_start.position, (328, 112));

    // She locks the pad at once, waits 120 frames, then speaks.
    world
        .update(Some(Direction::Down), Presses::default())
        .unwrap();
    assert_eq!(
        world.position(),
        (304, 112),
        "COP 2A $FF50 from the first frame"
    );
    let silent = frames_until(&mut world, 400, |world| world.dialogue().is_some());
    assert!((115..=125).contains(&silent), "spoke after {silent} frames");
    assert!(world.in_scene(), "COP 1F holds the world");

    // Five pages over three requests, each acknowledged with A.
    let mut pages = 0;
    while world.in_scene() {
        world.update(None, A).unwrap();
        pages += 1;
        assert!(pages < 20);
    }
    assert_eq!(pages, 5);
    assert!(flag(&world, 0x20), "set right after the last page");

    // She walks out while the pad stays locked for ten 32-frame steps, then
    // unlocks it; natively 5903 -> 6223.
    assert!(world.pad_locked());
    let locked = frames_until(&mut world, 600, |world| !world.pad_locked());
    assert!(
        (318..=322).contains(&locked),
        "unlocked after {locked} frames"
    );
    // Right, as the native control probe did: the bed is below. Walking
    // starts a few frames after the press.
    let (mut probe, before) = (world.clone(), world.position());
    for _ in 0..12 {
        probe
            .update(Some(Direction::Right), Presses::default())
            .unwrap();
    }
    assert_ne!(probe.position(), before, "Ark walks once unlocked");
    // Two more down steps to (392,256), then she deletes herself.
    let gone = frames_until(&mut world, 200, |world| elle(world).is_none());
    assert!(
        (94..=98).contains(&gone),
        "deleted {gone} frames after the unlock"
    );
    assert_eq!(world.items(), [0x7A, 0xA0], "she gave Ark two items");
}

#[test]
fn the_box_rooms_figure_stays_hidden_until_its_flag() {
    // `$83:9271` in `$21` hides itself (entity +$04 bit 15) and waits for
    // flag `$03`, as natively on the Pandora route: not drawn, not blocking.
    let Some(cartridge) = owned_rom() else {
        return;
    };
    let image = cartridge.image();
    let events = crysta_runtime::world::new_game_flags();
    let mut world = World::enter_with_events(image, 0x0021, 136, 368, events).unwrap();
    world.update(None, Presses::default()).unwrap();
    let figure = world
        .residents()
        .iter()
        .find(|r| r.record == 0x03_9271)
        .expect("present while $101 is clear");
    assert!(figure.body && figure.hidden);
    let (column, row) = figure.collision_cell();
    let cell = world.room().cells()
        [usize::from(row) * usize::from(world.dimensions().0) + usize::from(column)];
    // A body's cell is written solid (attribute 14); this one stays floor.
    assert_ne!(cell >> 9, 14, "a hidden body does not occupy its cell");
}
