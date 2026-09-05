//! Synthetic sprite composition contract: no cartridge or oracle required.
use assets::{
    graphics::{decode_tiles_4bpp, Bgr555},
    sprites::{SpriteFrame, SpritePixel},
};

fn tile(color: u8) -> Vec<u8> {
    let mut b = vec![0; 32];
    for y in 0..8 {
        for p in 0..4 {
            if color & (1 << p) != 0 {
                b[(p / 2) * 16 + y * 2 + p % 2] = 255;
            }
        }
    }
    b
}
fn frame(parts: &[[u8; 7]]) -> Vec<u8> {
    let mut b = vec![4, 12, 24, 8];
    b.extend([0; 12]);
    b.push(u8::try_from(parts.len()).unwrap());
    for p in parts {
        b.extend(p);
    }
    b
}
#[test]
fn preserves_source_offsets_priority_palette_and_transparency() {
    let bytes = frame(&[[0, 2, 10, 3, 13, 0, 0x2a]]);
    let f = SpriteFrame::decode(&bytes).unwrap();
    assert_eq!(f.source_bytes(), bytes);
    let tiles = decode_tiles_4bpp(&tile(3)).unwrap();
    assert_eq!(
        f.sample(&tiles, false, false, -2, -21).unwrap(),
        SpritePixel::Opaque {
            palette_index: 211,
            priority: 2,
            component: 0
        }
    );
    assert_eq!(
        f.sample(&tiles, false, false, -3, -21).unwrap(),
        SpritePixel::Transparent
    );
    assert_eq!(
        f.sample(&tiles, true, true, -2, 5).unwrap(),
        SpritePixel::Opaque {
            palette_index: 211,
            priority: 2,
            component: 0
        }
    );
    let zero = decode_tiles_4bpp(&tile(0)).unwrap();
    assert_eq!(
        f.sample(&zero, false, false, -2, -21).unwrap(),
        SpritePixel::Transparent
    );
    assert_eq!(Bgr555::new(0x7fff).rgb8(), [255; 3]);
}
#[test]
fn large_component_uses_snes_stride_and_whole_component_flips() {
    let mut b = vec![0; 18 * 32];
    for (i, color) in [(0, 1), (1, 2), (16, 3), (17, 4)] {
        b[i * 32..i * 32 + 32].copy_from_slice(&tile(color));
    }
    let tiles = decode_tiles_4bpp(&b).unwrap();
    let f = SpriteFrame::decode(&frame(&[[1, 4, 12, 24, 8, 0, 0xc0]])).unwrap();
    for (x, y, color) in [(0, 0, 4), (8, 0, 3), (0, 8, 2), (8, 8, 1)] {
        assert_eq!(
            f.sample(&tiles, false, false, x, y).unwrap(),
            SpritePixel::Opaque {
                palette_index: 128 + color,
                priority: 0,
                component: 0
            }
        );
    }
    assert_eq!(
        f.sample(&tiles, true, true, 0, 0).unwrap(),
        SpritePixel::Opaque {
            palette_index: 129,
            priority: 0,
            component: 0
        }
    );
}
#[test]
fn component_order_not_priority_wins_and_transparent_holes_show_later_parts() {
    let mut b = tile(0);
    b.extend(tile(2));
    b.extend(tile(3));
    let tiles = decode_tiles_4bpp(&b).unwrap();
    let f = SpriteFrame::decode(&frame(&[
        [0, 4, 12, 24, 8, 0, 0],
        [0, 4, 12, 24, 8, 1, 0],
        [0, 4, 12, 24, 8, 2, 0x30],
    ]))
    .unwrap();
    assert_eq!(
        f.sample(&tiles, false, false, 0, 0).unwrap(),
        SpritePixel::Opaque {
            palette_index: 130,
            priority: 0,
            component: 1
        }
    );
}
#[test]
fn rejects_malformed_records_and_missing_tiles_without_panics() {
    let b = frame(&[[1, 4, 12, 24, 8, 0, 0]]);
    for end in 0..b.len() {
        assert!(SpriteFrame::decode(&b[..end]).is_err());
    }
    let mut trailing = b.clone();
    trailing.push(0);
    assert!(SpriteFrame::decode(&trailing).is_err());
    let mut flags = b.clone();
    flags[17] = 2;
    assert!(SpriteFrame::decode(&flags).is_err());
    let f = SpriteFrame::decode(&b).unwrap();
    assert!(f.sample(&[], false, false, 0, 0).is_err());
    assert_eq!(
        f.sample(&[], false, false, i16::MIN, i16::MAX).unwrap(),
        SpritePixel::Transparent
    );
}

fn synthetic_rom() -> Vec<u8> {
    let mut r = vec![0; 0x40_0000];
    r[0xa252..0xa255].copy_from_slice(&[0, 0x80, 0xa0]);
    r[0xa258..0xa25b].copy_from_slice(&[0, 0x80, 0xa1]);
    r[0xf941..0xf948].copy_from_slice(&[2, 0x5a, 0xb2, 0, 0x80, 0x80, 0x10]);
    r[0x32_8002..0x32_8004].copy_from_slice(&0x7fffu16.to_le_bytes());
    for (base, count) in [(0x24_a1e4, 1), (0x1a_d064, 6)] {
        for axis in 0..3 {
            let seq = 0x100 + axis * 0x20;
            r[base + axis * 2..base + axis * 2 + 2]
                .copy_from_slice(&u16::try_from(seq).unwrap().to_le_bytes());
            for step in 0..count {
                let offset = 0x200 + (axis * 6 + step) * 0x40;
                r[base + seq + 4 * step + 2..base + seq + 4 * step + 4]
                    .copy_from_slice(&u16::try_from(offset).unwrap().to_le_bytes());
                let b = frame(&[[0, 4, 12, 24, 8, 0, 0x20]]);
                r[base + offset..base + offset + b.len()].copy_from_slice(&b);
            }
            r[base + seq + 4 * count..base + seq + 4 * count + 2].fill(255);
        }
    }
    r
}
#[test]
fn rom_loader_follows_relocated_pointers_and_retains_frame_ids() {
    use assets::sprites::ArkSprites;
    let r = synthetic_rom();
    let a = ArkSprites::from_rom(&r).unwrap();
    assert_eq!(a.frames().len(), 21);
    assert_eq!(a.frames()[0].id(), 0xa4_a3e8);
    assert_eq!(a.frames()[3].id(), 0x9a_d268);
    assert_eq!(a.palette()[1].raw(), 0x7fff);
    assert_eq!(a.graphics(0).unwrap().len(), 512);
    assert!(a.graphics(2).is_none());
    assert!(a.source_ranges().contains(&(0x20_8000..0x20_c000)));
    assert!(a.source_ranges().contains(&(0x32_8000..0x32_8020)));
    assert_eq!(a.frame(0xa4_a3e8).unwrap().resource(), 0);
    assert!(a.frame(0).is_none());
}
#[test]
fn rom_loader_rejects_changed_shapes_bad_offsets_and_unsupported_palette() {
    use assets::sprites::ArkSprites;
    assert!(ArkSprites::from_rom(&[]).is_err());
    for (offset, value) in [
        (0xf942, 0),
        (0xf947, 0x20),
        (0xa254, 0x7f),
        (0x24_a1e4, 0xff),
        (0x24_a1e5, 0xff),
        (0x24_a3e4 + 23, 0x22),
    ] {
        let mut r = synthetic_rom();
        r[offset] = value;
        assert!(ArkSprites::from_rom(&r).is_err(), "{offset:x}");
    }
}

#[test]
#[allow(clippy::many_single_char_names)] // Pixel/bounds coordinate fixture.
fn asymmetric_pixels_flip_within_tiles_and_bounds_keep_signed_offsets() {
    let mut b = vec![0; 32];
    b[0] = 0x80; // Only the top-left pixel is opaque.
    let tiles = decode_tiles_4bpp(&b).unwrap();
    let f = SpriteFrame::decode(&frame(&[[0, 2, 10, 3, 13, 0, 0]])).unwrap();
    assert_eq!(f.bounds(false, false), (-2, -21, 6, -13));
    assert_eq!(f.bounds(true, true), (-2, 5, 6, 13));
    for (h, v, x, y) in [
        (false, false, -2, -21),
        (true, false, 5, -21),
        (false, true, -2, 12),
        (true, true, 5, 12),
    ] {
        assert!(matches!(
            f.sample(&tiles, h, v, x, y).unwrap(),
            SpritePixel::Opaque { .. }
        ));
        let (l, t, r, b) = f.bounds(h, v);
        for (x, y) in [(l - 1, t), (r, t), (l, t - 1), (l, b)] {
            assert_eq!(
                f.sample(&tiles, h, v, x, y).unwrap(),
                SpritePixel::Transparent
            );
        }
    }
}
