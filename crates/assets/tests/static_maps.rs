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
fn accepts_exact_supported_layer_capacity() {
    let bytes = container(8, 4, &vec![0; 8192]);
    let layer = StaticLayer::from_rom(&bytes, 0).unwrap();
    assert_eq!((layer.width(), layer.height()), (128, 64));
    assert_eq!(layer.layer_bytes().len(), 0x4000);
}
