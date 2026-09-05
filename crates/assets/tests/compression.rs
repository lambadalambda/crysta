//! Synthetic packets only; no cartridge bytes are embedded here.

use assets::compression::{decode, DecodeError};

#[test]
fn first_byte_and_terminator_are_required_and_trailing_data_is_unconsumed() {
    // 01 + long length zero ends the packet, even with a nonzero offset field.
    let packet = [0, 1, 0, b'A', 0x7F, 0xFF, 0xF8, 0, 0xCC];
    let decoded = decode(&packet, 1).unwrap();
    assert_eq!(decoded.data, b"A");
    assert_eq!(decoded.consumed, 8);
}

#[test]
fn literals_refill_control_only_when_a_bit_is_requested() {
    // Eight literals use an entire control byte. The next byte is still the
    // eighth literal's payload, not the next control byte.
    let packet = [
        0, 9, 0, b'A', 0xFF, b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', 0x40, 0, 0, 0,
    ];
    assert_eq!(decode(&packet, 9).unwrap().data, b"ABCDEFGHI");
}

#[test]
fn short_copies_cover_all_lengths_and_overlap() {
    for length in 2..=5_u8 {
        // 00xx then 01 terminator, distance one (encoded as $FF).
        let packet = [
            0,
            length + 1,
            0,
            b'X',
            ((length - 2) << 4) | 4,
            0xFF,
            0,
            0,
            0,
        ];
        assert_eq!(
            decode(&packet, 6).unwrap().data,
            vec![b'X'; usize::from(length) + 1]
        );
    }
}

#[test]
fn long_copies_cover_short_and_extended_lengths() {
    for length in 3..=9_u8 {
        let packet = [
            0,
            length + 1,
            0,
            b'Y',
            0x50,
            0xFF,
            0xF8 | (length - 2),
            0,
            0,
            0,
        ];
        assert_eq!(
            decode(&packet, 10).unwrap().data,
            vec![b'Y'; usize::from(length) + 1]
        );
    }
    for length in [2_u16, 10, 256] {
        let size = (length + 1).to_le_bytes();
        let packet = [
            0,
            size[0],
            size[1],
            b'Z',
            0x50,
            0xFF,
            0xF8,
            u8::try_from(length - 1).unwrap(),
            0,
            0,
            0,
        ];
        assert_eq!(
            decode(&packet, 257).unwrap().data,
            vec![b'Z'; usize::from(length) + 1]
        );
    }
}

#[test]
fn a_control_code_can_straddle_control_bytes() {
    // Seven literals leave one bit; the four short-copy bits are split 0/001.
    // The offset follows the second control byte. Short copy repeats I three times.
    let packet = [
        0, 11, 0, b'B', 0xFE, b'C', b'D', b'E', b'F', b'G', b'H', b'I', 0x28, 0xFF, 0, 0, 0,
    ];
    assert_eq!(decode(&packet, 11).unwrap().data, b"BCDEFGHIIII");
}

#[test]
fn invalid_headers_limits_and_output_lengths_are_rejected() {
    assert_eq!(
        decode(&[1, 1, 0, 0], 1),
        Err(DecodeError::UnsupportedHeader { value: 1 })
    );
    assert_eq!(decode(&[0, 0, 0, 0], 1), Err(DecodeError::ZeroLength));
    assert_eq!(
        decode(&[0, 2, 0, 0], 1),
        Err(DecodeError::OutputLimitExceeded {
            declared: 2,
            limit: 1
        })
    );
    assert_eq!(
        decode(&[0, 2, 0, 0, 0x40, 0, 0, 0], 2),
        Err(DecodeError::LengthMismatch {
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        decode(&[0, 1, 0, 0, 0x80, 0xAA], 1),
        Err(DecodeError::LengthMismatch {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        decode(&[0, 2, 0, 0, 0x00, 0xFF], 2),
        Err(DecodeError::LengthMismatch {
            expected: 2,
            actual: 3
        })
    );
}

#[test]
fn copies_cannot_read_before_the_start_of_output() {
    assert_eq!(
        decode(&[0, 3, 0, 0, 0, 0], 3),
        Err(DecodeError::InvalidBackReference {
            distance: 256,
            produced: 1
        })
    );
    assert_eq!(
        decode(&[0, 4, 0, 0, 0x40, 0, 1], 4),
        Err(DecodeError::InvalidBackReference {
            distance: 8192,
            produced: 1
        })
    );
}

#[test]
fn every_truncated_prefix_fails_including_a_missing_terminator() {
    let packet = [0, 2, 0, b'A', 0xA0, b'B', 0, 0, 0];
    assert!(decode(&packet, 2).is_ok());
    for end in 0..packet.len() {
        assert!(
            matches!(
                decode(&packet[..end], 2),
                Err(DecodeError::UnexpectedEnd { .. })
            ),
            "prefix {end}"
        );
    }
}

#[test]
fn copies_at_maximum_distances_preserve_nonrepeating_source_bytes() {
    for (distance, length, tail) in [
        (256_u16, 2, vec![0x04, 0, 0, 0, 0]),
        (8192, 3, vec![0x50, 0, 1, 0, 0, 0]),
    ] {
        let size = (distance + 1 + length).to_le_bytes();
        let mut packet = vec![0, size[0], size[1], 0];
        // Full groups of eight literals leave the next control byte aligned.
        let literals: Vec<_> = (1..=distance).map(|value| value.to_le_bytes()[0]).collect();
        for chunk in literals.chunks_exact(8) {
            packet.push(0xFF);
            packet.extend_from_slice(chunk);
        }
        packet.extend_from_slice(&tail);
        let decoded = decode(&packet, usize::from(distance + 1 + length)).unwrap();
        let end = decoded.data.len();
        assert_eq!(
            &decoded.data[end - usize::from(length)..],
            &literals[..usize::from(length)]
        );
    }
}

#[test]
fn bounded_malformed_corpus_never_panics_or_returns_excess_output() {
    let mut state = 0x5EED_1234_u32;
    for length in 0..128 {
        for _ in 0..32 {
            let mut packet = Vec::new();
            for _ in 0..length {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                packet.push(state.to_le_bytes()[0]);
            }
            if length >= 4 {
                // Exercise token parsing rather than mostly rejecting headers.
                packet[0] = 0;
                packet[1] = (packet[1] % 64) + 1;
                packet[2] = 0;
            }
            if let Ok(decoded) = decode(&packet, 64) {
                assert!(decoded.data.len() <= 64);
                assert!(decoded.consumed <= packet.len());
                assert_eq!(decoded.data.len(), usize::from(packet[1]));
            }
        }
    }
}
