//! The text window as the game draws it on BG3 ([`WindowArt`]): the frame
//! one tile outside the content, the content shaded line by line, each
//! glyph in its palette, and the prompt on pages that wait for a press.
//! BG3 shows line `y + 1` on screen line `y`, so a window whose content
//! starts at tile row `r` starts on screen line `8r - 1`.

use crate::frame::{rgb, signed, Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use assets::graphics::Bgr555;
use assets::text::window::{WindowArt, PROMPT_TICKS};
use assets::text::{Acknowledgement, DialoguePage, Placement};

/// The standard window's content, in tiles: columns 2..30, rows 20..26
/// (`$C1`), or rows 4..10 above a player low on the screen (`$DA`). The
/// European windows hold four lines: rows 19..27 (base `$04C4`) and 4..12
/// (`$0104`).
const BOTTOM: (usize, usize) = (2, 20);
const EUROPEAN_BOTTOM: (usize, usize) = (2, 19);
const TOP: (usize, usize) = (2, 4);

/// Where a page's content starts on the canvas, in `image`'s revision.
#[must_use]
pub fn content_origin(
    image: &[u8],
    placement: Placement,
    player_screen_y: usize,
    view_width: usize,
) -> (i32, i32) {
    let (column, row) = match placement {
        Placement::AwayFromPlayer if player_screen_y >= VIEW_HEIGHT / 2 => TOP,
        Placement::Bottom | Placement::AwayFromPlayer => {
            assets::layout::per_revision(image, BOTTOM, EUROPEAN_BOTTOM)
        }
        Placement::Tile { column, row } => (usize::from(column), usize::from(row)),
    };
    let left = view_width.saturating_sub(CLASSIC_WIDTH) / 2;
    (
        i32::try_from(left + column * 8).unwrap_or(0),
        i32::try_from(row * 8).unwrap_or(0) - 1,
    )
}

/// Draws a page: `indexed` its typed pixels, `shown` the glyphs typed,
/// `tick` the frame count for the prompt.
pub fn draw_window(
    canvas: &mut Canvas,
    art: &WindowArt,
    page: &DialoguePage,
    (indexed, shown): (&[u8], usize),
    origin: (i32, i32),
    tick: u64,
) {
    let (width, height) = (usize::from(page.width()), usize::from(page.height()));
    if width == 0 || height == 0 || indexed.len() < width * height {
        return;
    }
    // A page without its window (`$C4 0`) shows only its glyphs.
    let windowed = page.background_index() != 0;
    let colour = |palette: usize, index: u8, y: i32| {
        if palette == 1 && index == 1 {
            page.speaker()
        } else {
            colour(art, palette, index, y)
        }
    };
    if windowed {
        draw_frame(canvas, art, origin, (width / 8, height / 8));
    }
    let glyphs = &page.glyphs()[..shown.min(page.glyphs().len())];
    for row in 0..height {
        for column in 0..width {
            let index = indexed[row * width + column] & 3;
            if index == 0 {
                continue;
            }
            // The last glyph over the pixel names its palette.
            let palette = glyphs
                .iter()
                .rev()
                .find(|glyph| {
                    let [x, y] = glyph.position.map(usize::from);
                    (x..x + 16).contains(&column) && (y..y + 16).contains(&row)
                })
                .map_or(0, |glyph| usize::from(glyph.palette));
            let at = (origin.0 + signed(column), origin.1 + signed(row));
            canvas.set(at, rgb(colour(palette, index, at.1)));
        }
    }
    // `$D5` pages wait with the prompt in the next cell (`$85:9D94`).
    let typed = shown >= page.glyphs().len();
    if let (true, Acknowledgement::Next, Some(last)) =
        (typed, page.acknowledgement(), page.glyphs().last())
    {
        let frame = usize::try_from(tick / PROMPT_TICKS % 4).unwrap_or(0);
        let [x, y] = page.end().map(|at| signed(usize::from(at)));
        for (at, &index) in art.prompt[frame].iter().enumerate() {
            let (px, py) = (x + signed(at % 16), y + signed(at / 16));
            if index != 0 && px < signed(width) {
                let at = (origin.0 + px, origin.1 + py);
                canvas.set(at, rgb(colour(usize::from(last.palette), index, at.1)));
            }
        }
    }
}

/// The frame one tile outside `(columns, rows)` of content at `origin`.
fn draw_frame(
    canvas: &mut Canvas,
    art: &WindowArt,
    origin: (i32, i32),
    (columns, rows): (usize, usize),
) {
    let (columns, rows) = (signed(columns), signed(rows));
    for row in -1..=rows {
        for column in -1..=columns {
            let tile = match (column == -1, column == columns, row == -1, row == rows) {
                (true, _, true, _) => &art.frame[0],
                (_, true, true, _) => &art.frame[2],
                (_, _, true, _) => &art.frame[1],
                (true, _, _, true) => &art.frame[5],
                (_, true, _, true) => &art.frame[7],
                (_, _, _, true) => &art.frame[6],
                (true, ..) => &art.frame[3],
                (_, true, ..) => &art.frame[4],
                _ => &art.interior,
            };
            for (at, &index) in tile.iter().enumerate() {
                let (x, y) = (
                    origin.0 + column * 8 + signed(at % 8),
                    origin.1 + row * 8 + signed(at / 8),
                );
                if index != 0 {
                    canvas.set((x, y), rgb(colour(art, 0, index, y)));
                }
            }
        }
    }
}

/// A colour of text palette `palette` on screen line `y`: its colour 3
/// is the shade.
fn colour(art: &WindowArt, palette: usize, index: u8, y: i32) -> Bgr555 {
    if index == 3 {
        art.shade(usize::try_from(y).unwrap_or(0))
    } else {
        art.colours[(palette * 4 + usize::from(index)).min(11)]
    }
}

/// The choice cursor at `[x, y]` in the page: a small white arrow, not the
/// game's cursor sprite, which is not decoded.
pub fn draw_cursor(canvas: &mut Canvas, art: &WindowArt, origin: (i32, i32), [x, y]: [u16; 2]) {
    let (x, y) = (origin.0 + i32::from(x), origin.1 + i32::from(y) + 4);
    for row in 0..7 {
        let width = 4 - (row - 3i32).abs();
        for column in 0..width {
            canvas.set((x + column, y + row), rgb(art.colours[1]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::text::HouseDialogue;

    fn rom() -> rom::Rom {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").expect("CRYSTA_JP_ROM")).unwrap();
        rom::Rom::load(&bytes).unwrap()
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_page_draws_its_frame_speaker_colour_and_prompt_once_typed() {
        let rom = rom();
        let image = rom.image();
        let art = WindowArt::from_rom(image).unwrap();
        let dialogue = HouseDialogue::from_rom(image).unwrap();
        let page = assets::text::TEXT_SOURCES
            .iter()
            .flat_map(|&source| dialogue.pages(source).unwrap_or(&[]))
            .find(|page| {
                page.acknowledgement() == Acknowledgement::Next
                    && page
                        .glyphs()
                        .first()
                        .is_some_and(|glyph| glyph.palette == 1)
            })
            .expect("a named page that waits");
        let origin = content_origin(image, page.placement(), 0, CLASSIC_WIDTH);
        let draw = |shown: usize| {
            let mut canvas = Canvas::new(CLASSIC_WIDTH);
            let typed = page.typed(image, shown);
            draw_window(&mut canvas, &art, page, (&typed, shown), origin, 0);
            canvas
        };
        let pixel = |canvas: &Canvas, (x, y): (i32, i32)| {
            canvas.pixels[usize::try_from(y).unwrap() * canvas.width + usize::try_from(x).unwrap()]
        };
        let all = page.glyphs().len();
        let done = draw(all);
        // The frame's corner tile, one tile up and left of the content.
        let corner =
            (origin.0 - 8..origin.0).flat_map(|x| (origin.1 - 8..origin.1).map(move |y| (x, y)));
        assert!(corner.clone().any(|at| pixel(&done, at) != 0));
        // The name's colour 1 is the speaker's.
        let name = page.glyphs()[0];
        let speaker = rgb(page.speaker());
        let cell = (0..16).flat_map(|x| (0..16).map(move |y| (x, y)));
        let [nx, ny] = name.position.map(i32::from);
        assert!(cell
            .clone()
            .any(|(x, y)| pixel(&done, (origin.0 + nx + x, origin.1 + ny + y)) == speaker));
        // The prompt shows at the page's end once typed, not before.
        let [ex, ey] = page.end().map(i32::from);
        let prompt = |canvas: &Canvas| {
            cell.clone()
                .filter(|&(x, _)| ex + x < i32::from(page.width()))
                .map(|(x, y)| pixel(canvas, (origin.0 + ex + x, origin.1 + ey + y)))
                .collect::<Vec<_>>()
        };
        assert_ne!(prompt(&done), prompt(&draw(all - 1)), "the prompt");
    }

    #[test]
    fn the_standard_window_starts_on_line_159_or_above_a_low_player() {
        let japan = [0; 16];
        assert_eq!(
            content_origin(&japan, Placement::Bottom, 100, 256),
            (16, 159)
        );
        assert_eq!(
            content_origin(&japan, Placement::AwayFromPlayer, 150, 256),
            (16, 31)
        );
        assert_eq!(
            content_origin(&japan, Placement::AwayFromPlayer, 50, 256),
            (16, 159)
        );
        assert_eq!(
            content_origin(&japan, Placement::Tile { column: 2, row: 23 }, 0, 400),
            (72 + 16, 183)
        );
    }

    #[test]
    fn the_european_bottom_window_starts_a_row_higher() {
        let mut europe = vec![b' '; 0x1_0000];
        europe[0xFFC0..0xFFCC].copy_from_slice(b"TERRANIGMA P");
        // `$C1` at base `$04C4`: row 19; `$DA` at `$0104`: row 4.
        assert_eq!(
            content_origin(&europe, Placement::Bottom, 100, 256),
            (16, 151)
        );
        assert_eq!(
            content_origin(&europe, Placement::AwayFromPlayer, 150, 256),
            (16, 31)
        );
    }
}
