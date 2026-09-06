use super::*;

const START: u32 = 0x88_8000;
fn image(stream: &[u8]) -> Vec<u8> {
    let mut image = vec![0; 0x40_0000];
    image[0x8_8000..0x8_8000 + stream.len()].copy_from_slice(stream);
    image
}

#[test]
fn glyphs_newlines_and_acknowledgements_are_not_flattened() {
    let mut rom = image(&[0xc0, 0x21, 0xcf, 0x80, 1, 0xd5, 0xd0, 0x22, 0xd1, 0xd3]);
    // Synthetic font pixels, not cartridge-derived glyph fixtures.
    rom[0x34_8000 + 0x21 * 64] = 0x80;
    let pages = decode(&rom, START).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].acknowledgement(), Acknowledgement::Next);
    assert_eq!(pages[1].acknowledgement(), Acknowledgement::End);
    assert_eq!(pages[0].boundary_source(), START + 5);
    assert_eq!(pages[0].glyphs()[0].font_source, 0xb4_8840);
    assert_eq!(pages[0].glyphs()[1].font_source, 0xb5_8040);
    assert_eq!(pages[0].glyphs()[1].position, [0, 16]);
    assert_eq!(pages[1].glyphs()[0].font_source, 0xb4_a880);
    assert_eq!(pages[0].indexed()[0], 1);
    assert_eq!(pages[0].width(), 224);
    assert_eq!(pages[0].height(), 48);
}

#[test]
fn subroutine_return_is_not_a_page_end() {
    let mut rom = image(&[0xd2, 1, 0x24, 0xd3]);
    rom[0x12_c449..0x12_c44b].copy_from_slice(&0xc800_u16.to_le_bytes());
    rom[0x12_c800..0x12_c803].copy_from_slice(&[0x23, 0xdc, 0xd4]);
    let pages = decode(&rom, START).unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].glyphs().len(), 2);
    // A palette reset flushes the packed half-tile ($85990E..992C).
    assert_eq!(pages[0].glyphs()[1].position, [16, 0]);
}

#[test]
fn unsupported_commands_and_choices_fail_closed() {
    for command in [
        0xc2, 0xc3, 0xc9, 0xcb, 0xcc, 0xcd, 0xce, 0xd6, 0xdb, 0xe3, 0xff,
    ] {
        assert!(
            decode(&image(&[command, 0xd3]), START).is_err(),
            "{command:02x}"
        );
    }
    assert!(decode(&image(&[0xc4, 0, 0xd3]), START).is_err());
    assert!(decode(&image(&[0xd2, 2, 0xd3]), START).is_err());
    assert!(decode(&image(&[0xca, 6, 0, 0, 0xd3]), START).is_err());
}

#[test]
fn truncation_missing_terminators_and_overflow_fail() {
    assert!(decode(&[], START).is_err());
    let mut rom = image(&[0x80]);
    rom.truncate(0x8_8001);
    assert!(decode(&rom, START).is_err());
    let mut rom = image(&[0xc5]);
    rom.truncate(0x8_8001);
    assert_eq!(decode(&rom, START).unwrap_err().reason, "truncated source");
    assert!(decode(&image(&[0xcf; 4096]), START).is_err());
    assert!(decode(&image(&[0xcf, 0xcf, 0xcf, 0x21, 0xd3]), START).is_err());
    let mut wide = vec![0x21; 19];
    wide.push(0xd3);
    assert!(decode(&image(&wide), START).is_err());
    assert!(decode(&image(&[0x21, 0xc0, 0xd3]), START).is_err());
    assert!(decode(&image(&[0xd3]), START).is_err());
}

#[test]
fn recursion_is_bounded() {
    let mut rom = image(&[0xd2, 1, 0xd3]);
    rom[0x12_c449..0x12_c44b].copy_from_slice(&0xc800_u16.to_le_bytes());
    rom[0x12_c800..0x12_c802].copy_from_slice(&[0xd2, 1]);
    assert!(decode(&rom, START).is_err());
}

#[test]
fn planar_quadrants_and_planes_are_in_native_order() {
    let mut source = [0; 64];
    source[0] = 0x80;
    source[17] = 0x80;
    source[32] = 0x80;
    source[33] = 0x80;
    source[62] = 1;
    let pixels = glyph_pixels(&source).unwrap();
    assert_eq!(
        (pixels[0], pixels[8], pixels[128], pixels[255]),
        (1, 2, 3, 1)
    );
    assert!(glyph_pixels(&source[..63]).is_err());
}

#[test]
fn palette_changes_flush_only_pending_half_tiles() {
    let rom = image(&[0x21, 0xc6, 4, 0x22, 0x23, 0xc6, 0, 0x24, 0xd3]);
    let pages = decode(&rom, START).unwrap();
    let positions: Vec<_> = pages[0].glyphs().iter().map(|g| g.position).collect();
    assert_eq!(positions, [[0, 0], [16, 0], [28, 0], [40, 0]]);
}

#[test]
fn top_level_return_retains_page_without_fabricated_acknowledgement() {
    let pages = decode(&image(&[0x21, 0xd4]), START).unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].acknowledgement(), Acknowledgement::None);
    assert_eq!(pages[0].boundary_source(), START + 1);
    assert_eq!(pages[0].glyphs().len(), 1);
}

#[test]
fn choice_catalog_retains_results_positions_and_navigation() {
    let mut rom = image(&[]);
    rom[0x12_c259..0x12_c25b].copy_from_slice(&0xc800_u16.to_le_bytes());
    for (index, words) in [
        [0x0082_u16, 0xc80a, 0xc80a, 0, 0],
        [0x0084, 0xc800, 0xc800, 0, 0],
    ]
    .iter()
    .enumerate()
    {
        for (word, value) in words.iter().enumerate() {
            let at = 0x12_c800 + index * 10 + word * 2;
            rom[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    let choice = decode_choice(&rom, 0).unwrap();
    assert_eq!(choice.options[0].result, 1);
    assert_eq!(choice.options[0].position, [0, 16]);
    assert_eq!(choice.options[1].position, [0, 32]);
    assert_eq!(choice.options[0].neighbors, [Some(2), Some(2), None, None]);
    assert_eq!(choice.options[1].neighbors, [Some(1), Some(1), None, None]);
    // Catalog 1 lookup, absolute row/column words and nonzero horizontal offset.
    rom[0x12_c25b..0x12_c25d].copy_from_slice(&0xc800_u16.to_le_bytes());
    rom[0x12_c800..0x12_c802].copy_from_slice(&0x0616_u16.to_le_bytes());
    rom[0x12_c80a..0x12_c80c].copy_from_slice(&0x0618_u16.to_le_bytes());
    let choice = decode_choice(&rom, 1).unwrap();
    assert_eq!(choice.options[0].position, [8, 16]);
    assert_eq!(choice.options[1].position, [8, 32]);
    rom[0x12_c802..0x12_c804].copy_from_slice(&0xc814_u16.to_le_bytes());
    assert!(decode_choice(&rom, 0).is_err());
    assert!(decode_choice(&rom, 2).is_err());
}

#[test]
fn public_loader_rejects_unqualified_images() {
    assert!(HouseDialogue::from_rom(&[]).is_err());
    assert!(HouseDialogue::from_rom(&image(&[0xd3])).is_err());
}
