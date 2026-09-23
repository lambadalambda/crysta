//! Source-derived ordinary COP26 action timing, one resident at a time.
//!
//! A step lasts one repetition of the walking display list for its direction
//! (`$80:8F65`); an idle or refusal repeats the facing's idle list `$80:8FB5`
//! times by `class & 3`. Only the class-0 movement row on the common `$6000`
//! base is admitted, which moves 0.5 px per tick. Anything else keeps the
//! actor's approximate projection.
use assets::compression::decode;
use assets::maps::actors::rom_offset as offset;
use room_core::Direction;

/// Ticks of one COP26 action, by the source chain of one resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cadence {
    /// Walking list length for down, up, left and right.
    pub(super) walk: [u16; 4],
    /// Idle and refusal length, the same whatever the facing.
    pub(super) idle: u16,
}

impl Cadence {
    pub(super) const fn walk(&self, direction: Direction) -> u16 {
        self.walk[direction as usize]
    }
}

/// Why a resident keeps the approximate projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The descriptor or its packet pointer is not readable ROM.
    Descriptor,
    /// `class & !3` selects a movement row other than class 0's.
    MovementRow(u8),
    /// Mode bits 4..6 select a private movement resource.
    PrivateMovement(u16),
    /// The class tables, load sites or common streams are not the audited ones.
    Tables,
    /// The composition packet or one of its six display lists does not decode.
    Lists,
    /// A walking list does not cover exactly 16 pixels.
    Distance(Direction),
    /// Idle lists of different lengths would make the idle depend on facing.
    IdleByFacing,
}

/// `$80:8F6D` class-0 row: (pose, movement) for down, up, left, right.
const MOVEMENT_ROW: [u8; 8] = [3, 0x68, 4, 0x69, 5, 0x60, 5, 0x60];
/// `$80:8FB5`: (idle pose, repetitions) by `class & 3`, then direction.
const IDLE_TABLE: [u8; 32] = [
    0, 16, 1, 16, 2, 16, 2, 16, 0, 8, 1, 8, 2, 8, 2, 8, 0, 1, 1, 1, 2, 1, 2, 1, 3, 1, 4, 1, 5, 1,
    5, 1,
];

/// Derives the action timing of a resident built from the descriptor at
/// `descriptor` (its own or the one it reuses), or says why it cannot.
pub(super) fn derive(image: &[u8], descriptor: usize) -> Result<Cadence, Refusal> {
    let (packet, mode) = image
        .get(descriptor..descriptor + 4)
        .and_then(|bytes| Some((offset(&bytes[..3])?, bytes[3])))
        .ok_or(Refusal::Descriptor)?;
    // `$80:FAA4`: class = mode & $0F. `$80:FAAF..FABD`: base $4000 + (mode & $70) << 8.
    let class = mode & 0x0F;
    if class & !3 != 0 {
        return Err(Refusal::MovementRow(class));
    }
    let base = 0x4000 + (u16::from(mode & 0x70) << 8);
    if base != 0x6000 {
        return Err(Refusal::PrivateMovement(base));
    }
    if image.get(0x8F6D..0x8F75) != Some(&MOVEMENT_ROW)
        || image.get(0x8FB5..0x8FD5) != Some(&IDLE_TABLE)
        || !common_streams(image)
    {
        return Err(Refusal::Tables);
    }
    let lists = display_lists(image, packet).ok_or(Refusal::Lists)?;
    let mut walk = [0; 4];
    for direction in [
        Direction::Down,
        Direction::Up,
        Direction::Left,
        Direction::Right,
    ] {
        let pose = MOVEMENT_ROW[direction as usize * 2];
        let ticks = lists[usize::from(pose)];
        // The stream applies 1, 0, 1, ... from the list's first tick.
        if ticks.div_ceil(2) != 16 {
            return Err(Refusal::Distance(direction));
        }
        walk[direction as usize] = ticks;
    }
    let row = usize::from(class) * 8;
    let idles: Vec<u16> = IDLE_TABLE[row..row + 8]
        .chunks(2)
        .map(|pair| lists[usize::from(pair[0])].checked_mul(u16::from(pair[1])))
        .collect::<Option<_>>()
        .filter(|idles: &Vec<u16>| idles[0] != 0)
        .ok_or(Refusal::Lists)?;
    if idles.iter().any(|idle| *idle != idles[0]) {
        return Err(Refusal::IdleByFacing);
    }
    Ok(Cadence {
        walk,
        idle: idles[0],
    })
}

fn display_lists(image: &[u8], packet: usize) -> Option<[u16; 6]> {
    list_ticks(&decode(image.get(packet..)?, 0x10000).ok()?.data)
}

/// Total ticks of each of the six ordinary display lists in a decoded
/// packet: raw duration + 1 per record (`$80:C72C` pre-decrements), up to
/// the `$FFFF` terminator. Control records and empty lists refuse.
fn list_ticks(body: &[u8]) -> Option<[u16; 6]> {
    let mut lists = [0; 6];
    for (selector, total) in lists.iter_mut().enumerate() {
        let mut at = usize::from(word(body, selector * 2)?);
        for _ in 0..16 {
            match word(body, at)? {
                0xFFFF => break,
                record if record >= 0x8000 => return None,
                _ => *total += u16::from(body[at]) + 1,
            }
            at += 4;
        }
        if word(body, at)? != 0xFFFF || *total == 0 {
            return None;
        }
    }
    Some(lists)
}

/// Both source load sites put the common resource at `$7F:6000`, and its
/// class-0 row streams are the audited 1,0 loops.
fn common_streams(image: &[u8]) -> bool {
    // Packed $09F037, base bank $98, resolves to $AB:F037; destination
    // operand 2 is $7F:6000. No relocation here.
    for site in [0x18_817D, 0x18_8272] {
        if image.get(site..site + 7) != Some(&[1, 1, 2, 0, 0x37, 0xF0, 9]) {
            return false;
        }
    }
    let Some(common) = decode(image.get(0x2B_F037..).unwrap_or(&[]), 0x1A0C)
        .ok()
        .filter(|packet| packet.data.len() == 0x1A0C)
        .map(|packet| packet.data)
    else {
        return false;
    };
    let streams: [(usize, u16, u16, u16, u16); 3] = [
        (0x60, 0x6D18, 0, 0x6D18, 1),
        (0x68, 0, 0x6FC8, 0x6FC8, 1),
        (0x69, 0, 0x6FD4, 0x6FD4, 0xFFFF),
    ];
    streams
        .into_iter()
        .all(|(selector, x, y, pointer, velocity)| {
            let at = usize::from(pointer) - 0x6000 + 2;
            let records = [0, velocity, 0, 0, 0xFFFF, x.max(y) + 2];
            word(&common, selector * 4) == Some(x)
                && word(&common, selector * 4 + 2) == Some(y)
                && records
                    .iter()
                    .enumerate()
                    .all(|(i, value)| word(&common, at + i * 2) == Some(*value))
        })
}

pub(super) fn word(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// Skipped services whose handlers write none of the state the derivation
/// assumes: class, movement base or bank, stream pointers, entity +$04
/// movement bits, +$06 bit $0080, the flip bit, the hitbox or the display
/// list. Anything else revokes admission, including COP06's long jump.
/// COP65's callback is installed but never run here, as for every callback.
pub(super) fn benign_skipped_service(image: &[u8], pc: usize) -> bool {
    match image.get(pc..pc + 3) {
        // Interaction, occupancy, and the collision callback, which sets
        // +$04 bit $0200 only.
        Some(&[2, 0x21 | 0x3B | 0x65, _]) => true,
        // +$08 = (+$08 & $F1FF) | operand << 8: below $40 the flip bit stays.
        Some(&[2, 0xBB, operand]) => operand < 0x40,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body whose six lists hold the given raw durations.
    fn body(lists: [&[u8]; 6]) -> Vec<u8> {
        let mut body = vec![0; 12];
        for (selector, durations) in lists.iter().enumerate() {
            let at = u16::try_from(body.len()).unwrap();
            body[selector * 2..selector * 2 + 2].copy_from_slice(&at.to_le_bytes());
            for duration in *durations {
                body.extend_from_slice(&[*duration, 0, 0, 0]);
            }
            body.extend_from_slice(&[0xFF, 0xFF]);
        }
        body
    }

    #[test]
    fn list_ticks_count_each_record_one_tick_past_its_duration() {
        let d = &[0][..];
        assert_eq!(
            list_ticks(&body([d, d, d, &[7; 4], &[7, 7, 7, 6], &[7, 7]])),
            Some([1, 1, 1, 32, 31, 16])
        );
        // Durations of 255 do not wrap: sixteen records are 4096 ticks.
        assert_eq!(
            list_ticks(&body([d, d, d, &[255; 16], d, d])).map(|l| l[3]),
            Some(4096)
        );
        // An empty list, a control record, a missing terminator.
        assert_eq!(list_ticks(&body([d, &[], d, d, d, d])), None);
        let mut control = body([d; 6]);
        control[13] = 0x80;
        assert_eq!(list_ticks(&control), None);
        assert_eq!(list_ticks(&body([d, d, d, &[0; 17], d, d])), None);
        assert_eq!(list_ticks(&[]), None);
    }

    #[test]
    fn skipped_services_are_allowed_by_what_their_handlers_write() {
        for (bytes, allowed) in [
            ([2, 0x3B, 0], true),
            ([2, 0x21, 0x94], true),
            ([2, 0x65, 0x24], true),
            ([2, 0x03, 0x02], false),
            ([2, 0xBB, 0x0E], true),
            ([2, 0xBB, 0x40], false),
            ([2, 0x06, 0x2D], false),
            ([2, 0x28, 0x04], false),
        ] {
            assert_eq!(benign_skipped_service(&bytes, 0), allowed, "{bytes:02X?}");
        }
        assert!(!benign_skipped_service(&[2, 0x3B], 0));
    }

    fn owned_rom() -> Option<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let bytes = std::fs::read(path).ok()?;
        Some(rom::Rom::load(&bytes).unwrap().image().to_vec())
    }

    #[test]
    fn every_slice_walker_derives_its_own_timing() {
        let Some(image) = owned_rom() else {
            return;
        };
        let class_zero = Cadence {
            walk: [32; 4],
            idle: 16,
        };
        for (map, record, expected) in [
            (0xD, 0x03_8CB4, class_zero),
            (0xA, 0x03_89FB, class_zero),
            // Class 2: one repetition of a two-record idle list.
            (0xA, 0x03_8A19, class_zero),
            (0xA, 0x03_8A23, class_zero),
            (0xA, 0x03_8A2D, class_zero),
            (0x15, 0x03_8F67, class_zero),
            // Its up list is 7,7,7,6: 31 ticks, still 16 pixels.
            (
                0x1A,
                0x03_90BD,
                Cadence {
                    walk: [32, 31, 32, 32],
                    idle: 16,
                },
            ),
            (0x1B, 0x03_90F7, class_zero),
        ] {
            let resident = residents(&image, map)
                .into_iter()
                .find(|resident| resident.record == record)
                .unwrap();
            assert_eq!(
                derive(&image, resident.descriptor.unwrap()),
                Ok(expected),
                "{map:#x} {record:06X}"
            );
        }
        // The two class-2 records without a descriptor take `$8A19`'s.
        let town = residents(&image, 0xA);
        for record in [0x03_8A19, 0x03_8A23, 0x03_8A2D] {
            let resident = town
                .iter()
                .find(|resident| resident.record == record)
                .unwrap();
            assert_eq!(resident.descriptor, Some(0x03_ED37));
        }
        // No other drawn resident in the slice runs COP26 then COP8F.
        let mut walkers = 0;
        for map in 0xA..=0x21 {
            for resident in residents(&image, map) {
                let Some(script) = resident.script.filter(|_| resident.body) else {
                    continue;
                };
                let start = usize::try_from(script & 0x3F_FFFF).unwrap();
                let walks = image[start..start + 0x80]
                    .windows(8)
                    .any(|w| w[..2] == [2, 0x26] && w[6..] == [2, 0x8F]);
                if walks {
                    walkers += 1;
                    let descriptor = resident.descriptor.unwrap();
                    assert!(derive(&image, descriptor).is_ok(), "{map:#x}");
                }
            }
        }
        assert_eq!(walkers, 8);
        // Bytes that are no descriptor: a pointer outside ROM.
        assert_eq!(derive(&image, 0), Err(Refusal::Descriptor));
    }

    fn residents(image: &[u8], map: u16) -> Vec<crate::residents::Resident> {
        let flags = crate::world::new_game_flags();
        let events = assets::maps::scripts::EventFlags::Bitmap(&flags);
        crate::residents::residents(image, map, events).unwrap()
    }

    #[test]
    fn changed_source_is_refused_rather_than_guessed() {
        let Some(image) = owned_rom() else {
            return;
        };
        let changed = |at: usize, value: u8| {
            let mut copy = image.clone();
            copy[at] = value;
            derive(&copy, 0x03_EDEB)
        };
        // Mode $24 is class 4, the 1 px/tick row; $30 a private resource.
        assert_eq!(changed(0x03_EDEE, 0x24), Err(Refusal::MovementRow(4)));
        assert_eq!(
            changed(0x03_EDEE, 0x30),
            Err(Refusal::PrivateMovement(0x7000))
        );
        // Class 1 has the audited tables and eight idle repetitions of one tick.
        assert_eq!(changed(0x03_EDEE, 0x21).map(|cadence| cadence.idle), Ok(8));
        for at in [0x8F6E, 0x8FB6, 0x18_8181, 0x2B_F037] {
            assert_eq!(changed(at, image[at] ^ 2), Err(Refusal::Tables), "{at:06X}");
        }
        assert_eq!(changed(0x18_1022, 0xFF), Err(Refusal::Lists));
        assert_eq!(changed(0x03_EDED, 0x70), Err(Refusal::Descriptor));
    }
}
