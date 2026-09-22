//! Native background presentation, separate from the asset inspector's checkerboard.
use assets::maps::visual::StaticBackground;

/// Load the static baseline without changing the inspector's export policy.
pub fn load(cartridge: &rom::Rom, map: u16) -> Result<crate::frame::Background, String> {
    let rendered = map_inspector::render_static_background(cartridge, map)
        .map_err(|error| error.to_string())?;
    let mut background =
        crate::frame::decode_bmp(&rendered.bitmap).ok_or("invalid static background bitmap")?;
    background.high = rendered.priorities.iter().map(|bit| *bit != 0).collect();
    if map == 0xA {
        let scene = StaticBackground::from_rom(cartridge.image(), map)
            .map_err(|error| error.to_string())?;
        let color = exterior_backdrop(cartridge.image(), &scene)?;
        composite_backdrop(&mut background.pixels, &rendered.indices, color)?;
    }
    Ok(background)
}

/// The ordinary map initialization copies staged palette color32 to backdrop0.
/// Keep the native policy explicit and map-A-only; static exports retain color0.
fn exterior_backdrop(image: &[u8], scene: &StaticBackground) -> Result<u32, String> {
    // $8D:8C52..8C62: LDA $7F0640 / STA $7F0600, then the high byte.
    // Source palette handler $86:8903 stages colors at $7F0600. The map-A
    // validated palette load supplies entries32..127, so this is its first word.
    const COPY: &[u8] = &[
        0xAF, 0x40, 0x06, 0x7F, 0x8F, 0x00, 0x06, 0x7F, 0xAF, 0x41, 0x06, 0x7F, 0x8F, 0x01, 0x06,
        0x7F, 0x60,
    ];
    if image.get(0xD_8C52..0xD_8C52 + COPY.len()) != Some(COPY) {
        return Err("unsupported exterior backdrop initialization".into());
    }
    let [r, g, b] = scene.palette()[32].rgb8();
    Ok(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
}

fn composite_backdrop(pixels: &mut [u32], indices: &[u8], color: u32) -> Result<(), String> {
    if pixels.len() != indices.len() {
        return Err("background color/index extent mismatch".into());
    }
    for (pixel, index) in pixels.iter_mut().zip(indices) {
        if *index == 0 {
            *pixel = color;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_transparent_indices_receive_the_backdrop() {
        let mut pixels = [0x0018_2228, 0x0024_2F37, 0x0018_2228];
        composite_backdrop(&mut pixels, &[0, 0, 1], 0x0012_3456).unwrap();
        assert_eq!(pixels, [0x0012_3456, 0x0012_3456, 0x0018_2228]);
    }

    #[test]
    fn mismatched_planes_do_not_partially_change_pixels() {
        let mut pixels = [1, 2];
        assert!(composite_backdrop(&mut pixels, &[0], 3).is_err());
        assert_eq!(pixels, [1, 2]);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn backdrop_is_read_from_source_and_changed_consumer_is_refused() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut image = rom.image().to_vec();
        let scene = StaticBackground::from_rom(&image, 0xA).unwrap();
        let palette = scene.resources()[1].source_range().start;
        image[palette..palette + 2].copy_from_slice(&0x001Fu16.to_le_bytes());
        let scene = StaticBackground::from_rom(&image, 0xA).unwrap();
        assert_eq!(exterior_backdrop(&image, &scene).unwrap(), 0x00FF_0000);
        image[0xD_8C53] ^= 1;
        assert!(exterior_backdrop(&image, &scene).is_err());
    }
}
