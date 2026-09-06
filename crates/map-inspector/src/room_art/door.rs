//! Final source wooden-door metatiles for the shared house sheet.
use crate::{invalid, Result};
use assets::{
    graphics::{sample_metatile, IndexedPixel},
    maps::visual::StaticBackground,
};
use serde_json::{json, Value};

pub(super) fn replacements(image: &[u8], background: &StaticBackground) -> Result<Vec<(u16, u16)>> {
    let mut replacements = Vec::new();
    // COP44 final writes; high operand byte is a delay, not a tile attribute.
    // Source placement is (136,336) with signed zero offsets; the second
    // instruction targets the metatile immediately above. See house-navigation.
    for (site, cell, closed, expected, raw) in [
        (0x07_9829, 616, 0xf2, 0xf6, 0x1cf6),
        (0x07_9819, 648, 0xf3, 0xf7, 0x00f7),
    ] {
        let op = image
            .get(site..site + 6)
            .ok_or_else(|| invalid("truncated door source"))?;
        if op[..4] != [0x02, 0x44, 0, 0] || usize::from(op[4]) != expected {
            return Err(invalid("unqualified door final write").into());
        }
        let tile = usize::from(op[4]);
        let attribute = background.resources()[3].decoded()[tile] & 0x7f;
        if background.layer().cells()[cell].raw() & 511 != closed
            || u16::from(op[4]) | (u16::from(attribute) << 9) != raw
        {
            return Err(invalid("door visual/collision source differs").into());
        }
        replacements.push((u16::try_from(cell)?, u16::try_from(tile)?));
    }
    Ok(replacements)
}

/// Cells are 16-aligned, matching the 16-pixel preview checker period. The
/// tile-local raster therefore works at every admitted cell on the same sheet.
pub(super) fn tile_pixels(background: &StaticBackground, tile: u16) -> Result<Value> {
    let metatile = background
        .metatiles()
        .get(usize::from(tile))
        .ok_or_else(|| invalid("missing replacement metatile"))?;
    let mut rgba = Vec::new();
    let mut high = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            let pixel = sample_metatile(metatile, background.tiles(), x, y)?;
            let index = match pixel {
                IndexedPixel::Transparent => 0,
                IndexedPixel::Opaque { palette_index, .. } => palette_index,
            };
            rgba.extend(crate::visual_export::pixel_rgb(index, background, x, y));
            rgba.push(255);
            high.push(super::occludes(pixel));
        }
    }
    Ok(json!({"rgba":rgba,"high":high}))
}

pub(super) fn compile(image: &[u8], background: &StaticBackground) -> Result<Value> {
    let mut patches = Vec::new();
    for (cell, tile) in replacements(image, background)? {
        let mut patch = tile_pixels(background, tile)?;
        patch["position"] = json!([(cell % 32) * 16, (cell / 32) * 16]);
        patches.push(patch);
    }
    Ok(json!(patches))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_door_operands_and_complete_replacement_pixels() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let background = StaticBackground::from_rom(rom.image(), 15).unwrap();
        let patches = compile(rom.image(), &background).unwrap();
        for (patch, tile, position) in [
            (&patches[0], 0xf6, [128, 304]),
            (&patches[1], 0xf7, [128, 320]),
        ] {
            assert_eq!(patch["position"], json!(position));
            assert_eq!(patch["rgba"].as_array().unwrap().len(), 1024);
            assert_eq!(patch["high"].as_array().unwrap().len(), 256);
            for i in 0..256 {
                let pixel = sample_metatile(
                    &background.metatiles()[tile],
                    background.tiles(),
                    i % 16,
                    i / 16,
                )
                .unwrap();
                let (index, high) = match pixel {
                    IndexedPixel::Transparent => (0, false),
                    IndexedPixel::Opaque {
                        palette_index,
                        priority,
                    } => (palette_index, priority),
                };
                assert_eq!(patch["high"][i], high);
                let mut rgba = crate::visual_export::pixel_rgb(
                    index,
                    &background,
                    position[0] + i % 16,
                    position[1] + i / 16,
                )
                .to_vec();
                rgba.push(255);
                assert_eq!(
                    &patch["rgba"].as_array().unwrap()[i * 4..i * 4 + 4],
                    &serde_json::to_value(rgba).unwrap().as_array().unwrap()[..]
                );
            }
        }
        let mut image = rom.image().to_vec();
        for at in [0x07_9819, 0x07_981b, 0x07_981d, 0x07_9829, 0x07_982d] {
            image[at] ^= 1;
            assert!(compile(&image, &background).is_err());
            image[at] ^= 1;
        }
        assert!(compile(&image[..0x07_982d], &background).is_err());
    }
}
