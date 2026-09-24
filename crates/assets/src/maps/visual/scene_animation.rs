//! Any map's graphics and palette animation, from the service actors its
//! spawn list installs: compact records whose script is the graphics
//! service `$87:98EB` or the palette service `$87:98C2`, with a selector
//! byte. The selector names a table through `$9B:8000` (graphics) or
//! `$9A:8000` (palette), read as [`super::crysta_animation`] documents for
//! the town: `(count, source, destination, size, delay)` records, each
//! transfer held `delay + 1` ticks, ended by a zero count, looping.

use super::VisualMapError;
use crate::graphics::{self, Bgr555, Tile4bpp};
use crate::maps::actors::SpawnRecord;

const GRAPHICS_SERVICE: u32 = 0x87_98EB;
const PALETTE_SERVICE: u32 = 0x87_98C2;
/// The graphics service's selector lookup at `$9B:8000` (`$8D:93D8`): a
/// word with bit 15 names a table bank in `$8D:9407`'s three-byte entries
/// (`$9B:8000`, `$9C:8000`); offsets there count from the entry's base.
const GRAPHICS_LOOKUP: usize = 0x1B_8000;
const GRAPHICS_BANKS: usize = 0x0D_9407;
/// The palette service's lookup at `$9A:8000`; its offsets are absolute in
/// the bank (`$8D:9331`).
const PALETTE: (usize, usize) = (0x1A_8000, 0x1A_0000);
/// Records one table may hold before it is refused.
const RECORDS: usize = 256;
const BAD: VisualMapError = VisualMapError::Unsupported("unqualified scene animation table");

/// One destination's transfers over its period.
#[derive(Debug)]
struct Track<T> {
    first: usize,
    period: u64,
    /// (tick, values), ascending.
    events: Vec<(u64, Vec<T>)>,
}

impl<T> Track<T> {
    fn sample(&self, age: u64) -> Option<(u64, &[T])> {
        let at = age % self.period;
        self.events
            .iter()
            .rev()
            .find(|(tick, _)| *tick <= at)
            .or_else(|| (age >= self.period).then(|| self.events.last()).flatten())
            .map(|(tick, values)| (*tick, values.as_slice()))
    }
}

/// The animation a map's services play.
#[derive(Debug, Default)]
pub struct SceneAnimation {
    tiles: Vec<Track<Tile4bpp>>,
    colors: Vec<Track<Bgr555>>,
}

impl SceneAnimation {
    /// Decodes the tables of the services among `records`, a map's spawn
    /// list. A map without services has an empty animation.
    ///
    /// # Errors
    /// Rejects a table that leaves its bank, is not terminated, or whose
    /// graphics do not decode.
    pub fn from_records(image: &[u8], records: &[SpawnRecord]) -> Result<Self, VisualMapError> {
        let mut animation = Self::default();
        for record in records.iter().filter(|record| record.opcode() == 0xFB) {
            let Some(&selector) = record.bytes().get(1) else {
                continue;
            };
            match record.script() {
                Some(GRAPHICS_SERVICE) => animation.graphics(image, selector)?,
                Some(PALETTE_SERVICE) => animation.palette(image, selector)?,
                _ => {}
            }
        }
        Ok(animation)
    }

    fn graphics(&mut self, image: &[u8], selector: u8) -> Result<(), VisualMapError> {
        let first = within(image, GRAPHICS_LOOKUP + usize::from(selector) * 2, 2)?;
        let base = if first[1] & 0x80 == 0 {
            GRAPHICS_LOOKUP
        } else {
            let entry = within(
                image,
                GRAPHICS_BANKS + usize::from(u16::from_le_bytes([first[0], first[1] & 0x7F])),
                3,
            )?;
            usize::from(entry[2] & 0x3F) << 16
                | usize::from(u16::from_le_bytes([entry[0], entry[1]]))
        };
        let records = table(image, (base, base), selector, 8)?;
        let mut tick = 0;
        let mut tracks: Vec<Track<Tile4bpp>> = Vec::new();
        for bytes in records {
            let source = base + usize::from(u16::from_le_bytes([bytes[1], bytes[2]]));
            let destination = usize::from(u16::from_le_bytes([bytes[3], bytes[4]])) * 2 / 32;
            let size = usize::from(u16::from_le_bytes([bytes[5], bytes[6]]));
            for repetition in 0..usize::from(bytes[0]) {
                let payload = within(image, source + repetition * size, size)?;
                let values =
                    graphics::decode_tiles_4bpp(payload).map_err(VisualMapError::Graphics)?;
                push(&mut tracks, destination, tick, values);
                tick += u64::from(bytes[7]) + 1;
            }
        }
        finish(&mut self.tiles, tracks, tick);
        Ok(())
    }

    fn palette(&mut self, image: &[u8], selector: u8) -> Result<(), VisualMapError> {
        let records = table(image, PALETTE, selector, 6)?;
        let mut tick = 0;
        let mut tracks: Vec<Track<Bgr555>> = Vec::new();
        for bytes in records {
            let source = PALETTE.1 + usize::from(u16::from_le_bytes([bytes[1], bytes[2]]));
            let size = usize::from(bytes[4]) + 1;
            for repetition in 0..usize::from(bytes[0]) {
                let payload = within(image, source + repetition * size, size)?;
                let values = payload
                    .chunks_exact(2)
                    .map(|b| Bgr555::new(u16::from_le_bytes([b[0], b[1]])))
                    .collect();
                push(&mut tracks, usize::from(bytes[3]), tick, values);
                tick += u64::from(bytes[5]) + 1;
            }
        }
        finish(&mut self.colors, tracks, tick);
        Ok(())
    }

    /// Whether the map animates nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty() && self.colors.is_empty()
    }

    /// Every tile index a transfer writes.
    pub fn tiles(&self) -> impl Iterator<Item = usize> + '_ {
        self.tiles.iter().flat_map(|track| {
            track.first..track.first + track.events.first().map_or(0, |e| e.1.len())
        })
    }

    /// Every palette index a transfer writes.
    pub fn colors(&self) -> impl Iterator<Item = usize> + '_ {
        self.colors.iter().flat_map(|track| {
            track.first..track.first + track.events.first().map_or(0, |e| e.1.len())
        })
    }

    /// Each destination's current transfer, as the tick it started; equal
    /// keys mean equal contents.
    #[must_use]
    pub fn phase_key(&self, age: u64) -> Vec<Option<u64>> {
        let tiles = self
            .tiles
            .iter()
            .map(|track| track.sample(age).map(|s| s.0));
        let colors = self
            .colors
            .iter()
            .map(|track| track.sample(age).map(|s| s.0));
        tiles.chain(colors).collect()
    }

    /// Writes the transfers in force at `age` over the base tiles and colours.
    /// Destinations past the slices' ends are left out.
    pub fn apply(&self, age: u64, tiles: &mut [Tile4bpp], palette: &mut [Bgr555]) {
        for track in &self.tiles {
            if let Some((_, values)) = track.sample(age) {
                if let Some(slot) = tiles.get_mut(track.first..track.first + values.len()) {
                    slot.copy_from_slice(values);
                }
            }
        }
        for track in &self.colors {
            if let Some((_, values)) = track.sample(age) {
                if let Some(slot) = palette.get_mut(track.first..track.first + values.len()) {
                    slot.copy_from_slice(values);
                }
            }
        }
    }
}

/// The records of a selector's table, up to its zero count.
fn table(
    image: &[u8],
    (lookup, base): (usize, usize),
    selector: u8,
    stride: usize,
) -> Result<Vec<&[u8]>, VisualMapError> {
    let offset = within(image, lookup + usize::from(selector) * 2, 2)?;
    let start = base + usize::from(u16::from_le_bytes([offset[0], offset[1]]));
    let mut records = Vec::new();
    for index in 0..RECORDS {
        let record = within(image, start + index * stride, stride)?;
        if record[0] == 0 {
            return if records.is_empty() {
                Err(BAD)
            } else {
                Ok(records)
            };
        }
        records.push(record);
    }
    Err(BAD)
}

/// Bytes at a normalized offset.
fn within(image: &[u8], at: usize, size: usize) -> Result<&[u8], VisualMapError> {
    image.get(at..at + size).ok_or(BAD)
}

fn push<T>(tracks: &mut Vec<Track<T>>, first: usize, tick: u64, values: Vec<T>) {
    match tracks.iter_mut().find(|track| track.first == first) {
        Some(track) => track.events.push((tick, values)),
        None => tracks.push(Track {
            first,
            period: 0,
            events: vec![(tick, values)],
        }),
    }
}

/// Sets each table's tracks to its period and keeps them.
fn finish<T>(all: &mut Vec<Track<T>>, tracks: Vec<Track<T>>, period: u64) {
    all.extend(tracks.into_iter().map(|track| Track {
        period: period.max(1),
        ..track
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A palette table for selector 2 at `$9A:9000`: color 40 cycles
    /// through two values, held 3 and 1 ticks.
    fn image() -> Vec<u8> {
        let mut image = vec![0; 0x1C_0000];
        image[0x1A_8004..0x1A_8006].copy_from_slice(&0x9000u16.to_le_bytes());
        image[0x1A_9000..0x1A_900C]
            .copy_from_slice(&[1, 0x00, 0xA0, 40, 1, 2, 1, 0x02, 0xA0, 40, 1, 0]);
        image[0x1A_A000..0x1A_A004].copy_from_slice(&[0x1F, 0x00, 0xE0, 0x03]);
        image
    }

    #[test]
    fn a_palette_table_cycles_by_its_delays() {
        let image = image();
        let mut animation = SceneAnimation::default();
        animation.palette(&image, 2).unwrap();
        assert_eq!(animation.colors().collect::<Vec<_>>(), [40]);
        let color = |age| {
            let mut palette = [Bgr555::new(0); 64];
            animation.apply(age, &mut [], &mut palette);
            palette[40]
        };
        assert_eq!(color(0), Bgr555::new(0x001F));
        assert_eq!(color(2), Bgr555::new(0x001F));
        assert_eq!(color(3), Bgr555::new(0x03E0));
        assert_eq!(color(4), Bgr555::new(0x001F), "a period of four ticks");
        assert_eq!(animation.phase_key(1), animation.phase_key(5));
    }

    #[test]
    fn an_unterminated_or_empty_table_is_refused() {
        let mut image = image();
        image[0x1A_9000] = 0;
        assert!(
            SceneAnimation::default().palette(&image, 2).is_err(),
            "empty"
        );
        let mut image = vec![1; 0x1A_9100];
        image[0x1A_8004..0x1A_8006].copy_from_slice(&0x9000u16.to_le_bytes());
        assert!(
            SceneAnimation::default().palette(&image, 2).is_err(),
            "unterminated"
        );
    }
}
