//! Source-projected baseclosed house geometry and cameras; no actor/event runtime.
use crate::{invalid, Result};
use assets::maps::visual::StaticBackground;
use room_core::Room;

pub(super) const MAPS: [u16; 6] = [0x0b, 0x0c, 0x0d, 0x0f, 0x10, 0x11];

/// Compile the source-setup passive occupancy, before navigation's door patch.
pub(super) fn compile(rom: &rom::Rom, map: u16, fresh: bool) -> Result<Room> {
    if fresh && map != 15 {
        return Err(invalid("fresh residue belongs to bedroom only").into());
    }
    // COP3B owners on the selected fresh branch. The second column is the
    // source-qualified stamp width, not a generic bounding-box intersection.
    let owners: &[(usize, usize)] = match map {
        0x0b => &[(0x83_8b96, 1), (0x83_8ba0, 1), (0x83_8ba7, 1)],
        0x0c => &[
            (0x83_8c0a, 1),
            (0x83_8c14, 1),
            (0x83_8c1e, 1),
            (0x83_8c28, 1),
        ],
        0x0d => &[(0x83_8cb4, 1), (0x83_8cc8, 1)], // source-origin wanderer + independent exterior gate
        0x0f => &[(0x83_8d4f, 1)],
        0x10 => &[(0x83_8d7c, 1), (0x83_8d86, 1), (0x83_8da6, 2)],
        0x11 => &[(0x83_8de2, 1), (0x83_8dec, 1)],
        _ => return Err(invalid("unqualified house profile").into()),
    };
    let background = StaticBackground::from_rom(rom.image(), map)?;
    if (background.layer().width(), background.layer().height()) != (32, 64) {
        return Err(invalid("unqualified house sheet extent").into());
    }
    let attributes: &[u8; 512] = background.resources()[3].decoded().try_into()?;
    let mut cells: Vec<_> = background
        .layer()
        .attributed_cells(attributes)
        .iter()
        .map(|c| c.raw())
        .collect();
    for &(record, columns) in owners {
        for index in stamp(rom.image(), record, columns)? {
            cells[index] |= 0x8000;
        }
    }
    // The departed intro actor stamps (392,256), then unlinks without clearing.
    // This history residue is not another resident or a second table-child stamp.
    if fresh {
        cells[504] |= 0x8000;
    }
    Ok(Room::new_passive(32, 64, cells)?)
}

fn stamp(image: &[u8], record: usize, columns: usize) -> Result<Vec<usize>> {
    let at = record & 0x3f_ffff;
    let position = image
        .get(at + 1..at + 3)
        .ok_or_else(|| invalid("truncated stamp owner"))?;
    let (x, y) = (usize::from(position[0]), usize::from(position[1]));
    if !(1..=2).contains(&columns) || x + columns > 32 || !(1..=64).contains(&y) {
        return Err(invalid("unqualified house stamp geometry").into());
    }
    // Native small-footprint origin is (entity.x-8,entity.y-16); source spawns
    // are (tile_x*16+8,tile_y*16). The one wide hidden owner stamps two columns.
    Ok((0..columns)
        .map(|column| (y - 1) * 32 + x + column)
        .collect())
}

pub(super) fn camera(image: &[u8], map: u16) -> Result<[u16; 2]> {
    if !MAPS.contains(&map) {
        return Err(invalid("unqualified house camera map").into());
    }
    let at = 0x16_be30 + usize::from(map) * 2;
    let record = image
        .get(at..at + 2)
        .ok_or_else(|| invalid("truncated camera source"))?;
    if record[0] >> 4 != 1 || record[1] >> 4 != 1 || record[0] & 15 > 1 || record[1] & 15 > 3 {
        return Err(invalid("unqualified house camera bounds").into());
    }
    Ok([
        u16::from(record[0] & 15) * 256,
        u16::from(record[1] & 15) * 256,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn stamps_follow_source_coordinates_and_qualified_column_geometry() {
        let mut image = vec![0; 0x38003];
        image[0x38001..0x38003].copy_from_slice(&[27, 23]);
        assert_eq!(stamp(&image, 0x83_8000, 2).unwrap(), vec![731, 732]);
        image[0x38001..0x38003].copy_from_slice(&[4, 42]);
        assert_eq!(stamp(&image, 0x83_8000, 1).unwrap(), vec![1316]);
        image[0x38001] = 31;
        assert!(stamp(&image, 0x83_8000, 2).is_err());
        image[0x38002] = 0;
        assert!(stamp(&image, 0x83_8000, 1).is_err());
        assert!(stamp(&image[..0x38002], 0x83_8000, 1).is_err());
    }

    #[test]
    fn cameras_follow_one_page_source_records_not_map_id_arithmetic() {
        let mut image = vec![0; 0x16_bf00];
        let at = 0x16_be30 + 12 * 2;
        image[at..at + 2].copy_from_slice(&[0x11, 0x12]);
        assert_eq!(camera(&image, 12).unwrap(), [256, 512]);
        image[at] = 0x21;
        assert!(camera(&image, 12).is_err());
        assert!(camera(&image, 14).is_err());
        assert!(camera(&image[..=at], 12).is_err());
    }

    #[test]
    fn owned_rom_profiles_match_every_source_qualified_grid_and_camera() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        if !path.try_exists().unwrap() {
            eprintln!("SKIP: local Japanese ROM absent");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
        let pins: Value = serde_json::from_str(include_str!(
            "../../../tools/house-background-qualification/profiles.json"
        ))
        .unwrap();
        assert_eq!(crate::sha256(rom.image()), pins["rom_sha256"]);
        for profile in pins["profiles"].as_array().unwrap() {
            let map = u16::try_from(profile["map"].as_u64().unwrap()).unwrap();
            let grid = compile(&rom, map, profile["history"] == "post-intro-first-load").unwrap();
            let bytes: Vec<_> = grid.cells().iter().flat_map(|c| c.to_le_bytes()).collect();
            assert_eq!(crate::sha256(&bytes), profile["grid_sha256"], "{map:X}");
            assert_eq!(
                serde_json::json!(camera(rom.image(), map).unwrap()),
                profile["camera"]
            );
        }
        assert!(compile(&rom, 12, true).is_err());
        assert!(compile(&rom, 14, false).is_err());
    }
}
