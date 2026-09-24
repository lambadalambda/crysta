//! The slice on the European English ROM (ADR 0004).
use assets::maps::exits::ExitList;
use assets::maps::scripts::EventFlags;
use crysta_runtime::art::{residents_art, Animation, ArkAtlas, Body, Placeholder};
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{fresh_game_flags, World};
use crysta_runtime::{BOX_MAPS, MAPS};
use rom::{Revision, Rom};
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
