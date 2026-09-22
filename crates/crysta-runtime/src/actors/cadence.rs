//! Source admission for the map-D ordinary class-0 COP26 action only.
//!
//! This is not a general movement-resource interpreter. Other descriptors,
//! classes and script waits retain their existing projection.
use assets::compression::decode;

pub(super) const STEP_SITE: usize = 0x08_A868;
pub(super) const WALK_TICKS: u16 = 32;
pub(super) const IDLE_TICKS: u16 = 16;

pub(super) fn qualifies(image: &[u8], map: u16, record: usize, script: Option<u32>) -> bool {
    if map != 0xD || record != 0x03_8CB4 || script != Some(0x88_A83C) {
        return false;
    }
    // Source identity, initial entity flags, class 0 and common ($6000) base.
    if image.get(record..record + 10) != Some(&[1, 4, 0x2A, 0, 0x37, 0xA8, 0x88, 0xEB, 0xED, 0x83])
        || image.get(0x08_A837..0x08_A83C) != Some(&[3, 0, 0x51, 0, 0])
        || image.get(0x03_EDEB..0x03_EDF0) != Some(&[0x22, 0x10, 0xD8, 0x20, 0])
        || image.get(STEP_SITE..STEP_SITE + 8) != Some(&[2, 0x26, 4, 10, 41, 41, 2, 0x8F])
        || image.get(0x8F6D..0x8F75) != Some(&[3, 0x68, 4, 0x69, 5, 0x60, 5, 0x60])
        || image.get(0x8FB5..0x8FBD) != Some(&[0, 16, 1, 16, 2, 16, 2, 16])
    {
        return false;
    }
    // Both source load sites resolve packed $09F037, base bank $98, to
    // $AB:F037 and destination operand 2 to $7F:6000. No relocation here.
    for site in [0x18_817D, 0x18_8272] {
        if image.get(site..site + 7) != Some(&[1, 1, 2, 0, 0x37, 0xF0, 9]) {
            return false;
        }
    }
    let Some(common) = packet(image, 0x2B_F037, 0x1A0C) else {
        return false;
    };
    for (selector, x, y, pointer, velocity) in [
        (0x60, 0x6D18, 0, 0x6D18, 1),
        (0x68, 0, 0x6FC8, 0x6FC8, 1),
        (0x69, 0, 0x6FD4, 0x6FD4, 0xFFFF),
    ] {
        let at = pointer - 0x6000 + 2;
        let records = [0, velocity, 0, 0, 0xFFFF, x.max(y) + 2];
        if word(&common, selector * 4) != Some(x)
            || word(&common, selector * 4 + 2) != Some(y)
            || !records
                .iter()
                .enumerate()
                .all(|(i, value)| word(&common, at + i * 2) == Some(*value))
        {
            return false;
        }
    }
    let Some(body) = packet(image, 0x18_1022, 0x28B) else {
        return false;
    };
    (0..6).all(|selector| {
        let Some(at) = word(&body, selector * 2).map(usize::from) else {
            return false;
        };
        let (records, duration) = if selector < 3 { (1, 0) } else { (4, 7) };
        (0..records).all(|i| {
            body.get(at + 4 * i) == Some(&duration)
                && word(&body, at + 4 * i).is_some_and(|record| record < 0x8000)
        }) && word(&body, at + 4 * records) == Some(0xFFFF)
    })
}

fn packet(image: &[u8], at: usize, size: usize) -> Option<Vec<u8>> {
    let decoded = decode(image.get(at..)?, size).ok()?;
    (decoded.data.len() == size).then_some(decoded.data)
}

fn word(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// Only these otherwise skipped services preserve the qualified state on
/// this entry/ambient path. COP06 (long jump), resource/class setters and
/// callbacks outside this interpreter must not silently retain admission.
pub(super) fn benign_skipped_service(image: &[u8], pc: usize) -> bool {
    let expected: &[u8] = match pc {
        0x08_A842 | 0x08_A870 => &[2, 0x3B], // occupancy, projected by World
        0x08_A84C => &[2, 0xBB, 0x0E],       // +$08 bits $0E00, not the class
        0x08_A854 => &[2, 0x21, 0x94, 0xA8], // interaction registration
        // $80:9D25: +$04 |= $0200; collision callback fields, not velocity.
        0x08_A858 => &[2, 0x65, 0x24, 0xD2, 0x5F, 0xA8, 0x88],
        _ => return false,
    };
    image.get(pc..pc + expected.len()) == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipped_service_exceptions_require_the_exact_site_and_operands() {
        let mut image = vec![0; 0x08_A854];
        image[0x08_A84C..0x08_A84F].copy_from_slice(&[2, 0xBB, 0x0E]);
        assert!(benign_skipped_service(&image, 0x08_A84C));
        image[0x08_A84E] = 0;
        assert!(!benign_skipped_service(&image, 0x08_A84C));
        image[0x08_A84F..0x08_A854].copy_from_slice(&[2, 6, 0x2D, 0xD3, 0x88]);
        assert!(!benign_skipped_service(&image, 0x08_A84F));
    }

    #[test]
    fn owned_source_admission_excludes_other_classes_private_tables_and_changed_lists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = rom.image();
        let admitted = |image: &[u8]| qualifies(image, 0xD, 0x03_8CB4, Some(0x88_A83C));
        assert!(admitted(image));
        assert!(!qualifies(image, 0xA, 0x03_8CB4, Some(0x88_A83C)));
        assert!(!qualifies(image, 0xD, 0x03_8CBE, Some(0x88_A83C)));
        assert!(!qualifies(image, 0xD, 0x03_8CB4, Some(0x88_A868)));
        for at in [
            0x03_EDEE,
            0x03_EDEF,
            0x08_A839,
            STEP_SITE + 6,
            0x8FB6,
            0x18_8181,
            0x2B_F037,
            0x18_1022,
        ] {
            let mut changed = image.to_vec();
            changed[at] ^= 2;
            assert!(!admitted(&changed), "changed source at {at:06X}");
        }
        let mut private = image.to_vec();
        private[0x03_EDEE] = 0;
        assert!(!admitted(&private));
        assert!(!admitted(&[]));
    }
}
