//! What an enemy sees of Ark (`docs/enemy-scripts.md` §3): the box in front
//! of it (`COP D2`, `$80:B26E`) and whether the two face each other.

use room_core::Direction;

/// Ark's probe (`$0966`, `$0968`): his position less 8 in y.
pub(super) const fn probe(player: (u16, u16)) -> (u16, u16) {
    (player.0, player.1.wrapping_sub(8))
}

/// A facing as the engine keeps it (`$0956`, entity `+$14`): 0 Down, 1 Up,
/// 2 Left, 3 Right.
pub(super) const fn code(facing: Direction) -> u8 {
    match facing {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

/// The box `w` to each side and `l` ahead of an actor at `at` facing
/// `facing` (a code): left, top, right, bottom, edges included, the low
/// edges clamped at 0.
pub(super) fn front(facing: u8, at: (u16, u16), w: u8, l: u8) -> (i32, i32, i32, i32) {
    let (x, y) = (i32::from(at.0), i32::from(at.1));
    let (w, l) = (i32::from(w), i32::from(l));
    let (left, top, right, bottom) = match facing & 3 {
        0 => (x - w, y, x + w, y + l),
        1 => (x - w, y - l, x + w, y),
        2 => (x - l, y - w, x, y + w),
        _ => (x, y - w, x + l, y + w),
    };
    (left.max(0), top.max(0), right, bottom)
}

/// Whether `COP D2`'s mode admits the facings: `7F` any, 0 facing each
/// other, 1 not facing each other, others never.
pub(super) const fn admits(mode: u8, facing: u8, player: u8) -> bool {
    let sum = facing.wrapping_add(player);
    match mode & 0x7F {
        0x7F => true,
        0 if mode == 0 => sum & 3 == 1,
        1 => sum & 1 == 0 || sum & 3 == 3,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_front_box_lies_ahead_and_clamps_low_edges() {
        assert_eq!(front(0, (100, 50), 16, 64), (84, 50, 116, 114));
        assert_eq!(front(1, (100, 50), 16, 64), (84, 0, 116, 50));
        assert_eq!(front(2, (100, 50), 16, 64), (36, 34, 100, 66));
        assert_eq!(front(3, (100, 50), 16, 64), (100, 34, 164, 66));
    }

    #[test]
    fn modes_test_the_facings() {
        // Down and Up face each other; Left and Right too.
        assert!(admits(0, 0, 1) && admits(0, 2, 3) && !admits(0, 0, 0));
        assert!(admits(1, 0, 0) && admits(1, 0, 3) && !admits(1, 1, 0));
        assert!(admits(0x7F, 0, 1) && admits(0xFF, 2, 2));
        // `$80` keeps 0 after the mask but is not the zero mode.
        assert!(!admits(0x80, 0, 1) && !admits(2, 0, 1));
    }
}
