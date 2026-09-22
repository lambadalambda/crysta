//! Bounded map $000A free-roam graphics/palette animation, not a PPU emulator.
//!
//! The caller authenticates the headerless Japanese ROM. Four scene services at
//! $83:8A73..8A86 select tables via $9B:8000 (graphics) and $9A:8000 (palette).
//! The source consumers are $8D:940D/9470 and $8D:9331/93B2; COP $95/$93
//! advance repetitions and records. $80:C70D/C72C decrement before checking
//! negative, so a stored delay `d` holds a transfer for `d + 1` scheduler ticks.
//! A zero-count sentinel restarts in the same scheduler visit, without a gap.
//!
//! Age zero is the first transfer of each service, not map entry or a native
//! video-frame timestamp. Pause, DMA congestion, fade/color math and event-branch
//! service replacement are outside this free-roam model. No captured assets are
//! used. Static backgrounds and backdrop color zero are unchanged.

use super::VisualMapError;
use crate::graphics::{self, Bgr555, Tile4bpp};
use std::ops::Range;

const SERVICES: [(usize, [u8; 5]); 4] = [
    (0x38a73, [0xfb, 0, 0xe8, 0x98, 0x87]),
    (0x38a78, [0xfb, 1, 0xe8, 0x98, 0x87]),
    (0x38a7d, [0xfb, 0x23, 0xbf, 0x98, 0x87]),
    (0x38a82, [0xfb, 6, 0xbf, 0x98, 0x87]),
];
const BAD: VisualMapError =
    VisualMapError::Unsupported("unqualified Crysta animation source or extent");

#[derive(Debug)]
struct Event<T> {
    at: u64,
    values: Vec<T>,
}

#[derive(Debug)]
struct Destination<T> {
    index: usize,
    period: u64,
    events: Vec<Event<T>>,
}
impl<T> Destination<T> {
    fn sample(&self, age: u64) -> Option<&Event<T>> {
        self.events
            .iter()
            .rev()
            .find(|e| e.at <= age % self.period)
            .or_else(|| (age >= self.period).then(|| self.events.last()).flatten())
    }
}

/// Immutable, source-decoded animation transfers for map $000A's free-roam scene.
///
/// Graphics writes are partial: selector 1 updates only four of its sixteen
/// tiles at a time. Sampling retains the latest write to *every* destination,
/// including writes from the preceding cycle. Work is bounded independently of
/// age; no elapsed-tick replay or full-map pixel compilation occurs here.
#[derive(Debug)]
pub struct CrystaAnimation {
    graphics: Vec<Destination<Tile4bpp>>,
    colors: Vec<Destination<Bgr555>>,
    periods: [u64; 4],
}

impl CrystaAnimation {
    /// Decodes only the four qualified services and their bounded tables.
    ///
    /// The caller must authenticate the Japanese ROM and select this model only
    /// for map $000A free roam. Checks source scene linkage, service bytes,
    /// selector targets, record shapes, bank extents, sentinels and periods; this
    /// is not a general spawn-list interpreter or whole-ROM authentication.
    ///
    /// # Errors
    /// Rejects missing bytes, changed linkage and unsupported transfer shapes.
    pub fn from_rom(image: &[u8]) -> Result<Self, VisualMapError> {
        if word(image, 0x28014)? != 0 || word(image, 0x38014)? != 0x89a7 {
            return Err(BAD);
        }
        for (at, bytes) in SERVICES {
            if get(image, at, 5)? != bytes {
                return Err(BAD);
            }
        }
        let mut graphics = Vec::new();
        let mut colors = Vec::new();
        let mut periods = [0; 4];
        for (i, &(table, records, expected_period)) in [
            (0x1b_807e, 12, 528),
            (0x1b_84df, 64, 64),
            (0x1a_97a1, 6, 48),
            (0x1a_8788, 7, 42),
        ]
        .iter()
        .enumerate()
        {
            let palette = i >= 2;
            let bank = if palette { 0x1a_0000 } else { 0x1b_8000 };
            let selector = usize::from(SERVICES[i].1[1]);
            let lookup = if palette { 0x1a_8000 } else { bank };
            if bank + usize::from(word(image, lookup + selector * 2)?) != table {
                return Err(BAD);
            }
            let stride = if palette { 6 } else { 8 };
            let mut age = 0;
            for record in 0..records {
                let bytes = get(image, table + record * stride, stride)?;
                let count = usize::from(bytes[0]);
                let source = bank + usize::from(u16::from_le_bytes([bytes[1], bytes[2]]));
                let (destination, size, delay) = if palette {
                    (usize::from(bytes[3]), usize::from(bytes[4]) + 1, bytes[5])
                } else {
                    (
                        usize::from(u16::from_le_bytes([bytes[3], bytes[4]])) * 2,
                        usize::from(u16::from_le_bytes([bytes[5], bytes[6]])),
                        bytes[7],
                    )
                };
                let valid = match i {
                    0 => {
                        count == 8
                            && destination == 0x120
                            && size == 128
                            && (2..=7).contains(&delay)
                    }
                    1 => {
                        count == 1
                            && destination == 0x3e00 + record % 4 * 128
                            && size == 128
                            && delay == 0
                    }
                    2 => count == 1 && destination == 96 && size == 32 && delay == 7,
                    3 => count == 1 && destination == 112 && size == 16 && delay == 5,
                    _ => unreachable!(),
                };
                if !valid || source < (bank | 0x8000) || source + count * size > (bank | 0xffff) + 1
                {
                    return Err(BAD);
                }
                for repetition in 0..count {
                    let payload = get(image, source + repetition * size, size)?;
                    if palette {
                        let values = payload
                            .chunks_exact(2)
                            .map(|b| Bgr555::new(u16::from_le_bytes([b[0], b[1]])))
                            .collect();
                        add(&mut colors, destination, expected_period, age, values);
                    } else {
                        let values = graphics::decode_tiles_4bpp(payload)
                            .map_err(VisualMapError::Graphics)?;
                        add(
                            &mut graphics,
                            destination / 32,
                            expected_period,
                            age,
                            values,
                        );
                    }
                    age += u64::from(delay) + 1;
                }
            }
            if get(image, table + records * stride, 1)? != [0] || age != expected_period {
                return Err(BAD);
            }
            periods[i] = age;
        }
        Ok(Self {
            graphics,
            colors,
            periods,
        })
    }

    /// Scheduler-tick periods: graphics selectors 0/1, palette selectors $23/$06.
    #[must_use]
    pub const fn periods(&self) -> [u64; 4] {
        self.periods
    }

    /// Joint period (LCM of the four qualified source periods), in scheduler ticks.
    /// Repeats are guaranteed at ages >= [`Self::repeat_from`], not during startup.
    #[must_use]
    pub const fn period(&self) -> u64 {
        14_784
    }

    /// Earliest age with all destinations initialized. Before tick 3, selector 1
    /// leaves not-yet-transferred destinations at the static base values.
    #[must_use]
    pub const fn repeat_from(&self) -> u64 {
        3
    }

    /// Tile indices potentially changed; use to precompute a pixel dependency mask.
    #[must_use]
    pub const fn affected_tiles(&self) -> [Range<usize>; 2] {
        [9..13, 496..512]
    }

    /// Palette indices potentially changed. In particular, excludes backdrop zero.
    #[must_use]
    pub const fn affected_colors(&self) -> Range<usize> {
        96..120
    }

    /// Per-destination transfer identities for invalidating only changed spans.
    /// Slots 0..5 are tiles starting at 9, 496, 500, 504, 508; slots 5..7 are
    /// palettes starting at 96 and 112. `None` means no transfer yet. Equal keys
    /// guarantee equal patches for this instance; different keys can still hold
    /// equal payloads. In particular slot 6 is the 42-tick river palette key.
    #[must_use]
    pub fn phase_key(&self, age: u64) -> [Option<u64>; 7] {
        std::array::from_fn(|i| {
            if i < 5 {
                self.graphics[i].sample(age).map(|e| e.at)
            } else {
                self.colors[i - 5].sample(age).map(|e| e.at)
            }
        })
    }

    /// Latest initialized tile spans at this age, as `(first_tile, decoded_tiles)`.
    /// Disjoint destination order; suitable for dirty-span/pixel updates.
    pub fn tile_updates(&self, age: u64) -> impl Iterator<Item = (usize, &[Tile4bpp])> {
        self.graphics
            .iter()
            .filter_map(move |d| d.sample(age).map(|e| (d.index, e.values.as_slice())))
    }

    /// Latest palette spans at this age, as `(first_color, raw_BGR555_colors)`.
    pub fn palette_updates(&self, age: u64) -> impl Iterator<Item = (usize, &[Bgr555])> {
        self.colors
            .iter()
            .filter_map(move |d| d.sample(age).map(|e| (d.index, e.values.as_slice())))
    }

    /// Applies only initialized animation spans, leaving everything else alone.
    ///
    /// Start with static base tiles/palette. For random seeks back into startup
    /// ages 0..2, reset to that base first; all later ages fully define affected
    /// spans, so monotonic updates and arbitrary seeks >= 3 need no reset.
    ///
    /// # Errors
    /// Rejects destinations shorter than 512 tiles / 120 colors before mutation.
    pub fn apply(
        &self,
        age: u64,
        tiles: &mut [Tile4bpp],
        palette: &mut [Bgr555],
    ) -> Result<(), VisualMapError> {
        if tiles.len() < 512 || palette.len() < 120 {
            return Err(BAD);
        }
        for (start, values) in self.tile_updates(age) {
            tiles[start..start + values.len()].copy_from_slice(values);
        }
        for (start, values) in self.palette_updates(age) {
            palette[start..start + values.len()].copy_from_slice(values);
        }
        Ok(())
    }
}

fn get(image: &[u8], at: usize, size: usize) -> Result<&[u8], VisualMapError> {
    image.get(at..at + size).ok_or(BAD)
}
fn word(image: &[u8], at: usize) -> Result<u16, VisualMapError> {
    let b = get(image, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn add<T>(
    destinations: &mut Vec<Destination<T>>,
    index: usize,
    period: u64,
    at: u64,
    values: Vec<T>,
) {
    if let Some(d) = destinations.iter_mut().find(|d| d.index == index) {
        d.events.push(Event { at, values });
    } else {
        destinations.push(Destination {
            index,
            period,
            events: vec![Event { at, values }],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::cast_possible_truncation)] // bounded synthetic byte/word fields
    fn fixture() -> Vec<u8> {
        let mut r = vec![0; 0x1c_0000];
        r[0x38014..0x38016].copy_from_slice(&0x89a7u16.to_le_bytes());
        for (at, bytes) in SERVICES {
            r[at..at + 5].copy_from_slice(&bytes);
        }
        for (selector, table) in [(0, 0x7eu16), (1, 0x4df)] {
            r[0x1b_8000 + selector * 2..0x1b_8002 + selector * 2]
                .copy_from_slice(&table.to_le_bytes());
        }
        for (i, d) in [7, 6, 5, 4, 3, 2, 2, 3, 4, 5, 6, 7].into_iter().enumerate() {
            let at = 0x1b_807e + i * 8;
            r[at..at + 8].copy_from_slice(&[8, 0xdf, 0, 0x90, 0, 0x80, 0, d]);
        }
        for i in 0..64 {
            let at = 0x1b_84df + i * 8;
            let src = (0x6e0 + i * 128) as u16;
            r[at..at + 8].copy_from_slice(&[
                1,
                src as u8,
                (src >> 8) as u8,
                (i % 4 * 64) as u8,
                0x1f,
                0x80,
                0,
                0,
            ]);
            // Distinguish every frame in its first decoded tile.
            r[0x1b_8000 + usize::from(src)] = i as u8;
        }
        for (selector, table, count, dest, size, delay) in [
            (0x23, 0x97a1u16, 6, 96, 32, 7),
            (6, 0x8788u16, 7, 112, 16, 5),
        ] {
            r[0x1a_8000 + selector * 2..0x1a_8002 + selector * 2]
                .copy_from_slice(&table.to_le_bytes());
            for i in 0..count {
                let at = 0x1a_0000 + usize::from(table) + i * 6;
                let src = 0xa000u16 + (selector * 256 + i * size) as u16;
                r[at..at + 6].copy_from_slice(&[
                    1,
                    src as u8,
                    (src >> 8) as u8,
                    dest,
                    (size - 1) as u8,
                    delay,
                ]);
                r[0x1a_0000 + usize::from(src)] = (i + 1) as u8;
            }
        }
        r
    }

    #[test]
    fn phase_keys_are_seekable_and_identify_each_destination() {
        let a = CrystaAnimation::from_rom(&fixture()).unwrap();
        assert_eq!(
            a.phase_key(0),
            [Some(0), Some(0), None, None, None, Some(0), Some(0)]
        );
        assert_ne!(a.phase_key(0), a.phase_key(64));
        assert_eq!(a.phase_key(1)[6], a.phase_key(4)[6]);
        assert_ne!(a.phase_key(4)[6], a.phase_key(6)[6]);
        for age in 3..a.period() {
            assert_eq!(a.phase_key(age), a.phase_key(age + a.period()));
        }
    }

    #[test]
    fn durations_repetitions_and_periods() {
        let a = CrystaAnimation::from_rom(&fixture()).unwrap();
        assert_eq!(a.periods(), [528, 64, 48, 42]);
        assert_eq!(a.period(), 14784);
        assert_eq!(a.repeat_from(), 3);
        let first = &a.graphics[0];
        assert_eq!(first.sample(0).unwrap().at, 0);
        assert_eq!(first.sample(7).unwrap().at, 0);
        assert_eq!(first.sample(8).unwrap().at, 8);
        assert_eq!(first.sample(63).unwrap().at, 56);
        assert_eq!(first.sample(64).unwrap().at, 64);
        assert_eq!(first.sample(70).unwrap().at, 64);
        assert_eq!(first.sample(71).unwrap().at, 71);
        assert_eq!(first.sample(528).unwrap().at, 0);
        assert_eq!(a.palette_updates(7).next().unwrap().1[0].raw(), 1);
        assert_eq!(a.palette_updates(8).next().unwrap().1[0].raw(), 2);
        assert_eq!(a.palette_updates(48).next().unwrap().1[0].raw(), 1);
    }

    #[test]
    fn ordered_partial_writes_keep_previous_destinations_and_wrap() {
        let a = CrystaAnimation::from_rom(&fixture()).unwrap();
        assert_eq!(a.tile_updates(0).count(), 2); // selector0 + first selector1 destination
        assert_eq!(a.tile_updates(2).count(), 4);
        assert_eq!(a.tile_updates(3).count(), 5);
        assert_eq!(a.graphics[1].sample(3).unwrap().at, 0);
        assert_eq!(a.graphics[2].sample(4).unwrap().at, 1);
        assert_eq!(a.graphics[1].sample(64).unwrap().at, 0);
        assert_eq!(a.graphics[2].sample(64).unwrap().at, 61);
        assert_eq!(a.graphics[3].sample(65).unwrap().at, 62);
        assert_eq!(a.graphics[4].sample(66).unwrap().at, 63);
        for age in [3, 4, 47, 63, 64, 527, 528, 14784] {
            assert_eq!(
                a.tile_updates(age).collect::<Vec<_>>(),
                a.tile_updates(age + a.period()).collect::<Vec<_>>()
            );
            assert_eq!(
                a.palette_updates(age).collect::<Vec<_>>(),
                a.palette_updates(age + a.period()).collect::<Vec<_>>()
            );
        }
        assert_eq!(
            a.tile_updates(u64::MAX).collect::<Vec<_>>(),
            a.tile_updates(a.period() + u64::MAX % a.period())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn apply_only_touches_declared_spans_and_refuses_short_destinations_atomically() {
        let a = CrystaAnimation::from_rom(&fixture()).unwrap();
        let tile = Tile4bpp::decode(&[0xff; 32]).unwrap();
        let mut tiles = vec![tile; 768];
        let mut colors = [Bgr555::new(0x7fff); 128];
        a.apply(0, &mut tiles, &mut colors).unwrap();
        assert_eq!(tiles[500], tile); // not transferred yet
        a.apply(10, &mut tiles, &mut colors).unwrap();
        for (i, value) in tiles.iter().enumerate() {
            if !a.affected_tiles().iter().any(|r| r.contains(&i)) {
                assert_eq!(*value, tile);
            }
        }
        for (i, value) in colors.iter().enumerate() {
            if !a.affected_colors().contains(&i) {
                assert_eq!(value.raw(), 0x7fff);
            }
        }
        let before = tiles.clone();
        assert!(a.apply(1, &mut tiles, &mut colors[..119]).is_err());
        assert_eq!(tiles, before);
    }

    #[test]
    fn rejects_changed_services_selectors_shapes_bounds_and_truncation() {
        let good = fixture();
        for at in [
            0x28014, 0x38014, 0x38a73, 0x38a79, 0x38a7f, 0x38a82, 0x1b_8000, 0x1a_8046, 0x1b_807e,
            0x1b_8081, 0x1b_8083, 0x1b_8085, 0x1b_80de, 0x1b_86df, 0x1a_97a4, 0x1a_97a5,
        ] {
            let mut r = good.clone();
            r[at] ^= 0x80;
            assert!(CrystaAnimation::from_rom(&r).is_err(), "accepted {at:x}");
        }
        let mut r = good.clone();
        r[0x1b_807f..0x1b_8081].copy_from_slice(&0x7fffu16.to_le_bytes());
        assert!(CrystaAnimation::from_rom(&r).is_err());
        for end in [0, 0x38015, 0x38a86, 0x1a_97a6, 0x1b_807f, 0x1b_86df] {
            assert!(CrystaAnimation::from_rom(&good[..end]).is_err());
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tools/crysta-animation-qualification/owned_rom.rs"]
mod owned_rom;
