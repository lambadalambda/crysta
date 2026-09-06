//! Synthetic source metadata and art; no ROM fixture needed.
use super::*;
use crate::{compression, sprites::SpritePixel};
fn put(r: &mut [u8], at: usize, b: &[u8]) {
    r[at..at + b.len()].copy_from_slice(b);
}
#[allow(clippy::too_many_lines)] // Sparse metadata fixture for nine source records.
fn fixture() -> Vec<u8> {
    let mut r = vec![0; 0x40_0000];
    // Metadata-only fixture: real record shapes, synthetic frames/planar pixels.
    for (map, list, player, branches) in [
        (11, 0x8b7c, [8, 13], [0xea28, 0x8bdf]),
        (12, 0x8bf0, [8, 22], [0xea34, 0x8c72]),
        (13, 0x8ca1, [7, 39], [0xea5e, 0x8ce2]),
        (15, 0x8d1e, [19, 7], [0xea6f, 0x8d58]),
        (16, 0x8d69, [24, 6], [0xea85, 0x8daf]),
        (17, 0x8dcf, [13, 27], [0xeaa0, 0x8e0e]),
    ] {
        put(
            &mut r,
            0x3_8000 + map * 2,
            &u16::try_from(list).unwrap().to_le_bytes(),
        );
        let mut prefix = vec![0, 6, 0xfd, player[0], player[1], 0, 0x29, 0xa1, 0x84];
        for (event, target) in [([0xac, 1], branches[0]), ([0x96, 1], branches[1])] {
            prefix.extend([0xfa, event[0], event[1]]);
            prefix.extend(u16::try_from(target).unwrap().to_le_bytes());
        }
        if map == 11 || map == 12 {
            prefix.extend([0xfa, 0xba, 0x10, 0xbb, 0]);
            prefix.extend((if map == 11 { 0x8bba_u16 } else { 0x8c4d }).to_le_bytes());
        }
        put(&mut r, 0x3_0000 + list, &prefix);
    }
    for (record, xy, header, descriptor, selector) in [
        (0x8b96, [7, 7], 0x8e4b, 0xed7f, 6),
        (0x8c0a, [5, 26], 0xa321, 0xed5a, 1),
        (0x8c14, [3, 24], 0xa4eb, 0, 2),
        (0x8c1e, [4, 23], 0x9a6a, 0xed74, 0),
        (0x8c28, [6, 23], 0xa1a4, 0xedf8, 0),
        (0x8cb4, [4, 42], 0xa837, 0xedeb, 3),
        (0x8d7c, [26, 26], 0x98a9, 0xed5a, 2),
        (0x8d86, [27, 26], 0x9991, 0xed74, 2),
        (0x8de2, [27, 40], 0xa9bd, 0xecae, 4),
    ] {
        let mut spawn = vec![1, xy[0], xy[1], 0];
        spawn.extend(u16::try_from(header).unwrap().to_le_bytes());
        spawn.push(0x88);
        spawn.extend(u16::try_from(descriptor).unwrap().to_le_bytes());
        spawn.push(if descriptor == 0 { 0 } else { 0x83 });
        put(&mut r, 0x3_0000 + record, &spawn);
        put(&mut r, 0x8_0000 + header, &[selector, 0, 0x51, 0, 0]);
    }
    for (at, target, flip, sel, tail) in [
        (0xa37c, 0xa37c, None, 4, true),
        (0xa531, 0xa531, Some(false), 5, true),
        (0x9ac5, 0x9ac5, None, 3, true),
        (0xa1ff, 0xa1ff, None, 3, true),
        (0x98d3, 0x98d3, Some(false), 2, true),
        (0x99bd, 0x99bd, Some(true), 2, true),
        (0xa9e4, 0xa9de, Some(true), 4, false),
    ] {
        let mut b = vec![2, 0x23];
        b.extend(u16::try_from(target).unwrap().to_le_bytes());
        if let Some(f) = flip {
            b.extend([2, if f { 0xb7 } else { 0xb6 }]);
        }
        b.extend([2, 0x80, sel, 2, 0x8e]);
        if tail {
            let n = b.len() + 2;
            b.extend([0x80, u8::try_from(256 - n).unwrap()]);
        }
        put(&mut r, 0x8_0000 + at, &b);
    }
    // Palette mode 0/1, movement-pointer variant, no-load graphics, descriptor reuse.
    for (at, pointer, movement, palette, load) in [
        (0xed5a, 0xd6_4b77, false, [0x81, 2, 2, 10], true),
        (0xed74, 0xd6_6385, false, [0x81, 4, 2, 8], false),
        (0xedf8, 0xd8_1022, false, [0x81, 4, 2, 8], false),
        (0xedeb, 0xd8_1022, false, [0x81, 4, 2, 8], true),
        (0xed7f, 0xd4_3acb, true, [0x81, 0, 2, 10], true),
        (0xecae, 0xd6_5ea2, false, [0x80, 0, 2, 8], true),
    ] {
        let mut d = u32::try_from(pointer).unwrap().to_le_bytes()[..3].to_vec();
        d.extend([if movement { 0 } else { 0x20 }, 0]);
        if movement {
            d.extend([0, 0x80, 0xd8]);
        }
        d.extend(palette);
        d.extend(if load {
            vec![0, 0, 0xc0, if at == 0xecae { 0 } else { 3 }]
        } else {
            vec![0xff, 0xff]
        });
        put(&mut r, 0x3_0000 + at, &d);
    }
    for (cpu, palette) in [
        (0xd6_4b77, 1),
        (0xd6_6385, 2),
        (0xd8_1022, 2),
        (0xd4_3acb, 0),
        (0xd6_5ea2, 0),
    ] {
        let mut b = vec![0; 0x100];
        let mut cursor = 16;
        for sel in 0..7 {
            put(
                &mut b,
                sel * 2,
                &u16::try_from(cursor).unwrap().to_le_bytes(),
            );
            let n = if (3..=5).contains(&sel) { 4 } else { 1 };
            for i in 0..n {
                put(
                    &mut b,
                    cursor,
                    &[
                        if n == 1 { 0 } else { 7 },
                        if sel == 2 || sel == 5 {
                            3
                        } else {
                            u8::from(sel == 1 || sel == 4)
                        },
                        0xd0,
                        0,
                    ],
                );
                cursor += 4;
                if i + 1 == n {
                    put(&mut b, cursor, &[0xff, 0xff]);
                    cursor += 2;
                }
            }
        }
        put(&mut b, 14, &[0xd0, 0]);
        put(&mut b, 0xd0, &[8, 8, 16, 0]);
        b[0xe0] = 1;
        put(&mut b, 0xe1, &[1, 0, 0, 0, 0, 0, 0x20 + palette * 2]);
        put(&mut r, cpu & 0x3f_ffff, &compression::encode(&b).unwrap());
    }
    for (table, cpu) in [
        (0xfc72, 0xcc_8000_u32),
        (0xfc75, 0xcc_9000),
        (0xfda4, 0xbf_8000),
        (0xfda7, 0xbf_9000),
    ] {
        put(&mut r, table, &cpu.to_le_bytes()[..3]);
    }
    let mut gfx = vec![0; 8192];
    gfx[0] = 128;
    for p in [0x3f_8000, 0x3f_9000] {
        put(&mut r, p, &compression::encode(&gfx).unwrap());
    }
    // Distinct colors prove graphics-only reuse does not reuse the palette.
    put(&mut r, 0x0c_9022, &[31, 0]);
    put(&mut r, 0x0c_9042, &[0xe0, 3]);
    // Prop resources are supplied by a separate focused fixture test below.
    r
}
#[test]
fn residents_share_resources_and_expose_source_setup_lists() {
    let r = fixture();
    let scenes = HouseScenes::residents_from_rom(&r).unwrap();
    assert_eq!(scenes.actors().len(), 9);
    let first = scenes.actor(0x83_8d7c).unwrap();
    let left = scenes.actor(0x83_8d86).unwrap();
    assert_eq!(first.position(), [424, 416]);
    assert_eq!(first.palette_base(), 208);
    assert_eq!(left.position(), [440, 416]);
    assert_eq!(left.facing(), 2);
    assert!(left.hflip());
    assert_eq!(
        first.setup_frame().key(),
        HousePoseKey::Compressed {
            packet: 0xd6_4b77,
            offset: 0xd4
        }
    );
    assert_eq!(
        first
            .setup_frame()
            .composition()
            .sample(first.graphics(), false, false, -8, -16)
            .unwrap(),
        SpritePixel::Opaque {
            palette_index: 209,
            priority: 2,
            component: 0
        }
    );
    assert!(std::ptr::eq(first.graphics(), left.graphics()));
    let reused = scenes.actor(0x83_8c14).unwrap();
    assert_eq!(reused.frames().len(), 4);
    assert_eq!(reused.frames()[0].duration(), 7);
    assert!(std::ptr::eq(first.graphics(), reused.graphics()));
    assert_eq!(first.palette()[1].rgb8(), [255, 0, 0]);
    assert_eq!(left.palette()[1].rgb8(), [0, 255, 0]);
    assert_eq!(reused.palette(), first.palette());
    assert!(reused.source_ranges().contains(&(0x3_ed5a..0x3_ed67)));
    let d = scenes.actor(0x83_8cb4).unwrap();
    assert_eq!(d.position(), [72, 672]);
    assert_eq!(d.selector(), 3);
    assert_eq!(d.facing(), 0);
    assert!(!d.hflip());
    assert_eq!(d.frames().len(), 4);
    assert_eq!(
        d.setup_frame().key(),
        HousePoseKey::Compressed {
            packet: 0xd8_1022,
            offset: 0xd4
        }
    );
    assert!(d.frames().iter().all(|f| f.duration() == 7));
    assert_eq!(d.frames()[1].key(), d.frames()[3].key());
    assert_eq!(HouseScenes::ark_tie_rank(12), Some(4));
    assert_eq!(left.tie_rank(), 0);
    assert_eq!(first.tie_rank(), 1);
    assert_eq!(HouseScenes::ark_tie_rank(33), None);
}
fn add_prop(r: &mut [u8]) {
    put(r, 0x3_8d4f, &[0xfd, 29, 10, 0, 0x0d, 0xd6, 0x88]);
    put(r, 0x8_d60d, &[0, 0, 0xd1, 0, 1]);
    put(r, 0x8_d618, &[2, 0x9c, 0x41, 0xd6, 0x88, 0, 0, 0xf0, 0xff]);
    put(
        r,
        0x8_d641,
        &[
            0xa9, 0, 0, 0x9d, 4, 0, 0xa9, 0, 0, 0x9d, 6, 0, 2, 0xd8, 0, 0xc0, 0xa2, 2, 0x80, 0x42,
            2, 0x8e, 0x80, 0xf9,
        ],
    );
    put(r, 0x22_c084, &[0, 1]);
    put(r, 0x22_c100, &[15, 3, 0, 2, 0xff, 0xff]);
    put(r, 0x22_c200, &[8, 10, 14, 0]);
    r[0x22_c210] = 1;
    put(r, 0x22_c211, &[0, 0, 0, 0, 0, 0, 0x24]);
    put(r, 0x6_95c9, &[0x96, 0x84, 0x98]);
    put(
        r,
        0x18_8496,
        &[
            8, 0xfa, 1, 0, 0x80, 0, 0x10, 0, 0x2f, 0xf0, 8, 0x40, 0, 0x40, 0, 0x20, 0x90, 0x38,
            0x0b, 0x0d, 0, 0x10, 0,
        ],
    );
    let mut graphics = vec![0; 0x2000];
    graphics[0] = 128;
    put(r, 0x29_f02f, &compression::encode(&graphics).unwrap());
    put(r, 0x32_8b5a, &[31, 0]);
}
#[test]
fn full_roster_includes_source_created_prop_with_direct_pose_key() {
    let mut r = fixture();
    add_prop(&mut r);
    let s = HouseScenes::from_rom(&r).unwrap();
    assert_eq!(
        s.actors()
            .iter()
            .map(|a| (a.source_id(), a.map_id()))
            .collect::<Vec<_>>(),
        [
            (0x83_8b96, 11),
            (0x83_8c0a, 12),
            (0x83_8c14, 12),
            (0x83_8c1e, 12),
            (0x83_8c28, 12),
            (0x83_8cb4, 13),
            (0x83_8d7c, 16),
            (0x83_8d86, 16),
            (0x83_8de2, 17),
            (0x88_d618, 15)
        ]
    );
    for excluded in [0x83_8d03, 0x83_9243, 0x83_928f, 0x83_8d4f, 0xa2_c825] {
        assert!(s.actor(excluded).is_none());
    }
    let p = s.actor(0x88_d618).unwrap();
    assert_eq!(p.map_id(), 15);
    assert_eq!(p.position(), [472, 144]);
    assert_eq!(p.palette_base(), 160);
    assert_eq!(p.setup_frame().key(), HousePoseKey::Direct(0xa2_c204));
    assert_eq!(p.setup_frame().duration(), 15);
    assert_eq!(
        p.setup_frame()
            .composition()
            .sample(p.graphics(), false, false, -8, -14)
            .unwrap(),
        SpritePixel::Opaque {
            palette_index: 161,
            priority: 2,
            component: 0
        }
    );
    assert_eq!(p.palette()[1].rgb8(), [255, 0, 0]);
    r[0x3_8d50] = 20;
    r[0x3_8d51] = 12;
    assert_eq!(
        HouseScenes::from_rom(&r)
            .unwrap()
            .actor(0x88_d618)
            .unwrap()
            .position(),
        [328, 176]
    );
    r[0x22_c217] = 0x25; // Tile $100 exceeds the qualified fixed 256-tile resource.
    assert!(HouseScenes::from_rom(&r).is_err());
}
#[test]
fn rejects_missing_reuse_and_unsupported_resource_shapes() {
    for (at, value) in [
        (0x3_8c14 + 7, 1),
        (0x3_ed74 + 9, 0),
        (0x3_ecae + 8, 7),
        (0x8_98d7, 0),
    ] {
        let mut r = fixture();
        r[at] = value;
        assert!(HouseScenes::residents_from_rom(&r).is_err(), "{at:x}");
    }
    assert!(HouseScenes::residents_from_rom(&fixture()[..0x3f_8010]).is_err());
}

#[test]
fn rejects_base_relative_bank_crossings() {
    for case in 0..4 {
        let mut r = fixture();
        add_prop(&mut r);
        match case {
            0 => {
                let bytes = r[0x3_8ca1..0x3_8ca1 + 29].to_vec();
                put(&mut r, 0x3_ffed, &bytes);
                put(&mut r, 0x3_801a, &[0xed, 0xff]);
            }
            1 => {
                let bytes = r[0x3_8d1e..0x3_8d1e + 56].to_vec();
                put(&mut r, 0x3_ffcf, &bytes);
                put(&mut r, 0x3_801e, &[0xcf, 0xff]);
            }
            2 => {
                let bytes = r[0x8_d60d..0x8_d60d + 20].to_vec();
                put(&mut r, 0x8_fff5, &bytes);
                put(&mut r, 0x3_8d53, &[0xf5, 0xff, 0x88]);
            }
            _ => {
                put(&mut r, 0x8_d64f, &[0x80, 0xff, 0xa2]);
                put(&mut r, 0x23_0004, &[0, 0]);
            }
        }
        assert!(HouseScenes::from_rom(&r).is_err(), "crossing {case}");
    }
}

fn list_spec(selector: u8) -> ListSpec {
    ListSpec {
        selector,
        count: 1,
        duration: 0,
        source_cpu: 0xd0_8000,
        direct: false,
        source_palette: 0,
        palette_base: 208,
        hflip: false,
    }
}
fn list_fixture() -> Vec<u8> {
    let mut b = vec![0; 256];
    put(&mut b, 0, &[16, 0]);
    put(&mut b, 14, &[0xd0, 0]);
    put(&mut b, 16, &[0, 3, 0xd0, 0, 0xff, 0xff]);
    b[0xe0] = 1;
    put(&mut b, 0xe1, &[0, 0, 0, 0, 0, 0, 0x20]);
    b
}
#[test]
fn selector_must_be_in_aligned_compressed_table_not_packet_payload() {
    let mut b = list_fixture();
    assert!(decode_list(&b, list_spec(0)).is_ok());
    // Payload contains a valid list pointer, but is not a selector entry.
    put(&mut b, 40, &[16, 0]);
    assert!(decode_list(&b, list_spec(20)).is_err());
    // Last table word is the first anchor, never a selectable list.
    put(&mut b, 0xd0, &[0, 3, 0xd0, 0, 0xff, 0xff]);
    assert!(decode_list(&b, list_spec(7)).is_err());
    b[0] = 15;
    assert!(decode_list(&b, list_spec(0)).is_err());
}

#[test]
fn list_rejects_unsupported_records_and_component_boundaries() {
    for (at, value) in [
        (16, 7),
        (17, 2),
        (20, 0),
        (0xe0, 0),
        (0xe0, 129),
        (0xe7, 0x22),
        (0xe7, 0x10),
        (0xe7, 0x21),
    ] {
        let mut b = list_fixture();
        b[at] = value;
        assert!(decode_list(&b, list_spec(0)).is_err(), "{at:x}/{value:x}");
    }
    for tile in [15, 240] {
        let mut b = list_fixture();
        b[0xe1] = 1;
        b[0xe6] = tile;
        assert!(decode_list(&b, list_spec(0)).is_err());
    }
}

#[test]
fn valid_relative_relocations_preserve_source_identity() {
    let mut r = fixture();
    add_prop(&mut r);
    let bytes = r[0x3_8ca1..0x3_8ca1 + 29].to_vec();
    put(&mut r, 0x3_9000, &bytes);
    put(&mut r, 0x3_801a, &[0, 0x90]);
    let bytes = r[0x22_c000..0x22_c300].to_vec();
    put(&mut r, 0x22_d000, &bytes);
    put(&mut r, 0x8_d64f, &[0, 0xd0, 0xa2]);
    let s = HouseScenes::from_rom(&r).unwrap();
    assert!(s.actor(0x83_8cb4).is_none());
    assert_eq!(s.actor(0x83_9013).unwrap().position(), [72, 672]);
    assert_eq!(
        s.actor(0x88_d618).unwrap().setup_frame().key(),
        HousePoseKey::Direct(0xa2_d204)
    );
}

#[test]
fn graphics_reuse_requires_same_room_predecessor() {
    for at in [0x3_ed7f + 12, 0x3_edeb + 9] {
        let mut r = fixture();
        put(&mut r, at, &[0xff, 0xff]);
        assert!(HouseScenes::residents_from_rom(&r).is_err());
    }
}

#[test]
fn rejects_changed_prop_instructions_and_child_underflow() {
    for (at, value) in [
        (0x3_8d4f, 0xfe),
        (0x8_d60f, 0),
        (0x8_d618, 0),
        (0x8_d641, 0),
        (0x8_d651, 0x7e),
        (0x18_8496, 0),
        (0x3_8d51, 0),
    ] {
        let mut r = fixture();
        add_prop(&mut r);
        r[at] = value;
        assert!(HouseScenes::from_rom(&r).is_err(), "{at:x}");
    }
}
