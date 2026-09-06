//! Immutable, bounded art adapter for the ordinary house preview.

use crate::{invalid, Result};
use assets::{
    graphics::{Bgr555, IndexedPixel, Tile4bpp},
    maps::visual::{StaticBackground, VisualResource},
    sprites::{ArkSprites, HouseNpc, SpriteFrame, SpritePixel},
};
use room_core::{AnimationFrame, AnimationSet};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const NPC_KEY: &str = "npc:house";

/// Ordinary world-space presentation; tie ranks come from the source draw list.
struct PlacedActor {
    id: String,
    key: String,
    position: [u16; 2],
    tie_rank: u16,
}
struct RoomActors {
    actors: Vec<PlacedActor>,
    ark_tie_rank: u16,
}

/// Immutable presentation metadata, not simulated NPC state or snapshot data.
pub(super) struct Art {
    pub(super) bytes: Vec<u8>,
    rooms: BTreeMap<u16, RoomActors>,
}
impl Art {
    pub(super) fn scene(&self, map: u16, key: &str, position: (u16, u16)) -> Value {
        let room = self.rooms.get(&map).expect("game map has a compiled scene");
        let mut entries: Vec<_> = room
            .actors
            .iter()
            .map(|actor| {
                (
                    actor.position[1],
                    actor.tie_rank,
                    json!({"id":actor.id,"key":actor.key,"position":actor.position}),
                )
            })
            .collect();
        entries.push((
            position.1,
            room.ark_tie_rank,
            json!({"id":"ark","key":key,"position":[position.0,position.1]}),
        ));
        // Ordinary depth uses world Y before anchor subtraction. Neither IDs,
        // resource reuse nor source-spawn order defines equal-Y precedence.
        entries.sort_by_key(|(y, tie, _)| (*y, *tie));
        Value::Array(entries.into_iter().map(|(_, _, entry)| entry).collect())
    }
}
fn membership(rooms: &BTreeMap<u16, RoomActors>) -> Value {
    json!(rooms
        .iter()
        .map(|(id, room)| (
            *id,
            std::iter::once("ark")
                .chain(room.actors.iter().map(|a| a.id.as_str()))
                .collect::<Vec<_>>()
        ))
        .collect::<BTreeMap<_, _>>())
}

/// Shared by atlas compilation and state selection; only core ordinary frames enter here.
fn frame_index(frame: AnimationFrame) -> usize {
    match frame.set {
        AnimationSet::Standing => usize::from(frame.sequence),
        AnimationSet::Walking => 3 + usize::from(frame.sequence) * 6 + usize::from(frame.record),
    }
}
pub(super) fn frame_key(frame: AnimationFrame) -> String {
    format!("{}:{}", frame_index(frame), u8::from(frame.mirror_x))
}

fn raster(
    frame: &SpriteFrame,
    tiles: &[Tile4bpp],
    palette: &[Bgr555; 16],
    palette_base: u8,
    mirror: bool,
) -> Result<Value> {
    let (left, top, right, bottom) = frame.bounds(mirror, false);
    let mut rgba = Vec::new();
    for y in top..bottom {
        for x in left..right {
            // Mirroring selects ROM alternate placements/anchors as well as pixels.
            // The frontend must not flip this precomposed raster again.
            match frame.sample(tiles, mirror, false, x, y)? {
                SpritePixel::Transparent => rgba.extend([0; 4]),
                SpritePixel::Opaque {
                    palette_index,
                    priority,
                    ..
                } => {
                    if priority != 2 {
                        return Err(invalid("unqualified ordinary sprite priority").into());
                    }
                    let color = palette_index
                        .checked_sub(palette_base)
                        .and_then(|index| palette.get(usize::from(index)))
                        .ok_or_else(|| invalid("unqualified ordinary sprite palette"))?;
                    rgba.extend(color.rgb8());
                    rgba.push(255);
                }
            }
        }
    }
    Ok(json!({"width":right-left,"height":bottom-top,"offset":[left,top],"rgba":rgba}))
}

fn occludes(pixel: IndexedPixel) -> bool {
    // House first background is hardware BG2: opaque high > OBJ2 > low.
    // A transparent tile sample never occludes, regardless of its tile priority.
    matches!(pixel, IndexedPixel::Opaque { priority: true, .. })
}
fn runs(pixels: impl IntoIterator<Item = bool>) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut active = false;
    for (index, high) in pixels.into_iter().enumerate() {
        if high {
            if active {
                *runs.last_mut().expect("active run length") += 1;
            } else {
                runs.extend([index, 1]);
            }
        }
        active = high;
    }
    runs
}
fn foreground(background: &StaticBackground) -> Result<Value> {
    let width = background.layer().width() * 16;
    let height = background.layer().height() * 16;
    let pixels = (0..width * height)
        .map(|index| background.pixel(index % width, index / width).map(occludes))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!({"width":width,"height":height,"runs":runs(pixels)}))
}

pub(super) fn compile(rom: &rom::Rom) -> Result<Art> {
    let sprites = ArkSprites::from_rom(rom.image())?;
    let mut frames = serde_json::Map::new();
    for set in [AnimationSet::Standing, AnimationSet::Walking] {
        for sequence in 0..3 {
            let count = if set == AnimationSet::Standing { 1 } else { 6 };
            for record in 0..count {
                for mirror_x in [false, true]
                    .into_iter()
                    .take(if sequence == 2 { 2 } else { 1 })
                {
                    let key = AnimationFrame {
                        set,
                        sequence,
                        record,
                        mirror_x,
                    };
                    let frame = &sprites.frames()[frame_index(key)];
                    let pixels = raster(
                        frame.composition(),
                        sprites
                            .graphics(frame.resource())
                            .ok_or_else(|| invalid("missing ordinary graphics"))?,
                        sprites.palette(),
                        128,
                        mirror_x,
                    )?;
                    frames.insert(frame_key(key), pixels);
                }
            }
        }
    }
    let npc = HouseNpc::from_rom(rom.image())?;
    frames.insert(
        NPC_KEY.into(),
        raster(
            npc.composition(),
            npc.graphics(),
            npc.palette(),
            npc.palette_base(),
            npc.hflip(),
        )?,
    );
    let bedroom = StaticBackground::from_rom(rom.image(), 15)?;
    let house = StaticBackground::from_rom(rom.image(), 16)?;
    // /map.bmp is fixed to map15. Require full decoded source identity, not an
    // assumption that the two room viewports happen to look alike.
    if bedroom.layer().layer_bytes() != house.layer().layer_bytes()
        || !bedroom
            .resources()
            .iter()
            .map(VisualResource::decoded)
            .eq(house.resources().iter().map(VisualResource::decoded))
    {
        return Err(invalid("house backgrounds differ: fixed /map.bmp cannot be reused").into());
    }
    let rooms = BTreeMap::from([
        (
            15,
            RoomActors {
                actors: vec![],
                ark_tie_rank: 0,
            },
        ),
        (
            npc.map_id(),
            RoomActors {
                actors: vec![PlacedActor {
                    id: NPC_KEY.into(),
                    key: NPC_KEY.into(),
                    position: npc.position(),
                    tie_rank: 0,
                }],
                ark_tie_rank: 1,
            },
        ),
    ]);
    Ok(Art {
        bytes: serde_json::to_vec(&json!({"schema_version":1,"frames":frames,
            "scene_ids":membership(&rooms),
            "npc":{"key":NPC_KEY,"map_id":npc.map_id(),"position":npc.position(),"policy":"frozen-ordinary-pose"},
            "foreground":{"15":foreground(&bedroom)?,"16":foreground(&house)?}}))?,
        rooms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::{graphics::decode_tiles_4bpp, sprites::SpriteFrame};
    use room_core::{AnimationState, Direction};

    #[test]
    fn multiple_actors_use_source_tie_ranks_not_ids_or_shared_art() {
        let actor = |id: &str, x, y, tie_rank| PlacedActor {
            id: id.into(),
            key: "shared".into(),
            position: [x, y],
            tie_rank,
        };
        let art = Art {
            bytes: Vec::new(),
            rooms: BTreeMap::from([
                (
                    15,
                    RoomActors {
                        actors: vec![],
                        ark_tie_rank: 0,
                    },
                ),
                (
                    16,
                    RoomActors {
                        actors: vec![
                            actor("a", 10, 100, 2),
                            actor("b", 20, 100, 0),
                            actor("c", 30, 99, 3),
                        ],
                        ark_tie_rank: 1,
                    },
                ),
            ]),
        };
        for (y, expected) in [
            (98, vec!["ark", "c", "b", "a"]),
            (100, vec!["c", "b", "ark", "a"]),
            (101, vec!["c", "b", "a", "ark"]),
        ] {
            let scene = art.scene(16, "player-frame", (40, y));
            assert_eq!(
                scene
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| e["id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert_eq!(
            art.scene(16, "player-frame", (40, 100)),
            json!([
                {"id":"c","key":"shared","position":[30,99]},
                {"id":"b","key":"shared","position":[20,100]},
                {"id":"ark","key":"player-frame","position":[40,100]},
                {"id":"a","key":"shared","position":[10,100]},
            ])
        );
        assert_eq!(
            art.scene(15, "player-frame", (40, 100)),
            json!([{"id":"ark","key":"player-frame","position":[40,100]}])
        );
        assert_eq!(
            membership(&art.rooms),
            json!({"15":["ark"],"16":["ark","a","b","c"]})
        );
    }

    #[test]
    fn frozen_npc_membership_and_native_depth_tie() {
        let art = Art {
            bytes: Vec::new(),
            rooms: BTreeMap::from([
                (
                    15,
                    RoomActors {
                        actors: vec![],
                        ark_tie_rank: 0,
                    },
                ),
                (
                    16,
                    RoomActors {
                        actors: vec![PlacedActor {
                            id: NPC_KEY.into(),
                            key: NPC_KEY.into(),
                            position: [424, 416],
                            tie_rank: 0,
                        }],
                        ark_tie_rank: 1,
                    },
                ),
            ]),
        };
        for (map, y, keys) in [
            (15, 416, vec!["ark"]),
            (16, 415, vec!["ark", NPC_KEY]),
            (16, 416, vec![NPC_KEY, "ark"]),
            (16, 417, vec![NPC_KEY, "ark"]),
        ] {
            let scene = art.scene(map, "ark", (400, y));
            let entries = scene.as_array().unwrap();
            assert_eq!(
                entries
                    .iter()
                    .map(|e| e["key"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                keys
            );
            for entry in entries {
                assert_eq!(
                    entry["position"],
                    if entry["key"] == NPC_KEY {
                        json!([424, 416])
                    } else {
                        json!([400, y])
                    }
                );
            }
        }
    }

    #[test]
    fn all_ordinary_animation_keys_cover_exactly_28_frames() {
        let mut keys = std::collections::BTreeSet::new();
        for (direction, sequence, mirror) in [
            (Direction::Down, 0, false),
            (Direction::Up, 1, false),
            (Direction::Right, 2, false),
            (Direction::Left, 2, true),
        ] {
            for walking in [false, true] {
                for record in 0..if walking { 6 } else { 1 } {
                    let frame = AnimationState::from_parts(direction, walking, record * 9)
                        .unwrap()
                        .frame();
                    let expected = if walking {
                        3 + sequence * 6 + record
                    } else {
                        sequence
                    };
                    assert_eq!(frame_index(frame), usize::from(expected));
                    assert_eq!(frame_key(frame), format!("{expected}:{}", u8::from(mirror)));
                    keys.insert(frame_key(frame));
                }
            }
        }
        assert_eq!(keys.len(), 28);
    }

    #[test]
    fn foreground_only_opaque_high_pixels_and_flattened_runs() {
        use IndexedPixel::{Opaque, Transparent};
        let low = Opaque {
            palette_index: 1,
            priority: false,
        };
        let high = Opaque {
            palette_index: 1,
            priority: true,
        };
        let pixels = [Transparent, high, high, low, Transparent, high];
        assert_eq!(runs(pixels.into_iter().map(occludes)), vec![1, 2, 5, 1]);
        assert_eq!(runs([true, true, true, true]), vec![0, 4]);
        assert!(runs([false, false]).is_empty());
    }

    #[test]
    fn raster_uses_alternate_anchor_and_mirrored_pixels_and_rejects_priority() {
        let mut source = vec![0; 17];
        source[..4].copy_from_slice(&[10, 30, 20, 20]);
        source[16] = 1;
        source.extend([0, 8, 40, 12, 12, 0, 0x20]);
        let mut planar = [0; 32];
        planar[0] = 0x80;
        let tiles = decode_tiles_4bpp(&planar).unwrap();
        let palette = [assets::graphics::Bgr555::new(31); 16];
        let frame = SpriteFrame::decode(&source).unwrap();
        let normal = raster(&frame, &tiles, &palette, 128, false).unwrap();
        let mirror = raster(&frame, &tiles, &palette, 128, true).unwrap();
        assert_eq!(normal["offset"], json!([-2, -8]));
        assert_eq!(mirror["offset"], json!([10, -8]));
        assert_eq!(
            (normal["width"].as_u64(), normal["height"].as_u64()),
            (Some(8), Some(8))
        );
        assert_eq!(
            &normal["rgba"].as_array().unwrap()[..4],
            &[json!(255), json!(0), json!(0), json!(255)]
        );
        assert_eq!(mirror["rgba"][3], 0);
        assert_eq!(mirror["rgba"][7 * 4 + 3], 255);
        source[23] = 0x2a; // OBJ palette5, unlike Ark's palette0.
        let resident = SpriteFrame::decode(&source).unwrap();
        assert_eq!(
            raster(&resident, &tiles, &palette, 208, false).unwrap(),
            normal
        );
        assert!(raster(&resident, &tiles, &palette, 128, false).is_err());
        source[23] = 0x10;
        assert!(raster(
            &SpriteFrame::decode(&source).unwrap(),
            &tiles,
            &palette,
            128,
            false
        )
        .is_err());
        let transparent_tiles = decode_tiles_4bpp(&[0; 32]).unwrap();
        let transparent = raster(
            &SpriteFrame::decode(&source).unwrap(),
            &transparent_tiles,
            &palette,
            128,
            false,
        )
        .unwrap();
        assert_eq!(transparent["rgba"], json!(vec![0; 8 * 8 * 4]));
    }

    fn assert_npc_raster(art: &Value) {
        let reference: Value = serde_json::from_str(include_str!(
            "../../../tools/house-npc-qualification/reference.json"
        ))
        .unwrap();
        assert_eq!(
            art["npc"],
            json!({"key":NPC_KEY,"map_id":16,"position":[424,416],"policy":"frozen-ordinary-pose"})
        );
        let resident = &art["frames"][NPC_KEY];
        assert_eq!(resident["offset"], json!([-8, -33]));
        assert_eq!(
            (resident["width"].as_u64(), resident["height"].as_u64()),
            (Some(16), Some(33))
        );
        let rgba: Vec<u8> = resident["rgba"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
            .collect();
        assert_eq!(crate::sha256(&rgba), reference["export"]["rgba_sha256"]);
        assert_eq!(rgba.chunks_exact(4).filter(|p| p[3] == 255).count(), 313);
    }

    #[test]
    fn local_rom_full_atlas_pixels_bounds_and_both_backgrounds() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("skipping: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let art: Value = serde_json::from_slice(&compile(&rom).unwrap().bytes).unwrap();
        assert_eq!(art["schema_version"], 1);
        assert_npc_raster(&art);
        assert_eq!(art["frames"].as_object().unwrap().len(), 29);
        let sprites = ArkSprites::from_rom(rom.image()).unwrap();
        for (index, frame) in sprites.frames().iter().enumerate() {
            for mirror in [false, true] {
                let key = format!("{index}:{}", u8::from(mirror));
                if mirror && index != 2 && !(15..21).contains(&index) {
                    assert!(art["frames"].get(&key).is_none());
                    continue;
                }
                let actual = &art["frames"][&key];
                let (l, t, r, b) = frame.composition().bounds(mirror, false);
                assert_eq!(actual["offset"], json!([l, t]));
                assert_eq!(actual["width"], r - l);
                assert_eq!(actual["height"], b - t);
                let mut expected = Vec::new();
                for y in t..b {
                    for x in l..r {
                        match frame
                            .composition()
                            .sample(
                                sprites.graphics(frame.resource()).unwrap(),
                                mirror,
                                false,
                                x,
                                y,
                            )
                            .unwrap()
                        {
                            SpritePixel::Transparent => expected.extend([0; 4]),
                            SpritePixel::Opaque {
                                palette_index,
                                priority,
                                ..
                            } => {
                                assert_eq!(priority, 2);
                                expected.extend(
                                    sprites.palette()[usize::from(palette_index - 128)].rgb8(),
                                );
                                expected.push(255);
                            }
                        }
                    }
                }
                assert_eq!(actual["rgba"], json!(expected), "{key}");
            }
        }
        let a = StaticBackground::from_rom(rom.image(), 15).unwrap();
        let b = StaticBackground::from_rom(rom.image(), 16).unwrap();
        assert_eq!(a.palette(), b.palette());
        for id in ["15", "16"] {
            let mask = &art["foreground"][id];
            let width = a.layer().width() * 16;
            let height = a.layer().height() * 16;
            assert_eq!(mask["width"], width);
            assert_eq!(mask["height"], height);
            let mut expanded = vec![false; width * height];
            for pair in mask["runs"].as_array().unwrap().chunks_exact(2) {
                let start = usize::try_from(pair[0].as_u64().unwrap()).unwrap();
                let length = usize::try_from(pair[1].as_u64().unwrap()).unwrap();
                assert!(length > 0);
                assert!(expanded[start..start + length].iter().all(|p| !p));
                expanded[start..start + length].fill(true);
            }
            assert!(expanded.iter().any(|p| *p));
            assert!(expanded.iter().any(|p| !p));
            for (index, high) in expanded.into_iter().enumerate() {
                let pixel = a.pixel(index % width, index / width).unwrap();
                assert_eq!(pixel, b.pixel(index % width, index / width).unwrap());
                assert_eq!(
                    high,
                    matches!(pixel, IndexedPixel::Opaque { priority: true, .. })
                );
            }
        }
    }
}
