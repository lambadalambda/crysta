use super::*;
use crate::text::Placement;

#[test]
fn pandora_loader_authenticates_before_decoding() {
    assert!(PandoraDialogue::from_rom(&[]).is_err());
    assert!(PandoraDialogue::from_rom(&vec![0; 0x40_0000]).is_err());
}

#[test]
fn logical_keys_are_source_scoped_ordered_and_bounded() {
    let request = DialogueRequest {
        source: 0x88_b758,
        pages: vec![super::super::empty_page(); 4],
        choice_catalog: None,
    };
    assert_eq!(request.source(), 0x88_b758);
    assert_eq!(request.page_id(0), Some(0x088b_7580));
    assert_eq!(request.page_id(3), Some(0x088b_7583));
    assert_eq!(request.page_id(4), None);
    assert_eq!(request.page_id(usize::MAX), None);
    assert_eq!(request.pages().len(), 4);
    assert_eq!(request.choice_catalog(), None);
}

#[test]
#[ignore = "requires caller-owned Japanese ROM in PANDORA_ROM"]
fn owned_rom_preserves_required_requests_and_page_identities() {
    let raw = std::fs::read(std::env::var("PANDORA_ROM").unwrap()).unwrap();
    let rom = rom::Rom::load(&raw).unwrap();
    let text = PandoraDialogue::from_rom(rom.image()).unwrap();
    assert_eq!(text.pages(0x88_b758).unwrap().len(), 4);
    assert_eq!(text.pages(0x89_d735).unwrap().len(), 4);
    assert_eq!(text.request(0x88_b6c7).unwrap().choice_catalog(), Some(1));
    assert_eq!(text.request(0x88_9efc).unwrap().choice_catalog(), Some(1));
    assert!(text.pages(super::super::ENTRY_TEXT).is_none());
    assert!(text.choice(0).is_none());
    assert!(text.choice(2).is_none());
    assert_eq!(text.choice(1).unwrap().catalog, 1);
    assert_eq!(text.requests().len(), 33);
    assert_eq!(
        text.requests()
            .iter()
            .map(|r| r.pages().len())
            .sum::<usize>(),
        76
    );
    let refusal = text.request(0x88_b7e3).unwrap();
    assert_eq!(text.requests().last().unwrap().source(), refusal.source());
    assert_eq!(refusal.choice_catalog(), None);
    assert_eq!(refusal.pages().len(), 2);
    assert_eq!(refusal.page_id(0), Some(0x088b_7e30));
    assert_eq!(refusal.page_id(1), Some(0x088b_7e31));
    assert_eq!(
        refusal.pages()[0].acknowledgement(),
        super::super::Acknowledgement::Next
    );
    assert_eq!(
        refusal.pages()[1].acknowledgement(),
        super::super::Acknowledgement::End
    );
    assert_eq!(DIRECT_INVOCATIONS.len(), 34);
    assert_eq!(DIRECT_INVOCATIONS.last(), Some(&(0x89_d48b, 0x89_d735)));
    assert_eq!(
        DIRECT_INVOCATIONS
            .iter()
            .filter(|(_, source)| *source == 0x89_d720)
            .count(),
        4
    );
    let warning = text.pages(0x88_adf2).unwrap();
    assert_eq!(
        warning
            .iter()
            .map(DialoguePage::boundary_source)
            .collect::<Vec<_>>(),
        [0x88_ae29, 0x88_ae5e]
    );
    assert_eq!(rom.image()[0x8_ae50], 0x56); // glyph, NOT a third ack boundary
    let mut ids = std::collections::BTreeSet::new();
    for request in text.requests() {
        assert!(!request.pages().is_empty());
        assert!(request.pages().len() <= 15);
        for (index, page) in request.pages().iter().enumerate() {
            assert!(ids.insert(request.page_id(index).unwrap()));
            assert_eq!(
                page.indexed().len(),
                usize::from(page.width()) * usize::from(page.height())
            );
            assert!(!page.glyphs().is_empty());
        }
        if request.choice_catalog().is_some() {
            assert_eq!(
                request.pages().last().unwrap().acknowledgement(),
                super::super::Acknowledgement::None
            );
        }
    }
}

#[test]
fn request_admission_rejects_sixteen_pages_and_unretained_choices() {
    let mut stream = vec![];
    for _ in 0..15 {
        stream.extend([0x21, 0xd5]);
    }
    stream.extend([0x21, 0xd3]);
    let image = synthetic(&stream);
    assert_eq!(
        DialogueRequest::compile(&image, 0x88_8000, None)
            .unwrap_err()
            .reason,
        "Pandora request exceeds 15 logical pages"
    );
    let image = synthetic(&[0x21, 0xd3]);
    assert!(DialogueRequest::compile(&image, 0x88_8000, Some(1)).is_err());
    let image = synthetic(&[0x21, 0xd4]);
    assert!(DialogueRequest::compile(&image, 0x88_8000, Some(1)).is_ok());
    assert!(DialogueRequest::compile(&image, 0x88_8000, Some(0)).is_err());
}

fn synthetic(stream: &[u8]) -> Vec<u8> {
    let mut image = vec![0; 0x40_0000];
    image[0x8_8000..0x8_8000 + stream.len()].copy_from_slice(stream);
    image
}

#[test]
fn adaptive_window_and_long_calls_keep_page_relative_content() {
    let mut image = synthetic(&[0xc4, 1, 0xda, 0xcc, 0x00, 0xc8, 0x92, 0x22, 0xd3]);
    image[0x12_c800..0x12_c802].copy_from_slice(&[0x21, 0xd4]);
    let pages = super::super::decode_profile(&image, 0x88_8000, true).unwrap();
    assert_eq!(
        pages[0]
            .glyphs()
            .iter()
            .map(|g| g.position)
            .collect::<Vec<_>>(),
        [[0, 0], [12, 0]]
    );
    assert_eq!(pages[0].glyphs()[0].text_source, 0x92_c800);
    assert_eq!(pages[0].boundary_source(), 0x88_8008);
    assert_eq!(pages[0].placement(), Placement::AwayFromPlayer);
}

#[test]
fn transparent_font_transforms_both_planes_and_survives_page_clear() {
    let mut image = synthetic(&[0xc4, 0, 0xc1, 0x21, 0xd5, 0x21, 0xd3]);
    // Four pixels: 0,1,2,3. C4 00 removes only the plane intersection (3 -> 0).
    image[0x34_8840] = 0x50;
    image[0x34_8841] = 0x30;
    let pages = super::super::decode_profile(&image, 0x88_8000, true).unwrap();
    for page in &pages {
        assert_eq!(&page.indexed()[..4], &[0, 1, 2, 0]);
        assert_eq!(page.background_index(), 0);
        assert_eq!(page.indexed()[223], 0);
    }
    // The ordinary decoder accepts it too: the Crysta friends' line uses it.
    let ordinary = super::super::decode(&image, 0x88_8000).unwrap();
    assert_eq!(&ordinary[0].indexed()[..4], &[0, 1, 2, 0]);
    assert!(
        super::super::decode_profile(&synthetic(&[0xc4, 2, 0x21, 0xd3]), 0x88_8000, true).is_err()
    );
}

#[test]
fn custom_window_size_is_not_the_house_stride() {
    let pages = super::super::decode_profile(
        &synthetic(&[0xc4, 0, 0xc2, 6, 6, 24, 6, 0x21, 0xcf, 0x22, 0xd3]),
        0x88_8000,
        true,
    )
    .unwrap();
    assert_eq!((pages[0].width(), pages[0].height()), (192, 48));
    assert_eq!(pages[0].indexed().len(), 192 * 48);
    assert_eq!(pages[0].glyphs()[1].position, [0, 16]);
    assert_eq!(pages[0].placement(), Placement::Tile { column: 6, row: 6 });
}

#[test]
fn controller_label_uses_source_defaults_and_actual_table_not_invented_text() {
    let mut image = synthetic(&[0xe3, 0x40, 0, 0xd3]);
    for (i, (value, target)) in [
        (0x80_u16, 0x634_u16),
        (0x8000, 0x636),
        (0x40, 0x638),
        (0x4000, 0x63a),
        (0x10, 0x63e),
        (0x20, 0x63c),
    ]
    .into_iter()
    .enumerate()
    {
        let at = 0x5_bef7 + i * 6;
        let [lo, hi] = value.to_le_bytes();
        let [tl, th] = target.to_le_bytes();
        image[at..at + 6].copy_from_slice(&[0xa9, lo, hi, 0x8d, tl, th]);
    }
    image[0x5_9711..0x5_9713].copy_from_slice(&0xc800_u16.to_le_bytes());
    image[0x5_c800..0x5_c802].copy_from_slice(&[0x21, 0xd4]);
    let pages = super::super::decode_profile(&image, 0x88_8000, true).unwrap();
    assert_eq!(pages[0].glyphs()[0].text_source, 0x85_c800);
    assert_eq!(pages[0].boundary_source(), 0x88_8003);
    image[0x5_bef7] = 0;
    assert!(super::super::decode_profile(&image, 0x88_8000, true).is_err());
}

#[test]
fn new_controls_fail_closed_on_truncation_recursion_and_unqualified_operands() {
    for stream in [vec![0xcc], vec![0xc2, 6], vec![0xe3, 0x40], vec![0xe4]] {
        let mut image = synthetic(&stream);
        image.truncate(0x8_8000 + stream.len());
        assert!(super::super::decode_profile(&image, 0x88_8000, true).is_err());
    }
    for stream in [
        vec![0xcc, 0, 0x80, 0x88],
        vec![0xcc, 0, 0, 0x7e],
        vec![0xe4, 7],
        vec![0xd2, 2],
        vec![0x21, 0xc2, 6, 6, 24, 6],
    ] {
        assert!(super::super::decode_profile(&synthetic(&stream), 0x88_8000, true).is_err());
    }
}

#[test]
fn source_label_tables_return_without_extra_acknowledgements() {
    let mut image = synthetic(&[0xd2, 3, 0xe4, 6, 0xe4, 0x25, 0xd3]);
    for at in [0x12_c44d, 0x12_c5f3, 0x12_c631] {
        image[at..at + 2].copy_from_slice(&0xc800_u16.to_le_bytes());
    }
    image[0x12_c800..0x12_c802].copy_from_slice(&[0x21, 0xd4]);
    let pages = super::super::decode_profile(&image, 0x88_8000, true).unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].glyphs().len(), 3);
    assert_eq!(pages[0].boundary_source(), 0x88_8006);
}

#[test]
fn a_speakers_name_keeps_its_palette_and_colour() {
    // Elle's pages (`$89:80B1`): `C6 04`, `CA 05 5E3F`, her name, then `DC`.
    let image = synthetic(&[0xc1, 0xc6, 4, 0xca, 5, 0x3f, 0x5e, 0x21, 0xdc, 0x22, 0xd3]);
    let pages = super::super::decode_profile(&image, 0x88_8000, true).unwrap();
    let palettes: Vec<u8> = pages[0]
        .glyphs()
        .iter()
        .map(|glyph| glyph.palette)
        .collect();
    assert_eq!(palettes, [1, 0]);
    assert_eq!(pages[0].speaker().raw(), 0x5E3F);
}
