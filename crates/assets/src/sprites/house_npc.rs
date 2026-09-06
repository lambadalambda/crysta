//! Compatibility wrapper for the first room-10 resident, using the shared house loader.
use super::{HouseActor, HouseGraphicsKey, HousePoseKey, HouseScenes, SpriteError, SpriteFrame};
use crate::graphics::{Bgr555, Tile4bpp};
use std::ops::Range;

/// The original first-resident API. New scene assembly should use `HouseScenes`.
#[derive(Debug)]
pub struct HouseNpc {
    actor: HouseActor,
    graphics_packet: u32,
    pose_key: (u32, u16),
}
impl HouseNpc {
    /// Decode the original first room-10 resident through the shared source loader.
    ///
    /// # Errors
    /// Rejects malformed or changed qualified source records/resources.
    pub fn from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        {
            let actor = HouseScenes::legacy_first(image)?;
            let HouseGraphicsKey::Compressed(graphics_packet) = actor.graphics_key();
            let HousePoseKey::Compressed { packet, offset } = actor.setup_frame().key() else {
                return Err(SpriteError::Invalid("legacy pose is not compressed"));
            };
            Ok(Self {
                actor,
                graphics_packet,
                pose_key: (packet, offset),
            })
        }
    }
    /// Qualified map ID.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        self.actor.map_id()
    }
    /// Source spawn position.
    #[must_use]
    pub const fn position(&self) -> [u16; 2] {
        self.actor.position()
    }
    /// Original (compressed packet CPU address, decoded composition offset) key.
    #[must_use]
    pub const fn pose_key(&self) -> (u32, u16) {
        self.pose_key
    }
    /// Source compressed graphics packet address.
    #[must_use]
    pub const fn graphics_packet(&self) -> u32 {
        self.graphics_packet
    }
    /// Native facing: 3/right.
    #[must_use]
    pub const fn facing(&self) -> u8 {
        self.actor.facing()
    }
    /// Ordinary horizontal mirror: false.
    #[must_use]
    pub const fn hflip(&self) -> bool {
        self.actor.hflip()
    }
    /// Original composition before native palette relocation.
    #[must_use]
    pub const fn source_composition(&self) -> &SpriteFrame {
        self.actor.setup_frame().source_composition()
    }
    /// Shared effective-pixel composition, palette relocated but no VRAM tile offset.
    #[must_use]
    pub const fn composition(&self) -> &SpriteFrame {
        self.actor.setup_frame().composition()
    }
    /// Source-indexed graphics.
    #[must_use]
    pub fn graphics(&self) -> &[Tile4bpp] {
        self.actor.graphics()
    }
    /// CGRAM palette base 208.
    #[must_use]
    pub const fn palette_base(&self) -> u8 {
        self.actor.palette_base()
    }
    /// Natural BGR555 colors.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 16] {
        self.actor.palette()
    }
    /// Exact headerless source ranges; legacy ordering is retained.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        self.actor.source_ranges()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprites::SpritePixel;
    fn put(r: &mut [u8], at: usize, bytes: &[u8]) {
        r[at..at + bytes.len()].copy_from_slice(bytes);
    }
    fn fixture() -> Vec<u8> {
        let mut r = vec![0; 0x40_0000];
        put(&mut r, 0x3_8020, &[0x69, 0x8d]);
        put(
            &mut r,
            0x3_8d69,
            &[
                0, 6, 0xfd, 0x18, 6, 0, 0x29, 0xa1, 0x84, 0xfa, 0xac, 1, 0x85, 0xea, 0xfa, 0x96, 1,
                0xaf, 0x8d,
            ],
        );
        put(
            &mut r,
            0x3_8d7c,
            &[1, 26, 26, 0, 0xa9, 0x98, 0x88, 0x5a, 0xed, 0x83],
        );
        put(&mut r, 0x8_98a9, &[2, 0, 0x51, 0, 0]);
        put(
            &mut r,
            0x8_98d3,
            &[
                2, 0x23, 0xd3, 0x98, 2, 0xb6, 2, 0x80, 2, 2, 0x8e, 0x80, 0xf3,
            ],
        );
        put(
            &mut r,
            0x3_ed5a,
            &[0x77, 0x4b, 0xd6, 0x20, 0, 0x81, 2, 2, 0x0a, 0, 0, 0xc0, 3],
        );
        put(&mut r, 0xfc75, &[0x0c, 0x2b, 0xcc]);
        put(&mut r, 0xfda7, &[0xf9, 0x96, 0xbf]);
        let mut frame = vec![0; 0x60];
        put(&mut frame, 0, &[0x10, 0]);
        put(&mut frame, 14, &[0x20, 0]);
        put(&mut frame, 4, &[0x10, 0]);
        put(&mut frame, 0x10, &[0, 3, 0x20, 0, 0xff, 0xff]);
        put(&mut frame, 0x20, &[8, 8, 16, 0]);
        frame[0x30] = 1;
        put(&mut frame, 0x31, &[1, 0, 0, 0, 0, 0, 0x22]);
        put(
            &mut r,
            0x16_4b77,
            &crate::compression::encode(&frame).unwrap(),
        );
        let mut tiles = vec![0; 0x2000];
        tiles[0] = 0x80;
        put(
            &mut r,
            0x3f_96f9,
            &crate::compression::encode(&tiles).unwrap(),
        );
        put(&mut r, 0xc_2b2e, &[0x1f, 0]);
        r
    }
    #[test]
    fn source_position_pose_palette_and_shared_sampling() {
        let mut r = fixture();
        let npc = HouseNpc::from_rom(&r).unwrap();
        assert_eq!(npc.map_id(), 16);
        assert_eq!(npc.position(), [424, 416]);
        assert_eq!(npc.pose_key(), (0xd6_4b77, 0x24));
        assert_eq!(npc.facing(), 3);
        assert!(!npc.hflip());
        assert_eq!(npc.palette_base(), 208);
        assert_eq!(npc.palette()[1].rgb8(), [255, 0, 0]);
        assert_eq!(npc.composition().bounds(false, false), (-8, -16, 8, 0));
        assert_eq!(
            npc.composition()
                .sample(npc.graphics(), false, false, -8, -16)
                .unwrap(),
            SpritePixel::Opaque {
                palette_index: 209,
                priority: 2,
                component: 0
            }
        );
        assert_eq!(
            npc.composition()
                .sample(npc.graphics(), false, false, -7, -16)
                .unwrap(),
            SpritePixel::Transparent
        );
        assert_eq!(npc.source_composition().components()[0].word(), 0x2200);
        r[0x3_8d7d] = 20;
        r[0x3_8d7e] = 22;
        assert_eq!(HouseNpc::from_rom(&r).unwrap().position(), [328, 352]);
    }
    #[test]
    fn follows_relocated_packets_and_palette_and_tracks_extents() {
        let mut r = fixture();
        let packet = crate::compression::encode(&vec![0; 0x2000]).unwrap();
        put(&mut r, 0xfda7, &[0, 0x80, 0xbe]);
        put(&mut r, 0x3e_8000, &packet);
        let colors = r[0xc_2b2c..0xc_2b4c].to_vec();
        put(&mut r, 0xfc75, &[0, 0x80, 0xcd]);
        put(&mut r, 0xd_8020, &colors);
        let n = HouseNpc::from_rom(&r).unwrap();
        assert!(
            n.source_ranges()
                .contains(&(0x3e_8000..0x3e_8000 + packet.len()))
        );
        assert!(n.source_ranges().contains(&(0xd_8020..0xd_8040)));
    }
    #[test]
    fn rejects_changed_branch_resource_pose_and_truncation() {
        for (at, value) in [
            (0x2_8020, 1),
            (0x3_8d72, 0),
            (0x8_98d7, 0),
            (0x3_ed62, 0x0c),
            (0x3_ed65, 0x80),
        ] {
            let mut r = fixture();
            r[at] = value;
            assert!(HouseNpc::from_rom(&r).is_err(), "{at:x}");
        }
        let r = fixture();
        for len in [0, 0x3_8021, 0x16_4b78, 0x3f_9700] {
            assert!(HouseNpc::from_rom(&r[..len]).is_err());
        }
        let mut r = fixture();
        let mut frame = vec![0; 0x60];
        put(&mut frame, 4, &[0x10, 0]);
        put(&mut frame, 0x10, &[0, 2, 0x20, 0, 0xff, 0xff]);
        put(
            &mut r,
            0x16_4b77,
            &crate::compression::encode(&frame).unwrap(),
        );
        assert!(HouseNpc::from_rom(&r).is_err());
    }
}
