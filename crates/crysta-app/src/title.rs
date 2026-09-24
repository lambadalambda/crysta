//! The area title shown on entering a map (`assets::labels`): its letters
//! type invisibly from the arrival's first lit frame, one a frame, then fly
//! in, hold and fly away, on screen coordinates. World maps show none
//! (`$0868` bit 7).

use crate::frame::{rgb, Canvas};
use assets::graphics::Bgr555;
use assets::labels::{area_title, label_palette, Glyph, TitleMotion};
use assets::maps::scripts::EventFlags;

/// A map and its entry flags, and their title.
type Entered = ((u16, Vec<u8>), Option<Vec<Glyph>>);

/// The titles' art, decoded once, and the title of the last map entered.
#[derive(Debug, Default)]
pub struct Titles {
    art: std::cell::OnceCell<Option<(TitleMotion, [Bgr555; 16])>>,
    title: Option<Entered>,
}

impl Titles {
    /// Draws `map`'s title, entered under `events`, `arrived` frames after
    /// the arrival's fade began.
    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        image: &[u8],
        (map, events): (u16, &[u8]),
        arrived: u32,
    ) {
        if crysta_runtime::WORLD_MAPS.contains(&map) {
            return;
        }
        let Some((motion, palette)) = self.art.get_or_init(|| {
            Some((
                TitleMotion::from_rom(image).ok()?,
                label_palette(image).ok()?,
            ))
        }) else {
            return;
        };
        if self
            .title
            .as_ref()
            .is_none_or(|(key, _)| key.0 != map || key.1 != events)
        {
            let glyphs = area_title(image, map, EventFlags::Bitmap(events))
                .ok()
                .flatten();
            self.title = Some(((map, events.to_vec()), glyphs));
        }
        let Some((_, Some(glyphs))) = &self.title else {
            return;
        };
        // The effect starts once every letter has typed, a frame each, from
        // the first lit frame (`arrived` 1).
        let t = i64::from(arrived) - 1 - (i64::try_from(glyphs.len()).unwrap_or(0) + 1);
        let left = i32::try_from(canvas.width.saturating_sub(256) / 2).unwrap_or(0);
        for (index, x, y) in motion.positions(glyphs.len(), t) {
            let glyph = &glyphs[index];
            for (row, py) in (0..16).zip(0_usize..) {
                for (column, px) in (0..16).zip(0_usize..) {
                    let colour = glyph[py * 16 + px];
                    if colour != 0 {
                        canvas.set(
                            (left + x + column, y + row),
                            rgb(palette[usize::from(colour)]),
                        );
                    }
                }
            }
        }
    }
}
