//! Synthetic static-layer containers; no extracted map bytes.
use assets::{compression, maps::StaticLayer};

fn container(width: u8, height: u8, words: &[u16]) -> Vec<u8> {
    let data: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut bytes = vec![width, height];
    bytes.extend(compression::encode(&data).unwrap());
    bytes
}

#[test]
fn decodes_non_square_layer_and_preserves_source_extent() {
    let mut words = vec![0; 16 * 32];
    words[17] = 0x1D45;
    words[511] = 0x0101;
    let source = container(1, 2, &words);
    let mut image = vec![0xFF; 7];
    image.extend(&source);
    image.extend([0xDE, 0xAD]);
    let layer = StaticLayer::from_rom(&image, 7).unwrap();
    assert_eq!((layer.width(), layer.height()), (16, 32));
    assert_eq!(layer.source_range(), 7..7 + source.len());
    assert_eq!(layer.source_bytes(), source);
    assert_eq!(layer.cell(1, 1).unwrap().raw(), 0x1D45);
    assert_eq!(layer.cell(15, 31).unwrap().raw(), 0x0101);
    assert_eq!(layer.cell(16, 0), None);
    assert_eq!(layer.cell(0, 32), None);
    assert_eq!(layer.cell(usize::MAX, usize::MAX), None);
    assert_eq!(layer.cells().len(), words.len());
    assert_eq!(
        layer.layer_bytes(),
        words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>()
    );
}

#[test]
fn rejects_bad_offsets_dimensions_packets_and_output_extent() {
    let bytes = container(1, 1, &vec![0; 256]);
    for offset in [bytes.len(), bytes.len() + 1, usize::MAX] {
        assert!(StaticLayer::from_rom(&bytes, offset).is_err());
    }
    for end in 0..bytes.len() {
        assert!(
            StaticLayer::from_rom(&bytes[..end], 0).is_err(),
            "prefix {end}"
        );
    }
    for dimensions in [[0, 1], [1, 0], [255, 255], [9, 4]] {
        let mut invalid = bytes.clone();
        invalid[..2].copy_from_slice(&dimensions);
        assert!(StaticLayer::from_rom(&invalid, 0).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[2] = 1;
    assert!(StaticLayer::from_rom(&invalid, 0).is_err());
    assert!(StaticLayer::from_rom(&container(1, 1, &[0; 255]), 0).is_err());
    assert!(StaticLayer::from_rom(&container(1, 1, &[0; 257]), 0).is_err());
}

#[test]
fn applies_loader_attributes_without_mutating_static_words() {
    let mut words = vec![0; 256];
    words[0] = 0x8145;
    words[1] = 0xFFFF;
    let layer = StaticLayer::from_rom(&container(1, 1, &words), 0).unwrap();
    let mut table = [0; 512];
    table[0x145] = 0x8E; // Table bit 7 is discarded; lower seven become bits 9..15.
    table[0x1FF] = 0x7F;
    let attributed = layer.attributed_cells(&table);
    assert_eq!(attributed[0].raw(), 0x1D45);
    assert_eq!(attributed[1].raw(), 0xFFFF);
    assert_eq!(attributed[2].raw(), 0);
    assert_eq!(layer.cells()[0].raw(), 0x8145);
}

#[test]
fn accepts_exact_supported_layer_capacity() {
    let bytes = container(8, 4, &vec![0; 8192]);
    let layer = StaticLayer::from_rom(&bytes, 0).unwrap();
    assert_eq!((layer.width(), layer.height()), (128, 64));
    assert_eq!(layer.layer_bytes().len(), 0x4000);
}

#[test]
fn enforces_rom_and_bank_boundaries_without_reencoding() {
    let mut bytes = container(1, 1, &vec![0; 256]);
    // Ignored long-terminator offset bits: legal, intentionally noncanonical.
    let end = bytes.len();
    bytes[end - 3] = 0xFF;
    bytes[end - 2] = 0xF8;
    let layer = StaticLayer::from_rom(&bytes, 0).unwrap();
    assert_eq!(layer.source_bytes(), bytes);
    assert_ne!(
        &layer.source_bytes()[2..],
        compression::encode(&layer.layer_bytes()).unwrap()
    );
    let start = 0x10000 - bytes.len();
    let mut image = vec![0; start];
    image.extend(&bytes);
    assert!(StaticLayer::from_rom(&image, start).is_ok());
    image.insert(0, 0); // Container now crosses the bank even though all bytes exist.
    assert!(StaticLayer::from_rom(&image, start + 1).is_err());
    assert!(StaticLayer::from_rom(&image, 0xFFFF).is_err());
    let mut large = vec![0; 0x40_0000];
    large.extend(bytes);
    assert!(StaticLayer::from_rom(&large, 0x40_0000).is_err());
}
