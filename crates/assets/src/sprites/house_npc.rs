//! One source-derived house resident, with an explicit frozen ordinary pose.

use super::{SpriteError, SpriteFrame, bank_range, take, word};
use crate::{
    compression,
    graphics::{Bgr555, Tile4bpp, decode_tiles_4bpp},
};
use std::ops::Range;

/// The first ordinary resident in room $10. No AI, dialogue, collision or clock.
/// Caller authenticates the Japanese ROM. This is a frozen presentation, not an
/// evaluator for the conditional spawn list or the resident's interaction script.
#[derive(Debug)]
pub struct HouseNpc {
    position: [u16; 2],
    pose_key: (u32, u16),
    graphics_packet: u32,
    source_frame: SpriteFrame,
    frame: SpriteFrame,
    graphics: Vec<Tile4bpp>,
    palette: [Bgr555; 16],
    ranges: Vec<Range<usize>>,
}
impl HouseNpc {
    /// Follow the room's spawn/header/resource pointers and decode animation 2.
    /// Only the witnessed ordinary right-facing, palette-5, OBJ2 family is accepted.
    ///
    /// # Errors
    /// Rejects changed branch/list/descriptor shapes, unsupported pose attributes,
    /// malformed packets, bank crossings, missing tiles and truncated source data.
    #[allow(clippy::too_many_lines)] // One bounded pointer chain, kept in source load order.
    pub fn from_rom(image: &[u8]) -> Result<Self, SpriteError> {
        let mut ranges = Vec::new();
        let mut read = |at, len| {
            let bytes = take(image, at, len)?;
            ranges.push(at..at + len);
            Ok::<_, SpriteError>(bytes)
        };
        if read(0x2_8020, 2)? != [0, 0] {
            return Err(SpriteError::Invalid("changed house actor table branch"));
        }
        let entry = read(0x3_8020, 2)?;
        let list = pointer(&[entry[0], entry[1], 0x83])?;
        bank_range(list, 29)?;
        if read(list, 19)?
            != [
                0, 6, 0xfd, 0x18, 6, 0, 0x29, 0xa1, 0x84, 0xfa, 0xac, 1, 0x85, 0xea, 0xfa, 0x96, 1,
                0xaf, 0x8d,
            ]
        {
            return Err(SpriteError::Invalid("changed house ordinary spawn prefix"));
        }
        let spawn = read(list + 19, 10)?;
        if spawn[0] != 1 || spawn[3] != 0 {
            return Err(SpriteError::Invalid("changed house resident spawn flags"));
        }
        let position = [u16::from(spawn[1]) * 16 + 8, u16::from(spawn[2]) * 16];
        let header = pointer(&spawn[4..7])?;
        bank_range(header, 0x37)?;
        if read(header, 5)? != [2, 0, 0x51, 0, 0] {
            return Err(SpriteError::Invalid("changed house resident header"));
        }
        let ordinary = header + 0x2a;
        let script = read(ordinary, 13)?;
        if script[..2] != [2, 0x23]
            || usize::from(word(script, 2)) != ordinary & 0xffff
            || script[4..] != [2, 0xb6, 2, 0x80, 2, 2, 0x8e, 0x80, 0xf3]
        {
            return Err(SpriteError::Invalid("changed house ordinary pose script"));
        }
        let descriptor = pointer(&spawn[7..10])?;
        bank_range(descriptor, 13)?;
        let desc = read(descriptor, 13)?;
        if desc[3..] != [0x20, 0, 0x81, 2, 2, 0x0a, 0, 0, 0xc0, 3] {
            return Err(SpriteError::Invalid(
                "unsupported house resource descriptor",
            ));
        }
        let composition_start = pointer(&desc[..3])?;
        let palette_start = pointer(read(0xfc75, 3)?)? + 0x20;
        let graphics_pointer = read(0xfda7, 3)?;
        let graphics_start = pointer(graphics_pointer)?;
        let graphics_packet = u32::from_le_bytes([
            graphics_pointer[0],
            graphics_pointer[1],
            graphics_pointer[2],
            0,
        ]);
        bank_range(palette_start - 0x20, 0x40)?;
        let colors = read(palette_start, 32)?;
        let palette = std::array::from_fn(|i| Bgr555::new(word(colors, i * 2)));
        let decoded = packet(image, composition_start, &mut ranges)?;
        let sequence = usize::from(word(take(&decoded, 4, 2)?, 0));
        let record = take(&decoded, sequence, 6)?;
        if record[..2] != [0, 3] || record[4..] != [0xff, 0xff] {
            return Err(SpriteError::Invalid("changed house ordinary frame list"));
        }
        let anchor = usize::from(word(record, 2));
        let count = usize::from(take(&decoded, anchor, 17)?[16]);
        let raw = take(&decoded, anchor, 17 + count * 7)?;
        let source_frame = SpriteFrame::decode(raw)?;
        let mut adjusted = raw.to_vec();
        for (i, c) in source_frame.components().iter().enumerate() {
            if c.word() & 0x3e00 != 0x2200
                || usize::from(c.word() & 511) + if c.size() == 16 { 17 } else { 0 } >= 256
            {
                return Err(SpriteError::Invalid(
                    "unsupported house component palette/priority/tile",
                ));
            }
            // Native $80:FE8F replaces palette 1 with palette 5. Source tile IDs
            // remain unchanged; the later OAM +$100 VRAM relocation is NOT applied.
            let word = (c.word() & !0x0e00) | 0x0a00;
            adjusted[22 + i * 7..24 + i * 7].copy_from_slice(&word.to_le_bytes());
        }
        let frame = SpriteFrame::decode(&adjusted)?;
        let graphics_bytes = packet(image, graphics_start, &mut ranges)?;
        if graphics_bytes.len() != 0x2000 {
            return Err(SpriteError::Invalid("changed house graphics extent"));
        }
        let graphics = decode_tiles_4bpp(&graphics_bytes)?;
        let decoded_pointer = u16::try_from(anchor + 4)
            .map_err(|_| SpriteError::Invalid("oversized house composition offset"))?;
        let pose_key = (
            u32::from_le_bytes([desc[0], desc[1], desc[2], 0]),
            decoded_pointer,
        );
        Ok(Self {
            position,
            pose_key,
            graphics_packet,
            source_frame,
            frame,
            graphics,
            palette,
            ranges,
        })
    }
    /// Qualified map; remove this presentation in every other room.
    #[must_use]
    pub const fn map_id(&self) -> u16 {
        0x10
    }
    /// Source spawn coordinates, not a runtime snapshot or collision bounds.
    #[must_use]
    pub const fn position(&self) -> [u16; 2] {
        self.position
    }
    /// (CPU compressed-packet address, decoded composition offset). Not additive.
    #[must_use]
    pub const fn pose_key(&self) -> (u32, u16) {
        self.pose_key
    }
    /// CPU address of the descriptor-selected compressed graphics resource.
    #[must_use]
    pub const fn graphics_packet(&self) -> u32 {
        self.graphics_packet
    }
    /// Qualified native facing byte: 3 = right.
    #[must_use]
    pub const fn facing(&self) -> u8 {
        3
    }
    /// Ordinary script explicitly clears horizontal mirroring.
    #[must_use]
    pub const fn hflip(&self) -> bool {
        false
    }
    /// Original decompressed composition, before native palette relocation.
    #[must_use]
    pub const fn source_composition(&self) -> &SpriteFrame {
        &self.source_frame
    }
    /// Shared compositor with native palette relocation applied. Effective Y
    /// already accounts for OAM Y+1. No additional tile-slot relocation is needed.
    #[must_use]
    pub const fn composition(&self) -> &SpriteFrame {
        &self.frame
    }
    /// Source-indexed 256-tile resource from the descriptor's graphics packet.
    #[must_use]
    pub fn graphics(&self) -> &[Tile4bpp] {
        &self.graphics
    }
    /// CGRAM base for indexing `palette_index - palette_base()` from `SpritePixel`.
    #[must_use]
    pub const fn palette_base(&self) -> u8 {
        208
    }
    /// Sixteen natural BGR555 colors; color zero is transparent.
    #[must_use]
    pub const fn palette(&self) -> &[Bgr555; 16] {
        &self.palette
    }
    /// Exact headerless ROM extents read, including compressed packet boundaries.
    #[must_use]
    pub fn source_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
}
fn pointer(p: &[u8]) -> Result<usize, SpriteError> {
    let bank = p[2];
    let offset = word(p, 0);
    if bank < 0x80 || (bank < 0xc0 && offset < 0x8000) {
        return Err(SpriteError::Invalid("unsupported house ROM pointer"));
    }
    Ok((usize::from(bank & 63) << 16) | usize::from(offset))
}
fn packet(
    image: &[u8],
    start: usize,
    ranges: &mut Vec<Range<usize>>,
) -> Result<Vec<u8>, SpriteError> {
    let end = ((start >> 16) + 1) << 16;
    let bytes = image
        .get(start..end.min(image.len()))
        .ok_or(SpriteError::Invalid("truncated house packet"))?;
    let decoded = compression::decode(bytes, 0xffff)
        .map_err(|_| SpriteError::Invalid("invalid house compressed packet"))?;
    ranges.push(start..start + decoded.consumed);
    Ok(decoded.data)
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
