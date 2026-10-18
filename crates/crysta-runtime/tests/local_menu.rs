//! Yomi's box, the hall (`docs/menu.md`), against the native captures.

use crysta_runtime::menu::{Event, Hall, Menu, Page, Screen, View, ITEM_DOOR};
use crysta_runtime::scene::Presses;
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
        if let View::Screen(page) = menu.view() {
            return page;
        }
        menu.step(NONE, Presses::NONE);
    }
    panic!("the hall never showed");
}

#[test]
fn the_hall_has_its_three_by_three_hotspots() {
    for rom in roms() {
        let hall = Hall::from_rom(rom.image()).unwrap();
        let places: Vec<_> = hall
            .hotspots(Screen::Hall)
            .iter()
            .map(|hotspot| hotspot.at)
            .collect();
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
        assert_eq!(
            hall.hotspots(Screen::Hall)[7].next,
            [None, Some(3), Some(6), Some(8)]
        );
    }
}

#[test]
fn the_hall_shows_at_once_after_the_fields_fade() {
    for rom in roms() {
        let mut menu = open(&rom);
        let mut views = vec![menu.view()];
        for _ in 0..61 {
            menu.step(NONE, Presses::NONE);
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
        assert!(matches!(views[hall], View::Screen(page) if page.hotspot == ITEM_DOOR));
    }
}

#[test]
fn yomi_eases_to_the_weapon_door() {
    // `docs/menu.md`: 136 → 72 over 15 frames from the press.
    for rom in roms() {
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Left), Presses::NONE);
        let xs: Vec<_> = (0..15)
            .map(|_| {
                menu.step(NONE, Presses::NONE);
                let View::Screen(page) = menu.view() else {
                    panic!()
                };
                page.yomi.unwrap().0
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
        menu.step(pad(Direction::Down), Presses::NONE);
        let label = |menu: &Menu| match menu.view() {
            View::Screen(page) => (page.hotspot, page.label),
            other => panic!("{other:?}"),
        };
        assert_eq!(label(&menu), (3, Some(7)));
        let mut seen = Vec::new();
        for _ in 0..17 {
            menu.step(pad(Direction::Down), Presses::NONE);
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
        menu.step(pad(Direction::Left), Presses::NONE);
        menu.step(NONE, Presses::NONE);
        menu.step(pad(Direction::Right), Presses::NONE);
        menu.step(NONE, Presses::NONE);
        assert_eq!(label(&menu).0, 6, "the tap at P+2");
        menu.step(pad(Direction::Right), Presses::NONE);
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
            let event = menu.step(NONE, if frame == 0 { select() } else { Presses::NONE });
            if event == Some(Event::Closed) && closed.is_none() {
                closed = Some(frame);
            }
            views.push(menu.view());
        }
        assert!(matches!(views[10], View::Screen(_)), "C+10");
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
            assert!(matches!(world.menu(), Some(View::Screen(page)) if page.hotspot == 7));
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
            View::Screen(page) => page.hotspot,
            other => panic!("{other:?}"),
        };
        // Left held from the armor door: the item door at P, the weapon
        // door at P+16; nothing further left.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Right), Presses::NONE);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE);
        }
        assert_eq!(hotspot(&menu), 8);
        menu.step(pad(Direction::Left), Presses::NONE);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Left), Presses::NONE);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[15], seen[16], seen[21]), (7, 7, 6, 6));
        // Down held from the item door: the jewel box at P, the memo at
        // P+16, then nothing below it at P+21.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Down), Presses::NONE);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[16], seen[21]), (3, 0, 0));
        // Released and pressed again: it moves at once after the redraw.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE);
        }
        menu.step(pad(Direction::Up), Presses::NONE);
        assert_eq!(hotspot(&menu), 7);
        // A diagonal: Up before Left.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE);
        }
        menu.step([Some(Direction::Left), Some(Direction::Up)], Presses::NONE);
        assert_eq!(hotspot(&menu), 7, "{:?}", rom.revision());
    }
}

fn press_a() -> Presses {
    Presses {
        confirm: true,
        ..Presses::NONE
    }
}

#[test]
fn the_rooms_have_their_twenty_positions() {
    for rom in roms() {
        let hall = Hall::from_rom(rom.image()).unwrap();
        for screen in [Screen::Weapon, Screen::Armor] {
            let positions = hall.hotspots(screen);
            let camera = screen.camera();
            let on_screen = |index: usize| {
                let (x, y) = positions[index].at;
                (x - camera.0, y - camera.1)
            };
            assert_eq!(positions.len(), 20);
            assert_eq!(on_screen(0), (72, 56));
            assert_eq!(on_screen(3), (132, 88), "the trash hole");
            assert_eq!(on_screen(19), (196, 170));
            assert_eq!(positions[4].next, [Some(9), Some(6), Some(12), Some(5)]);
        }
    }
}

/// The page while the menu shows a screen.
fn page(menu: &Menu) -> Option<Page> {
    match menu.view() {
        View::Screen(page) => Some(page),
        _ => None,
    }
}

#[test]
fn the_weapon_door_leads_into_the_weapon_room_and_back() {
    for rom in roms() {
        let japan = rom.revision() == Revision::Japan;
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Left), Presses::NONE);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE);
        }
        // A on the weapon door at P.
        let mut events = vec![menu.step(NONE, press_a())];
        let mut pages = vec![page(&menu)];
        for _ in 0..180 {
            events.push(menu.step(NONE, Presses::NONE));
            pages.push(page(&menu));
        }
        let (dark, label) = if japan {
            ((62, 114), 171)
        } else {
            ((61, 108), 163)
        };
        assert!(
            pages[dark.0 - 1].is_some() && pages[dark.0].is_none(),
            "{:?}",
            rom.revision()
        );
        assert!(pages[dark.1].is_none() && pages[dark.1 + 1].is_some());
        let first = pages[dark.1 + 1].unwrap();
        assert_eq!((first.screen, first.hotspot), (Screen::Weapon, 0));
        // At the left door from P+99, in the window opening on her.
        assert_eq!(first.yomi, Some((72, 56)));
        assert_eq!(pages[label - 1].unwrap().label, None);
        assert_eq!(pages[label].unwrap().label, Some(0), "{:?}", rom.revision());
        assert_eq!(
            events.iter().flatten().collect::<Vec<_>>(),
            [&Event::Sort(Screen::Weapon)],
            "the slots are sorted once on entry"
        );
        // B: back to the hall door, which A takes back to the hall.
        menu.step(pad(Direction::Down), Presses::NONE);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE);
        }
        assert_eq!(page(&menu).unwrap().hotspot, 8);
        menu.step(
            NONE,
            Presses {
                cancel: true,
                ..Presses::NONE
            },
        );
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE);
        }
        assert_eq!(page(&menu).unwrap().hotspot, 0);
        menu.step(NONE, press_a());
        for _ in 0..200 {
            menu.step(NONE, Presses::NONE);
        }
        let back = page(&menu).unwrap();
        assert_eq!(
            (back.screen, back.hotspot, back.label),
            (Screen::Hall, 6, Some(6))
        );
    }
}
