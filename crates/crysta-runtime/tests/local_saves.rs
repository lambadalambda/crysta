//! The native save slot through the world (`docs/saves.md`).
use crysta_runtime::scene::Presses;
use crysta_runtime::world::World;
use rom::Rom;
use room_core::Direction;
use std::path::Path;

fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    Rom::load(&std::fs::read(path).ok()?).ok()
}

#[test]
fn a_saved_world_resumes_where_it_stood_with_its_flags_and_items() {
    for name in ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"] {
        let Some(rom) = load(name) else {
            continue;
        };
        let image = rom.image();
        let mut events = crysta_runtime::world::new_game_flags();
        events[0x26 / 8] |= 1 << (0x26 % 8);
        let mut world = World::enter_with_events(image, 0x000A, 538, 815, events).unwrap();
        world.give_money(60);
        world.face(Direction::Left);
        for _ in 0..30 {
            world.update(None, Presses::default()).unwrap();
        }
        let slot = world.save_slot();
        assert_eq!(slot.map(), 0x000A, "{name}");
        let resumed = World::resume(image, &slot)
            .unwrap()
            .expect("flag $20 is set");
        assert_eq!(
            (resumed.map(), resumed.position(), resumed.facing()),
            (world.map(), world.position(), world.facing()),
            "{name}"
        );
        assert_eq!(resumed.money(), 60);
        assert_eq!(&resumed.events()[..0x140], &world.events()[..0x140]);
        // The resumed world captures the same slot again.
        assert_eq!(resumed.save_slot(), slot, "{name}");
    }
}

#[test]
fn a_slot_before_the_wake_up_resumes_nothing() {
    // Flag `$20` clear: the native load goes to the prologue.
    let Some(rom) = load("Tenchi Souzou (Japan).sfc") else {
        return;
    };
    let slot = crysta_runtime::save::SaveSlot::default();
    assert!(World::resume(rom.image(), &slot).unwrap().is_none());
}
