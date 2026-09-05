//! Authored synthetic exit tables and records, not extracted ROM bytes.
use assets::maps::exits::{ExitError, ExitList, MAX_RECORDS, SUPPORTED_MAP_COUNT};

const TABLE: usize = 0x01_8000;
const SOURCE: usize = 0x01_9000;
const RECORD: [u8; 12] = [7, 9, 3, 2, 0x34, 0x12, 0xE7, 0xD6, 0xCD, 0xAB, 0x67, 0x45];

fn image_at(pointer: u16, stream: &[u8]) -> Vec<u8> {
    let start = 0x01_0000 + usize::from(pointer);
    let mut image =
        vec![0; (TABLE + usize::from(SUPPORTED_MAP_COUNT) * 2).max(start + stream.len())];
    image[TABLE..TABLE + 2].copy_from_slice(&pointer.to_le_bytes());
    image[start..start + stream.len()].copy_from_slice(stream);
    image
}

fn list(records: &[[u8; 12]]) -> ExitList {
    let stream: Vec<_> = records.iter().flatten().copied().chain([0xFF]).collect();
    ExitList::from_rom(&image_at(0x9000, &stream), 0).unwrap()
}

#[test]
fn decodes_words_flags_and_exact_sentinel_extent() {
    let mut second = RECORD;
    second[0] = 11;
    second[4..6].copy_from_slice(&0xFFFF_u16.to_le_bytes());
    let stream: Vec<_> = RECORD
        .into_iter()
        .chain(second)
        .chain([0xFF, 0xA5])
        .collect();
    let image = image_at(0x9000, &stream);
    let exits = ExitList::from_rom(&image, 0).unwrap();
    assert_eq!(exits.entry().unwrap().value(), 0x81_9000);
    assert_eq!(exits.source_range(), Some(SOURCE..SOURCE + 25));
    assert_eq!(exits.source_bytes(), &stream[..25]);
    assert_eq!(exits.records().len(), 2);
    let record = &exits.records()[0];
    assert_eq!(record.source_range(), SOURCE..SOURCE + 12);
    assert_eq!(record.bytes(), &RECORD);
    assert_eq!(
        (record.x(), record.y(), record.width(), record.height()),
        (7, 9, 3, 2)
    );
    assert_eq!(record.raw_destination(), 0x1234);
    assert_eq!(record.direct_destination(), Ok(0x1234));
    assert_eq!(record.transition_mode(), 0xE7);
    assert_eq!(record.selector(), 0xD6);
    assert_eq!(record.destination_position(), (0xABCD, 0x4567));
    assert_eq!(exits.records()[1].raw_destination(), 0xFFFF);
    assert_eq!(
        exits.records()[1].direct_destination(),
        Err(ExitError::ConditionalDestination { raw: 0xFFFF })
    );
    assert_eq!(exits.records()[1].source_range(), SOURCE + 12..SOURCE + 24);
}

#[test]
fn null_pointer_and_sentinel_only_are_distinct_empty_lists() {
    let mut image = vec![0; TABLE + usize::from(SUPPORTED_MAP_COUNT) * 2];
    let exits = ExitList::from_rom(&image, SUPPORTED_MAP_COUNT - 1).unwrap();
    assert!(exits.records().is_empty());
    assert_eq!(exits.entry(), None);
    assert_eq!(exits.source_range(), None);
    assert!(exits.source_bytes().is_empty());
    assert!(exits.select(0, 0).is_none());
    image[TABLE..TABLE + 2].copy_from_slice(&0xFFFF_u16.to_le_bytes());
    image.resize(0x02_0000, 0);
    image[0x01_FFFF] = 0xFF;
    let exits = ExitList::from_rom(&image, 0).unwrap();
    assert!(exits.records().is_empty());
    assert_eq!(exits.source_range(), Some(0x01_FFFF..0x02_0000));
    assert_eq!(exits.source_bytes(), &[0xFF]);
}

#[test]
fn highest_supported_id_resolves_a_nonzero_little_endian_pointer() {
    let stream: Vec<_> = RECORD.into_iter().chain([0xFF]).collect();
    let mut image = image_at(0x9000, &stream);
    image[TABLE..TABLE + 2].fill(0);
    let slot = TABLE + usize::from(SUPPORTED_MAP_COUNT - 1) * 2;
    image[slot..slot + 2].copy_from_slice(&0x9000_u16.to_le_bytes());
    let exits = ExitList::from_rom(&image, SUPPORTED_MAP_COUNT - 1).unwrap();
    assert_eq!(exits.entry().unwrap().value(), 0x81_9000);
    assert_eq!(exits.source_range(), Some(SOURCE..SOURCE + 13));
    assert_eq!(exits.records()[0].bytes(), &RECORD);
    assert!(matches!(
        ExitList::from_rom(&image[..=slot], SUPPORTED_MAP_COUNT - 1),
        Err(ExitError::Truncated { .. })
    ));
}

#[test]
fn rejects_unqualified_ids_non_rom_and_table_pointers() {
    assert_eq!(SUPPORTED_MAP_COUNT, 0x450); // A conservative prefix, not the full exit table.
    assert_eq!(
        ExitList::from_rom(&[], 0x450),
        Err(ExitError::MapIndex { index: 0x450 })
    );
    for pointer in [1_u16, 0x7FFF, 0x8000, 0x889F, 0x88A0, 0x88AB] {
        let mut image = vec![0; TABLE + 2];
        image[TABLE..TABLE + 2].copy_from_slice(&pointer.to_le_bytes());
        assert_eq!(
            ExitList::from_rom(&image, 0),
            Err(ExitError::InvalidPointer { pointer })
        );
    }
    // Earliest observed data pointer; do not infer more supported map IDs from it.
    assert!(ExitList::from_rom(&image_at(0x88AC, &[0xFF]), 0).is_ok());
}

#[test]
fn rejects_every_truncated_table_record_and_missing_sentinel() {
    for length in [0, TABLE, TABLE + 1] {
        assert!(matches!(
            ExitList::from_rom(&vec![0; length], 0),
            Err(ExitError::Truncated { .. })
        ));
    }
    for length in 0..=12 {
        assert!(matches!(
            ExitList::from_rom(&image_at(0x9000, &RECORD[..length]), 0),
            Err(ExitError::Truncated { .. })
        ));
    }
}

#[test]
fn never_reads_a_record_or_sentinel_across_bank_81() {
    for pointer in [0xFFF5, 0xFFFF] {
        assert_eq!(
            ExitList::from_rom(&image_at(pointer, &RECORD), 0),
            Err(ExitError::BankCrossing)
        );
    }
    let stream: Vec<_> = RECORD.into_iter().chain([0xFF]).collect();
    assert_eq!(
        ExitList::from_rom(&image_at(0xFFF4, &stream), 0),
        Err(ExitError::BankCrossing)
    );
    let exits = ExitList::from_rom(&image_at(0xFFF3, &stream), 0).unwrap();
    assert_eq!(exits.source_range(), Some(0x01_FFF3..0x02_0000));
}

#[test]
fn permits_exactly_256_records_but_requires_the_sentinel() {
    assert_eq!(MAX_RECORDS, 256);
    assert_eq!(
        list(&vec![RECORD; MAX_RECORDS]).records().len(),
        MAX_RECORDS
    );
    let stream: Vec<_> = vec![RECORD; MAX_RECORDS + 1]
        .into_iter()
        .flatten()
        .chain([0xFF])
        .collect();
    assert_eq!(
        ExitList::from_rom(&image_at(0x9000, &stream), 0),
        Err(ExitError::RecordLimit)
    );
}

#[test]
fn fine_bounds_are_origin_based_and_exclusive_on_both_axes() {
    let exits = list(&[RECORD]);
    // Tile origin (112,144); accepted offsets are x<33, y<17.
    for (x, y) in [(112, 144), (144, 160), (112, 160), (144, 144)] {
        assert_eq!(exits.select(x, y), Some(&exits.records()[0]));
    }
    for (x, y) in [
        (111, 144),
        (112, 143),
        (145, 144),
        (112, 161),
        (160, 144),
        (112, 176),
    ] {
        assert!(exits.select(x, y).is_none(), "{x},{y}");
    }
    let mut single = RECORD;
    single[2..4].copy_from_slice(&[1, 1]);
    let exits = list(&[single]);
    assert!(exits.select(112, 144).is_some());
    assert!(exits.select(113, 144).is_none());
    assert!(exits.select(112, 145).is_none());
}

#[test]
fn first_coarse_match_blocks_later_fine_matches() {
    let mut small = RECORD;
    small[2] = 1;
    let exits = list(&[small, RECORD]);
    assert_eq!(exits.select(112, 144), Some(&exits.records()[0]));
    assert!(exits.select(113, 144).is_none()); // Later record would pass fine X.
    assert_eq!(exits.select(128, 144), Some(&exits.records()[1])); // First fails coarse X.
    small = RECORD;
    small[3] = 1;
    assert!(list(&[small, RECORD]).select(112, 145).is_none());
    for (width, height) in [(0, 2), (3, 0), (0, 0)] {
        let mut empty = RECORD;
        empty[2..4].copy_from_slice(&[width, height]);
        let exits = list(&[empty, RECORD]);
        assert_eq!(exits.select(112, 144), Some(&exits.records()[1]));
    }
}

#[test]
fn coarse_uses_wrapping_bytes_but_fine_uses_wrapping_words() {
    let mut edge = RECORD;
    edge[..4].copy_from_slice(&[254, 254, 4, 4]);
    let exits = list(&[edge, RECORD]);
    // Coarse wraps tile 256 to byte zero; fine remains at pixel 4096.
    assert_eq!(exits.select(4096, 4096), Some(&exits.records()[0]));
    assert!(exits.select(0, 0).is_none()); // Fine subtraction must not wrap at 4096.
    assert!(exits.select(4080, 0).is_none());
    assert!(exits.select(0, 4080).is_none());
    // High origin bits are discarded for coarse matching, but not for fine matching.
    edge[..4].copy_from_slice(&[0, 0, 255, 255]);
    assert!(list(&[edge]).select(0xF000, 0xF000).is_none());
}
