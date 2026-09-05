//! Fixed frame-1601 visual checks; effect matching is not the static renderer.
use crate::{invalid, sha256, Result};
use assets::{graphics::IndexedPixel, maps::visual::CavernBackground};
use oracle::Session;
use rom::Rom;
use serde_json::{json, Value};

pub(super) fn check(session: &Session, rom: &Rom) -> Result<Value> {
    let scene = CavernBackground::from_rom(rom.image())?;
    let wram = session.wram_image();
    let vram = session.vram();
    let cgram = session.cgram();
    let graphics: Vec<_> = vram[..0x2000]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let palette: Vec<_> = cgram[32..128]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    if assets::maps::LoadedMap::from_wram(&wram)?.camera() != (648, 0) {
        return Err(invalid("visual qualification requires stationary camera (648,0)").into());
    }
    // The strict recipe orders these as graphics, cavern palette, definitions.
    let definitions = &wram[0x2000..0x3000];
    if session.frame_state().frames != 1601
        || graphics != scene.resources()[0].decoded()
        || palette != scene.resources()[1].decoded()
        || definitions != scene.resources()[2].decoded()
        || wram[0x846..0x848] != [0, 0]
        || wram[0x46A..0x46E] != [0x80, 0xA1, 0, 0x38]
        || wram[0x471..0x474] != [0xCF, 0, 0]
    {
        return Err(
            invalid("cavern visual resources or effect mirrors disagree with frame 1601").into(),
        );
    }
    // BG1SC=$38: 32x32 circular tilemap at VRAM word $3800. Check
    // world tile columns 82..110 and rows 2..26, not a flat substring search.
    let mut tilemap_words = 0;
    for y in 2..26 {
        for x in 82..110 {
            let cell = scene.layer().cells()[(y / 2) * scene.layer().width() + x / 2];
            let expected =
                scene.metatiles()[usize::from(cell.raw() & 511)][(y % 2) * 2 + x % 2].raw();
            if vram[0x3800 + (y % 32) * 32 + x % 32] != expected {
                return Err(
                    invalid("metatile quadrant expansion disagrees with BG1 tilemap").into(),
                );
            }
            tilemap_words += 1;
        }
    }
    // A sprite-free, opaque 16x16 patch at world (656,16). The qualified
    // camera is (648,0); framebuffer has an eight-row border. This profile
    // comparison applies observed fixed-color subtraction and ares's default
    // Deep Black Boost ramp. Neither belongs to the ROM-only natural renderer.
    let mut compared = Vec::new();
    for y in 16..32 {
        for x in 656..672 {
            let IndexedPixel::Opaque { palette_index, .. } = scene.pixel(x, y)? else {
                return Err(invalid("qualified background patch became transparent").into());
            };
            let expected = oracle_effect_rgb(scene.palette()[usize::from(palette_index)].raw());
            let at = ((y + 8) * 512 + (x - 648) * 2) * 4;
            let pixel = &session.pixels()[at..at + 4];
            if expected != [pixel[2], pixel[1], pixel[0]] {
                return Err(invalid("static background patch disagrees with oracle pixels").into());
            }
            compared.extend(expected);
        }
    }
    Ok(
        json!({"frame":1601,"graphics_bytes_equal":graphics.len(),"definition_bytes_equal":definitions.len(),
        "palette_bytes_equal":palette.len(),"tilemap_words_equal":tilemap_words,"reference_pixels_equal":compared.len()/3,
        "graphics_sha256":sha256(&graphics),"definitions_sha256":sha256(definitions),"palette_sha256":sha256(&palette),
        "patch_world_rectangle":[656,16,16,16],"patch_rgb_sha256":sha256(&compared),
        "effect_profile":"Fixed G/B subtraction 15, full brightness, ares Deep Black Boost. Qualification-only, not static RGB.",
        "limitations":"One opaque sprite-free patch, not full-frame equality; frame 1841 scroll and raster/animation behavior remain outside this check."}),
    )
}

// Matches the licensed vendored ares PPU-performance/color.cpp's default ramp.
fn oracle_effect_rgb(word: u16) -> [u8; 3] {
    const RAMP: [u8; 32] = [
        0, 1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 66, 78, 91, 105, 120, 136, 144, 152, 160, 168, 176,
        184, 192, 200, 208, 216, 224, 232, 240, 248, 255,
    ];
    [0, 5, 10].map(|shift| {
        let value = ((word >> shift) & 31) as u8;
        RAMP[usize::from(value.saturating_sub(if shift == 0 { 0 } else { 15 }))]
    })
}
#[cfg(test)]
mod tests {
    use super::oracle_effect_rgb;
    #[test]
    fn qualification_effect_saturates_each_channel_independently() {
        assert_eq!(
            oracle_effect_rgb(0x0A | 0x19 << 5 | 0x19 << 10),
            [55, 55, 55]
        );
        assert_eq!(oracle_effect_rgb(0), [0, 0, 0]);
        assert_eq!(oracle_effect_rgb(0x7FFF), [255, 136, 136]);
        assert_eq!(oracle_effect_rgb(31), [255, 0, 0]);
        assert_eq!(oracle_effect_rgb(15 << 5 | 15 << 10), [0, 0, 0]);
    }
}
