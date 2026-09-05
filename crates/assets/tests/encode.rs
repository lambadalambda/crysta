//! Canonical encoder fixtures are constructed from synthetic inputs only.

use assets::compression::{decode, encode, EncodeError, MAX_OUTPUT_SIZE};

#[test]
fn empty_and_oversized_inputs_are_rejected() {
    assert_eq!(encode(&[]), Err(EncodeError::EmptyInput));
    assert_eq!(
        encode(&vec![0; MAX_OUTPUT_SIZE + 1]),
        Err(EncodeError::InputTooLarge {
            length: MAX_OUTPUT_SIZE + 1
        })
    );
}

#[test]
fn exact_literals_and_short_copy_encodings() {
    assert_eq!(encode(b"A").unwrap(), [0, 1, 0, b'A', 0x40, 0, 0, 0]);
    assert_eq!(encode(b"AB").unwrap(), [0, 2, 0, b'A', 0xA0, b'B', 0, 0, 0]);
    for length in 2..=5_u8 {
        let expected = [
            0,
            length + 1,
            0,
            b'A',
            ((length - 2) << 4) | 4,
            0xFF,
            0,
            0,
            0,
        ];
        assert_eq!(
            encode(&vec![b'A'; usize::from(length) + 1]).unwrap(),
            expected
        );
    }
}

#[test]
fn exact_long_copy_encodings_and_maximum_length() {
    for length in 6..=9_u8 {
        let expected = [
            0,
            length + 1,
            0,
            b'A',
            0x50,
            0xFF,
            0xF8 | (length - 2),
            0,
            0,
            0,
        ];
        assert_eq!(
            encode(&vec![b'A'; usize::from(length) + 1]).unwrap(),
            expected
        );
    }
    for length in [10_u16, 256] {
        let size = (length + 1).to_le_bytes();
        let expected = [
            0,
            size[0],
            size[1],
            b'A',
            0x50,
            0xFF,
            0xF8,
            u8::try_from(length - 1).unwrap(),
            0,
            0,
            0,
        ];
        assert_eq!(
            encode(&vec![b'A'; usize::from(length) + 1]).unwrap(),
            expected
        );
    }
}

#[test]
fn exact_control_refills_and_split_terminator() {
    assert_eq!(
        encode(b"ABCDEFGHI").unwrap(),
        [0, 9, 0, b'A', 0xFF, b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', 0x40, 0, 0, 0]
    );
    // The terminator's 0 exhausts the first control byte; 1 opens the second.
    assert_eq!(
        encode(b"ABCDEFGH").unwrap(),
        [0, 8, 0, b'A', 0xFE, b'B', b'C', b'D', b'E', b'F', b'G', b'H', 0x80, 0, 0, 0]
    );
}

#[test]
fn nearest_equal_length_match_is_selected() {
    // After ABxABy, either prior AB is a length-two match; use distance three.
    assert_eq!(
        encode(b"ABxAByAB").unwrap(),
        [0, 8, 0, b'A', 0xC2, b'B', b'x', 0xFD, b'y', 0x08, 0xFD, 0, 0, 0]
    );
}

#[test]
fn padded_match_selection_precedes_final_length_clamp() {
    // The final AB chooses the older AB\0 rather than the nearer ABx: the
    // zero-padding participates in the search even though it is not emitted.
    assert_eq!(
        encode(b"AB\0ABxAB").unwrap(),
        [0, 8, 0, b'A', 0xC2, b'B', 0, 0xFD, b'x', 0x08, 0xFA, 0, 0, 0]
    );
}

#[test]
fn all_byte_values_alternating_runs_and_maximum_input_round_trip() {
    let mut cases = vec![
        (0..=255).collect::<Vec<u8>>(),
        b"AB".repeat(1024),
        vec![0; MAX_OUTPUT_SIZE],
        vec![0xFF; MAX_OUTPUT_SIZE],
    ];
    let mut state = 0xCAFE_1234_u32;
    for length in [1, 2, 7, 8, 9, 255, 256, 257, 8192, 8193, MAX_OUTPUT_SIZE] {
        let mut data = Vec::new();
        for _ in 0..length {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            data.push(state.to_le_bytes()[0]);
        }
        cases.push(data);
    }
    for data in cases {
        let compressed = encode(&data).unwrap();
        assert_eq!(
            compressed,
            encode(&data).unwrap(),
            "encoding must be deterministic"
        );
        let decoded = decode(&compressed, data.len()).unwrap();
        assert_eq!(decoded.data, data);
        assert_eq!(decoded.consumed, compressed.len());
        assert_eq!(encode(&decoded.data).unwrap(), compressed);
    }
}

#[test]
fn legal_noncanonical_packets_normalize_without_changing_output() {
    for packet in [
        vec![0, 1, 0, b'A', 0x7F, 0xFF, 0xF8, 0],
        vec![0, 3, 0, b'A', 0xD0, b'A', b'A', 0, 0, 0],
    ] {
        let data = decode(&packet, 3).unwrap().data;
        let canonical = encode(&data).unwrap();
        assert_ne!(canonical, packet);
        assert_eq!(decode(&canonical, 3).unwrap().data, data);
        assert_eq!(
            encode(&decode(&canonical, 3).unwrap().data).unwrap(),
            canonical
        );
    }
}
