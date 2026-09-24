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
            // Known gap: the box in `$21` is drawn from the Pandora sprite
            // library, which reads Japanese addresses still.
            if (map, index) == (0x21, 3) {
                assert!(eu.is_err());
                continue;
            }
            assert!(
                eu == jp,
                "{map:#x} resident {index}: {:?}",
                eu.as_ref().err()
            );
        }
    }
}
