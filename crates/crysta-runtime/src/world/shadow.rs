//! Ark's ground shadow (`$84:A917`, `docs/jump.md`): a list of the helper
//! art at his ground place, always on off the world map; a jump picks a
//! smaller one by his height the frame before (`$84:A9F1`).

use super::World;

/// The lists by height: 48 up or more the dot, 24 or more the small one,
/// else the ground one.
const LISTS: [(u16, u8); 2] = [(48, 0), (24, 1)];
const GROUND: u8 = 2;
/// Cell attributes the shadow is not drawn on (`$84:AAC9`): the lips and
/// the like. Natively it moves a row down there (`$84:A957`, code only).
const UNDRAWN: [u16; 8] = [0x08, 0x09, 0x0A, 0x0B, 0x0E, 0x0F, 0x1A, 0x1F];

/// The shadow's list for a height above the ground.
pub(super) const fn list(height: i16) -> u8 {
    let up = height.unsigned_abs();
    if up >= LISTS[0].0 {
        LISTS[0].1
    } else if up >= LISTS[1].0 {
        LISTS[1].1
    } else {
        GROUND
    }
}

impl World<'_> {
    /// Ark's shadow: its place (his ground place), the helper art and its
    /// list; `None` on the world map, while he falls, drops from a lip or
    /// hangs on the rope (`$097C & $EA`), while he is hidden, and over the
    /// lips and the like (`$84:AAC9`).
    #[must_use]
    pub fn shadow(&self) -> Option<((u16, u16), u32, u8)> {
        if self.plane.is_some()
            || self.fall.is_some()
            || self.lip_drop.is_some()
            || self.rope.is_some()
            || self.ark_blinks()
        {
            return None;
        }
        let (x, y) = self.position();
        let under = self.attribute((x / 16, y.wrapping_sub(8) / 16));
        if under.is_some_and(|attribute| UNDRAWN.contains(&attribute)) {
            return None;
        }
        let helper = crate::actors::helper(self.image);
        Some(((x, y), helper, list(self.jump_height_before())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_shrinks_with_height() {
        assert_eq!(
            [list(0), list(-23), list(-24), list(-47), list(-48)],
            [2, 2, 1, 1, 0]
        );
    }
}
