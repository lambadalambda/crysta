//! Source-derived ordinary COP26 action timing, one resident at a time.
//!
//! A step lasts one repetition of the walking display list for its direction
//! (`$80:8F65`); an idle or refusal repeats the facing's idle list `$80:8FB5`
//! times by `class & 3`. Only the class-0 movement row on the common `$6000`
//! base is admitted, which moves 0.5 px per tick. Anything else keeps the
//! actor's approximate projection.
use super::{motion, COMMON_SIZE, COMMON_SOURCE};
use assets::compression::decode;
use assets::layout;
use assets::maps::actors::rom_offset as offset;
use assets::maps::scripts::unpack_pointer;
use assets::sprites::boxes::Record;
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
    let lists: Vec<u16> = display_lists(image, packet)
        .and_then(|lists| lists.get(..6)?.iter().copied().collect::<Option<Vec<_>>>())
        .filter(|lists| lists.iter().all(|ticks| *ticks > 0))
        .ok_or(Refusal::Lists)?;
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

/// Ticks of every display list of the packet a descriptor names, which a
/// `COP 8E` pose wait holds for.
pub(super) fn pose_ticks(image: &[u8], descriptor: usize) -> Option<Vec<Option<u16>>> {
    display_lists(image, offset(image.get(descriptor..descriptor + 3)?)?)
}

/// The records and boxes of every display list in a descriptor's packet
/// (`docs/combat.md`), for the hit scans.
pub(super) fn pose_boxes(image: &[u8], descriptor: usize) -> Option<Vec<Vec<Record>>> {
    let packet = offset(image.get(descriptor..descriptor + 3)?)?;
    lists_boxes(&decode(image.get(packet..)?, 0x10000).ok()?.data)
}

/// The same for the packet a `COP D8` pointer names: an LZ packet, or
/// direct lists as the helper art's (`$A2:C000`).
pub(super) fn packet_boxes(image: &[u8], pointer: &[u8]) -> Option<Vec<Vec<Record>>> {
    let packet = offset(pointer)?;
    if let Some(boxes) = decode(image.get(packet..)?, 0x10000)
        .ok()
        .and_then(|packet| lists_boxes(&packet.data))
    {
        return Some(boxes);
    }
    let end = ((packet >> 16) + 1).checked_mul(0x1_0000)?.min(image.len());
    lists_boxes(image.get(packet..end)?)
}

fn lists_boxes(data: &[u8]) -> Option<Vec<Vec<Record>>> {
    let lists = list_ticks(data)?.len();
    Some(
        (0..lists)
            .map(|list| {
                u8::try_from(list)
                    .ok()
                    .and_then(|list| assets::sprites::boxes::packet_list(data, list).ok())
                    .unwrap_or_default()
            })
            .collect(),
    )
}

/// Ticks of every display list of the packet a `COP D8` pointer names: an
/// LZ packet, or direct lists as Ark's and his helper's art (`$A2:C000`).
pub(super) fn packet_ticks(image: &[u8], pointer: &[u8]) -> Option<Vec<Option<u16>>> {
    let packet = offset(pointer)?;
    display_lists(image, packet).or_else(|| {
        let end = ((packet >> 16) + 1).checked_mul(0x1_0000)?.min(image.len());
        list_ticks(image.get(packet..end)?)
    })
}

/// Ticks of every direct Ark display list in one `$80:A24F` resource entry.
///
/// Unlike resident packets these lists are uncompressed in the bank named by
/// the resource table. The table itself is revision-aware; its pointers are
/// already for that revision.
pub(super) fn player_pose_ticks(image: &[u8], resource: u8) -> Option<Vec<Option<u16>>> {
    let table = layout::offset(image, 0xA24F)?;
    let entry = table.checked_add(usize::from(resource) * 6)?;
    let packet = offset(image.get(entry..entry + 3)?)?;
    let end = ((packet >> 16) + 1).checked_mul(0x1_0000)?.min(image.len());
    list_ticks(image.get(packet..end)?)
}

/// The complete frozen-return player stream relative to the map `$21` guide
/// header (`$88:AEB8`, European `$88:B73C`). The same relative offsets are
/// retained by both supported revisions.
const FROZEN_RETURN_HEADER: usize = 0x08_AEB8;
const FROZEN_RETURN_GUIDE: usize = 0x78;
const FROZEN_RETURN_PLAYER: usize = 0xBA;
const FROZEN_RETURN_SOURCE: &[u8] = &[
    0x02, 0x84, 0x17, 0x0F, 0x01, 0x02, 0x8E, 0x02, 0x84, 0x09, 0x1B, 0x00, 0x02, 0x8E, 0x02, 0x84,
    0x00, 0x00, 0x00, 0x02, 0x8E, 0x02, 0xC1, 0x3C, 0x00, 0x02, 0x84, 0x01, 0x00, 0x00, 0x02, 0x8E,
    0x02, 0xC1, 0x3C, 0x00, 0x02, 0x84, 0x00, 0x00, 0x00, 0x02, 0x8E, 0x02, 0xCB, 0x01, 0xC1, 0x87,
    0x84, 0x02, 0xBC, 0x6B,
];
const FROZEN_RETURN_SELECTIONS: &[(usize, [u8; 3])] = &[
    (0, [0x17, 0x0F, 1]),
    (7, [0x09, 0x1B, 0]),
    (14, [0, 0, 0]),
    (25, [1, 0, 0]),
    (36, [0, 0, 0]),
];

/// The normalized `COP DF` site in this revision's map `$21` guide.
pub(super) fn frozen_return_guide(image: &[u8]) -> Option<usize> {
    layout::offset(image, FROZEN_RETURN_HEADER)?.checked_add(FROZEN_RETURN_GUIDE)
}

/// The normalized entry of the frozen-return player script in this revision.
pub(super) fn frozen_return_start(image: &[u8]) -> Option<usize> {
    layout::offset(image, FROZEN_RETURN_HEADER)?.checked_add(FROZEN_RETURN_PLAYER)
}

/// Authenticates the exact map `$21` `COP DF` source, complete player stream,
/// direct Ark pose resources and every movement sample the stream consumes.
/// Returns the normalized player entry only when the whole bounded profile is
/// retained.
pub(super) fn frozen_return_profile(image: &[u8]) -> Option<usize> {
    let header = layout::offset(image, FROZEN_RETURN_HEADER)?;
    let guide = header.checked_add(FROZEN_RETURN_GUIDE)?;
    let player = header.checked_add(FROZEN_RETURN_PLAYER)?;
    let pointer = [
        u8::try_from(player & 0xFF).ok()?,
        u8::try_from(player >> 8 & 0xFF).ok()?,
        0x80 | u8::try_from(player >> 16).ok()?,
    ];
    let expected_guide = [0x02, 0xDF, pointer[0], pointer[1], pointer[2]];
    if image.get(guide..guide + expected_guide.len())? != expected_guide
        || image.get(player..player + FROZEN_RETURN_SOURCE.len())? != FROZEN_RETURN_SOURCE
        || !frozen_return_streams(image)
    {
        return None;
    }

    let table = layout::offset(image, 0xA24F)?;
    let resource_zero = [0xE4, 0xA1, layout::per_revision(image, 0xA4, 0xA6)];
    let resource_one = [0x64, 0xD0, layout::per_revision(image, 0x9A, 0x9C)];
    if image.get(table..table + 3)? != resource_zero
        || image.get(table + 6..table + 9)? != resource_one
    {
        return None;
    }
    let zero = player_pose_ticks(image, 0)?;
    let one = player_pose_ticks(image, 1)?;
    (zero.first() == Some(&Some(1))
        && zero.get(1) == Some(&Some(1))
        && zero.get(9) == Some(&Some(16))
        && one.get(0x17) == Some(&Some(36)))
    .then_some(player)
}

/// Whether one `COP 84` is at one of the five exact instruction sites in the
/// authenticated stream (four distinct selections; stationary zero repeats).
pub(super) fn frozen_return_selection(image: &[u8], player: usize, instruction: usize) -> bool {
    FROZEN_RETURN_SELECTIONS.iter().any(|&(offset, operands)| {
        player.checked_add(offset) == Some(instruction)
            && image.get(instruction..instruction + 5)
                == Some(&[0x02, 0x84, operands[0], operands[1], operands[2]])
    })
}

/// Whether the common resource retains the two source-pinned frozen-return
/// streams. The first emits 3,2,2 twelve times; the second emits the recorded
/// sixteen-tick braking tail. Selector zero must remain stationary.
pub(super) fn frozen_return_streams(image: &[u8]) -> bool {
    if !common_streams(image) {
        return false;
    }
    let Some(source) = layout::offset(image, COMMON_SOURCE) else {
        return false;
    };
    let Some(packet) = decode(image.get(source..).unwrap_or(&[]), COMMON_SIZE)
        .ok()
        .filter(|packet| packet.data.len() == COMMON_SIZE)
    else {
        return false;
    };
    let resource = motion::Resource {
        base: 0x6000,
        bytes: packet.data.into(),
    };
    let leg = |selector, expected: &[i16]| {
        let Some(mut motion) =
            motion::Motion::start(resource.clone(), selector, false, false, None)
        else {
            return false;
        };
        expected.iter().all(|&dy| motion.step() == Some((0, dy)))
    };
    leg(0, &[0])
        && leg(0x0F, &[3, 2, 2].repeat(12))
        && leg(0x1B, &[2, 2, 1, 2, 1, 2, 1, 2, 1, 0, 0, 1, 0, 0, 0, 0])
}

fn display_lists(image: &[u8], packet: usize) -> Option<Vec<Option<u16>>> {
    list_ticks(&decode(image.get(packet..)?, 0x10000).ok()?.data)
}

/// Total ticks of every display list in a decoded packet: raw duration + 1
/// per record (`$80:C72C` pre-decrements), up to the first word with bit 15
/// set, which ends a list (`$80:ED75`). The pointer table runs up to the
/// lowest list it points at, and an entry pointing back into the table ends
/// it. The last entry may point at frame data rather than a list, so its
/// total need not mean anything; no slice script selects it. An entry that
/// does not end within the art decoder's 64 records is `None`.
fn list_ticks(body: &[u8]) -> Option<Vec<Option<u16>>> {
    let (mut lists, mut table_end) = (Vec::new(), usize::MAX);
    while (lists.len() + 1) * 2 <= table_end {
        let at = usize::from(word(body, lists.len() * 2)?);
        if at < (lists.len() + 1) * 2 {
            break;
        }
        table_end = table_end.min(at);
        lists.push(list_total(body, at));
    }
    Some(lists)
}

/// Records a list may hold, as `assets::sprites` decodes them.
const MAX_LIST_RECORDS: usize = 64;

fn list_total(body: &[u8], mut at: usize) -> Option<u16> {
    let mut total = 0;
    for _ in 0..MAX_LIST_RECORDS {
        if word(body, at)? >= 0x8000 {
            return Some(total);
        }
        total += u16::from(body[at]) + 1;
        at += 4;
    }
    (word(body, at)? >= 0x8000).then_some(total)
}

/// Whether the descriptor's movement base is the common `$6000` resource
/// (`$80:FAAF`: `$4000 + (mode & $70) << 8`), which scripted legs use.
pub(super) fn common_base(image: &[u8], descriptor: usize) -> bool {
    image
        .get(descriptor + 3)
        .is_some_and(|mode| mode & 0x70 == 0x20)
}

/// Both source load sites put the common resource at `$7F:6000`, and its
/// nine audited streams are as the legs expect: `$60`/`$68`/`$69` step
/// 1, 0, 1, ... (half a pixel a frame); `$70`/`$78`/`$79` one pixel every
/// frame; `$80`/`$88`/`$89` two.
pub(super) fn common_streams(image: &[u8]) -> bool {
    // Packed $09F037, base bank $98, resolves to $AB:F037 (European: packed
    // $0A0000 from bank $9A, $AE:8000); destination operand 2 is $7F:6000.
    let Some(source) = layout::offset(image, COMMON_SOURCE) else {
        return false;
    };
    for site in [0x98_817D, 0x98_8272] {
        let Some(at) = layout::at(image, site) else {
            return false;
        };
        let [_, _, bank, _] = at.to_le_bytes();
        let at = (at & 0x3F_FFFF) as usize;
        let Some(&[1, 1, 2, 0, low, high, increment]) = image.get(at..at + 7) else {
            return false;
        };
        let target = unpack_pointer([low, high, increment], bank)
            .ok()
            .map(|pointer| pointer.normalized().value() as usize);
        if target != Some(source) {
            return false;
        }
    }
    let Some(common) = decode(image.get(source..).unwrap_or(&[]), COMMON_SIZE)
        .ok()
        .filter(|packet| packet.data.len() == COMMON_SIZE)
        .map(|packet| packet.data)
    else {
        return false;
    };
    let streams: [(usize, u16, u16, u16, u16); 9] = [
        (0x60, 0x6D18, 0, 0x6D18, 1),
        (0x68, 0, 0x6FC8, 0x6FC8, 1),
        (0x69, 0, 0x6FD4, 0x6FD4, 0xFFFF),
        (0x70, 0x7014, 0, 0x7014, 1),
        (0x78, 0, 0x71F0, 0x71F0, 1),
        (0x79, 0, 0x71F8, 0x71F8, 0xFFFF),
        (0x80, 0x7230, 0, 0x7230, 2),
        (0x88, 0, 0x7418, 0x7418, 2),
        (0x89, 0, 0x7420, 0x7420, 0xFFFE),
    ];
    streams
        .into_iter()
        .all(|(selector, x, y, pointer, velocity)| {
            let at = usize::from(pointer) - 0x6000 + 2;
            let (half, whole) = (
                [0, velocity, 0, 0, 0xFFFF, x.max(y) + 2],
                [1, velocity, 0xFFFF, x.max(y) + 2, 0, 0],
            );
            let records = if selector & 0xF0 == 0x60 {
                &half[..]
            } else {
                &whole[..4]
            };
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
        // Interaction, the collision callback, which sets +$04 bit $0200
        // only, the re-entry record at `$0600..$060F` (`$80:8B35`), sprite
        // priority (`BA`) and the art pointer (`D8`).
        Some(&[2, 0x19 | 0x21 | 0x65 | 0xBA | 0xD8, _]) => true,
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
            Some([1, 1, 1, 32, 31, 16].map(Some).to_vec())
        );
        // Durations of 255 do not wrap: sixty-four records are 16384 ticks.
        let long = list_ticks(&body([d, d, d, &[255; 64], d, d])).unwrap();
        assert_eq!(long[3], Some(16384));
        // An empty list is zero ticks; any word with bit 15 set ends a list.
        assert_eq!(list_ticks(&body([d, &[], d, d, d, d])).unwrap()[1], Some(0));
        let mut control = body([&[3, 3], d, d, d, d, d]);
        control[17] = 0x80;
        assert_eq!(list_ticks(&control).unwrap()[0], Some(4));
        // Up to 64 records, as the art decoder reads them; 65 are no list,
        // and the other lists still are.
        let long = list_ticks(&body([d, d, d, &[1; 64], d, d])).unwrap();
        assert_eq!(long[3], Some(128));
        let endless = list_ticks(&body([d, d, d, &[0; 65], d, d])).unwrap();
        assert_eq!((endless[3], endless[4]), (None, Some(1)));
        // A pointer back into the table ends it: none, or a zero pointer.
        let mut into = body([d; 6]);
        into[4..6].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(list_ticks(&into).map(|lists| lists.len()), Some(2));
        assert_eq!(list_ticks(&[0, 0, 0xFF, 0xFF]), Some(vec![]));
        assert_eq!(list_ticks(&[]), None);
    }

    #[test]
    fn skipped_services_are_allowed_by_what_their_handlers_write() {
        for (bytes, allowed) in [
            ([2, 0x21, 0x94], true),
            ([2, 0x65, 0x24], true),
            ([2, 0x19, 0x0F], true),
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

    #[test]
    fn direct_player_resource_ticks_follow_the_resource_table() {
        let mut image = vec![0; 0x1_0000];
        image[0xA24F..0xA252].copy_from_slice(&[0x00, 0xC0, 0xC0]);
        image[0xC000..0xC004].copy_from_slice(&[4, 0, 10, 0]);
        image[0xC004..0xC00A].copy_from_slice(&[2, 0, 0, 0, 0xFF, 0xFF]);
        image[0xC00A..0xC010].copy_from_slice(&[4, 0, 0, 0, 0xFF, 0xFF]);
        assert_eq!(player_pose_ticks(&image, 0), Some(vec![Some(3), Some(5)]));
        assert_eq!(player_pose_ticks(&image, 1), None);
    }

    fn owned_rom() -> Option<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local/Tenchi Souzou (Japan).sfc");
        let bytes = std::fs::read(path).ok()?;
        Some(rom::Rom::load(&bytes).unwrap().image().to_vec())
    }

    #[test]
    fn frozen_return_selections_are_bound_to_their_exact_instruction_sites() {
        let player = 4;
        let mut image = vec![0xFF; player + FROZEN_RETURN_SOURCE.len() * 2 + 8];
        image[player..player + FROZEN_RETURN_SOURCE.len()].copy_from_slice(FROZEN_RETURN_SOURCE);
        for &(offset, _) in FROZEN_RETURN_SELECTIONS {
            assert!(frozen_return_selection(&image, player, player + offset));
        }
        let elsewhere = player + FROZEN_RETURN_SOURCE.len();
        image[elsewhere..elsewhere + 5].copy_from_slice(&FROZEN_RETURN_SOURCE[..5]);
        assert!(!frozen_return_selection(&image, player, elsewhere));
        image[player + 2] ^= 1;
        assert!(!frozen_return_selection(&image, player, player));
    }

    #[test]
    fn frozen_return_player_resources_have_the_source_pinned_lengths_and_streams() {
        let Some(image) = owned_rom() else {
            return;
        };
        let player = frozen_return_profile(&image).expect("frozen-return profile");
        assert_eq!(player, 0x08_AF72);
        assert!(frozen_return_streams(&image));
        assert_eq!(player_pose_ticks(&image, 1).unwrap()[0x17], Some(36));
        assert_eq!(player_pose_ticks(&image, 0).unwrap()[0x09], Some(16));
        assert_eq!(
            player_pose_ticks(&image, 0).unwrap()[..2],
            [Some(1), Some(1)]
        );
        for &(offset, _) in FROZEN_RETURN_SELECTIONS {
            assert!(frozen_return_selection(&image, player, player + offset));
        }

        let mut changed_stream = image.clone();
        changed_stream[player + 2] ^= 1;
        assert_eq!(frozen_return_profile(&changed_stream), None);

        let mut changed_guide = image.clone();
        let guide = layout::offset(&image, FROZEN_RETURN_HEADER).unwrap() + FROZEN_RETURN_GUIDE;
        changed_guide[guide + 2] ^= 1;
        assert_eq!(frozen_return_profile(&changed_guide), None);

        let mut changed_resource = image.clone();
        let table = layout::offset(&image, 0xA24F).unwrap();
        changed_resource[table] ^= 1;
        assert_eq!(frozen_return_profile(&changed_resource), None);
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
