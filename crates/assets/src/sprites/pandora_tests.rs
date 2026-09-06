//! Synthetic tests for the additive Pandora compiler (no cartridge fixtures).
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn bounded_lists_preserve_records_and_reject_mutations() {
    let mut bytes = vec![0; 64];
    bytes[..2].copy_from_slice(&4_u16.to_le_bytes());
    bytes[4..14].copy_from_slice(&[8, 3, 16, 0, 2, 1, 16, 0, 255, 255]);
    bytes[16..40].copy_from_slice(&[
        8, 8, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0x24,
    ]);
    let list = pose_list(&bytes, 0xa2_c000, true, 0, 192, 256).unwrap();
    assert_eq!(list.frames.len(), 2);
    assert_eq!(list.frames[0].duration(), 8);
    assert_eq!(list.frames[1].facing(), 1);
    assert_eq!(list.frames[0].key(), HousePoseKey::Direct(0xa2_c014));
    assert_eq!(list.frames[0].composition().components()[0].word(), 0x2800);
    assert_eq!(
        list.frames[0].source_composition().components()[0].word(),
        0x2400
    );
    for (at, value) in [(12, 0), (5, 255), (32, 0), (33, 2), (39, 0x25)] {
        let mut changed = bytes.clone();
        changed[at] = value;
        assert!(
            pose_list(&changed, 0xa2_c000, true, 0, 192, 256).is_err(),
            "mutation {at}"
        );
    }
    assert!(pose_list(&bytes[..39], 0xa2_c000, true, 0, 192, 256).is_err());
}

#[test]
#[allow(clippy::many_single_char_names)]
fn source_raster_nonempty_exact_and_mirrored() {
    let mut bytes = vec![0; 48];
    bytes[..2].copy_from_slice(&4_u16.to_le_bytes());
    bytes[4..10].copy_from_slice(&[0, 3, 16, 0, 255, 255]);
    bytes[16..40].copy_from_slice(&[
        8, 8, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 8, 0, 0, 0, 0x24,
    ]);
    let list = pose_list(&bytes, 0xa2_c000, true, 0, 192, 256).unwrap();
    let mut planar = [0; 32];
    planar[0] = 0x80;
    let tiles = decode_tiles_4bpp(&planar).unwrap();
    let frame = list.frames[0].composition();
    for flip in [false, true] {
        let (l, t, r, b) = frame.bounds(flip, false);
        let pixels: Vec<_> = (t..b)
            .flat_map(|y| (l..r).map(move |x| (x, y)))
            .map(|(x, y)| frame.sample(&tiles, flip, false, x, y).unwrap())
            .collect();
        let mut expected = vec![crate::sprites::SpritePixel::Transparent; 64];
        expected[if flip { 7 } else { 0 }] = crate::sprites::SpritePixel::Opaque {
            palette_index: 193,
            priority: 2,
            component: 0,
        };
        assert_eq!(pixels, expected);
    }
}

#[test]
#[ignore = "requires PANDORA_ROM owned Japanese cartridge"]
#[allow(clippy::many_single_char_names)]
fn authenticated_pandora_sources() {
    let bytes = std::fs::read(std::env::var("PANDORA_ROM").expect("PANDORA_ROM")).unwrap();
    let image = rom::Rom::load(&bytes).unwrap();
    assert_eq!(image.revision(), rom::Revision::Japan);
    let sprites = PandoraSprites::from_rom(image.image()).unwrap();
    assert_eq!(sprites.art.len(), 16);
    assert!(sprites.get(0x83_f984).unwrap().list(3).is_some());
    assert!(sprites.get(0x96_e1a6).unwrap().list(25).is_some());
    assert!(sprites.get(0x96_e1ab).unwrap().list(46).is_some());
    assert!(sprites.get(0x89_d9f9).unwrap().list(8).is_some());
    for art in sprites.art() {
        for list in art.lists() {
            for frame in list.frames() {
                let c = frame.composition();
                let (l, t, r, b) = c.bounds(false, false);
                let mut opaque = 0;
                for y in t..b {
                    for x in l..r {
                        if matches!(
                            c.sample(art.graphics(), false, false, x, y).unwrap(),
                            crate::sprites::SpritePixel::Opaque { .. }
                        ) {
                            opaque += 1;
                        }
                    }
                }
                assert!(
                    opaque > 0,
                    "empty source art {:x}/{}",
                    art.source_id(),
                    list.selector()
                );
            }
        }
    }
}

#[test]
#[ignore = "requires PANDORA_ROM owned Japanese cartridge"]
fn phase_source_positions_and_qualified_membership() {
    let bytes = std::fs::read(std::env::var("PANDORA_ROM").expect("PANDORA_ROM")).unwrap();
    let scenes = PandoraSprites::from_rom(&bytes).unwrap();
    let c = scenes.phase("c-direct").unwrap();
    assert_eq!(
        c.actors().iter().map(|a| a.position).collect::<Vec<_>>(),
        [[152, 368], [56, 384], [184, 416], [216, 368]]
    );
    assert_eq!(
        c.actors().iter().map(|a| a.tie_rank).collect::<Vec<_>>(),
        [3, 2, 1, 0]
    );
    assert!(scenes.phase("cellar-e").unwrap().actors().is_empty());
    assert_eq!(
        scenes.phase("tour-41-3").unwrap().actors()[0].position,
        [88, 152]
    );
    assert_eq!(
        scenes.phase("tour-44").unwrap().actors()[0].position,
        [392, 368]
    );
    for (id, count) in [
        ("c-color-math", 3),
        ("c-reaction-right", 3),
        ("c-reaction-left", 2),
        ("c-reaction-final", 1),
        ("c-departed", 0),
    ] {
        assert_eq!(scenes.phase(id).unwrap().actors().len(), count);
    }
    assert_eq!(
        scenes
            .motions()
            .iter()
            .filter(|m| m.removes_actor)
            .map(|m| (m.actor, m.to))
            .collect::<Vec<_>>(),
        [
            (0x83_8c14, [120, 512]),
            (0x83_8c28, [120, 496]),
            (0x83_8c0a, [120, 496]),
            (0x83_8c1e, [120, 512])
        ]
    );
    assert!(
        scenes
            .phase("box-opening")
            .unwrap()
            .limits()
            .contains(&PandoraSceneLimit::OpeningPalette)
    );
    assert_eq!(
        scenes.phase("tour-control").unwrap().actors()[0].priority_override,
        Some(3)
    );
    for motion in scenes.motions() {
        let actor = c
            .actors()
            .iter()
            .find(|a| a.source_id == motion.actor)
            .unwrap();
        assert!(
            scenes
                .get(actor.art_id)
                .unwrap()
                .list(motion.selector)
                .is_some()
        );
    }
    for id in [0x96_e1a6, 0x96_e1ab] {
        assert!(scenes.get(id).unwrap().list(60).is_some());
    }
    for phase in scenes.phases() {
        for actor in phase.actors() {
            assert!(
                scenes
                    .get(actor.art_id)
                    .unwrap()
                    .list(actor.selector)
                    .is_some(),
                "{} / {:x}",
                phase.id(),
                actor.source_id
            );
        }
    }
}

#[test]
fn compressed_selector_cannot_read_trailing_anchor_as_list() {
    let mut bytes = vec![0; 128];
    bytes[..2].copy_from_slice(&8_u16.to_le_bytes());
    bytes[6..8].copy_from_slice(&32_u16.to_le_bytes());
    bytes[32..38].copy_from_slice(&[0, 3, 64, 0, 255, 255]);
    bytes[64..88].copy_from_slice(&[
        8, 8, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0x24,
    ]);
    assert!(pose_list(&bytes, 0xd4_518c, false, 3, 192, 256).is_err());
}

#[test]
#[ignore = "requires PANDORA_ROM owned Japanese cartridge"]
fn source_operand_mutations_fail_closed() {
    let original = std::fs::read(std::env::var("PANDORA_ROM").expect("PANDORA_ROM")).unwrap();
    // COP13 selector must resolve against its resource's admitted lists.
    let mut image = original.clone();
    image[0x8_a38b] = 254;
    assert!(PandoraSprites::from_rom(&image).is_err());
    let mut image = original.clone();
    image[0x8_a56f] = 254;
    assert!(PandoraSprites::from_rom(&image).is_err());
    // Palette pointer + source offset must remain in the original bank.
    let mut image = original.clone();
    image[0xfc75..0xfc78].copy_from_slice(&[0xf0, 0xff, 0xcc]);
    assert!(PandoraSprites::from_rom(&image).is_err());
    let mut image = original.clone();
    image[0x3_f98d..0x3_f990].copy_from_slice(&[0xf0, 0xff, 0xcc]);
    assert!(PandoraSprites::from_rom(&image).is_err());
}

#[test]
fn carry_selection_is_explicit_and_separate_from_physics() {
    let left = PandoraSprites::carry_pose(PandoraCarryMotion::Walking, 2).unwrap();
    assert_eq!(
        (left.ark_art, left.ark_selector, left.pot_selector),
        (0x80_a255, 11, 30)
    );
    assert!(left.ark_hflip && left.pot_hflip);
    let up = PandoraSprites::carry_pose(PandoraCarryMotion::Lifting, 1).unwrap();
    assert_eq!(
        (up.ark_art, up.ark_selector, up.pot_selector),
        (0x80_a261, 25, 44)
    );
    assert!(!up.ark_hflip && !up.pot_hflip);
    assert!(PandoraSprites::carry_pose(PandoraCarryMotion::Standing, 4).is_none());
}
