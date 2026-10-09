//! Rust models of the native code Shadowkeeper's fight on `$123` runs
//! (`docs/tower-five.md`), which the native runs cannot follow: numeric
//! index loops, VRAM writes, the camera's target. Each is keyed by its
//! image offset and checked against its bytes, and says where the script goes
//! on. The intro's code is the same in both ROMs.

use super::{Actor, Surroundings};

/// `JSR $8289` at `$8F:8009`: the intro clears the map's BG3 tilemap
/// (`$6C00`, 1024 words of `$1E00`), which the hosts do not draw.
const CLEAR_BG3: (usize, [u8; 3]) = (0x0F_8009, [0x20, 0x89, 0x82]);
/// `$8F:8025`..`8043`: the torch lit for the camera's band, `$04A4 |=
/// $8F:80AA[n]` for the first limit at `$8F:809A` above the camera's y
/// (`$0822`).
const TORCH_BAND: (usize, [u8; 4]) = (0x0F_8025, [0xDA, 0xAD, 0x22, 0x08]);
const TORCH_BAND_END: usize = 0x0F_8044;
const BAND_LIMITS: usize = 0x0F_809A;
const BAND_BITS: usize = 0x0F_80AA;
const BANDS: usize = 8;
/// `TXA; STA $0DEC`: the camera follows this actor (`$8F:8060`).
const FOLLOW_ME: [u8; 4] = [0x8A, 0x8D, 0xEC, 0x0D];
/// `$04A4`, the lit torches.
pub(crate) const TORCHES: u16 = 0x04A4;

impl Actor {
    /// Runs the routine at the script's place, if one is modelled. Returns
    /// whether it did.
    pub(super) fn model_routine(&mut self, around: &mut Surroundings<'_>) -> bool {
        let image = around.image;
        let at = self.pc;
        let code = |bytes: &[u8]| image.get(at..at + bytes.len()) == Some(bytes);
        if at == CLEAR_BG3.0 && code(&CLEAR_BG3.1) {
            self.pc = at + 3;
            return true;
        }
        if at == TORCH_BAND.0 && code(&TORCH_BAND.1) {
            let camera = around.globals.view.map_or(0, |view| view.1);
            let torches = around.globals.scratch.entry(TORCHES).or_insert(0);
            *torches |= u16::from(torch_band(image, camera));
            self.pc = TORCH_BAND_END;
            return true;
        }
        if code(&FOLLOW_ME) {
            around.globals.camera_target = Some(self.id);
            self.pc = at + FOLLOW_ME.len();
            return true;
        }
        false
    }
}

/// The torch bit of the band the camera's y falls in.
fn torch_band(image: &[u8], camera: u16) -> u8 {
    let word = |at: usize| {
        image
            .get(at..at + 2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
    };
    let band = (0..BANDS)
        .find(|&band| word(BAND_LIMITS + 2 * band).is_none_or(|limit| camera < limit))
        .unwrap_or(BANDS - 1);
    image.get(BAND_BITS + band).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_band_picks_its_torch() {
        let mut image = vec![0; BAND_BITS + BANDS];
        for (band, limit) in [0x70u16, 0xF0, 0x160, 0x1D0, 0x240, 0x2B0, 0x320, 0x330]
            .into_iter()
            .enumerate()
        {
            image[BAND_LIMITS + 2 * band..BAND_LIMITS + 2 * band + 2]
                .copy_from_slice(&limit.to_le_bytes());
        }
        image[BAND_BITS..].copy_from_slice(&[0x7F, 0x7E, 0x7C, 0x78, 0x70, 0x60, 0x40, 0]);
        assert_eq!(torch_band(&image, 0), 0x7F);
        assert_eq!(torch_band(&image, 0x70), 0x7E);
        assert_eq!(torch_band(&image, 0x2AF), 0x60);
        assert_eq!(torch_band(&image, 0x400), 0);
    }
}
