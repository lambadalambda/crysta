//! Finite effective-room tile projection; no event/phase inference.
use super::{invalid, json, BTreeMap, Result, Value};
use room_core::Room;
use std::collections::BTreeSet;

struct Sheet {
    width: u16,
    height: u16,
    base: Vec<u16>,
    candidates: BTreeMap<u16, BTreeSet<u16>>,
    tiles: BTreeMap<u16, Value>,
}

impl Sheet {
    fn patches(&self, room: &Room) -> Result<Value> {
        if room.width() != self.width || room.height() != self.height {
            return Err(invalid("world patch room extent differs").into());
        }
        let mut patches = Vec::new();
        for (cell, (&original, &raw)) in self.base.iter().zip(room.cells()).enumerate() {
            let tile = raw & 511;
            if tile == original {
                continue;
            }
            let cell = u16::try_from(cell)?;
            if !self
                .candidates
                .get(&cell)
                .is_some_and(|tiles| tiles.contains(&tile))
            {
                return Err(invalid("unqualified effective-room visual patch").into());
            }
            patches.push(json!({"cell":cell,"tile":tile}));
        }
        Ok(json!(patches))
    }
}

fn background_key(map: u16) -> Result<&'static str> {
    match map {
        0xa => Ok("exterior"),
        0xb | 0xc | 0xd | 0xf | 0x10 | 0x11 => Ok("house"),
        0xe | 0x20 => Ok("cellars"),
        0x13 => Ok("town13"),
        0x21 => Ok("box"),
        0x41..=0x44 => Ok("tour"),
        _ => Err(invalid("unqualified world background map").into()),
    }
}

pub(super) struct World {
    sheets: BTreeMap<&'static str, Sheet>,
}
impl World {
    pub(super) fn manifest(&self) -> Value {
        json!(self
            .sheets
            .iter()
            .map(|(key, s)| (*key, json!({"tiles":s.tiles,"candidates":s.candidates})))
            .collect::<BTreeMap<_, _>>())
    }
    pub(super) fn state(&self, map: u16, effective: &Room) -> Result<Value> {
        let key = background_key(map)?;
        let sheet = self
            .sheets
            .get(key)
            .ok_or_else(|| invalid("world background absent"))?;
        Ok(json!({"key":key,"patches":sheet.patches(effective)?}))
    }
}

pub(super) fn append(rom: &rom::Rom, bundle: &mut Value) -> Result<World> {
    let world = compile(rom)?;
    bundle["world_backgrounds"] = world.manifest();
    Ok(world)
}

fn compile(rom: &rom::Rom) -> Result<World> {
    use assets::maps::visual::{pandora::PandoraBackground, StaticBackground};
    let nav = crate::pandora_navigation::compile(rom.image())?;
    let objects = crate::pandora_navigation::source_objects(rom.image(), &nav)?;
    let house = StaticBackground::from_rom(rom.image(), 0xf)?;
    let wood = super::door::replacements(rom.image(), &house)?;
    let mut sheets = BTreeMap::new();
    for (key, map) in [
        ("house", 0xf),
        ("exterior", 0xa),
        ("cellars", 0xe),
        ("town13", 0x13),
        ("box", 0x21),
        ("tour", 0x41),
    ] {
        let source;
        let background = if key == "house" {
            &house
        } else {
            source = PandoraBackground::from_rom(rom.image(), map)?;
            source.background()
        };
        let mut sheet = Sheet {
            width: u16::try_from(background.layer().width())?,
            height: u16::try_from(background.layer().height())?,
            base: background
                .layer()
                .cells()
                .iter()
                .map(|cell| cell.raw() & 511)
                .collect(),
            candidates: BTreeMap::new(),
            tiles: BTreeMap::new(),
        };
        let shared = matches!(key, "house" | "cellars");
        for profile in nav.profiles() {
            let profile_key = background_key(profile.map())?;
            if profile_key != key && !(shared && matches!(profile_key, "house" | "cellars")) {
                continue;
            }
            if profile.room().width() != sheet.width || profile.room().height() != sheet.height {
                return Err(invalid("shared world profile extent differs").into());
            }
            for (cell, (&base, raw)) in sheet.base.iter().zip(profile.room().cells()).enumerate() {
                let tile = raw & 511;
                if base != tile {
                    sheet
                        .candidates
                        .entry(u16::try_from(cell)?)
                        .or_default()
                        .insert(tile);
                }
            }
        }
        if shared {
            if sheet.base
                != house
                    .layer()
                    .cells()
                    .iter()
                    .map(|cell| cell.raw() & 511)
                    .collect::<Vec<_>>()
            {
                return Err(invalid("shared world bitmap source grid differs").into());
            }
            for &(cell, tile) in &wood {
                sheet.candidates.entry(cell).or_default().insert(tile);
            }
            // Exactly the same immutable bounded object catalog as the core compiler,
            // not a second scan of all FA/FB words across the sheet.
            for object in &objects {
                if sheet.base.get(usize::from(object.cell)) != Some(&(object.raw & 511)) {
                    return Err(invalid("shared source object base differs").into());
                }
                sheet
                    .candidates
                    .entry(object.cell)
                    .or_default()
                    .insert(object.replacement & 511);
            }
        }
        let tiles: BTreeSet<_> = sheet.candidates.values().flatten().copied().collect();
        for tile in tiles {
            sheet
                .tiles
                .insert(tile, super::door::tile_pixels(background, tile)?);
        }
        sheets.insert(key, sheet);
    }
    Ok(World { sheets })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned_rom() -> Option<rom::Rom> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return None;
        }
        Some(rom::Rom::load(&std::fs::read(path).unwrap()).unwrap())
    }
    #[test]
    fn authenticated_candidates_share_geometry_not_palettes_and_accept_every_profile() {
        let Some(rom) = owned_rom() else {
            return;
        };
        let world = compile(&rom).unwrap();
        let nav = crate::pandora_navigation::compile(rom.image()).unwrap();
        let house = &world.sheets["house"];
        let cellars = &world.sheets["cellars"];
        assert_eq!(house.base, cellars.base);
        assert_eq!(house.candidates, cellars.candidates);
        assert_ne!(house.tiles, cellars.tiles);
        for (cell, tiles) in [
            (616, vec![0xf6]),
            (648, vec![0xf7]),
            (651, vec![0xf6, 0x1a7]),
            (683, vec![0xcb]),
            (675, vec![0xf8]),
            (676, vec![0xf8]),
            (677, vec![0xf8]),
        ] {
            assert_eq!(house.candidates[&cell], tiles.into_iter().collect());
        }
        assert_eq!(
            world.sheets["exterior"].candidates,
            BTreeMap::from([
                (1053, BTreeSet::from([0xf6])),
                (1117, BTreeSet::from([0xf7])),
                (2911, BTreeSet::from([0xf6])),
                (2975, BTreeSet::from([0xf7]))
            ])
        );
        for profile in nav.profiles() {
            world.state(profile.map(), profile.room()).unwrap();
        }
        for key in ["town13", "box", "tour"] {
            assert!(world.sheets[key].candidates.is_empty());
        }
        assert_eq!(world.manifest().as_object().unwrap().len(), 6);
        let mut changed = rom.image().to_vec();
        changed[0x8aba8] ^= 1;
        assert!(rom::Rom::load(&changed).is_err());
        assert!(crate::pandora_navigation::compile(&changed).is_err());
    }

    #[test]
    fn every_candidate_pixel_and_mask_matches_its_source_palette_at_world_cell() {
        use assets::{
            graphics::{sample_metatile, IndexedPixel},
            maps::visual::{pandora::PandoraBackground, StaticBackground},
        };
        let Some(rom) = owned_rom() else {
            return;
        };
        let world = compile(&rom).unwrap();
        let house = StaticBackground::from_rom(rom.image(), 0xf).unwrap();
        let mut pixels = 0;
        let mut high_pixels = 0;
        let mut transparent_pixels = 0;
        for (key, map) in [("house", 0xf), ("cellars", 0xe), ("exterior", 0xa)] {
            let source;
            let bg = if key == "house" {
                &house
            } else {
                source = PandoraBackground::from_rom(rom.image(), map).unwrap();
                source.background()
            };
            let sheet = &world.sheets[key];
            for (&cell, tiles) in &sheet.candidates {
                let (wx, wy) = (
                    usize::from(cell % sheet.width) * 16,
                    usize::from(cell / sheet.width) * 16,
                );
                for &tile in tiles {
                    let raster = &sheet.tiles[&tile];
                    assert_eq!(raster["rgba"].as_array().unwrap().len(), 1024);
                    assert_eq!(raster["high"].as_array().unwrap().len(), 256);
                    for i in 0..256 {
                        let sample = sample_metatile(
                            &bg.metatiles()[usize::from(tile)],
                            bg.tiles(),
                            i % 16,
                            i / 16,
                        )
                        .unwrap();
                        let (index, high) = match sample {
                            IndexedPixel::Transparent => {
                                transparent_pixels += 1;
                                (0, false)
                            }
                            IndexedPixel::Opaque {
                                palette_index,
                                priority,
                            } => (palette_index, priority),
                        };
                        let mut rgba =
                            crate::visual_export::pixel_rgb(index, bg, wx + i % 16, wy + i / 16)
                                .to_vec();
                        rgba.push(255);
                        assert_eq!(
                            &raster["rgba"].as_array().unwrap()[i * 4..i * 4 + 4],
                            json!(rgba).as_array().unwrap()
                        );
                        assert_eq!(raster["high"][i], high);
                        high_pixels += usize::from(high);
                        pixels += 1;
                    }
                }
            }
        }
        // All admitted replacement samples are opaque low BG in the owned ROM.
        // High/transparent replacement behavior is exercised by synthetic frontend
        // controls, not falsely attributed to this source catalog.
        assert_eq!((pixels, high_pixels, transparent_pixels), (5120, 0, 0));
    }

    fn sheet() -> Sheet {
        Sheet {
            width: 2,
            height: 2,
            base: vec![0x180, 0x181, 0xfa, 0xfb],
            candidates: BTreeMap::from([
                (0, BTreeSet::from([0x1a7, 0xf6])),
                (2, BTreeSet::from([0xf8])),
            ]),
            tiles: BTreeMap::new(),
        }
    }
    #[test]
    fn diff_uses_original_bitmap_tiles_ignores_attributes_and_sorts() {
        let sheet = sheet();
        let room = Room::new_passive(2, 2, vec![0x9da7, 0x0b81, 0x00f8, 0x98fb]).unwrap();
        assert_eq!(
            sheet.patches(&room).unwrap(),
            json!([{"cell":0,"tile":0x1a7},{"cell":2,"tile":0xf8}])
        );
        let restored = Room::new_passive(2, 2, vec![0x9d80, 0x8181, 0x18fa, 0x18fb]).unwrap();
        assert_eq!(sheet.patches(&restored).unwrap(), json!([]));
    }
    #[test]
    fn unknown_cell_tile_and_extent_fail_without_mutation() {
        let sheet = sheet();
        for cells in [
            vec![0x0a7, 0x181, 0xfa, 0xfb],
            vec![0x180, 0xf6, 0xfa, 0xfb],
            vec![0x180, 0x181, 0xfa, 0xf8],
        ] {
            assert!(sheet
                .patches(&Room::new_passive(2, 2, cells).unwrap())
                .is_err());
        }
        assert!(sheet
            .patches(&Room::new_passive(1, 4, sheet.base.clone()).unwrap())
            .is_err());
        assert_eq!(sheet.base, vec![0x180, 0x181, 0xfa, 0xfb]);
    }
}
