//! Drawing the desk's Records screen ([notes](../../../docs/records-screen.md)):
//! mode 1, back to front the backdrop, BG1's scroll, the rollers and the
//! cursor, the title, then BG3's text, levels and times. The screen is 256
//! pixels wide; a wide view centres it on the backdrop.

use crate::frame::{rgb, signed, Canvas, VIEW_HEIGHT};
use assets::graphics::Bgr555;
use assets::records::{self, RecordsArt, Slot};
use assets::shop_display::ShopArt;
use assets::sprites::{SpriteFrame, SpritePixel};
use assets::text::DialoguePage;
use crysta_runtime::records::{Page, Text};
use crysta_runtime::save::SaveSlot;
use crysta_runtime::sram::Sram;

/// The screen's width.
const WIDTH: usize = 256;
/// The rollers' actors, and the cursor's x and its y by slot (`$87:84D4`).
const ROLLERS: [(i32, i32); 2] = [(128, 80), (128, 228)];
const CURSOR: (i32, [i32; 3]) = (24, [104, 120, 136]);
/// The OBJ palettes the actors set: the rollers 6, the cursor and the
/// title 4.
const ROLLER_PALETTE: usize = 6;
const CURSOR_PALETTE: usize = 4;
/// The title's first glyph and pitch, and its top line.
const TITLE: (i32, i32, i32) = (100, 12, 25);
/// The text area's origin: tile column 3, row 11.
const TEXT: (i32, i32) = (24, 88);
/// The rows of a slot's level and time, and the current game's.
const SLOT_ROW: u8 = 11;
const CURRENT_ROW: u8 = 21;

/// The screen's art, decoded on first use.
#[derive(Default)]
pub struct RecordsArtCache {
    art: std::cell::OnceCell<Option<(RecordsArt, ShopArt)>>,
}

impl RecordsArtCache {
    /// The art, or `None` when the ROM does not decode.
    pub fn get(&self, image: &[u8]) -> Option<&(RecordsArt, ShopArt)> {
        self.art
            .get_or_init(|| {
                RecordsArt::from_rom(image)
                    .and_then(|art| Ok((art, ShopArt::from_rom(image)?)))
                    .inspect_err(|error| eprintln!("Records art unavailable: {error}"))
                    .ok()
            })
            .as_ref()
    }
}

/// What the screen shows of the slots and the current game.
pub struct Games<'a> {
    /// The SRAM's slots.
    pub sram: &'a Sram,
    /// The game being played.
    pub current: &'a SaveSlot,
}

/// Draws `page` over the whole canvas.
pub fn draw(
    canvas: &mut Canvas,
    image: &[u8],
    (art, panel): &(RecordsArt, ShopArt),
    page: Page,
    games: &Games<'_>,
) {
    let left = signed(canvas.width.saturating_sub(WIDTH) / 2);
    canvas.pixels.fill(rgb(art.bg_colours[0]));
    draw_bg1(canvas, art, left);
    for actor in ROLLERS {
        draw_actor(canvas, art, &art.roller, actor, ROLLER_PALETTE, left);
    }
    let cursor = (CURSOR.0, CURSOR.1[usize::from(page.cursor.min(2))]);
    draw_actor(canvas, art, &art.cursor, cursor, CURSOR_PALETTE, left);
    if page.title {
        draw_title(canvas, art, left);
    }
    let slots: Vec<Option<SaveSlot>> = (0..3).map(|n| games.sram.slot(n)).collect();
    let names = [0, 1, 2].map(|n| slots[n].as_ref().map(|slot| Slot { name: slot.name() }));
    let (pages, stats) = match page.text {
        Text::None => (Vec::new(), Vec::new()),
        Text::Entry => {
            let mut stats: Vec<(u8, &SaveSlot)> = slot_stats(&slots, 3);
            stats.push((CURRENT_ROW, games.current));
            let pages = records::entry_text(image, &names, games.current.name());
            (pages.map(typed_all).unwrap_or_default(), stats)
        }
        Text::Saved {
            slot,
            lines,
            message,
            slots: typed,
        } => {
            let mut pages = records::saved_text(image, slot, &names)
                .unwrap_or_default()
                .into_iter();
            let mut shown = Vec::new();
            if let Some(first) = pages.next() {
                // ` 1`–` 3`, then the message: typed in order.
                let glyphs = first.glyphs();
                let count = if message {
                    glyphs.len()
                } else {
                    let typed = 16 * u16::from(lines);
                    glyphs
                        .iter()
                        .take_while(|glyph| glyph.position[1] < typed)
                        .count()
                };
                shown.push((first, count));
                shown.extend(
                    pages
                        .take(usize::from(typed))
                        .map(|page| (page, usize::MAX)),
                );
            }
            (shown, slot_stats(&slots, typed))
        }
    };
    for (text, glyphs) in &pages {
        draw_text(canvas, image, panel, text, *glyphs, left);
    }
    for (row, slot) in stats {
        draw_stats(canvas, panel, slot, row, left);
    }
}

/// Every glyph of each page.
fn typed_all(pages: Vec<DialoguePage>) -> Vec<(DialoguePage, usize)> {
    pages.into_iter().map(|page| (page, usize::MAX)).collect()
}

/// The first `count` slots' level and time rows, for those with data.
fn slot_stats(slots: &[Option<SaveSlot>], count: u8) -> Vec<(u8, &SaveSlot)> {
    (0u8..)
        .zip(slots)
        .take(usize::from(count))
        .filter_map(|(n, slot)| slot.as_ref().map(|slot| (SLOT_ROW + 2 * n, slot)))
        .collect()
}

fn set(canvas: &mut Canvas, left: i32, (x, y): (i32, i32), colour: Bgr555) {
    if (0..signed(WIDTH)).contains(&x) && (0..signed(VIEW_HEIGHT)).contains(&y) {
        canvas.set((left + x, y), rgb(colour));
    }
}

/// BG1: 4bpp tiles, palettes from CGRAM `$20`.
fn draw_bg1(canvas: &mut Canvas, art: &RecordsArt, left: i32) {
    for (index, &entry) in art.bg_map.iter().enumerate() {
        let (column, row) = (signed(index % 32) * 8, signed(index / 32) * 8);
        let Some(tile) = art.bg_tiles.get(usize::from(entry & 0x3FF)) else {
            continue;
        };
        let palette = usize::from(entry >> 10 & 7) * 16;
        for (y, x) in (0..8).flat_map(|y| (0..8).map(move |x| (y, x))) {
            let sx = if entry & 0x4000 != 0 { 7 - x } else { x };
            let sy = if entry & 0x8000 != 0 { 7 - y } else { y };
            let colour = tile.pixel(sx, sy).unwrap_or(0);
            if let Some(&colour) = (colour != 0)
                .then(|| {
                    art.bg_colours
                        .get((palette + usize::from(colour)).checked_sub(0x20)?)
                })
                .flatten()
            {
                set(canvas, left, (column + signed(x), row + signed(y)), colour);
            }
        }
    }
}

/// An OBJ colour: palette `palette`, colour `colour`, from CGRAM `$B0`.
fn obj_colour(art: &RecordsArt, palette: usize, colour: u8) -> Option<Bgr555> {
    art.obj_colours
        .get((0x80 + palette * 16 + usize::from(colour)).checked_sub(0xB0)?)
        .copied()
}

/// An actor's frame at `(x, y)` in `palette`; tiles from OBJ `$100`.
fn draw_actor(
    canvas: &mut Canvas,
    art: &RecordsArt,
    frame: &SpriteFrame,
    origin: (i32, i32),
    palette: usize,
    left: i32,
) {
    let (first_x, top, right, bottom) = frame.bounds(false, false);
    for dy in top..bottom {
        for dx in first_x..right {
            let Ok(SpritePixel::Opaque { palette_index, .. }) =
                frame.sample(&art.obj_tiles, false, false, dx, dy)
            else {
                continue;
            };
            if let Some(colour) = obj_colour(art, palette, palette_index & 15) {
                let at = (origin.0 + i32::from(dx), origin.1 + i32::from(dy));
                set(canvas, left, at, colour);
            }
        }
    }
}

/// The title's 16×16 glyphs, colours 1 and 2 of OBJ palette 4.
fn draw_title(canvas: &mut Canvas, art: &RecordsArt, left: i32) {
    let (x0, pitch, top) = TITLE;
    for (glyph, x) in art.title.iter().zip((0..).map(|i| x0 + pitch * i)) {
        for (index, &colour) in glyph.iter().enumerate() {
            if let Some(colour) = (colour != 0)
                .then(|| obj_colour(art, CURSOR_PALETTE, colour))
                .flatten()
            {
                set(
                    canvas,
                    left,
                    (x + signed(index % 16), top + signed(index / 16)),
                    colour,
                );
            }
        }
    }
}

/// A text page's first `glyphs` glyphs in BG3 palette 0, colour 0 clear.
fn draw_text(
    canvas: &mut Canvas,
    image: &[u8],
    panel: &ShopArt,
    page: &DialoguePage,
    glyphs: usize,
    left: i32,
) {
    let width = usize::from(page.width());
    for (index, &colour) in page.typed(image, glyphs).iter().enumerate() {
        if let Some(&colour) = (colour != 0)
            .then(|| panel.panel_palette.get(usize::from(colour)))
            .flatten()
        {
            let at = (
                TEXT.0 + signed(index % width),
                TEXT.1 + signed(index / width),
            );
            set(canvas, left, at, colour);
        }
    }
}

/// A slot's level and time: BG3 panel tiles by their entries' palettes.
fn draw_stats(canvas: &mut Canvas, panel: &ShopArt, slot: &SaveSlot, row: u8, left: i32) {
    for (column, row, entry) in records::stats_tiles(slot.level(), slot.seconds(), row) {
        let Some(tile) = panel.panel.get(usize::from(entry & 0x3FF)) else {
            continue;
        };
        let palette = usize::from(entry >> 10 & 7) * 4;
        for (index, &colour) in tile.iter().enumerate() {
            if let Some(&colour) = (colour != 0)
                .then(|| panel.panel_palette.get(palette + usize::from(colour)))
                .flatten()
            {
                let at = (
                    i32::from(column) * 8 + signed(index % 8),
                    i32::from(row) * 8 + signed(index / 8),
                );
                set(canvas, left, at, colour);
            }
        }
    }
}
