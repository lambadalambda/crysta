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

#[test]
fn a_native_save_from_2008_decodes_and_resumes_at_the_desk() {
    // `local/saves/Terranigma.srm`: three European saves, two in the
    // bedroom `$0F`, one on the world map `$128`.
    use crysta_runtime::sram::Sram;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/saves/Terranigma.srm");
    let (Ok(bytes), Some(rom)) = (std::fs::read(path), load("Terranigma (E) [!].smc")) else {
        return;
    };
    let sram = Sram::from_bytes(&bytes).unwrap();
    let maps: Vec<u16> = (0..3).map(|n| sram.slot(n).unwrap().map()).collect();
    assert_eq!(maps, [0x0F, 0x0F, 0x128]);
    // As `save1.txt` lists them: level 1 at 0:18, level 7 at 1:21 and 1:43.
    let shown: Vec<(u8, u32)> = (0..3)
        .map(|n| sram.slot(n).unwrap())
        .map(|slot| (slot.level(), slot.seconds() / 60))
        .collect();
    assert_eq!(shown, [(1, 18), (7, 81), (7, 103)]);
    // Writing each slot back reproduces the file.
    let mut again = sram.clone();
    let last = sram.last_slot();
    for n in (0..3).filter(|&n| n != last).chain([last]) {
        again.write_slot(n, &sram.slot(n).unwrap());
    }
    assert_eq!(again.bytes()[..], bytes[..]);
    // The first resumes where the desk's save leaves Ark.
    let world = World::resume(rom.image(), &sram.slot(0).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        (world.map(), world.position(), world.facing()),
        (0x0F, (472, 176), Direction::Up)
    );
}

#[test]
fn a_new_game_slot_is_the_native_one_byte_for_byte() {
    // `local/saves/newgame-*.block`: the native block captured at `$87:8164`,
    // the default name confirmed (`docs/saves.md`, "New game").
    for (name, block) in [
        ("Tenchi Souzou (Japan).sfc", "newgame-jp.block"),
        ("Terranigma (E) [!].smc", "newgame-eu.block"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/saves")
            .join(block);
        let (Ok(native), Some(rom)) = (std::fs::read(path), load(name)) else {
            continue;
        };
        let slot = crysta_runtime::save::SaveSlot::new_game(rom.image());
        assert_eq!(slot.bytes()[..], native[..], "{name}");
        // A world that starts a game saves from that block.
        let world = World::enter(rom.image(), 0x0F, 472, 176).unwrap();
        let saved = world.save_slot();
        assert_eq!((saved.name(), saved.level()), (slot.name(), 1), "{name}");
        // The play clock: the first frame wraps into second 1, then 60 a
        // second.
        let mut world = world;
        for _ in 0..61 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert_eq!(world.save_slot().seconds(), 2, "{name}");
    }
}

#[test]
fn copy_and_erase_change_the_bytes_the_native_game_changes() {
    // `docs/restart-screen.md`: erasing the 2008 save's slot 2 changes 4
    // bytes; copying slot 1 over slot 3 of `local/restart/copy.srm`, 110.
    use crysta_runtime::sram::Sram;
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let (Ok(saves), Ok(copy)) = (
        std::fs::read(local.join("saves/Terranigma.srm")),
        std::fs::read(local.join("restart/copy.srm")),
    ) else {
        return;
    };
    let changed = |after: &Sram, before: &[u8]| {
        after
            .bytes()
            .iter()
            .zip(before)
            .filter(|(a, b)| a != b)
            .count()
    };
    let mut sram = Sram::from_bytes(&saves).unwrap().repaired();
    sram.erase(1);
    sram.repair();
    assert_eq!(changed(&sram, &saves), 4);
    let mut sram = Sram::from_bytes(&copy).unwrap().repaired();
    assert!(sram.copy_slot(0, 2));
    sram.repair();
    assert_eq!(changed(&sram, &copy), 110);
}
