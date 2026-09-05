//! Synthetic SNES graphics fixtures; no ROM resources.
use assets::graphics::{
    decode_tiles_4bpp, sample_metatile, BgTileWord, Bgr555, GraphicsError, IndexedPixel, Tile4bpp,
};

#[test]
fn decodes_literal_planar_fixture() {
    let bytes = [
        0xaa, 0xcc, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0, // Planes 0/1.
        0xf0, 0x0f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80, // Planes 2/3.
    ];
    let tile = Tile4bpp::decode(&bytes).unwrap();
    assert_eq!(&tile.pixels()[..8], &[7, 6, 5, 4, 11, 10, 9, 8]);
    assert_eq!(&tile.pixels()[8..56], &[0; 48]);
    assert_eq!(&tile.pixels()[56..], &[8, 0, 0, 0, 0, 0, 0, 1]);
}

#[test]
fn only_the_sampled_quadrant_requires_graphics() {
    let words = [0, 1, 2, 3].map(BgTileWord::new);
    let tiles = [Tile4bpp::decode(&[0xff; 32]).unwrap()];
    assert_eq!(
        sample_metatile(&words, &tiles, 7, 7),
        Ok(IndexedPixel::Opaque {
            palette_index: 15,
            priority: false
        })
    );
    for (x, y, index) in [(8, 0, 1), (0, 8, 2), (8, 8, 3)] {
        assert_eq!(
            sample_metatile(&words, &tiles, x, y),
            Err(GraphicsError::MissingTile { index })
        );
    }
}

#[test]
fn decodes_every_plane_bit_in_snes_order() {
    for plane in 0..4 {
        for y in 0..8 {
            for x in 0..8 {
                let mut bytes = [0; 32];
                bytes[(plane / 2) * 16 + y * 2 + plane % 2] = 0x80 >> x;
                let tile = Tile4bpp::decode(&bytes).unwrap();
                for py in 0..8 {
                    for px in 0..8 {
                        let expected = if (px, py) == (x, y) { 1 << plane } else { 0 };
                        assert_eq!(tile.pixel(px, py), Some(expected));
                        assert_eq!(tile.pixels()[py * 8 + px], expected);
                    }
                }
            }
        }
    }
    assert_eq!(Tile4bpp::decode(&[0xff; 32]).unwrap().pixels(), &[15; 64]);
}

#[test]
fn requires_exact_tile_sizes_and_bounds() {
    for length in [0, 1, 31, 33, 64] {
        assert_eq!(
            Tile4bpp::decode(&vec![0; length]),
            Err(GraphicsError::TileSize { actual: length })
        );
    }
    for length in [1, 31, 33, 63, 65] {
        assert_eq!(
            decode_tiles_4bpp(&vec![0; length]),
            Err(GraphicsError::TilesSize { actual: length })
        );
    }
    assert!(decode_tiles_4bpp(&[]).unwrap().is_empty());
    let bytes: Vec<_> = [0; 32].into_iter().chain([0xff; 32]).collect();
    let tiles = decode_tiles_4bpp(&bytes).unwrap();
    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].pixels(), &[0; 64]);
    assert_eq!(tiles[1].pixels(), &[15; 64]);
    for (x, y) in [(8, 0), (0, 8), (usize::MAX, 0), (0, usize::MAX)] {
        assert_eq!(tiles[0].pixel(x, y), None);
    }
}

#[test]
fn bgr555_preserves_raw_and_expands_at_full_brightness() {
    assert_eq!(Bgr555::new(0x001f).rgb8(), [255, 0, 0]);
    assert_eq!(Bgr555::new(0x03e0).rgb8(), [0, 255, 0]);
    assert_eq!(Bgr555::new(0x7c00).rgb8(), [0, 0, 255]);
    assert_eq!(Bgr555::new(0).rgb8(), [0, 0, 0]);
    assert_eq!(Bgr555::new(0xffff).rgb8(), [255; 3]);
    for raw in 0..=u16::MAX {
        let color = Bgr555::new(raw);
        assert_eq!(color.raw(), raw);
        let expected = [0, 5, 10].map(|shift| {
            let value = ((raw >> shift) & 31) as u8;
            (value << 3) | (value >> 2)
        });
        assert_eq!(color.rgb8(), expected);
        assert_eq!(Bgr555::from_rgb8(color.rgb8()).raw(), raw & 0x7fff);
    }
    // RGB packing truncates the low three bits and clears unused bit 15.
    assert_eq!(Bgr555::from_rgb8([7, 8, 248]).raw(), 0x7c20);
}

#[test]
fn background_words_retain_all_bits_and_extract_independent_fields() {
    for raw in 0..=u16::MAX {
        let word = BgTileWord::new(raw);
        assert_eq!(word.raw(), raw);
        assert_eq!(word.tile_index(), raw & 0x03ff);
        assert_eq!(word.palette(), ((raw >> 10) & 7) as u8);
        assert_eq!(word.priority(), raw & 0x2000 != 0);
        assert_eq!(word.hflip(), raw & 0x4000 != 0);
        assert_eq!(word.vflip(), raw & 0x8000 != 0);
    }
}

fn color(tile: usize, x: usize, y: usize) -> u8 {
    u8::try_from((tile * 3 + x + y * 5) % 16).unwrap()
}

fn patterned_tile(index: usize) -> Tile4bpp {
    let mut bytes = [0; 32];
    for y in 0..8 {
        for x in 0..8 {
            for plane in 0..4 {
                bytes[(plane / 2) * 16 + y * 2 + plane % 2] |=
                    ((color(index, x, y) >> plane) & 1) << (7 - x);
            }
        }
    }
    Tile4bpp::decode(&bytes).unwrap()
}

#[test]
fn samples_row_major_quadrants_with_tile_indices_palettes_and_all_flips() {
    let tiles: Vec<_> = (0..4).map(patterned_tile).collect();
    // TL/TR/BL/BR: deliberately not graphics-index order; all flip combinations.
    let words = [0x0402, 0x6800, 0x8c03, 0xfc01].map(BgTileWord::new);
    let indices = [2, 0, 3, 1];
    let palettes = [1, 2, 3, 7];
    for y in 0..16 {
        for x in 0..16 {
            let quadrant = (y / 8) * 2 + x / 8;
            let tx = if quadrant & 1 != 0 { 7 - x % 8 } else { x % 8 };
            let ty = if quadrant & 2 != 0 { 7 - y % 8 } else { y % 8 };
            let value = color(indices[quadrant], tx, ty);
            let expected = if value == 0 {
                IndexedPixel::Transparent
            } else {
                IndexedPixel::Opaque {
                    palette_index: palettes[quadrant] * 16 + value,
                    priority: quadrant & 1 != 0,
                }
            };
            assert_eq!(sample_metatile(&words, &tiles, x, y), Ok(expected));
        }
    }
}

#[test]
fn transparency_is_not_an_opaque_backdrop_and_bad_samples_are_errors() {
    let words = [BgTileWord::new(0x1c00); 4];
    let tiles = [Tile4bpp::decode(&[0; 32]).unwrap()];
    let transparent = sample_metatile(&words, &tiles, 0, 0).unwrap();
    assert_eq!(transparent, IndexedPixel::Transparent);
    assert_ne!(
        transparent,
        IndexedPixel::Opaque {
            palette_index: 0,
            priority: false
        }
    );
    for (x, y) in [(16, 0), (0, 16), (usize::MAX, usize::MAX)] {
        assert_eq!(
            sample_metatile(&words, &tiles, x, y),
            Err(GraphicsError::PixelOutOfBounds { x, y })
        );
    }
    assert_eq!(
        sample_metatile(&words, &[], 0, 0),
        Err(GraphicsError::MissingTile { index: 0 })
    );
    let words = [BgTileWord::new(0x03ff); 4];
    assert_eq!(
        sample_metatile(&words, &tiles, 0, 0),
        Err(GraphicsError::MissingTile { index: 1023 })
    );
    let tiles = vec![tiles[0]; 1024];
    assert_eq!(
        sample_metatile(&words, &tiles, 15, 15),
        Ok(IndexedPixel::Transparent)
    );
}
