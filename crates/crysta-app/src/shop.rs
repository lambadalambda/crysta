//! The shop's display over the counter, as a native capture of `$1E` shows
//! it (`docs/shops.md`): the name in 16×16 glyphs, the icon, "×count=", the
//! coin and the price in BG3's 8×16 digits, and the money bag with the
//! money. Places are relative to the talk target's screen position.

use crate::frame::{rgb, Canvas};
use assets::graphics::{Bgr555, Tile4bpp};
use assets::shop_display::{halve, item_icon, name_glyphs, ShopArt};
use crysta_runtime::shop::Display;
use std::collections::HashMap;

/// An item's icon tiles and colours.
type Icon = ([Tile4bpp; 4], [Bgr555; 8]);
/// An item's name: its width byte and glyphs.
type Name = (u8, Vec<[u8; 256]>);

/// The display's art, decoded once, with each item's icon and name.
#[derive(Debug, Default)]
pub struct ShopArtCache {
    art: std::cell::OnceCell<Option<ShopArt>>,
    icons: HashMap<u8, Option<Icon>>,
    names: HashMap<u8, Option<Name>>,
}

impl ShopArtCache {
    /// Draws `display` with the talk target at `target` and Ark at `ark`,
    /// both on screen, and `money`. Art the decoder refuses is left out.
    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        image: &[u8],
        display: &Display,
        (target, ark): ((i32, i32), (i32, i32)),
        money: u32,
    ) {
        let Some(art) = self.art.get_or_init(|| ShopArt::from_rom(image).ok()) else {
            return;
        };
        let (tx, ty) = target;
        if let Some((width, glyphs)) = self
            .names
            .entry(display.item)
            .or_insert_with(|| name_glyphs(image, display.item).ok())
        {
            // `$92:D1D3`: the width byte shifts the name; past `$D0` on the
            // classic screen it wraps to `$18`.
            let left = i32::try_from(canvas.width.saturating_sub(256) / 2).unwrap_or(0);
            let mut x = tx - 56 + i32::from(*width) * 4;
            if x - left >= 0xD0 {
                x = left + 0x18;
            }
            let shown = usize::from(display.age / 2).min(glyphs.len());
            for (index, glyph) in (0_i32..).zip(&glyphs[..shown]) {
                blit(canvas, (x + index * 12, ty - 32), 16, |px, py| {
                    let colour = glyph[py * 16 + px];
                    (colour != 0).then(|| rgb(art.sprite_palette[usize::from(colour)]))
                });
            }
        }
        if !display.holding && display.quantity <= 9 {
            let digit = ShopArt::DIGITS + display.quantity;
            for (tile, x) in [(ShopArt::TIMES, -12), (digit, -6), (ShopArt::EQUALS, 1)] {
                sprite(canvas, art, tile, (tx + x, ty + 4), &art.count_palette);
            }
        }
        sprite(
            canvas,
            art,
            ShopArt::COIN,
            (tx + 12, ty + 7),
            &art.sprite_palette,
        );
        if let Some((tiles, colours)) = self
            .icons
            .entry(display.item)
            .or_insert_with(|| item_icon(image, display.item).ok())
        {
            let at = if display.holding {
                (ark.0 - 8, ark.1 - 45)
            } else {
                (tx - 24, ty - 3)
            };
            let colours = colours.map(|colour| if display.dim { halve(colour) } else { colour });
            // Its colours are OBJ palette 7's upper eight, 8..15.
            blit(canvas, at, 16, |px, py| {
                let colour = tiles[py / 8 * 2 + px / 8].pixels()[py % 8 * 8 + px % 8];
                (colour >= 8).then(|| rgb(colours[usize::from(colour - 8)]))
            });
        }
        number(canvas, art, display.price, (tx + 40, ty - 1));
        for (index, &tile) in (0_i32..).zip(&ShopArt::BAG) {
            panel(
                canvas,
                art,
                tile,
                3,
                (tx + 8 + index % 2 * 8, ty + 31 + index / 2 * 8),
            );
        }
        number(canvas, art, money, (tx + 40, ty + 31));
    }
}

/// A number in the panel's 8×16 digits, the ones at `ones`, without
/// leading zeros (`$85:ECB7`).
fn number(canvas: &mut Canvas, art: &ShopArt, value: u32, ones: (i32, i32)) {
    let digits = value.to_string();
    for (index, digit) in (0_i32..).zip(digits.bytes().rev()) {
        let digit = digit - b'0';
        let x = ones.0 - index * 8;
        panel(canvas, art, ShopArt::TOP + digit, 3, (x, ones.1));
        panel(canvas, art, ShopArt::BOTTOM + digit, 4, (x, ones.1 + 8));
    }
}

/// A BG3 character with its 4-colour palette; colour 0 is clear.
fn panel(canvas: &mut Canvas, art: &ShopArt, tile: u8, palette: usize, at: (i32, i32)) {
    let Some(tile) = art.panel.get(usize::from(tile)) else {
        return;
    };
    blit(canvas, at, 8, |px, py| {
        let colour = tile[py * 8 + px];
        (colour != 0).then(|| rgb(art.panel_palette[palette * 4 + usize::from(colour)]))
    });
}

/// An 8×8 sprite tile of the sheet (`$40..$5F`); colour 0 is clear.
fn sprite(canvas: &mut Canvas, art: &ShopArt, tile: u8, at: (i32, i32), palette: &[Bgr555; 16]) {
    let Some(tile) = tile
        .checked_sub(ShopArt::DIGITS)
        .and_then(|index| art.sprites.get(usize::from(index)))
    else {
        return;
    };
    blit(canvas, at, 8, |px, py| {
        let colour = tile.pixels()[py * 8 + px];
        (colour != 0).then(|| rgb(palette[usize::from(colour)]))
    });
}

/// Draws a square of `size` pixels at a screen position, clipped.
fn blit(
    canvas: &mut Canvas,
    (x, y): (i32, i32),
    size: i32,
    pixel: impl Fn(usize, usize) -> Option<u32>,
) {
    for (py, row) in (0..size).zip(0_usize..) {
        for (px, column) in (0..size).zip(0_usize..) {
            if let Some(colour) = pixel(column, row) {
                canvas.set((x + px, y + py), colour);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blit_clips_at_the_canvas_edges() {
        let mut canvas = Canvas::new(4);
        blit(&mut canvas, (2, -1), 4, |_, _| Some(7));
        assert_eq!(canvas.pixels[..8], [0, 0, 7, 7, 0, 0, 7, 7]);
    }
}
