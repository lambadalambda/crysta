//! Drawing the desk's Records screen ([notes](../../../docs/records-screen.md)):
//! mode 1, back to front the backdrop, BG1's scroll, the rollers and the
//! cursor, the title, then BG3's text, levels and times. The screen is 256
//! pixels wide; a wide view centres it on the backdrop.

use crate::frame::{rgb, signed, Canvas, VIEW_HEIGHT};
use assets::graphics::Bgr555;
use assets::labels::Glyph;
use assets::records::{self, RecordsArt, Slot};
use assets::shop_display::ShopArt;
use assets::sprites::{SpriteFrame, SpritePixel};
use assets::text::DialoguePage;
use crysta_runtime::records::{Page, Text};
use crysta_runtime::save::SaveSlot;
use crysta_runtime::sram::Sram;

/// The screen's width.
const WIDTH: usize = 256;
/// The rollers' actors, and the cursor's x and its first y (`$87:84D4`:
/// 104, then 16 an entry).
const ROLLERS: [(i32, i32); 2] = [(128, 80), (128, 228)];
const CURSOR: (i32, i32) = (24, 104);
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
    let mut plane = Plane::new(canvas, art.bg_colours[0], 0);
    draw_bg1(&mut plane, art, &art.bg_map);
    draw_rollers(&mut plane, art);
    draw_cursor(&mut plane, art, page.cursor.min(2));
    if page.title {
        draw_title(&mut plane, art, &art.title);
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
        draw_text(&mut plane, (image, panel), text, *glyphs);
    }
    for (row, slot) in stats {
        draw_stats(&mut plane, panel, (slot, row), None);
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

/// The 256-pixel screen on the canvas: its left edge, and the first line
/// the BG layers show on (an HDMA split hides them above it).
pub(crate) struct Plane<'a> {
    pub(crate) canvas: &'a mut Canvas,
    pub(crate) left: i32,
    pub(crate) bg_top: i32,
}

impl Plane<'_> {
    /// The screen on `canvas`, centred on a wide view, its backdrop filled.
    pub(crate) fn new(canvas: &mut Canvas, backdrop: Bgr555, bg_top: i32) -> Plane<'_> {
        canvas.pixels.fill(rgb(backdrop));
        let left = signed(canvas.width.saturating_sub(WIDTH) / 2);
        Plane {
            canvas,
            left,
            bg_top,
        }
    }

    /// A sprite pixel.
    pub(crate) fn obj(&mut self, (x, y): (i32, i32), colour: Bgr555) {
        if (0..signed(WIDTH)).contains(&x) && (0..signed(VIEW_HEIGHT)).contains(&y) {
            self.canvas.set((self.left + x, y), rgb(colour));
        }
    }

    /// A BG pixel, above the split hidden.
    pub(crate) fn bg(&mut self, (x, y): (i32, i32), colour: Bgr555) {
        if y >= self.bg_top {
            self.obj((x, y), colour);
        }
    }
}

/// BG1: 4bpp tiles, palettes from CGRAM `$20`.
pub(crate) fn draw_bg1(plane: &mut Plane<'_>, art: &RecordsArt, map: &[u16]) {
    for (index, &entry) in map.iter().enumerate() {
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
                plane.bg((column + signed(x), row + signed(y)), colour);
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

/// The rollers, and the cursor on `entry` (the y table `$87:84D4`).
pub(crate) fn draw_rollers(plane: &mut Plane<'_>, art: &RecordsArt) {
    for actor in ROLLERS {
        draw_actor(plane, art, &art.roller, actor, ROLLER_PALETTE);
    }
}

/// The cursor on entry `entry` (slots 1–3, then the Restart screen's).
pub(crate) fn draw_cursor(plane: &mut Plane<'_>, art: &RecordsArt, entry: u8) {
    let y = CURSOR.1 + 16 * i32::from(entry);
    draw_actor(plane, art, &art.cursor, (CURSOR.0, y), CURSOR_PALETTE);
}

/// An actor's frame at `origin` in `palette`; tiles from OBJ `$100`.
fn draw_actor(
    plane: &mut Plane<'_>,
    art: &RecordsArt,
    frame: &SpriteFrame,
    origin: (i32, i32),
    palette: usize,
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
                plane.obj((origin.0 + i32::from(dx), origin.1 + i32::from(dy)), colour);
            }
        }
    }
}

/// A title's 16×16 glyphs, colours 1 and 2 of OBJ palette 4.
pub(crate) fn draw_title(plane: &mut Plane<'_>, art: &RecordsArt, title: &[Glyph]) {
    let (x0, pitch, top) = TITLE;
    for (glyph, x) in title.iter().zip((0..).map(|i| x0 + pitch * i)) {
        for (index, &colour) in glyph.iter().enumerate() {
            if let Some(colour) = (colour != 0)
                .then(|| obj_colour(art, CURSOR_PALETTE, colour))
                .flatten()
            {
                plane.obj((x + signed(index % 16), top + signed(index / 16)), colour);
            }
        }
    }
}

/// A text page's first `glyphs` glyphs in BG3 palette 0, colour 0 clear.
pub(crate) fn draw_text(
    plane: &mut Plane<'_>,
    (image, panel): (&[u8], &ShopArt),
    page: &DialoguePage,
    glyphs: usize,
) {
    let width = usize::from(page.width());
    for (index, &colour) in page.typed(image, glyphs).iter().enumerate() {
        if let Some(&colour) = (colour != 0)
            .then(|| panel.panel_palette.get(usize::from(colour)))
            .flatten()
        {
            plane.bg(
                (
                    TEXT.0 + signed(index % width),
                    TEXT.1 + signed(index / width),
                ),
                colour,
            );
        }
    }
}

/// BG3 panel tiles at (column, row, entry), by the entries' palettes;
/// `colour_3` stands in for colour 3 of palette 2 (CGRAM `$0B`).
pub(crate) fn draw_panel(
    plane: &mut Plane<'_>,
    panel: &ShopArt,
    tiles: &[(u8, u8, u16)],
    colour_3: Option<Bgr555>,
) {
    for &(column, row, entry) in tiles {
        let Some(tile) = panel.panel.get(usize::from(entry & 0x3FF)) else {
            continue;
        };
        let palette = usize::from(entry >> 10 & 7) * 4;
        for (index, &colour) in tile.iter().enumerate().filter(|&(_, &colour)| colour != 0) {
            let colour = match (palette + usize::from(colour), colour_3) {
                (0x0B, Some(stand_in)) => Some(stand_in),
                (at, _) => panel.panel_palette.get(at).copied(),
            };
            if let Some(colour) = colour {
                let at = (
                    i32::from(column) * 8 + signed(index % 8),
                    i32::from(row) * 8 + signed(index / 8),
                );
                plane.bg(at, colour);
            }
        }
    }
}

/// A slot's level and time on its rows; `colour_3` as for [`draw_panel`].
pub(crate) fn draw_stats(
    plane: &mut Plane<'_>,
    panel: &ShopArt,
    (slot, row): (&SaveSlot, u8),
    colour_3: Option<Bgr555>,
) {
    let tiles = records::stats_tiles(slot.level(), slot.seconds(), row);
    draw_panel(plane, panel, &tiles, colour_3);
}
