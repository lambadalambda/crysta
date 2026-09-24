//! Source camera regions: the part of a shared layer that one map may show.

use super::VisualMapError;
use crate::layout;

/// A map's camera bounds and vertical clamp, as the map loader sets them.
///
/// Several maps share one layer; the region is what keeps the camera inside
/// the room being played. Ordinary settled follow only: no transition pans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraRegion {
    /// Normalized offset of the two-byte record at `$96BE30 + 2*map`.
    pub record_offset: usize,
    /// Left, top, right, bottom in layer pixels (exclusive right/bottom).
    pub bounds: [u16; 4],
    /// Source `$0866`: vertical clamp extent, NOT the 224-line viewport height.
    pub vertical_extent: u16,
}

impl CameraRegion {
    /// Reads the camera record and the scene's display profile.
    /// # Errors
    /// Refuses truncated input, a scene outside bank `$83`, a display profile
    /// whose clamp height is not the audited 256, and a region too small to clamp in.
    pub fn from_rom(image: &[u8], map_id: u16) -> Result<Self, VisualMapError> {
        let unsupported = VisualMapError::Unsupported;
        let map = usize::from(map_id) * 2;
        let bytes = |at: usize, n: usize| {
            image
                .get(at..at + n)
                .ok_or(unsupported("truncated camera source"))
        };
        let word = |at: usize| bytes(at, 2).map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])));
        // `$86955C..959B` takes bank `$83` only after a zero bank-`$82` entry.
        if word(0x28000 + map)? != 0 {
            return Err(unsupported("scene outside bank $83"));
        }
        let prefix = bytes(0x30000 + word(0x38000 + map)?, 2)?;
        // `$868C77..8C86` indexes `$96BB64 + 2*(selector & $3F)`.
        if prefix[0] != 0 || prefix[1] & 0xc0 != 0 {
            return Err(unsupported("unaudited scene prefix"));
        }
        // The European table is `$99:C2AE` (the operand at `$86:8C85`).
        let located = |japan: usize| {
            layout::offset(image, japan).ok_or(unsupported("unrecorded camera source"))
        };
        let table = located(0x16_bb64)?;
        let display = (table & 0x3f_0000) + word(table + usize::from(prefix[1]) * 2)?;
        // Profile byte +4 bit 6 selects `$0866 = 256` at `$868CDE..8CE6`.
        if bytes(display + 4, 1)?[0] & 0x40 == 0 {
            return Err(unsupported("unaudited camera clamp height"));
        }
        let vertical_extent = 256;
        // European `$99:C57A`, the operand at `$86:9375`.
        let record_offset = located(0x16_be30)? + map;
        let record = bytes(record_offset, 2)?;
        let (a, b) = (u16::from(record[0]), u16::from(record[1]));
        // `$869371..93F7`: low nibble is the first page, high nibble the page count.
        let (left, top) = ((a & 15) * 256, (b & 15) * 256);
        let bounds = [left, top, left + (a >> 4) * 256, top + (b >> 4) * 256];
        if bounds[2] - left < 256 || bounds[3] - top < vertical_extent {
            return Err(unsupported("camera region smaller than its clamp"));
        }
        Ok(Self {
            record_offset,
            bounds,
            vertical_extent,
        })
    }

    /// Ordinary follow/clamp at a settled state (`$8790A1..9105`).
    /// # Panics
    /// Caller-manufactured bounds may panic if they do not hold a 256-pixel-wide
    /// region of at least `vertical_extent` height. ROM-derived regions always do.
    #[must_use]
    pub fn settled_origin(&self, player: [u16; 2]) -> [u16; 2] {
        clamp_origin(self.bounds, self.vertical_extent, player)
    }
}

/// `clamp(x-128, left, right-256)` and `clamp(y-112, top, bottom-$0866)`.
/// # Panics
/// If the bounds do not hold a 256-pixel-wide region of at least
/// `vertical_extent` height; ROM-derived regions always do.
#[must_use]
pub(super) fn clamp_origin(bounds: [u16; 4], vertical_extent: u16, player: [u16; 2]) -> [u16; 2] {
    [
        player[0]
            .saturating_sub(128)
            .clamp(bounds[0], bounds[2] - 256),
        player[1]
            .saturating_sub(112)
            .clamp(bounds[1], bounds[3] - vertical_extent),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Map `id` with scene prefix `[0, selector]`, display byte +4 `clamp` and `record`.
    fn fixture(id: u16, selector: u8, clamp: u8, record: [u8; 2]) -> Vec<u8> {
        let mut image = vec![0; 0x17_0000];
        let (map, scene, display) = (usize::from(id) * 2, 0x9000, 0xbc00);
        image[0x38000 + map..0x38002 + map].copy_from_slice(&u16::to_le_bytes(scene));
        image[0x30000 + usize::from(scene) + 1] = selector;
        let table = 0x16_bb64 + usize::from(selector) * 2;
        image[table..table + 2].copy_from_slice(&u16::to_le_bytes(display));
        image[0x16_0000 + usize::from(display) + 4] = clamp;
        image[0x16_be30 + map..0x16_be32 + map].copy_from_slice(&record);
        image
    }

    #[test]
    fn a_record_selects_whole_pages_of_the_shared_layer() {
        let room = CameraRegion::from_rom(&fixture(0x11, 6, 0x64, [0x11, 0x12]), 0x11).unwrap();
        assert_eq!(room.record_offset, 0x16_be52);
        assert_eq!(room.bounds, [256, 512, 512, 768]);
        assert_eq!(room.vertical_extent, 256);
        let exterior = CameraRegion::from_rom(&fixture(0xa, 8, 0x64, [0x40, 0x40]), 0xa).unwrap();
        assert_eq!(exterior.bounds, [0, 0, 1024, 1024]);
    }

    #[test]
    fn unknown_scenes_profiles_and_regions_are_refused() {
        let good = fixture(0x11, 6, 0x64, [0x11, 0x12]);
        let refuse = |image: &[u8]| CameraRegion::from_rom(image, 0x11).is_err();
        assert!(!refuse(&good));
        // A bank-$82 scene entry takes a path not audited here.
        let mut image = good.clone();
        image[0x28000 + 0x22] = 1;
        assert!(refuse(&image));
        // So do a nonzero first prefix byte and selector bits above `& $3F`.
        let mut image = good.clone();
        image[0x39000] = 1;
        assert!(refuse(&image));
        let mut image = good.clone();
        image[0x39001] |= 0x40;
        assert!(refuse(&image));
        // Clamp height other than the audited bit-6 256.
        assert!(refuse(&fixture(0x11, 6, 0x24, [0x11, 0x12])));
        // A region with no page, or shorter than its clamp height.
        assert!(refuse(&fixture(0x11, 6, 0x64, [0x01, 0x12])));
        assert!(refuse(&fixture(0x11, 6, 0x64, [0x11, 0x02])));
        for at in [0x28023, 0x38023, 0x39001, 0x16_bb71, 0x16_bc04, 0x16_be53] {
            assert!(refuse(&good[..at]), "truncated at {at:x}");
        }
    }

    #[test]
    fn the_settled_camera_clamps_to_the_region_not_the_layer() {
        let region = |bounds| CameraRegion {
            record_offset: 0,
            bounds,
            vertical_extent: 256,
        };
        // One page: fixed wherever the player stands in it.
        let room = region([256, 512, 512, 768]);
        for player in [[256, 512], [384, 640], [511, 767], [0, 0], [u16::MAX; 2]] {
            assert_eq!(room.settled_origin(player), [256, 512]);
        }
        // Four pages square: follows, then stops at right-256 and bottom-$0866.
        let exterior = region([0, 0, 1024, 1024]);
        assert_eq!(exterior.settled_origin([364, 815]), [236, 703]);
        assert_eq!(exterior.settled_origin([0, 0]), [0, 0]);
        assert_eq!(exterior.settled_origin([u16::MAX; 2]), [768, 768]);
        // Two pages tall.
        let tall = region([0, 0, 256, 512]);
        assert_eq!(tall.settled_origin([128, 359]), [0, 247]);
        assert_eq!(tall.settled_origin([128, 500]), [0, 256]);
    }
}
