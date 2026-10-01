//! The towers' push routines (`docs/tower-two.md` §2): statues and blocks
//! call shared native code to test whether Ark pushes them (`$90:FE3D`,
//! `FEAC`, `FF12`, `FF85`), to take the pad from him (`$90:FDA7`) and to
//! give it back (`$90:FDBF`). The runtime recognises them by their bytes
//! and evaluates them here.

/// A shared push routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Routine {
    /// Carry set when Ark pushes the actor toward this facing code.
    Test(u8),
    /// Ark's controller waits: the pad does nothing.
    Take,
    /// The pad comes back, carry clear; carry set when Ark is busy.
    Give,
}

/// The routine at `at`, if it is one: by its first bytes, which are the
/// same in both revisions (only the addresses inside differ).
pub(super) fn routine(image: &[u8], at: usize) -> Option<Routine> {
    let code = image.get(at..at + 16)?;
    match code {
        // LDY $0DEA; LDA $0014,Y; CMP #d; BNE; LDA $066C
        [0xAC, 0xEA, 0x0D, 0xB9, 0x14, 0x00, 0xC9, d, 0x00, 0xD0, _, 0xAD, 0x6C, 0x06, ..]
            if *d < 4 =>
        {
            Some(Routine::Test(*d))
        }
        // LDY $0DEE; LDA #wait; STA $000A,Y; LDA #bank; STA $000C,Y; RTS
        [0xAC, 0xEE, 0x0D, 0xA9, _, _, 0x99, 0x0A, 0x00, 0xA9, _, 0x00, 0x99, 0x0C, 0x00, 0x60] => {
            Some(Routine::Take)
        }
        // COP 71 0 $FFFF busy; CLC; LDA $066C
        [0x02, 0x71, 0x00, 0x00, 0xFF, 0xFF, _, _, 0x18, 0xAD, 0x6C, 0x06, ..] => {
            Some(Routine::Give)
        }
        _ => None,
    }
}

/// The pad bit of a facing code: Down `$0400`, Up `$0800`, Left `$0200`,
/// Right `$0100`.
const fn pad_bit(facing: u8) -> u16 {
    match facing {
        0 => 0x0400,
        1 => 0x0800,
        2 => 0x0200,
        _ => 0x0100,
    }
}

/// Whether Ark at `ark`, facing `facing` with `pad` held, pushes the actor
/// at `at` with box (x offset, width, y offset, height) toward `direction`:
/// he faces it, holds that way, and stands at the box's edge (within one
/// pixel) inside its span.
pub(super) fn pushes(
    direction: u8,
    (ark, facing, pad): ((u16, u16), u8, u16),
    at: (u16, u16),
    [bx, bw, by, bh]: [i8; 4],
) -> bool {
    if facing != direction || pad & pad_bit(direction) == 0 {
        return false;
    }
    let (x, y) = (i32::from(at.0), i32::from(at.1));
    let (ax, ay) = (i32::from(ark.0), i32::from(ark.1));
    let [bx, bw, by, bh] = [bx, bw, by, bh].map(i32::from);
    let near = |a: i32, b: i32| (a - b).abs() < 2;
    let across = |low: i32, high: i32, value: i32| low < value && value <= high;
    match direction {
        0 => near(ay, y + by) && across(x + bx, x + bx + bw, ax),
        1 => near(ay, y + by + bh + 16) && across(x + bx, x + bx + bw, ax),
        2 => near(ax, x + bx + bw + 8) && across(y + by + 7, y + by + 7 + bh, ay),
        _ => near(ax, x + bx - 8) && across(y + by + 7, y + by + 7 + bh, ay),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: [i8; 4] = [-8, 16, -16, 16];

    #[test]
    fn ark_pushes_right_from_the_left_edge_holding_right() {
        // A statue at (152,368): its box spans x 144..160, y 352..368.
        let at = (152, 368);
        assert!(pushes(3, ((136, 368), 3, 0x0100), at, BOX));
        // One pixel off still pushes; two do not.
        assert!(pushes(3, ((137, 368), 3, 0x0100), at, BOX));
        assert!(!pushes(3, ((138, 368), 3, 0x0100), at, BOX));
        // Not facing it, or not holding the way.
        assert!(!pushes(3, ((136, 368), 0, 0x0100), at, BOX));
        assert!(!pushes(3, ((136, 368), 3, 0x0400), at, BOX));
        // Out of its span.
        assert!(!pushes(3, ((136, 390), 3, 0x0100), at, BOX));
    }

    #[test]
    fn up_and_down_take_ark_below_and_above() {
        let at = (152, 368);
        assert!(pushes(1, ((152, 384), 1, 0x0800), at, BOX));
        assert!(pushes(0, ((152, 352), 0, 0x0400), at, BOX));
        assert!(pushes(2, ((168, 368), 2, 0x0200), at, BOX));
    }

    #[test]
    fn the_routines_are_known_by_their_bytes() {
        let mut image = vec![0; 0x40];
        image[..14].copy_from_slice(&[
            0xAC, 0xEA, 0x0D, 0xB9, 0x14, 0x00, 0xC9, 0x03, 0x00, 0xD0, 0x61, 0xAD, 0x6C, 0x06,
        ]);
        image[0x10..0x20].copy_from_slice(&[
            0xAC, 0xEE, 0x0D, 0xA9, 0xB7, 0xFD, 0x99, 0x0A, 0x00, 0xA9, 0x90, 0x00, 0x99, 0x0C,
            0x00, 0x60,
        ]);
        image[0x20..0x2C].copy_from_slice(&[
            0x02, 0x71, 0x00, 0x00, 0xFF, 0xFF, 0x3B, 0xFE, 0x18, 0xAD, 0x6C, 0x06,
        ]);
        assert_eq!(routine(&image, 0), Some(Routine::Test(3)));
        assert_eq!(routine(&image, 0x10), Some(Routine::Take));
        assert_eq!(routine(&image, 0x20), Some(Routine::Give));
        assert_eq!(routine(&image, 1), None);
    }
}
