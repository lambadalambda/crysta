use super::*;
use assets::sprites::{PandoraSprites, SpritePixel};

#[test]
fn opt_in_atlas_preserves_house_and_every_pandora_source_pixel() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local/Tenchi Souzou (Japan).sfc");
    if !path.try_exists().unwrap() {
        eprintln!("SKIP: local Japanese ROM absent");
        return;
    }
    let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
    let house = super::super::compile(&rom).unwrap();
    let compiled = super::super::compile_profile(&rom, true).unwrap();
    let old: Value = serde_json::from_slice(&house.bytes).unwrap();
    let new: Value = serde_json::from_slice(&compiled.bytes).unwrap();
    for (key, value) in old["frames"].as_object().unwrap() {
        assert_eq!(&new["frames"][key], value);
    }
    for key in [
        "scene_ids",
        "actors",
        "foreground",
        "backgrounds",
        "background_keys",
        "door_background",
        "door_patches",
    ] {
        assert_eq!(old[key], new[key], "unchanged house field: {key}");
    }
    assert!(old.get("pandora_scenes").is_none());
    assert_eq!(new["frames"].as_object().unwrap().len(), 38 + 482);
    assert_eq!(new["pandora_scenes"].as_object().unwrap().len(), 33);
    let sprites = PandoraSprites::from_rom(rom.image()).unwrap();
    assert_all_source_pixels(&sprites, &new);
    for phase in sprites.phases() {
        let manifest = &new["pandora_scenes"][phase.id()];
        assert_eq!(manifest["map_id"], phase.map_id());
        assert_eq!(
            manifest["actors"].as_array().unwrap().len(),
            phase.actors().len()
        );
        let scene = compiled
            .scene_phase(phase.map_id(), Some(phase.id()), "0:0", (136, 208))
            .unwrap();
        assert_eq!(scene.as_array().unwrap().len(), phase.actors().len() + 1);
        for actor in phase.actors() {
            let entry = scene
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == format!("pandora:{:06x}", actor.source_id))
                .unwrap();
            assert_eq!(entry["position"], json!(actor.position));
            assert_eq!(
                entry["key"],
                frame_key(actor.art_id, actor.selector, 0, actor.hflip)
            );
            assert_eq!(entry["priority"], actor.priority_override.unwrap_or(2));
        }
    }
    assert!(compiled
        .scene_phase(0x13, Some("tour-control"), "0:0", (136, 208))
        .is_err());
    assert!(compiled
        .scene_phase(0x41, Some("unknown"), "0:0", (136, 208))
        .is_err());
    assert!(house
        .scene_phase(0x41, Some("tour-control"), "0:0", (136, 208))
        .is_err());
    assert_eq!(
        house.scene(15, "0:0", (304, 112)),
        compiled.scene(15, "0:0", (304, 112))
    );
    assert_eq!(new["dialogue_pages"].as_object().unwrap().len(), 100);
}

fn assert_all_source_pixels(sprites: &PandoraSprites, new: &Value) {
    for art in sprites.art() {
        for list in art.lists() {
            for (record, frame) in list.frames().iter().enumerate() {
                for mirror in [false, true] {
                    let key = frame_key(art.source_id(), list.selector(), record, mirror);
                    let raster = &new["frames"][&key];
                    let (left, top, right, bottom) = frame.composition().bounds(mirror, false);
                    assert_eq!(raster["offset"], json!([left, top]));
                    assert_eq!(raster["width"], right - left);
                    assert_eq!(raster["height"], bottom - top);
                    for y in top..bottom {
                        for x in left..right {
                            let color = match frame
                                .composition()
                                .sample(art.graphics(), mirror, false, x, y)
                                .unwrap()
                            {
                                SpritePixel::Transparent => [0; 4],
                                SpritePixel::Opaque {
                                    palette_index,
                                    priority,
                                    ..
                                } => {
                                    assert_eq!(priority, 2);
                                    let [r, g, b] = art.palette()
                                        [usize::from(palette_index - art.palette_base())]
                                    .rgb8();
                                    [r, g, b, 255]
                                }
                            };
                            let at = usize::try_from((y - top) * (right - left) + (x - left))
                                .unwrap()
                                * 4;
                            assert_eq!(
                                &raster["rgba"].as_array().unwrap()[at..at + 4],
                                json!(color).as_array().unwrap()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn phase_depth_uses_world_y_and_source_ties_not_obj_priority() {
    let actor = |id, y, tie, priority| assets::sprites::PandoraActorPhase {
        source_id: id,
        art_id: 0x83_f8a8,
        position: [100, y],
        position_source: id,
        selector: 3,
        hflip: false,
        tie_rank: tie,
        priority_override: priority,
    };
    let art = Presentation {
        scenes: BTreeMap::from([(
            "test",
            (
                0x41,
                vec![
                    actor(1, 100, 1, Some(3)),
                    actor(2, 101, 2, None),
                    actor(3, 100, 0, None),
                ],
            ),
        )]),
    };
    for (y, expected) in [
        (
            99,
            vec!["ark", "pandora:000003", "pandora:000001", "pandora:000002"],
        ),
        (
            100,
            vec!["pandora:000003", "pandora:000001", "ark", "pandora:000002"],
        ),
        (
            101,
            vec!["pandora:000003", "pandora:000001", "pandora:000002", "ark"],
        ),
    ] {
        let scene = art.scene(0x41, "test", "0:0", (136, y)).unwrap();
        assert_eq!(
            scene
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| entry["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
