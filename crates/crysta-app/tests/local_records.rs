//! The Records screen drawn from the ROM against native frames
//! (`docs/records-screen.md`): `local/records/*.rgb`, 256×240 RGB captures
//! from the reference emulator.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "frame coordinates fit every width"
)]
use crysta_app::frame::{Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use crysta_app::records::{self, Games, RecordsArtCache};
use crysta_runtime::records::{Page, Text};
use crysta_runtime::save::SaveSlot;
use crysta_runtime::sram::Sram;
use rom::Rom;
use std::path::Path;

fn local(name: &str) -> Option<Vec<u8>> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../local")
            .join(name),
    )
    .ok()
}

/// Pixels of `canvas` unlike the capture at `(dx, dy)`, up to the
/// emulator's colour curve: every native colour must stand for one of ours
/// and each of ours for one native colour, and each channel must go
/// through one curve; a pixel off the most common pairing counts.
fn unlike(canvas: &Canvas, native: &[u8], (dx, dy): (i32, i32)) -> usize {
    use std::collections::HashMap;
    let mut pairs: HashMap<(u32, u32), usize> = HashMap::new();
    // Line 0 is never shown natively: the capture has it black.
    for y in 1..VIEW_HEIGHT {
        for x in 0..CLASSIC_WIDTH {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if !(0..256).contains(&nx) || !(0..240).contains(&ny) {
                continue;
            }
            let at = (ny as usize * 256 + nx as usize) * 3;
            let theirs = u32::from_be_bytes([0, native[at], native[at + 1], native[at + 2]]);
            *pairs
                .entry((canvas.pixels[y * CLASSIC_WIDTH + x], theirs))
                .or_default() += 1;
        }
    }
    let off = |key: fn(&(u32, u32)) -> u32| {
        let mut best: HashMap<u32, usize> = HashMap::new();
        let mut total: HashMap<u32, usize> = HashMap::new();
        for (pair, &count) in &pairs {
            let best = best.entry(key(pair)).or_default();
            *best = (*best).max(count);
            *total.entry(key(pair)).or_default() += count;
        }
        total
            .iter()
            .map(|(colour, total)| total - best[colour])
            .sum::<usize>()
    };
    // Each channel through one curve: our 8-bit value (from 5 bits) stands
    // for one native value, whatever colour it is part of.
    let mut channels: HashMap<(usize, u8, u8), usize> = HashMap::new();
    for (&(ours, theirs), &count) in &pairs {
        for channel in 0..3 {
            let shift = 16 - 8 * channel;
            let key = (channel, (ours >> shift) as u8, (theirs >> shift) as u8);
            *channels.entry(key).or_default() += count;
        }
    }
    let mut best: HashMap<(usize, u8), usize> = HashMap::new();
    let mut total: HashMap<(usize, u8), usize> = HashMap::new();
    for (&(channel, ours, _), &count) in &channels {
        let best = best.entry((channel, ours)).or_default();
        *best = (*best).max(count);
        *total.entry((channel, ours)).or_default() += count;
    }
    let curve: usize = total.iter().map(|(key, total)| total - best[key]).sum();
    off(|pair| pair.0).max(off(|pair| pair.1)) + curve
}

/// The fewest unlike pixels over the capture's offsets, and where.
fn best(canvas: &Canvas, native: &[u8]) -> (usize, (i32, i32)) {
    (-4..=4)
        .flat_map(|dx| (0..=32).map(move |dy| (dx, dy)))
        .map(|offset| (unlike(canvas, native, offset), offset))
        .min()
        .unwrap()
}

#[test]
fn the_screen_matches_native_frames_on_both_roms() {
    for (rom, prefix) in [
        ("Tenchi Souzou (Japan).sfc", "jp"),
        ("Terranigma (E) [!].smc", "eu"),
    ] {
        let Some(bytes) = local(rom) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let image = rom.image();
        // The captured game: a new one, 61 seconds in (0:01).
        let mut current = SaveSlot::new_game(image);
        for _ in 0..61 * 60 {
            current.tick_clock();
        }
        let mut saved = Sram::default();
        saved.write_slot(0, &current);
        let cache = RecordsArtCache::default();
        let art = cache.get(image).unwrap();
        let entry = Page {
            cursor: 0,
            title: true,
            text: Text::Entry,
        };
        let typed = Page {
            text: Text::Saved {
                slot: 0,
                lines: 3,
                message: true,
                slots: 3,
            },
            ..entry
        };
        for (capture, page, sram) in [("menu", entry, Sram::default()), ("saved", typed, saved)] {
            let native = local(&format!("records/{prefix}-{capture}.rgb"))
                .expect("with the ROM, its captures in `local/records`");
            let mut canvas = Canvas::new(CLASSIC_WIDTH);
            let games = Games {
                sram: &sram,
                current: &current,
            };
            records::draw(&mut canvas, image, art, page, &games);
            let (unlike, offset) = best(&canvas, &native);
            assert_eq!(unlike, 0, "{prefix}-{capture} at {offset:?}");
        }
    }
}
