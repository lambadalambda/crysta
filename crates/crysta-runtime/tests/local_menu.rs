//! Yomi's box, the hall (`docs/menu.md`), against the native captures.

use crysta_runtime::menu::{Hall, Menu, Page, View, ITEM_DOOR};
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn load(name: &str) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Rom::load(&bytes).ok()
}

fn roms() -> Vec<Rom> {
    ["Tenchi Souzou (Japan).sfc", "Terranigma (E) [!].smc"]
        .into_iter()
        .filter_map(load)
        .collect()
}

fn open(rom: &Rom) -> Menu {
    Menu::open(rom.image()).expect("the hall's tables")
}

const NONE: [Option<Direction>; 2] = [None, None];

fn pad(direction: Direction) -> [Option<Direction>; 2] {
    [Some(direction), None]
}

/// Steps until the hall shows.
fn in_hall(menu: &mut Menu) -> Page {
    for _ in 0..80 {
        if let View::Hall(page) = menu.view() {
            return page;
        }
        menu.step(NONE, false);
    }
    panic!("the hall never showed");
}

#[test]
fn the_hall_has_its_three_by_three_hotspots() {
    for rom in roms() {
        let hall = Hall::from_rom(rom.image()).unwrap();
        let places: Vec<_> = hall.hotspots.iter().map(|hotspot| hotspot.at).collect();
        assert_eq!(
            places,
            [
                (136, 136),
                (88, 136),
                (184, 136),
                (136, 104),
                (64, 104),
                (208, 104),
                (72, 56),
                (136, 56),
                (200, 56)
            ],
            "{:?}",
            rom.revision()
        );
        // The item door: Down to the jewel box, Left the weapon door, Right
        // the armor door, no Up.
        assert_eq!(hall.hotspots[7].next, [None, Some(3), Some(6), Some(8)]);
    }
}

#[test]
fn the_hall_shows_at_once_after_the_fields_fade() {
    for rom in roms() {
        let mut menu = open(&rom);
        let mut views = vec![menu.view()];
        for _ in 0..61 {
            menu.step(NONE, false);
            views.push(menu.view());
        }
        let hall = if rom.revision() == Revision::Japan {
            61
        } else {
            58
        };
        assert_eq!(views[19], View::Field(15));
        assert_eq!(views[20], View::Field(14));
        assert_eq!(views[34], View::Field(0));
        assert_eq!(views[35], View::Blank);
        assert_eq!(views[hall - 1], View::Blank, "{:?}", rom.revision());
        assert!(matches!(views[hall], View::Hall(page) if page.hotspot == ITEM_DOOR));
    }
}

#[test]
fn yomi_eases_to_the_weapon_door() {
    // `docs/menu.md`: 136 → 72 over 15 frames from the press.
    for rom in roms() {
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Left), false);
        let xs: Vec<_> = (0..15)
            .map(|_| {
                menu.step(NONE, false);
                let View::Hall(page) = menu.view() else {
                    panic!()
                };
                page.yomi.0
            })
            .collect();
        assert_eq!(
            xs,
            [135, 132, 129, 124, 118, 111, 104, 98, 91, 85, 80, 76, 73, 73, 72],
            "{:?}",
            rom.revision()
        );
    }
}

#[test]
fn the_label_is_redrawn_and_a_held_direction_repeats() {
    for rom in roms() {
        let mut menu = open(&rom);
        in_hall(&mut menu);
        // Down from the item door at P: the jewel box.
        menu.step(pad(Direction::Down), false);
        let label = |menu: &Menu| match menu.view() {
            View::Hall(page) => (page.hotspot, page.label),
            other => panic!("{other:?}"),
        };
        assert_eq!(label(&menu), (3, Some(7)));
        let mut seen = Vec::new();
        for _ in 0..17 {
            menu.step(pad(Direction::Down), false);
            seen.push(label(&menu));
        }
        assert_eq!(seen[0], (3, Some(7)), "P+1");
        assert_eq!(seen[1], (3, None), "P+2");
        assert_eq!(seen[3], (3, Some(3)), "P+4");
        // Held, it moves again at P+16: to the settings memo.
        assert_eq!(seen[14], (3, Some(3)), "P+15");
        assert_eq!(seen[15].0, 0, "P+16");
        // A tap at P+2 is lost; at P+4 a new direction is taken.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Left), false);
        menu.step(NONE, false);
        menu.step(pad(Direction::Right), false);
        menu.step(NONE, false);
        assert_eq!(label(&menu).0, 6, "the tap at P+2");
        menu.step(pad(Direction::Right), false);
        assert_eq!(label(&menu).0, 7, "P+4");
    }
}

#[test]
fn select_closes_and_the_field_fades_back() {
    for rom in roms() {
        let mut menu = open(&rom);
        in_hall(&mut menu);
        let japan = rom.revision() == Revision::Japan;
        let mut views = Vec::new();
        let mut closed = None;
        for frame in 0..70u16 {
            if menu.step(NONE, frame == 0) && closed.is_none() {
                closed = Some(frame);
            }
            views.push(menu.view());
        }
        assert!(matches!(views[10], View::Hall(_)), "C+10");
        assert_eq!(views[11], View::Blank, "C+11");
        let back = if japan { 35 } else { 32 };
        assert_eq!(views[back - 1], View::Blank);
        assert_eq!(views[back], View::Field(1));
        assert_eq!(views[back + 14], View::Field(15));
        assert_eq!(closed, Some(if japan { 57 } else { 54 }));
    }
}

fn select() -> crysta_runtime::scene::Presses {
    crysta_runtime::scene::Presses {
        select: true,
        ..crysta_runtime::scene::Presses::NONE
    }
}

#[test]
fn select_opens_the_box_once_the_frozen_return_is_past() {
    use crysta_runtime::scene::Presses;
    use crysta_runtime::world::{fresh_game_flags, World};
    for rom in roms() {
        for frozen_return in [false, true] {
            let mut events = fresh_game_flags();
            if frozen_return {
                events[0xFE / 8] |= 1 << (0xFE % 8);
            }
            let mut world =
                World::enter_with_events(rom.image(), 0x0102, 520, 460, events).unwrap();
            for _ in 0..10 {
                world.update(None, Presses::NONE).unwrap();
            }
            world.update(None, select()).unwrap();
            assert_eq!(
                world.menu().is_some(),
                frozen_return,
                "{:?}",
                rom.revision()
            );
            if !frozen_return {
                continue;
            }
            // The field waits: Ark does not walk.
            let before = world.position();
            for _ in 0..80 {
                world.update(Some(Direction::Up), Presses::NONE).unwrap();
            }
            assert_eq!(world.position(), before);
            assert!(matches!(world.menu(), Some(View::Hall(page)) if page.hotspot == 7));
            world.update(None, select()).unwrap();
            for _ in 0..60 {
                world.update(None, Presses::NONE).unwrap();
            }
            assert_eq!(world.menu(), None, "{:?}", rom.revision());
            for _ in 0..4 {
                world.update(Some(Direction::Up), Presses::NONE).unwrap();
            }
            assert_ne!(world.position(), before, "the field runs again");
        }
    }
}

#[test]
fn held_releases_and_diagonals_move_as_natively() {
    for rom in roms() {
        let hotspot = |menu: &Menu| match menu.view() {
            View::Hall(page) => page.hotspot,
            other => panic!("{other:?}"),
        };
        // Left held from the armor door: the item door at P, the weapon
        // door at P+16; nothing further left.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Right), false);
        for _ in 0..4 {
            menu.step(NONE, false);
        }
        assert_eq!(hotspot(&menu), 8);
        menu.step(pad(Direction::Left), false);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Left), false);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[15], seen[16], seen[21]), (7, 7, 6, 6));
        // Down held from the item door: the jewel box at P, the memo at
        // P+16, then nothing below it at P+21.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), false);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Down), false);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[16], seen[21]), (3, 0, 0));
        // Released and pressed again: it moves at once after the redraw.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), false);
        for _ in 0..4 {
            menu.step(NONE, false);
        }
        menu.step(pad(Direction::Up), false);
        assert_eq!(hotspot(&menu), 7);
        // A diagonal: Up before Left.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), false);
        for _ in 0..4 {
            menu.step(NONE, false);
        }
        menu.step([Some(Direction::Left), Some(Direction::Up)], false);
        assert_eq!(hotspot(&menu), 7, "{:?}", rom.revision());
    }
}
