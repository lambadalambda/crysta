//! Yomi's box, the hall (`docs/menu.md`), against the native captures.

use crysta_runtime::menu::{Event, Hall, Held, Menu, Page, Screen, View, ITEM_DOOR};
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

fn open(rom: &Rom) -> Menu<'_> {
    Menu::open(rom.image()).expect("the hall's tables")
}

const NONE: [Option<Direction>; 2] = [None, None];

/// Nothing held, nothing equipped.
const HELD: Held<'static> = Held {
    slots: &[],
    weapon: None,
    armor: None,
};

fn pad(direction: Direction) -> [Option<Direction>; 2] {
    [Some(direction), None]
}

/// Steps until the hall shows.
fn in_hall(menu: &mut Menu) -> Page {
    for _ in 0..80 {
        if let View::Screen(page) = menu.view() {
            return page;
        }
        menu.step(NONE, Presses::NONE, &HELD);
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
            menu.step(NONE, Presses::NONE, &HELD);
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
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        let xs: Vec<_> = (0..15)
            .map(|_| {
                menu.step(NONE, Presses::NONE, &HELD);
                let View::Screen(page) = menu.view() else {
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
        menu.step(pad(Direction::Down), Presses::NONE, &HELD);
        let label = |menu: &Menu| match menu.view() {
            View::Screen(page) => (page.hotspot, page.label),
            other => panic!("{other:?}"),
        };
        assert_eq!(label(&menu), (3, Some(7)));
        let mut seen = Vec::new();
        for _ in 0..17 {
            menu.step(pad(Direction::Down), Presses::NONE, &HELD);
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
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        menu.step(NONE, Presses::NONE, &HELD);
        menu.step(pad(Direction::Right), Presses::NONE, &HELD);
        menu.step(NONE, Presses::NONE, &HELD);
        assert_eq!(label(&menu).0, 6, "the tap at P+2");
        menu.step(pad(Direction::Right), Presses::NONE, &HELD);
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
            let event = menu.step(
                NONE,
                if frame == 0 { select() } else { Presses::NONE },
                &HELD,
            );
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
        menu.step(pad(Direction::Right), Presses::NONE, &HELD);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        assert_eq!(hotspot(&menu), 8);
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Left), Presses::NONE, &HELD);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[15], seen[16], seen[21]), (7, 7, 6, 6));
        // Down held from the item door: the jewel box at P, the memo at
        // P+16, then nothing below it at P+21.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE, &HELD);
        let mut seen = vec![hotspot(&menu)];
        for _ in 0..21 {
            menu.step(pad(Direction::Down), Presses::NONE, &HELD);
            seen.push(hotspot(&menu));
        }
        assert_eq!((seen[0], seen[16], seen[21]), (3, 0, 0));
        // Released and pressed again: it moves at once after the redraw.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE, &HELD);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        menu.step(pad(Direction::Up), Presses::NONE, &HELD);
        assert_eq!(hotspot(&menu), 7);
        // A diagonal: Up before Left.
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Down), Presses::NONE, &HELD);
        for _ in 0..4 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        menu.step(
            [Some(Direction::Left), Some(Direction::Up)],
            Presses::NONE,
            &HELD,
        );
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
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        // A on the weapon door at P.
        let mut events = vec![menu.step(NONE, press_a(), &HELD)];
        let mut pages = vec![page(&menu)];
        for _ in 0..180 {
            events.push(menu.step(NONE, Presses::NONE, &HELD));
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
        assert_eq!(first.yomi, (72, 56));
        assert_eq!(pages[label - 1].unwrap().label, None);
        assert_eq!(pages[label].unwrap().label, Some(0), "{:?}", rom.revision());
        assert_eq!(
            events.iter().flatten().collect::<Vec<_>>(),
            [&Event::Sort(Screen::Weapon)],
            "the slots are sorted once on entry"
        );
        // B: back to the hall door, which A takes back to the hall.
        menu.step(pad(Direction::Down), Presses::NONE, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        assert_eq!(page(&menu).unwrap().hotspot, 8);
        let b = Presses {
            cancel: true,
            ..Presses::NONE
        };
        menu.step(NONE, b, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        assert_eq!(page(&menu).unwrap().hotspot, 0);
        menu.step(NONE, press_a(), &HELD);
        for _ in 0..200 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        let back = page(&menu).unwrap();
        assert_eq!(
            (back.screen, back.hotspot, back.label),
            (Screen::Hall, 6, Some(6))
        );
    }
}

#[test]
fn the_armor_door_and_the_rooms_doors_lead_on() {
    // From the hall the armor room starts at its position 2; its door 1
    // leads to the weapon room's 1, whose door 0 leads to the hall's
    // weapon door, the label at P+162 (Japanese; the European 154 is not
    // measured).
    for rom in roms() {
        let japan = rom.revision() == Revision::Japan;
        let mut menu = open(&rom);
        in_hall(&mut menu);
        menu.step(pad(Direction::Right), Presses::NONE, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        let through = |menu: &mut Menu, frames: usize| {
            menu.step(NONE, press_a(), &HELD);
            (0..frames)
                .filter_map(|_| menu.step(NONE, Presses::NONE, &HELD))
                .collect::<Vec<_>>()
        };
        assert_eq!(through(&mut menu, 200), [Event::Sort(Screen::Armor)]);
        let at = |menu: &Menu| page(menu).map(|page| (page.screen, page.hotspot));
        assert_eq!(at(&menu), Some((Screen::Armor, 2)));
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        assert_eq!(through(&mut menu, 200), [Event::Sort(Screen::Weapon)]);
        assert_eq!(at(&menu), Some((Screen::Weapon, 1)));
        menu.step(pad(Direction::Left), Presses::NONE, &HELD);
        for _ in 0..20 {
            menu.step(NONE, Presses::NONE, &HELD);
        }
        menu.step(NONE, press_a(), &HELD);
        let label = if japan { 162 } else { 154 };
        let labels: Vec<_> = (0..label)
            .map(|_| {
                menu.step(NONE, Presses::NONE, &HELD);
                page(&menu).and_then(|page| page.label)
            })
            .collect();
        assert_eq!(labels[label - 2], None);
        assert_eq!(labels[label - 1], Some(6), "{:?}", rom.revision());
        assert_eq!(at(&menu), Some((Screen::Hall, 6)));
    }
}

/// A world in `$102` after the frozen return, holding `items`.
fn world_with<'a>(rom: &'a Rom, items: &[u8]) -> crysta_runtime::world::World<'a> {
    let mut events = crysta_runtime::world::fresh_game_flags();
    events[0xFE / 8] |= 1 << (0xFE % 8);
    let mut world =
        crysta_runtime::world::World::enter_with_events(rom.image(), 0x0102, 520, 460, events)
            .unwrap();
    for &item in items {
        world.give_item(item);
    }
    world
}

/// Opens the box and walks Yomi into the weapon room, onto its first
/// pedestal (position 4), where the sort puts the strongest weapon.
fn at_first_weapon(world: &mut crysta_runtime::world::World<'_>) {
    world.update(None, select()).unwrap();
    for _ in 0..80 {
        world.update(None, Presses::NONE).unwrap();
    }
    world.update(Some(Direction::Left), Presses::NONE).unwrap();
    for _ in 0..20 {
        world.update(None, Presses::NONE).unwrap();
    }
    world.update(None, press_a()).unwrap();
    for _ in 0..200 {
        world.update(None, Presses::NONE).unwrap();
    }
    for direction in [Direction::Down, Direction::Right, Direction::Down] {
        world.update(Some(direction), Presses::NONE).unwrap();
        for _ in 0..20 {
            world.update(None, Presses::NONE).unwrap();
        }
    }
    assert!(matches!(world.menu(), Some(View::Screen(page)) if page.hotspot == 4));
}

#[test]
fn a_pedestal_equips_its_weapon_and_unequips_it() {
    // `docs/menu.md`: the question from P+2, complete at P+19; A on its
    // first answer: `$064A` at P+1, the window closed and `$0659` at P+46
    // (Japanese, `$81`: 3) or P+68 (European, `$82`: 6); A again: the
    // unequip question, `$0659` = 0.
    for rom in roms() {
        let (item, power, close) = if rom.revision() == Revision::Japan {
            (0x81, 3, 46)
        } else {
            (0x82, 6, 68)
        };
        let mut world = world_with(&rom, &[item]);
        at_first_weapon(&mut world);
        world.update(None, press_a()).unwrap();
        for _ in 0..40 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert!(world.menu_dialogue_open(), "the question waits");
        world.update(None, press_a()).unwrap();
        assert_eq!(world.weapon_item(), None);
        world.update(None, Presses::NONE).unwrap();
        assert_eq!(world.weapon_item(), Some(item), "P+1");
        let powers: Vec<_> = (2..=close + 2)
            .map(|_| {
                world.update(None, Presses::NONE).unwrap();
                world.stats().weapon
            })
            .collect();
        // The European answer closes 2 frames sooner here than natively
        // (`meta/issues/box-equip-doors.md`).
        let arrived = powers.iter().position(|&p| p == power).map(|at| at + 2);
        let expected = if close == 46 { close } else { close - 2 };
        assert_eq!(arrived, Some(expected), "{:?}", rom.revision());
        assert!(!world.menu_dialogue_open());
        // Equipped: A asks to unequip; its first answer takes it off.
        for _ in 0..10 {
            world.update(None, Presses::NONE).unwrap();
        }
        world.update(None, press_a()).unwrap();
        for _ in 0..60 {
            world.update(None, Presses::NONE).unwrap();
        }
        world.update(None, press_a()).unwrap();
        for _ in 0..80 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert_eq!((world.weapon_item(), world.stats().weapon), (None, 0));
    }
}

#[test]
fn an_empty_pedestal_says_so() {
    // 何もないぜ！ from the press's second frame to its 20th, the label
    // again at the 22nd.
    for rom in roms() {
        let mut world = world_with(&rom, &[]);
        at_first_weapon(&mut world);
        world.update(None, press_a()).unwrap();
        let frames: Vec<_> = (1..=22)
            .map(|_| {
                world.update(None, Presses::NONE).unwrap();
                let label =
                    matches!(world.menu(), Some(View::Screen(page)) if page.label.is_some());
                (world.menu_dialogue_open(), label)
            })
            .collect();
        let at = |frame: usize| frames[frame - 1];
        assert_eq!(at(1), (false, false), "{:?}", rom.revision());
        assert_eq!(at(2), (true, false));
        assert_eq!(at(20), (true, false));
        assert_eq!(at(21), (false, false));
        assert_eq!(at(22), (false, true));
    }
}

#[test]
fn unequipping_clears_the_power_at_the_52nd_frame() {
    let Some(rom) = roms()
        .into_iter()
        .find(|rom| rom.revision() == Revision::Japan)
    else {
        return;
    };
    let mut world = world_with(&rom, &[0x81]);
    world.equip_weapon(0x81);
    at_first_weapon(&mut world);
    world.update(None, press_a()).unwrap();
    for _ in 0..60 {
        world.update(None, Presses::NONE).unwrap();
    }
    world.update(None, press_a()).unwrap();
    let powers: Vec<_> = (1..=53)
        .map(|_| {
            world.update(None, Presses::NONE).unwrap();
            world.stats().weapon
        })
        .collect();
    assert_eq!(world.weapon_item(), None);
    assert_eq!((powers[50], powers[51]), (3, 0), "P+51, P+52");
}

#[test]
fn the_armor_rooms_pedestal_puts_the_cloth_on() {
    // The armor door, then its position 4: cloth `$A0`, power 3.
    for rom in roms() {
        let mut world = world_with(&rom, &[0xA0]);
        world.update(None, select()).unwrap();
        for _ in 0..80 {
            world.update(None, Presses::NONE).unwrap();
        }
        world.update(Some(Direction::Right), Presses::NONE).unwrap();
        for _ in 0..20 {
            world.update(None, Presses::NONE).unwrap();
        }
        world.update(None, press_a()).unwrap();
        for _ in 0..200 {
            world.update(None, Presses::NONE).unwrap();
        }
        for direction in [
            Direction::Down,
            Direction::Left,
            Direction::Down,
            Direction::Left,
        ] {
            world.update(Some(direction), Presses::NONE).unwrap();
            for _ in 0..20 {
                world.update(None, Presses::NONE).unwrap();
            }
        }
        assert!(matches!(world.menu(), Some(View::Screen(page)) if page.hotspot == 4));
        world.update(None, press_a()).unwrap();
        for _ in 0..40 {
            world.update(None, Presses::NONE).unwrap();
        }
        world.update(None, press_a()).unwrap();
        for _ in 0..90 {
            world.update(None, Presses::NONE).unwrap();
        }
        assert_eq!(
            (world.armor(), world.stats().armor),
            (Some(0xA0), 3),
            "{:?}",
            rom.revision()
        );
    }
}
