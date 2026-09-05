//! Synthetic loaded-map metadata; no ROM-derived fixture bytes.
use assets::maps::{LoadedMap, MapError};

fn wram() -> Vec<u8> {
    let mut data = vec![0; 0x20000];
    data[0x47E..0x480].copy_from_slice(&0x0128_u16.to_le_bytes());
    data[0x826..0x828].copy_from_slice(&256_u16.to_le_bytes());
    data[0x82A..0x82C].copy_from_slice(&256_u16.to_le_bytes());
    data[0x80E..0x810].copy_from_slice(&32_u16.to_le_bytes());
    data[0x812..0x814].copy_from_slice(&48_u16.to_le_bytes());
    data[0x1000..0x1002].copy_from_slice(&64_u16.to_le_bytes());
    data[0x1002..0x1004].copy_from_slice(&80_u16.to_le_bytes());
    data[0xA022..0xA024].copy_from_slice(&0x1D45_u16.to_le_bytes());
    data
}

#[test]
fn extracts_metadata_and_preserves_raw_cell_words() {
    let data = wram();
    let map = LoadedMap::from_wram(&data).unwrap();
    assert_eq!(map.map_id(), 0x0128);
    assert_eq!((map.width(), map.height()), (16, 16));
    assert_eq!(map.camera(), (32, 48));
    assert_eq!(map.player(), (64, 80));
    let cell = map.cell(1, 1).unwrap();
    assert_eq!(cell.raw(), 0x1D45);
    assert_eq!(cell.tile_index(), 0x145);
    assert_eq!(cell.collision_code(), 0x1C);
    assert_eq!(map.cell_at_pixel(31, 31), Some(cell));
    assert_eq!(map.cell(16, 0), None);
    assert_eq!(map.cell(0, 16), None);
    assert_eq!(map.cell_at_pixel(256, 0), None);
    assert_eq!(map.layer_bytes(), data[0xA000..0xA200]);
}

#[test]
fn rejects_truncated_memory_invalid_dimensions_and_layer_overflow() {
    assert!(matches!(
        LoadedMap::from_wram(&[]),
        Err(MapError::WramSize { .. })
    ));
    let mut data = wram();
    data[0x826..0x828].fill(0);
    assert!(matches!(
        LoadedMap::from_wram(&data),
        Err(MapError::InvalidDimensions { .. })
    ));
    data[0x826..0x828].copy_from_slice(&257_u16.to_le_bytes());
    assert!(matches!(
        LoadedMap::from_wram(&data),
        Err(MapError::InvalidDimensions { .. })
    ));
    data[0x826..0x828].copy_from_slice(&4096_u16.to_le_bytes());
    data[0x82A..0x82C].copy_from_slice(&4096_u16.to_le_bytes());
    assert!(matches!(
        LoadedMap::from_wram(&data),
        Err(MapError::LayerTooLarge { .. })
    ));
}
