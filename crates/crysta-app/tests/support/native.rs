//! Comparing composed frames with native captures: 256×240 RGB frames
//! from the reference emulator, up to its colour curve.
#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "frame coordinates fit every width; each pixel test uses part"
)]
use crysta_app::frame::{Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use std::path::Path;

pub fn local(name: &str) -> Option<Vec<u8>> {
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
pub fn unlike(canvas: &Canvas, native: &[u8], offset: (i32, i32)) -> usize {
    unlike_where(canvas, native, offset, |_, _| true)
}

/// As [`unlike`], over the canvas pixels `keep` selects.
pub fn unlike_where(
    canvas: &Canvas,
    native: &[u8],
    (dx, dy): (i32, i32),
    keep: impl Fn(usize, usize) -> bool,
) -> usize {
    use std::collections::HashMap;
    let mut pairs: HashMap<(u32, u32), usize> = HashMap::new();
    // Line 0 is never shown natively: the capture has it black.
    for y in 1..VIEW_HEIGHT {
        for x in 0..CLASSIC_WIDTH {
            if !keep(x, y) {
                continue;
            }
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
pub fn best(canvas: &Canvas, native: &[u8]) -> (usize, (i32, i32)) {
    (-4..=4)
        .flat_map(|dx| (0..=32).map(move |dy| (dx, dy)))
        .map(|offset| (unlike(canvas, native, offset), offset))
        .min()
        .unwrap()
}
