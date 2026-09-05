//! Immutable, bounded art adapter for the ordinary house preview.

use crate::{invalid, Result};
use assets::{
    graphics::{Bgr555, IndexedPixel, Tile4bpp},
    maps::visual::{StaticBackground, VisualResource},
    sprites::{ArkSprites, SpriteFrame, SpritePixel},
};
use room_core::{AnimationFrame, AnimationSet};
use serde_json::{json, Value};

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
                        .checked_sub(128)
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

pub(super) fn compile(rom: &rom::Rom) -> Result<Vec<u8>> {
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
                        mirror_x,
                    )?;
                    frames.insert(frame_key(key), pixels);
                }
            }
        }
    }
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
    Ok(serde_json::to_vec(
        &json!({"schema_version":1,"frames":frames,
        "foreground":{"15":foreground(&bedroom)?,"16":foreground(&house)?}}),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::{graphics::decode_tiles_4bpp, sprites::SpriteFrame};
    use room_core::{AnimationState, Direction};

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
        let normal = raster(&frame, &tiles, &palette, false).unwrap();
        let mirror = raster(&frame, &tiles, &palette, true).unwrap();
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
        source[23] = 0x10;
        assert!(raster(
            &SpriteFrame::decode(&source).unwrap(),
            &tiles,
            &palette,
            false
        )
        .is_err());
        let transparent_tiles = decode_tiles_4bpp(&[0; 32]).unwrap();
        let transparent = raster(
            &SpriteFrame::decode(&source).unwrap(),
            &transparent_tiles,
            &palette,
            false,
        )
        .unwrap();
        assert_eq!(transparent["rgba"], json!(vec![0; 8 * 8 * 4]));
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
        let art: Value = serde_json::from_slice(&compile(&rom).unwrap()).unwrap();
        assert_eq!(art["schema_version"], 1);
        assert_eq!(art["frames"].as_object().unwrap().len(), 28);
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
