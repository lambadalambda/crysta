//! Drawing the Restart file select ([notes](../../../docs/restart-screen.md)):
//! the Records screen's layers over map `$04`'s scroll, a BG3 border, and
//! an HDMA split that shows only the sprites above line 78.

use crate::frame::{dim, Canvas};
use crate::records::{
    draw_bg1, draw_cursor, draw_panel, draw_rollers, draw_stats, draw_text, draw_title, Plane,
};
use assets::records::{RecordsArt, Slot};
use assets::restart::{self, RestartArt};
use assets::shop_display::ShopArt;
use crysta_runtime::restart::View;
use crysta_runtime::save::SaveSlot;
use crysta_runtime::sram::Sram;

/// The first line BG1 and BG3 show on (`$87:9076`).
const SPLIT: i32 = 78;
/// The border (`$85:E716`): tile `$20`, palette 2, on columns 0 and 31 of
/// rows 0–27 and all of row 28. The setup puts the backdrop into palette
/// 2's colour 3 (CGRAM `$0B`), which the labels use too.
const BORDER: u16 = 0x2820;
/// The rows of slot n's level and time: 11 + 2n.
const SLOT_ROW: u8 = 11;

/// The screen's art, decoded on first use.
#[derive(Default)]
pub struct RestartArtCache {
    art: std::cell::OnceCell<Option<(RecordsArt, ShopArt, RestartArt)>>,
}

impl RestartArtCache {
    /// The art, or `None` when the ROM does not decode.
    pub fn get(&self, image: &[u8]) -> Option<&(RecordsArt, ShopArt, RestartArt)> {
        self.art
            .get_or_init(|| {
                let art = || -> Result<_, assets::shops::ShopError> {
                    Ok((
                        RecordsArt::from_rom(image)?,
                        ShopArt::from_rom(image)?,
                        RestartArt::from_rom(image)?,
                    ))
                };
                art()
                    .inspect_err(|error| eprintln!("Restart art unavailable: {error}"))
                    .ok()
            })
            .as_ref()
    }
}

/// The border's BG3 entries.
fn border() -> Vec<(u8, u8, u16)> {
    let sides = (0..28).flat_map(|row| [(0, row, BORDER), (31, row, BORDER)]);
    sides
        .chain((0..32).map(|column| (column, 28, BORDER)))
        .collect()
}

/// Draws the screen as `view` shows it, the slots from `sram`.
pub fn draw(
    canvas: &mut Canvas,
    image: &[u8],
    (art, panel, restart): &(RecordsArt, ShopArt, RestartArt),
    view: View,
    sram: &Sram,
) {
    let View::Screen(page) = view else {
        canvas.pixels.fill(0);
        return;
    };
    let backdrop = art.bg_colours[0];
    let mut plane = Plane::new(canvas, backdrop, SPLIT);
    draw_bg1(&mut plane, art, &restart.bg_map);
    // OAM order: the title, the cursors, then the rollers under them.
    draw_rollers(&mut plane, art);
    for entry in [page.second, page.cursor].into_iter().flatten() {
        draw_cursor(&mut plane, art, entry);
    }
    draw_title(&mut plane, art, &restart.title);
    let slots: Vec<Option<SaveSlot>> = (0..3).map(|n| sram.slot(n)).collect();
    let names = [0, 1, 2].map(|n| slots[n].as_ref().map(|slot| Slot { name: slot.name() }));
    let pages = restart::page_text(image, &names).unwrap_or_default();
    if let Some((first, rest)) = pages.split_first() {
        let lines = 16 * u16::from(page.lines);
        let typed = first
            .glyphs()
            .iter()
            .take_while(|glyph| glyph.position[1] < lines)
            .count();
        draw_text(&mut plane, (image, panel), first, typed);
        for text in rest.iter().take(usize::from(page.slots)) {
            draw_text(&mut plane, (image, panel), text, usize::MAX);
        }
    }
    for (n, slot) in (0u8..).zip(&slots).take(usize::from(page.slots)) {
        if let Some(slot) = slot {
            draw_stats(&mut plane, panel, (slot, SLOT_ROW + 2 * n), Some(backdrop));
        }
    }
    draw_panel(&mut plane, panel, &border(), Some(backdrop));
    dim(canvas, page.brightness);
}
