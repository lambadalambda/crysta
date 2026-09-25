//! The slice on the European English ROM (ADR 0004).
use assets::maps::exits::ExitList;
use assets::maps::scripts::EventFlags;
use crysta_runtime::art::{residents_art, Animation, ArkAtlas, Body, Placeholder};
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, Step, World};
use crysta_runtime::{BOX_MAPS, MAPS};
use rom::{Revision, Rom};
use room_core::Direction;
use std::path::Path;

fn load(name: &str, revision: Revision) -> Option<Rom> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(name);
    let rom = Rom::load(&std::fs::read(path).ok()?).ok()?;
    (rom.revision() == revision).then_some(rom)
}

fn european() -> Option<Rom> {
    load("Terranigma (E) [!].smc", Revision::EuropeEnglish)
}

fn japanese() -> Option<Rom> {
    load("Tenchi Souzou (Japan).sfc", Revision::Japan)
}

#[test]
fn the_european_wake_up_releases_ark_after_elles_dialogue() {
    let Some(rom) = european() else {
        return;
    };
    let mut world =
        World::enter_with_events(rom.image(), 0x000F, 304, 112, fresh_game_flags()).unwrap();
    for _ in 0..400 {
        if world.dialogue().is_some() {
            break;
        }
        world.update(None, Presses::NONE).unwrap();
    }
    assert!(world.dialogue().is_some(), "Elle must speak in the bedroom");
    let mut acknowledgements = 0;
    while world.in_scene() {
        for _ in 0..600 {
            if !world.typing() {
                break;
            }
            world.update(None, Presses::NONE).unwrap();
        }
        assert!(!world.typing(), "English page must finish typing");
        world
            .update(
                None,
                Presses {
                    confirm: true,
                    ..Presses::NONE
                },
            )
            .unwrap();
        acknowledgements += 1;
        assert!(acknowledgements < 20, "European dialogue did not finish");
    }
    assert_ne!(world.events()[0x20 / 8] & (1 << (0x20 % 8)), 0);
    for _ in 0..600 {
        if !world.pad_locked() {
            break;
        }
        world.update(None, Presses::NONE).unwrap();
    }
    assert!(!world.pad_locked(), "Ark can walk after Elle leaves");
    assert_eq!(world.items(), [0x7A, 0xA0]);
    assert!(world.frozen_scripts().is_empty());
    for _ in 0..62 {
        world.update(Some(Direction::Right), Presses::NONE).unwrap();
    }
    for _ in 0..38 {
        world.update(None, Presses::NONE).unwrap();
    }
    for _ in 0..67 {
        world.update(Some(Direction::Down), Presses::NONE).unwrap();
    }
    for _ in 0..83 {
        world.update(None, Presses::NONE).unwrap();
    }
    assert_eq!(world.map(), 0x0010, "leave through the bedroom doorway");
    assert_eq!(world.position(), (392, 353), "the native doorway landing");
}

#[test]
fn the_european_weaver_grants_28_after_the_first_answer() {
    let Some(rom) = european() else {
        return;
    };
    let mut events = fresh_game_flags();
    events[0x26 / 8] |= 1 << (0x26 % 8);
    let mut world = World::enter_with_events(rom.image(), 0x0013, 360, 144, events).unwrap();
    world.face(Direction::Up);
    let mut talked = false;
    let mut chose = false;
    for frame in 0..4000 {
        let choosing = world.dialogue().and_then(|view| view.cursor).is_some();
        let reading = world.dialogue().is_some() || world.in_scene();
        let confirm = if choosing {
            chose = true;
            true
        } else if reading && !world.typing() && frame % 2 == 0 {
            true
        } else if !reading && !talked && frame > 50 {
            talked = true;
            true
        } else {
            false
        };
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
        assert!(world.frozen_scripts().is_empty());
        if chose && !world.in_scene() && world.dialogue().is_none() {
            break;
        }
    }
    assert!(chose, "the European choice opened");
    assert_ne!(world.events()[0x28 / 8] & (1 << (0x28 % 8)), 0);
}

#[test]
fn the_bedroom_loads_and_runs() {
    let Some(rom) = european() else {
        return;
    };
    let mut world =
        World::enter_with_events(rom.image(), 0x000F, 304, 112, fresh_game_flags()).unwrap();
    for _ in 0..600 {
        world.update(None, Presses::default()).unwrap();
    }
    assert!(
        world.frozen_scripts().is_empty(),
        "{:x?}",
        world.frozen_scripts()
    );
}

/// Where the Japanese exits send the player into `map`, or a fallback for
/// the box's tour, which script transfers reach.
fn arrival(image: &[u8], map: u16) -> (u16, u16) {
    MAPS.chain(BOX_MAPS)
        .filter_map(|source| ExitList::from_rom(image, source).ok())
        .flat_map(|list| list.records().to_vec())
        .find(|record| record.direct_destination().ok() == Some(map))
        .map_or((128, 128), |record| record.destination_position())
}

/// What a player sees of a world, without its text: the residents (where,
/// whether a body, the pose, hidden), the collision, the patches, the
/// flags, and whether input is locked, a scene runs or text is shown.
fn state(world: &World<'_>) -> impl PartialEq + std::fmt::Debug {
    let residents: Vec<_> = world
        .residents()
        .iter()
        .map(|resident| {
            (
                resident.position,
                resident.body,
                resident.selector,
                resident.hidden,
            )
        })
        .collect();
    (
        residents,
        world.room().clone(),
        world.patched_cells().to_vec(),
        world.events().to_vec(),
        (
            world.pad_locked(),
            world.in_scene(),
            world.dialogue().is_some(),
        ),
    )
}

#[test]
fn every_slice_map_loads_and_runs_as_the_japanese_one() {
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    for map in MAPS.chain(BOX_MAPS) {
        let (x, y) = arrival(japan.image(), map);
        let enter = |image| World::enter_with_events(image, map, x, y, fresh_game_flags());
        let mut eu = enter(europe.image()).unwrap_or_else(|e| panic!("{map:#x}: {e}"));
        let mut jp = enter(japan.image()).unwrap();
        assert_eq!(state(&eu), state(&jp), "{map:#x} on entry");
        // The cues, but for the typing blips: the English pages are longer.
        let (mut eu_cues, mut jp_cues) = (Vec::new(), Vec::new());
        for frame in 0..300 {
            for (world, cues) in [(&mut eu, &mut eu_cues), (&mut jp, &mut jp_cues)] {
                let typing = world.typing();
                world.update(None, Presses::default()).unwrap();
                let frame_cues = world.take_cues();
                if !typing {
                    cues.extend(frame_cues.into_iter().map(|cue| (frame, cue)));
                }
            }
        }
        assert_eq!(eu_cues, jp_cues, "{map:#x} cues");
        assert_eq!(
            eu.frozen_scripts().len(),
            jp.frozen_scripts().len(),
            "{map:#x}: {:x?}",
            eu.frozen_scripts()
        );
        assert_eq!(state(&eu), state(&jp), "{map:#x} after 300 frames");
    }
}

/// A body's first pose, unmirrored, as rasters; or why there is none.
fn first_pose(body: &Result<Body, Placeholder>) -> Result<Animation, String> {
    match body {
        Ok(body) => body
            .animation(body.initial(), false)
            .map_err(|e| format!("{e:?}")),
        Err(placeholder) => Err(format!("{placeholder:?}")),
    }
}

#[test]
fn the_slice_art_is_the_japanese_art() {
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    let (eu, jp) = (europe.image(), japan.image());
    let ark = |image| format!("{:?}", ArkAtlas::from_rom(image).unwrap());
    assert_eq!(ark(eu), ark(jp));
    for map in MAPS.chain(BOX_MAPS) {
        let (x, y) = arrival(jp, map);
        let bodies = |image| {
            let world = World::enter_with_events(image, map, x, y, fresh_game_flags()).unwrap();
            let flags = EventFlags::Bitmap(world.events());
            residents_art(image, map, world.residents(), flags, flags)
                .iter()
                .map(first_pose)
                .collect::<Vec<_>>()
        };
        let (eu, jp) = (bodies(eu), bodies(jp));
        assert_eq!(eu.len(), jp.len(), "{map:#x}");
        for (index, (eu, jp)) in eu.iter().zip(&jp).enumerate() {
            assert!(
                eu == jp,
                "{map:#x} resident {index}: {:?}",
                eu.as_ref().err()
            );
        }
    }
}

#[test]
fn the_pandora_art_is_the_japanese_art_under_the_japanese_keys() {
    use assets::sprites::{PandoraArt, PandoraCarryMotion, PandoraRunMotion, PandoraSprites};
    use crysta_runtime::art::CarryArt;
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    let (eu, jp) = (
        PandoraSprites::from_rom(europe.image()).unwrap(),
        PandoraSprites::from_rom(japan.image()).unwrap(),
    );
    // The phases and motions name their sources by the Japanese addresses.
    assert_eq!(format!("{:?}", eu.phases()), format!("{:?}", jp.phases()));
    assert_eq!(format!("{:?}", eu.motions()), format!("{:?}", jp.motions()));
    let ids = |sprites: &PandoraSprites| {
        sprites
            .art()
            .iter()
            .map(PandoraArt::source_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&eu), ids(&jp));
    let (eu_art, jp_art) = (
        CarryArt::from_rom(europe.image()).unwrap(),
        CarryArt::from_rom(japan.image()).unwrap(),
    );
    let same = |art: u32, selector: u8, hflip: bool| {
        let eu = eu_art.animation(art, selector, hflip);
        let jp = jp_art.animation(art, selector, hflip).unwrap();
        assert!(
            eu.as_ref().is_ok_and(|eu| *eu == jp),
            "{art:#x} list {selector} {hflip}: {:?}",
            eu.err()
        );
    };
    for art in jp.art() {
        for list in art.lists() {
            for hflip in [false, true] {
                same(art.source_id(), list.selector(), hflip);
            }
        }
    }
    for facing in 0..4 {
        for motion in [PandoraRunMotion::Dashing, PandoraRunMotion::Braking] {
            let (art, selector, hflip) = PandoraSprites::run_pose(motion, facing).unwrap();
            same(art, selector, hflip);
        }
        for motion in [
            PandoraCarryMotion::Lifting,
            PandoraCarryMotion::Standing,
            PandoraCarryMotion::Walking,
            PandoraCarryMotion::Throwing,
        ] {
            let pose = PandoraSprites::carry_pose(motion, facing).unwrap();
            same(pose.ark_art, pose.ark_selector, pose.ark_hflip);
            for pot in [0x96_E1A6, 0x96_E1AB] {
                same(pot, pose.pot_selector, pose.pot_hflip);
            }
        }
    }
}

/// The item shop in `$1E` as `local_story.rs` plays it: browse, a refusal
/// for want of money, a purchase, leaving. What the player sees of it but
/// the text and the display's place (the European spawner sets the talk
/// target 8 pixels higher), and where each page shown ends.
fn shop_story(image: &[u8]) -> (Vec<String>, Vec<u32>) {
    use crysta_runtime::audio::Cue;
    use crysta_runtime::shop::Shop;
    use room_core::Direction;
    let a = Presses {
        confirm: true,
        ..Presses::NONE
    };
    let mut world = World::enter(image, 0x001E, 632, 144).unwrap();
    world
        .update(Some(Direction::Up), Presses::default())
        .unwrap();
    for _ in 0..200 {
        if !world.in_transition() {
            break;
        }
        world.update(None, Presses::default()).unwrap();
    }
    world.take_cues();
    let (mut log, mut pages) = (Vec::new(), Vec::new());
    let step = |world: &mut World<'_>, presses: Presses, pages: &mut Vec<u32>| {
        world.update(None, presses).unwrap();
        if let Some(view) = world.dialogue() {
            let end = view.page.boundary_source();
            if pages.last() != Some(&end) {
                pages.push(end);
            }
        }
    };
    // Frames until the shop browses, acknowledging each typed page with A
    // (the confirm's cursor starts on "buy").
    let browse = |world: &mut World<'_>, pages: &mut Vec<u32>| {
        for _ in 0..2000 {
            if world.shop().is_some_and(Shop::browsing) {
                return;
            }
            let waiting = world.dialogue().is_some() && !world.typing();
            step(world, if waiting { a } else { Presses::NONE }, pages);
        }
        panic!("the shop does not browse");
    };
    let observe = |world: &mut World<'_>, log: &mut Vec<String>| {
        let sounds: Vec<_> = world
            .take_cues()
            .into_iter()
            .filter(|cue| !matches!(cue, Cue::Sound(0x2800 | 0x2500)))
            .collect();
        let display = world.shop().and_then(Shop::display);
        log.push(format!(
            "{:?} {:?} {} {:?} {:?}",
            world.shop().and_then(Shop::showing),
            display.map(|d| (d.item, d.quantity, d.price, d.dim, d.holding)),
            world.money(),
            world.items(),
            sounds
        ));
    };
    step(&mut world, a, &mut pages);
    assert!(world.in_scene(), "the shop holds the world");
    browse(&mut world, &mut pages);
    observe(&mut world, &mut log);
    for presses in [
        Presses {
            right: true,
            ..Presses::NONE
        },
        Presses {
            up: true,
            ..Presses::NONE
        },
    ] {
        step(&mut world, presses, &mut pages);
    }
    observe(&mut world, &mut log);
    // No money: the refusal, then the help again.
    step(&mut world, a, &mut pages);
    browse(&mut world, &mut pages);
    observe(&mut world, &mut log);
    // With 60 the two cost 50: confirm, buy, thanks.
    world.give_money(60);
    step(&mut world, a, &mut pages);
    browse(&mut world, &mut pages);
    observe(&mut world, &mut log);
    step(
        &mut world,
        Presses {
            cancel: true,
            ..Presses::NONE
        },
        &mut pages,
    );
    for _ in 0..600 {
        if !world.in_scene() {
            break;
        }
        world.update(None, Presses::default()).unwrap();
    }
    assert!(world.shop().is_none(), "Ark leaves");
    observe(&mut world, &mut log);
    (log, pages)
}

#[test]
fn the_item_shop_sells_as_the_japanese_one_with_english_texts() {
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    let (eu, jp) = (shop_story(europe.image()), shop_story(japan.image()));
    assert_eq!(eu.0, jp.0);
    // The pages are the texts the shop code requests (`COP 1C` in
    // `$92:CD70`, European `$92:E3D4..E599`), each decoding whole: the
    // greeting, the help, "no money", the help, the confirm, the thanks,
    // the help.
    let texts = |image, texts: [u32; 7]| {
        let mut ends: Vec<u32> = Vec::new();
        for text in texts {
            let pages = assets::text::HouseDialogue::decode_reading(image, text, |address| {
                (address == 0x0DE8).then_some(0)
            })
            .unwrap();
            ends.extend(
                pages
                    .iter()
                    .map(assets::text::DialoguePage::boundary_source),
            );
        }
        ends.dedup();
        ends
    };
    let (greeting, help, money, confirm, thanks) =
        (0x92_A2A0, 0x92_A355, 0x92_A541, 0x92_A765, 0x92_A86C);
    let japanese = [greeting, help, money, help, confirm, thanks, help];
    assert_eq!(jp.1, texts(japan.image(), japanese));
    let (greeting, help, money, confirm, thanks) =
        (0x92_A53D, 0x92_A65E, 0x92_A79D, 0x92_A9E7, 0x92_AAE0);
    let european = [greeting, help, money, help, confirm, thanks, help];
    assert_eq!(eu.1, texts(europe.image(), european));
}

#[test]
fn every_european_shop_text_decodes_as_the_japanese_one() {
    // Sold out, greeting, help, description, the four refusals, confirm,
    // thanks; the farewell only closes the window. The "no free slot"
    // refusal's second page does not decode in either revision.
    let (Some(europe), Some(japan)) = (european(), japanese()) else {
        return;
    };
    let decode = |rom: &Rom, text: u32, kind: u8| {
        assets::text::HouseDialogue::decode_reading(rom.image(), text, |at| match at {
            0x0DE8 => Some(kind),
            0x0DD0 => Some(0x10),
            _ => None,
        })
        .map(|pages| pages.iter().all(|page| !page.glyphs().is_empty()))
        .map_err(|_| ())
    };
    for (jp, eu) in [
        (0x92_A1ED, 0x92_A438),
        (0x92_A2A0, 0x92_A53D),
        (0x92_A355, 0x92_A65E),
        (0x92_A4FB, 0x92_A792),
        (0x92_A541, 0x92_A79D),
        (0x92_A65D, 0x92_A8BD),
        (0x92_A8EE, 0x92_ABAE),
        (0x92_A911, 0x92_ABCD),
        (0x92_A765, 0x92_A9E7),
        (0x92_A86C, 0x92_AAE0),
    ] {
        for kind in [0, 3] {
            let expected = decode(&japan, jp, kind);
            assert!(expected != Ok(false), "{jp:#x} type {kind}");
            assert_eq!(decode(&europe, eu, kind), expected, "{eu:#x} type {kind}");
        }
    }
    let farewell = |rom: &Rom, at: usize| rom.image()[at];
    assert_eq!(farewell(&europe, 0x12_80AF), 0xD7, "closes the window");
    assert_eq!(farewell(&japan, 0x12_8095), 0xD7);
}

#[test]
fn the_elders_question_opens_and_either_answer_continues() {
    // The European Elder asks catalog 2 ("Apologize"); the story goes on as
    // the Japanese one: `$26` set, the pad free again.
    use room_core::Direction;
    let Some(rom) = european() else {
        return;
    };
    let image = rom.image();
    let a = Presses {
        confirm: true,
        ..Presses::NONE
    };
    for answer in [
        a,
        Presses {
            cancel: true,
            ..Presses::NONE
        },
    ] {
        let events = crysta_runtime::world::new_game_flags();
        let mut world = World::enter_with_events(image, 0x0B, 120, 128, events).unwrap();
        world.face(Direction::Up);
        let (mut talked, mut asked) = (false, false);
        for frame in 0..4000 {
            let reading = (world.dialogue().is_some() || world.in_scene()) && !world.typing();
            let choosing = world.dialogue().and_then(|view| view.cursor).is_some();
            let press = if choosing {
                asked = true;
                answer
            } else if reading && frame % 2 == 0 {
                a
            } else if !reading && !talked && frame > 400 {
                talked = true;
                a
            } else {
                Presses::NONE
            };
            world.update(None, press).unwrap();
            assert!(
                world.frozen_scripts().is_empty(),
                "{:x?}",
                world.frozen_scripts()
            );
            if asked && !world.in_scene() && world.dialogue().is_none() && !world.pad_locked() {
                break;
            }
        }
        assert!(asked, "the question opened");
        assert!(world.events()[0x26 / 8] & (1 << (0x26 % 8)) != 0, "$26 set");
    }
}

#[test]
fn european_frozen_elder_assigns_the_world_map_mission() {
    let Some(rom) = european() else {
        return;
    };
    let mut events = fresh_game_flags();
    for id in [
        0x20, 0x22, 0x23, 0x26, 0x27, 0x28, 0x2e, 0xfe, 0x240, 0x241, 0x242, 0x243, 0x244, 0x292,
    ] {
        events[id / 8] |= 1 << (id % 8);
    }
    let mut world = World::enter_with_events(rom.image(), 0x000d, 120, 704, events).unwrap();
    world.face(Direction::Down);
    let mut started = false;
    for frame in 0..6000 {
        let confirm = if started {
            (world.dialogue().is_some() || world.in_scene()) && !world.typing() && frame % 2 == 0
        } else {
            started = true;
            true
        };
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
        if world.events()[0x296 / 8] & (1 << (0x296 % 8)) != 0 && !world.pad_locked() {
            break;
        }
    }
    assert_ne!(world.events()[0x21 / 8] & (1 << (0x21 % 8)), 0);
    assert_ne!(
        world.events()[0x296 / 8] & (1 << (0x296 % 8)),
        0,
        "mission absent: scene={} locked={} typing={} dialogue={} cursor={} frozen={:x?}",
        world.in_scene(),
        world.pad_locked(),
        world.typing(),
        world.dialogue().is_some(),
        world.dialogue().and_then(|view| view.cursor).is_some(),
        world.frozen_scripts()
    );
    assert_eq!(world.map(), 0x000d);
}

#[test]
fn european_frozen_town_releases_ark_to_the_underworld() {
    let Some(rom) = european() else {
        return;
    };
    // The native European route sets these before the town presentation;
    // $3C belongs to the town script, not the seeded return state.
    let mut events = fresh_game_flags();
    for id in [
        0x20, 0x21, 0x22, 0x23, 0x26, 0x27, 0x28, 0x2e, 0xfe, 0x240, 0x241, 0x242, 0x243, 0x244,
        0x292, 0x296,
    ] {
        events[id / 8] |= 1 << (id % 8);
    }
    let mut world = World::enter_with_events(rom.image(), 0x000a, 504, 769, events).unwrap();
    for frame in 0..4000 {
        let confirm = world.dialogue().is_some() && !world.typing() && frame % 2 == 0;
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
        if world.events()[0x3c / 8] & (1 << (0x3c % 8)) != 0 && !world.pad_locked() {
            break;
        }
    }
    assert_ne!(world.events()[0x3c / 8] & (1 << (0x3c % 8)), 0);
    assert!(!world.pad_locked(), "town scene released Ark");
    for _ in 0..300 {
        world.update(Some(Direction::Down), Presses::NONE).unwrap();
        if world.map() == 0x0003 {
            break;
        }
    }
    assert_eq!(world.map(), 0x0003, "south gate opens after the mission");
    for _ in 0..200 {
        world.update(None, Presses::NONE).unwrap();
        if !world.in_transition() {
            break;
        }
    }
    assert_eq!(world.position(), (536, 528), "raw world-map placement");
    // Plane::new queues the native 16-pixel entrance walk. The old test
    // supplied Down here, but the world player moves even with no input.
    for step in 1..=16 {
        world.update(None, Presses::NONE).unwrap();
        assert_eq!(world.position(), (536, 528 + step));
    }
    assert_eq!(
        world.position(),
        (536, 544),
        "European settled route position"
    );
    for id in [0x21, 0x23, 0x3c, 0xfe, 0x242, 0x296] {
        assert_ne!(world.events()[id / 8] & (1 << (id % 8)), 0);
    }
}

fn eu_flag(world: &World<'_>, id: usize) -> bool {
    world.events()[id / 8] & (1 << (id % 8)) != 0
}

fn eu_frames(world: &mut World<'_>, count: usize, direction: Option<Direction>) {
    for _ in 0..count {
        world.update(direction, Presses::NONE).unwrap();
    }
}

/// Play a scene using the English page/cursor state, not Japanese page counts
/// or the native emulator's fixed acknowledgement intervals.
fn eu_finish_scene(world: &mut World<'_>, limit: usize) -> bool {
    let mut chose = false;
    let mut quiet = 0;
    for frame in 0..limit {
        let view = world.dialogue();
        let choosing = view.as_ref().and_then(|page| page.cursor).is_some();
        let confirm = view.is_some() && !world.typing() && frame % 2 == 0;
        if choosing && confirm {
            chose = true;
        }
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
        if !world.in_scene() && world.dialogue().is_none() && !world.pad_locked() {
            quiet += 1;
            if quiet == 30 {
                return chose;
            }
        } else {
            quiet = 0;
        }
    }
    panic!(
        "English scene did not release Ark: map={:#x} pos={:?} typing={} cursor={} frozen={:x?}",
        world.map(),
        world.position(),
        world.typing(),
        world.dialogue().and_then(|page| page.cursor).is_some(),
        world.frozen_scripts()
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Keep the single-World journey and its checkpoints together.
fn european_single_world_replays_bedroom_through_pandora_tour() {
    let Some(rom) = european() else { return };
    let mut world =
        World::enter_with_events(rom.image(), 0x000f, 304, 112, fresh_game_flags()).unwrap();
    for _ in 0..400 {
        if world.dialogue().is_some() {
            break;
        }
        world.update(None, Presses::NONE).unwrap();
    }
    assert!(world.dialogue().is_some(), "Elle speaks in the bedroom");
    eu_finish_scene(&mut world, 6000);
    assert!(eu_flag(&world, 0x20));
    assert_eq!(world.items(), [0x7a, 0xa0]);

    for (frames, direction) in [
        (62, Some(Direction::Right)),
        (38, None),
        (67, Some(Direction::Down)),
        (83, None),
    ] {
        eu_frames(&mut world, frames, direction);
    }
    assert_eq!((world.map(), world.position()), (0x10, (392, 353)));

    // The native European route supplies walking candidates; all movement
    // remains in this world and crosses the ordinary exit/door geometry.
    for (leg, (frames, direction)) in [
        (42, Some(Direction::Down)),
        (78, Some(Direction::Left)),
        (90, None),
        (12, Some(Direction::Left)),
        (100, None),
        (55, Some(Direction::Left)),
        (70, Some(Direction::Up)),
        (100, None),
        (1, Some(Direction::Up)),
        (100, None),
        (50, Some(Direction::Up)),
        (100, None),
        (180, Some(Direction::Up)),
    ]
    .into_iter()
    .enumerate()
    {
        if leg == 8 {
            world
                .update(
                    direction,
                    Presses {
                        confirm: true,
                        ..Presses::NONE
                    },
                )
                .unwrap();
        } else {
            eu_frames(&mut world, frames, direction);
        }
        if leg == 4 {
            assert_eq!(world.map(), 0x0c, "crossed into C");
        }
    }
    assert_eq!(world.map(), 0x0b);
    // The first B doorway scene can still be talking when portable Ark reaches
    // (120,191); the native route's remaining Up frames ran after its pages.
    eu_finish_scene(&mut world, 4000);
    eu_frames(&mut world, 180, Some(Direction::Up));
    eu_frames(&mut world, 100, None);
    assert_eq!(world.position(), (120, 128), "approach the Elder on foot");
    assert!(!eu_flag(&world, 0x26));
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    // The Elder's first request grants $26 before the visible choice is answered.
    for frame in 0..4000 {
        if world.dialogue().and_then(|page| page.cursor).is_some() {
            break;
        }
        let confirm = world.dialogue().is_some() && !world.typing() && frame % 2 == 0;
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
    }
    assert!(
        world.dialogue().and_then(|page| page.cursor).is_some(),
        "Elder choice: pos={:?} flag26={} scene={} locked={} typing={} page={} frozen={:x?}",
        world.position(),
        eu_flag(&world, 0x26),
        world.in_scene(),
        world.pad_locked(),
        world.typing(),
        world.dialogue().is_some(),
        world.frozen_scripts()
    );
    assert!(eu_flag(&world, 0x26));
    assert!(eu_finish_scene(&mut world, 4000), "Elder's answer selected");
    assert!(eu_flag(&world, 0x26));

    for (leg, (frames, direction)) in [
        (60, Some(Direction::Down)),
        (100, None),
        (10, Some(Direction::Left)),
        (82, Some(Direction::Down)),
        (100, None),
        (45, Some(Direction::Down)),
        (140, None),
        (22, Some(Direction::Down)),
        (25, None),
        (15, None),
        (140, None),
    ]
    .into_iter()
    .enumerate()
    {
        eu_frames(&mut world, frames, direction);
        if leg == 1 {
            assert_eq!(
                world.map(),
                0x0c,
                "leaving B returns through C: pos={:?} locked={} scene={} frozen={:x?}",
                world.position(),
                world.pad_locked(),
                world.in_scene(),
                world.frozen_scripts()
            );
        }
        if leg == 4 {
            assert_eq!(world.map(), 0x0d, "cross the house lobby");
        }
    }
    assert_eq!((world.map(), world.position()), (0x0a, (504, 769)));

    for (leg, (frames, direction)) in [
        (32, Some(Direction::Down)),
        (60, None),
        (24, Some(Direction::Right)),
        (90, None),
        (120, Some(Direction::Left)),
        (60, None),
        (300, Some(Direction::Up)),
        (60, None),
        (132, Some(Direction::Right)),
        (60, None),
        (65, Some(Direction::Up)),
        (60, None),
        (56, Some(Direction::Left)),
        (60, None),
        (25, Some(Direction::Up)),
        (100, None),
        (1, Some(Direction::Up)),
        (100, None),
        (40, Some(Direction::Up)),
        (180, None),
        (43, Some(Direction::Up)),
        (60, None),
        (22, Some(Direction::Left)),
        (60, None),
        (20, Some(Direction::Up)),
        (60, None),
    ]
    .into_iter()
    .enumerate()
    {
        if leg == 16 {
            assert_eq!(world.facing(), Direction::Up);
            world
                .update(
                    None,
                    Presses {
                        confirm: true,
                        ..Presses::NONE
                    },
                )
                .unwrap();
        } else {
            eu_frames(&mut world, frames, direction);
        }
    }
    assert_eq!((world.map(), world.position()), (0x13, (360, 144)));
    assert!(!eu_flag(&world, 0x28));
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    assert!(eu_finish_scene(&mut world, 4000), "weaver's choice opened");
    assert!(eu_flag(&world, 0x28), "weaver grants $28");

    // Back through the town into C: the friends stop Ark at the blue door.
    for (leg, (frames, direction)) in [
        (22, Some(Direction::Down)),
        (40, None),
        (22, Some(Direction::Right)),
        (40, None),
        (55, Some(Direction::Down)),
        (160, None),
        (10, Some(Direction::Down)),
        (40, None),
        (55, Some(Direction::Right)),
        (40, None),
        (55, Some(Direction::Down)),
        (40, None),
        (132, Some(Direction::Left)),
        (40, None),
        (280, Some(Direction::Down)),
        (80, None),
        (101, Some(Direction::Right)),
        (40, None),
        (38, Some(Direction::Up)),
        (60, None),
        (1, None), // A opens the house door.
        (100, None),
        (40, Some(Direction::Up)),
        (160, None),
        (100, Some(Direction::Up)),
        (160, None),
    ]
    .into_iter()
    .enumerate()
    {
        if leg == 20 {
            world
                .update(
                    None,
                    Presses {
                        confirm: true,
                        ..Presses::NONE
                    },
                )
                .unwrap();
        } else {
            eu_frames(&mut world, frames, direction);
        }
        if leg == 13 {
            // The portable resolver stops the 132-frame Left leg seven
            // pixels earlier than the native town route. Finish the same
            // sidewalk lane on foot before walking south past the wall.
            assert_eq!(world.map(), 0x0a);
            assert_eq!(world.position().1, 400);
            for _ in 0..16 {
                if world.position().0 <= 356 {
                    break;
                }
                world.update(Some(Direction::Left), Presses::NONE).unwrap();
            }
            eu_frames(&mut world, 40, None);
            assert!(world.position().0 <= 356, "align with the south lane");
        }
        if leg == 23 {
            assert_eq!(world.map(), 0x0d, "return through D");
        }
    }
    assert_eq!(world.map(), 0x0c, "walked back home through D");
    assert!(world.dialogue().is_some(), "friend's request is on screen");
    assert!(eu_flag(&world, 0x27), "friend arrives at the blue door");
    assert!(!eu_flag(&world, 0x2e));
    assert!(eu_finish_scene(&mut world, 5000), "accepted friend's help");
    assert!(eu_flag(&world, 0x2e));

    // Take a pot from C and miss the blue door on purpose: the native route
    // also misses from (136,368) before moving farther left for the first hit.
    for (frames, direction) in [
        (11, Some(Direction::Right)),
        (60, None),
        (67, Some(Direction::Up)),
        (80, None),
        (24, Some(Direction::Left)),
        (60, None),
    ] {
        eu_frames(&mut world, frames, direction);
    }
    assert_eq!(world.position(), (104, 352), "first pot on foot");
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    for (leg, (frames, direction)) in [
        (120, None),
        (22, Some(Direction::Down)),
        (60, None),
        (55, Some(Direction::Right)),
        (60, None),
        (1, Some(Direction::Up)),
        (40, None),
    ]
    .into_iter()
    .enumerate()
    {
        eu_frames(&mut world, frames, direction);
        if leg == 0 {
            assert_eq!(world.pot().map(|pot| pot.tile), Some(0xfa));
            assert!(world.patched_cells().contains(&(5, 21, 0xf8)));
        }
    }
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    eu_frames(&mut world, 180, None);
    assert_eq!(world.position(), (136, 368), "missed throw");
    assert!(world.pot().is_none());
    assert!(
        !world.patched_cells().contains(&(11, 21, 0x181)),
        "miss did not hit the door"
    );
    assert!(!eu_flag(&world, 0x292));

    for (frames, direction) in [
        (55, Some(Direction::Left)),
        (60, None),
        (14, Some(Direction::Up)),
        (60, None),
        (16, Some(Direction::Left)),
        (60, None),
        (20, Some(Direction::Up)),
        (60, None),
        (1, Some(Direction::Right)),
        (30, None),
    ] {
        eu_frames(&mut world, frames, direction);
    }
    assert_eq!(world.position(), (40, 352), "second pot on foot");
    assert_eq!(world.facing(), Direction::Right);
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    for (leg, (frames, direction)) in [
        (120, None),
        (45, Some(Direction::Down)),
        (60, None),
        (22, Some(Direction::Down)),
        (60, None),
        (66, Some(Direction::Right)),
        (60, None),
        (40, Some(Direction::Up)),
        (60, None),
        (33, Some(Direction::Right)),
        (60, None),
        (20, Some(Direction::Up)),
        (80, None),
    ]
    .into_iter()
    .enumerate()
    {
        eu_frames(&mut world, frames, direction);
        if leg == 0 {
            assert_eq!(world.pot().map(|pot| pot.tile), Some(0xfa));
            assert!(world.patched_cells().contains(&(3, 21, 0xf8)));
        }
    }
    assert_eq!(world.position(), (184, 368), "aim at the door");
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    eu_frames(&mut world, 180, None);
    assert!(
        world.patched_cells().contains(&(11, 21, 0x181)),
        "first pot hit"
    );
    assert!(!eu_flag(&world, 0x292), "one hit does not open the door");
    eu_finish_scene(&mut world, 4000);

    for (frames, direction) in [
        (16, Some(Direction::Down)),
        (60, None),
        (33, Some(Direction::Left)),
        (60, None),
        (28, Some(Direction::Up)),
        (60, None),
        (33, Some(Direction::Left)),
        (60, None),
    ] {
        eu_frames(&mut world, frames, direction);
    }
    assert_eq!(world.position(), (88, 352), "third pot on foot");
    assert_eq!(world.facing(), Direction::Left);
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    for (leg, (frames, direction)) in [
        (120, None),
        (33, Some(Direction::Right)),
        (60, None),
        (28, Some(Direction::Down)),
        (60, None),
        (33, Some(Direction::Right)),
        (60, None),
        (20, Some(Direction::Up)),
        (80, None),
    ]
    .into_iter()
    .enumerate()
    {
        eu_frames(&mut world, frames, direction);
        if leg == 0 {
            assert_eq!(world.pot().map(|pot| pot.tile), Some(0xfb));
            assert!(world.patched_cells().contains(&(4, 21, 0xf8)));
        }
    }
    assert_eq!(
        world.position(),
        (184, 368),
        "second shot from the native lane"
    );
    world
        .update(
            None,
            Presses {
                confirm: true,
                ..Presses::NONE
            },
        )
        .unwrap();
    eu_frames(&mut world, 240, None);
    assert!(eu_flag(&world, 0x292), "the second pot opens the blue door");
    eu_finish_scene(&mut world, 5000);
    assert!(world.patched_cells().contains(&(11, 21, 0xcb)));
    assert!(world.patched_cells().contains(&(11, 20, 0xf6)));
    assert!(
        eu_flag(&world, 0x09),
        "friends' reaction reached its last local"
    );
    assert!(
        world
            .residents()
            .iter()
            .all(|resident| resident.record != 0x03_8c32),
        "the broken door has gone"
    );

    for (frames, direction, map, position) in [
        (30, Direction::Up, 0x0e, (152, 880)),
        (35, Direction::Up, 0x20, (408, 880)),
        (35, Direction::Up, 0x21, (136, 128)),
    ] {
        if map != 0x0e {
            eu_frames(&mut world, 33, Some(Direction::Left));
            eu_frames(&mut world, 80, None);
        }
        eu_frames(&mut world, frames, Some(direction));
        eu_frames(&mut world, 400, None);
        assert_eq!((world.map(), world.position()), (map, position));
    }
    assert!(!eu_flag(&world, 0x22));
    assert!(world.dialogue().is_some(), "the box's entry voice");
    eu_finish_scene(&mut world, 4000);
    eu_frames(&mut world, 150, Some(Direction::Down));
    eu_frames(&mut world, 180, None);
    assert_eq!(world.position(), (136, 351), "approach the box");
    eu_frames(&mut world, 25, Some(Direction::Down));
    eu_frames(&mut world, 180, None);
    assert_eq!(world.position(), (136, 359), "first contact recoils");
    assert!(!eu_flag(&world, 0x22));
    assert!(world.dialogue().is_some(), "the box warns Ark");
    eu_finish_scene(&mut world, 4000);
    let mut reloaded = false;
    for (frames, direction) in [(15, Some(Direction::Down)), (300, None)] {
        for _ in 0..frames {
            let (movement, interaction) = world.update(direction, Presses::NONE).unwrap();
            reloaded |= [movement, interaction.unwrap_or(Step::Stayed)]
                .iter()
                .any(|step| {
                    matches!(
                        step,
                        Step::Entered {
                            from: 0x21,
                            to: 0x21
                        }
                    )
                });
        }
    }
    assert!(reloaded, "Pandora's Box reloads $21 before the tour");
    assert!(eu_flag(&world, 0x22), "the second approach opens the box");
    assert_eq!((world.map(), world.position()), (0x21, (136, 368)));

    // The box and its guide control the transfers; no map is re-entered or
    // progression flag seeded. A is sent only for finished English pages.
    let mut landings = Vec::new();
    for frame in 0..16000 {
        let confirm = world.dialogue().is_some() && !world.typing() && frame % 2 == 0;
        let before = world.map();
        world
            .update(
                None,
                Presses {
                    confirm,
                    ..Presses::NONE
                },
            )
            .unwrap();
        if world.map() != before {
            landings.push((world.map(), world.position()));
        }
        if eu_flag(&world, 0x244) && !world.pad_locked() && !world.in_transition() {
            break;
        }
    }
    assert_eq!(
        landings,
        [
            (0x41, (136, 208)),
            (0x44, (392, 464)),
            (0x42, (136, 464)),
            (0x43, (392, 208)),
            (0x41, (136, 208))
        ]
    );
    assert!(eu_flag(&world, 0x243) && eu_flag(&world, 0x244));
    assert!(!world.pad_locked(), "guide releases Ark");
    assert!(!world.in_scene() && world.dialogue().is_none());
    eu_frames(&mut world, 12, Some(Direction::Left));
    eu_frames(&mut world, 120, None);
    eu_frames(&mut world, 12, Some(Direction::Up));
    eu_frames(&mut world, 120, None);
    assert_eq!((world.map(), world.position()), (0x41, (120, 192)));
}
