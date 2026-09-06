#![allow(clippy::cast_possible_truncation)] // Fixed synthetic addresses/counts.
use super::*;
use crate::compression;
fn packed(image: &mut [u8], at: usize, target: u32) {
    let pointer = (((target >> 16) + 0x80 - 0x98) << 15) | (target & 0x7fff);
    image[at..at + 3].copy_from_slice(&pointer.to_le_bytes()[..3]);
}
fn set_table(image: &mut [u8], at: usize, target: usize) {
    image[at..at + 3].copy_from_slice(&((target as u32) | 0x80_0000).to_le_bytes()[..3]);
}
pub(in super::super) fn test_setup(image: &mut [u8]) {
    for span in [&CELLAR, &HOUSE13]
        .into_iter()
        .chain(ROOTS.iter().map(|r| &r.1))
    {
        image[span.offset..span.offset + span.bytes.len()].copy_from_slice(span.bytes);
    }
    for &(id, ref span) in ROOTS {
        set_table(image, 0x6959c + id as usize * 3, span.offset);
    }
    for (id, at) in [(3, 0x18_841e), (4, 0x18_84bf), (0x1f, 0x18_8461)] {
        set_table(image, 0x6a28c + id * 3, at);
    }
    for (at, target) in [
        (0x18_8465, 0x21_8000),
        (0x18_846c, 0x20_8000),
        (0x18_8476, 0x22_8000),
        (0x18_847e, 0x23_8000),
        (0x18_84c3, 0x21_8000),
        (0x18_84c8, 0x24_8800),
        (0x18_845a, 0x24_8000),
        (0x18_85ef, 0x24_8000),
        (0x18_85fb, 0x24_9000),
        (0x18_8326, 0x22_8000),
        (0x18_832e, 0x23_8000),
        (0x18_831e, 0x24_9800),
        (0x18_833a, 0x24_9800),
        (0x18_8342, 0x24_9800),
        (0x18_834a, 0x24_9800),
        (0x18_8361, 0x24_a000),
    ] {
        packed(image, at, target);
    }
    for (at, w, h) in [
        (0x24_8000, 2, 4),
        (0x24_8800, 4, 2),
        (0x24_9000, 1, 2),
        (0x24_9800, 2, 2),
        (0x24_a000, 4, 5),
    ] {
        let packet = compression::encode(&vec![0; w * h * 512]).unwrap();
        image[at..at + 2].copy_from_slice(&[w as u8, h as u8]);
        image[at + 2..at + 2 + packet.len()].copy_from_slice(&packet);
    }
    for i in 0..32 {
        image[0x25_8000 + i * 2..0x25_8002 + i * 2]
            .copy_from_slice(&(0x1234 + i as u16).to_le_bytes());
    }
    for (pointer, at, cell) in [
        (0x18_833a, 0x24_c000, 1_u16),
        (0x18_8342, 0x24_c800, 2),
        (0x18_834a, 0x24_d000, 3),
    ] {
        packed(image, pointer, at as u32);
        let mut cells = vec![0; 2048];
        cells[..2].copy_from_slice(&cell.to_le_bytes());
        let packet = compression::encode(&cells).unwrap();
        image[at..at + 2].copy_from_slice(&[2, 2]);
        image[at + 2..at + 2 + packet.len()].copy_from_slice(&packet);
    }
    image[0x9d24e..0x9d253].copy_from_slice(&[0, 0, 0xd0, 0, 0]);
    image[0x9d253..0x9d26c].copy_from_slice(CONTROLLER);
    image[0x9d255..0x9d257].copy_from_slice(&[0, 0x80]);
    image[0x9d25a] = 0xe6;
    let packet = compression::encode(&vec![0; 0x3000]).unwrap();
    image[0x26_8000..0x26_8000 + packet.len()].copy_from_slice(&packet);
    for (index, &(at, d, n)) in PALETTES.iter().enumerate() {
        let index = if index == 5 {
            3
        } else if index == 6 {
            5
        } else {
            index
        };
        let source = 0x27_8000 + index * 256;
        image[at..at + 7].copy_from_slice(&[
            2,
            0x5a,
            0xe7,
            source as u8,
            (source >> 8) as u8,
            d,
            n,
        ]);
        for bytes in image[source..source + usize::from(n) * 2].chunks_exact_mut(2) {
            bytes.copy_from_slice(&(index as u16 + 1).to_le_bytes());
        }
    }
    image[0x9d29d..0x9d2a1].copy_from_slice(&[0x22, 0x5c, 0x92, 0x86]);
}
pub(in super::super) fn test_mutations() -> Vec<(u16, usize)> {
    let mut result = Vec::new();
    for &(id, ref span) in ROOTS {
        for at in (0..span.bytes.len())
            .filter(|i| !span.pointers.iter().any(|p| (*p..*p + 3).contains(i)))
        {
            result.push((id, span.offset + at));
        }
        result.extend((0..3).map(|i| (id, 0x6959c + id as usize * 3 + i)));
    }
    for (id, span) in [
        (0xe, &CELLAR),
        (0x13, &HOUSE13),
        (0x44, &ROOTS[4].1),
        (0x44, &ROOM_SHARED),
    ] {
        result.extend(
            (0..span.bytes.len())
                .filter(|i| !span.pointers.iter().any(|p| (*p..*p + 3).contains(i)))
                .map(|i| (id, span.offset + i)),
        );
    }
    result.extend(
        (0x9d24e..0x9d2a1)
            .filter(|i| {
                ![0x9d255, 0x9d256, 0x9d25a].contains(i)
                    && !PALETTES.iter().any(|&(a, _, _)| (a + 2..a + 5).contains(i))
            })
            .map(|i| (0x44, i)),
    );
    for (id, sub) in [
        (0x13, 3),
        (0x13, 4),
        (0xe, 0x1f),
        (0x44, 0x1f),
        (0x44, 0x10),
    ] {
        result.extend((0..3).map(|i| (id, 0x6a28c + sub * 3 + i)));
    }
    result
}
