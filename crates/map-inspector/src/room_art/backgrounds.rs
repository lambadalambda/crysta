//! Additional source sheets; no scene progression, patches or capture-fed grids.
use super::{foreground, invalid, json, BTreeMap, Result, Value};
use assets::maps::visual::pandora::{PandoraBackground, SourceCamera};

pub(super) struct Backgrounds {
    pub(super) sheets: BTreeMap<&'static str, Vec<u8>>,
    pub(super) masks: BTreeMap<u16, Value>,
    pub(super) keys: BTreeMap<u16, &'static str>,
    pub(super) manifest: Value,
    pub(super) cameras: BTreeMap<u16, SourceCamera>,
}

pub(super) fn compile(rom: &rom::Rom) -> Result<Backgrounds> {
    let mut result = Backgrounds {
        sheets: BTreeMap::new(),
        masks: BTreeMap::new(),
        keys: BTreeMap::new(),
        manifest: json!({}),
        cameras: BTreeMap::new(),
    };
    for (map, key, url) in [
        (0x13, "town13", "/town13.bmp"),
        (0xe, "cellars", "/cellars.bmp"),
        (0x20, "cellars", "/cellars.bmp"),
        (0x21, "box", "/box.bmp"),
        (0x41, "tour", "/tour.bmp"),
        (0x42, "tour", "/tour.bmp"),
        (0x43, "tour", "/tour.bmp"),
        (0x44, "tour", "/tour.bmp"),
    ] {
        let source = PandoraBackground::from_rom(rom.image(), map)?;
        let background = source.background();
        let width = background.layer().width() * 16;
        let height = background.layer().height() * 16;
        let rgb = (0..width * height)
            .map(|i| {
                let index = match background.pixel(i % width, i / width)? {
                    assets::graphics::IndexedPixel::Transparent => 0,
                    assets::graphics::IndexedPixel::Opaque { palette_index, .. } => palette_index,
                };
                Ok(crate::visual_export::pixel_rgb(
                    index,
                    background,
                    i % width,
                    i / width,
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let bitmap = crate::bitmap(&rgb, width, height)?;
        if let Some(existing) = result.sheets.get(key) {
            if existing != &bitmap {
                return Err(invalid("Pandora shared sheet pixels differ").into());
            }
        } else {
            result.sheets.insert(key, bitmap);
            result.manifest[key] = json!({"url":url,"width":width,"height":height});
        }
        result.keys.insert(map, key);
        result.masks.insert(map, foreground(background)?);
        result.cameras.insert(map, *source.camera());
    }
    Ok(result)
}

pub(super) fn append(
    rom: &rom::Rom,
    bundle: &mut Value,
) -> Result<BTreeMap<&'static str, Vec<u8>>> {
    let backgrounds = compile(rom)?;
    let cameras: BTreeMap<_, _> = backgrounds
        .cameras
        .iter()
        .map(|(&map, camera)| {
            (
                map,
                json!({
                    "bounds":camera.bounds,"vertical_extent":camera.vertical_extent,
                    "hardware_background":camera.hardware_background,"bgmode":camera.bgmode,
                    "policy":"settled-source-clamp-not-transition-pan"
                }),
            )
        })
        .collect();
    // Kept separate until the aggregate runtime/frontend admits these maps.
    bundle["pandora_backgrounds"] = json!({"backgrounds":backgrounds.manifest,
        "background_keys":backgrounds.keys,"foreground":backgrounds.masks,"cameras":cameras,
        "policy":"natural-palette-first-background-checker-transparency-no-phase-patches"});
    Ok(backgrounds.sheets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::graphics::IndexedPixel;

    fn assert_transport(rom: &rom::Rom, compiled: &Backgrounds) {
        let mut bundle = json!({"backgrounds":{"house":"unchanged"}});
        assert_eq!(append(rom, &mut bundle).unwrap(), compiled.sheets);
        assert_eq!(bundle["backgrounds"], json!({"house":"unchanged"}));
        let data = &bundle["pandora_backgrounds"];
        assert_eq!(data["foreground"], json!(compiled.masks));
        assert_eq!(
            data["background_keys"],
            json!({"19":"town13","14":"cellars","32":"cellars","33":"box","65":"tour","66":"tour","67":"tour","68":"tour"})
        );
        assert_eq!(
            data["backgrounds"],
            json!({
                "town13":{"url":"/town13.bmp","width":1024,"height":512},
                "cellars":{"url":"/cellars.bmp","width":512,"height":1024},
                "box":{"url":"/box.bmp","width":256,"height":512},
                "tour":{"url":"/tour.bmp","width":512,"height":512}
            })
        );
        assert_eq!(data["cameras"].as_object().unwrap().len(), 8);
        for (map, camera) in &compiled.cameras {
            assert_eq!(
                data["cameras"][map.to_string()],
                json!({
                    "bounds":camera.bounds,"vertical_extent":camera.vertical_extent,
                    "hardware_background":camera.hardware_background,"bgmode":camera.bgmode,
                    "policy":"settled-source-clamp-not-transition-pan"
                })
            );
        }
    }

    #[test]
    fn extra_sheets_and_masks_match_every_source_pixel_and_camera() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let compiled = compile(&rom).unwrap();
        assert_transport(&rom, &compiled);
        assert_eq!(compiled.sheets.len(), 4);
        assert_eq!(compiled.keys.len(), 8);
        for (map, key) in &compiled.keys {
            let source = PandoraBackground::from_rom(rom.image(), *map).unwrap();
            let background = source.background();
            assert_eq!(&compiled.cameras[map], source.camera());
            let bmp = &compiled.sheets[key];
            assert_eq!(&bmp[..2], b"BM");
            let width = background.layer().width() * 16;
            let height = background.layer().height() * 16;
            assert_eq!(
                i32::from_le_bytes(bmp[18..22].try_into().unwrap()),
                i32::try_from(width).unwrap()
            );
            assert_eq!(
                i32::from_le_bytes(bmp[22..26].try_into().unwrap()),
                -i32::try_from(height).unwrap()
            );
            let mut high = vec![false; width * height];
            for run in compiled.masks[map]["runs"]
                .as_array()
                .unwrap()
                .chunks_exact(2)
            {
                let start = usize::try_from(run[0].as_u64().unwrap()).unwrap();
                let length = usize::try_from(run[1].as_u64().unwrap()).unwrap();
                high[start..start + length].fill(true);
            }
            for (i, actual_high) in high.into_iter().enumerate() {
                let pixel = background.pixel(i % width, i / width).unwrap();
                let index = match pixel {
                    IndexedPixel::Transparent => 0,
                    IndexedPixel::Opaque { palette_index, .. } => palette_index,
                };
                assert_eq!(
                    actual_high,
                    matches!(pixel, IndexedPixel::Opaque { priority: true, .. })
                );
                let [r, g, b] =
                    crate::visual_export::pixel_rgb(index, background, i % width, i / width);
                assert_eq!(&bmp[54 + i * 3..54 + i * 3 + 3], &[b, g, r]);
            }
        }
        assert_eq!(compiled.manifest["town13"]["width"], 1024);
        assert_eq!(compiled.manifest["town13"]["height"], 512);
        assert_eq!(compiled.manifest["box"]["width"], 256);
        assert_eq!(compiled.cameras[&0xe].settled_origin([152, 880]), [0, 768]);
        assert_eq!(
            compiled.cameras[&0x20].settled_origin([408, 880]),
            [256, 768]
        );
        let house = assets::maps::visual::StaticBackground::from_rom(rom.image(), 15).unwrap();
        let cellar = PandoraBackground::from_rom(rom.image(), 0xe).unwrap();
        assert_ne!(
            house.palette(),
            cellar.background().palette(),
            "cellar cannot reuse the ordinary house palette/sheet"
        );
    }
}
