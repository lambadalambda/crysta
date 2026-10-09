//! Rust models of the native code Shadowkeeper's fight on `$123` runs
//! (`docs/tower-five.md`), which the native runs cannot follow: numeric
//! index loops, VRAM writes, the camera's target. Each is keyed by its
//! image offset and checked against its bytes, and says where the script goes
//! on. The intro's code is the same in both ROMs; the body's moved from
//! bank `$93` to `$99` in the European one.

use super::Poke;
use super::{Actor, Surroundings};
use std::ops::Range;

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
/// The wisps the head's window lets out (`$8F:80B5`): the common movement
/// base (`7F:0022 = $6000`), then (`+$06 |= $2040` the idiom's) a flip by the
/// frame counter's two low bits, a random stream of `$8F:8109`'s 32
/// (`JSL $8F:8129`).
const WISP_BASE: (usize, [u8; 7]) = (0x0F_80B5, [0xA9, 0x00, 0x60, 0x9F, 0x22, 0x00, 0x7F]);
const WISP_FLIP: (usize, [u8; 4]) = (0x0F_80D6, [0xAD, 0x42, 0x00, 0x4A]);
const WISP_FLIP_END: usize = 0x0F_80E7;
const WISP_STREAM: (usize, [u8; 4]) = (0x0F_80E9, [0xAD, 0x08, 0x04, 0xDA]);
const WISP_STREAM_END: usize = 0x0F_80FD;
const WISP_STREAMS: usize = 0x0F_8109;
/// `COP AA $8182` at `$8F:8011`: the darkness's child, which the hosts draw
/// ([`crate::world::Darkness`]).
const DARKNESS: (usize, [u8; 5]) = (0x0F_8011, [0x02, 0xAA, 0x82, 0x81, 0x8F]);
/// `$04A4`, the lit torches.
pub(crate) const TORCHES: u16 = 0x04A4;
/// The body's code in the Japanese image, and how far the European one
/// moved it.
const BODY: Range<usize> = 0x13_D7BF..0x13_E900;
const EUROPE_SHIFT: usize = 0x05_C2E8;
/// `$93:D992`: Ark kept between 96 and 223 pixels below the camera's top
/// (`$0812`): below it is set to 96, beyond to 223.
const CLAMP: (usize, [u8; 3]) = (0x13_D992, [0xAC, 0xEA, 0x0D]);
const CLAMP_END: usize = 0x13_D9B3;
const CLAMP_SPAN: (u16, u16) = (0x60, 0xE0);
/// `JSR $E715` (`$93:DC35`, the tail's controller): its two parts (`+$14`,
/// `+$16`) at its parent's place.
const FOLLOW: (usize, u16) = (0x13_DC35, 0xE715);
const FOLLOW_END: usize = 0x13_DC38;
/// `$93:E03D` (the claw holder): `$00 = 1; JSR $E717`, the claws and the
/// holder a pixel below its parent.
const HOLD: (usize, u16) = (0x13_E03D, 0xE717);
const HOLD_END: usize = 0x13_E04F;
/// `$93:D947`: the head (`+$14`) and the body at OBJ priority 0
/// (`JSR $E6F9`: `+$08 &= $CFFF`).
const SINK: (usize, [u8; 3]) = (0x13_D947, [0xBD, 0x14, 0x00]);
const SINK_END: usize = 0x13_D952;

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
        if at == DARKNESS.0 && code(&DARKNESS.1) {
            around.globals.torch_darkness = Some(around.globals.frames);
            self.pc = at + DARKNESS.1.len();
            return true;
        }
        if at == TORCH_BAND.0 && code(&TORCH_BAND.1) {
            let camera = around.globals.view.map_or(0, |view| view.1);
            let torches = around.globals.scratch.entry(TORCHES).or_insert(0);
            *torches |= u16::from(torch_band(image, camera));
            self.pc = TORCH_BAND_END;
            return true;
        }
        if at == WISP_BASE.0 && code(&WISP_BASE.1) {
            self.base = super::Base::Common;
            self.pc = at + WISP_BASE.1.len();
            return true;
        }
        if at == WISP_FLIP.0 && code(&WISP_FLIP.1) {
            let frames = around.globals.frames;
            self.hflip |= frames & 1 != 0;
            self.vflip |= frames & 2 == 0;
            self.pc = WISP_FLIP_END;
            return true;
        }
        if at == WISP_STREAM.0 && code(&WISP_STREAM.1) {
            let pick = usize::from(around.globals.random.word() & 0x1F);
            let selector = image.get(WISP_STREAMS + pick).copied().unwrap_or(0);
            let stops = self.interaction & 0x80 != 0;
            self.motion = self
                .movement(image)
                .and_then(|resource| super::motion::Motion::start(resource, selector, stops, None));
            self.pc = WISP_STREAM_END;
            return true;
        }
        if code(&FOLLOW_ME) {
            around.globals.camera_target = Some(self.id);
            self.pc = at + FOLLOW_ME.len();
            return true;
        }
        self.body_routine(around)
    }

    /// A routine of the body's code, in the Japanese image's offsets.
    fn body_routine(&mut self, around: &mut Surroundings<'_>) -> bool {
        let image = around.image;
        let shift = assets::layout::per_revision(image, 0, EUROPE_SHIFT);
        let Some(at) = self.pc.checked_sub(shift).filter(|at| BODY.contains(at)) else {
            return false;
        };
        let code = |bytes: &[u8]| image.get(self.pc..self.pc + bytes.len()) == Some(bytes);
        let jsr = |address: u16| {
            let [low, high] = super::shadowkeeper::local(image, address).to_le_bytes();
            [0x20, low, high]
        };
        let next = match at {
            _ if at == CLAMP.0 && code(&CLAMP.1) => {
                let camera = around.globals.view.map_or(0, |view| view.1);
                if let Some(y) = clamp(around.player.1, camera) {
                    around.globals.pokes.push(Poke::ArkAt { at: 2, value: y });
                }
                CLAMP_END
            }
            _ if at == SINK.0 && code(&SINK.1) => {
                let head = self.own_word(0x14);
                around.globals.pokes.push(Poke::Priority {
                    id: head,
                    priority: 0,
                });
                self.priority = 0;
                SINK_END
            }
            _ if at == FOLLOW.0 && code(&jsr(FOLLOW.1)) => {
                self.follow_parent(around, 0);
                FOLLOW_END
            }
            _ if at == HOLD.0
                && code(&[[0xA9, 0x01, 0x00, 0x85, 0x00].as_slice(), &jsr(HOLD.1)].concat()) =>
            {
                if let Some(place) = self.follow_parent(around, 1) {
                    self.position = place;
                }
                HOLD_END
            }
            _ if self.direct_part(around) => return true,
            _ => match self.shadowkeeper(at, around) {
                Some(next) => next,
                None => return false,
            },
        };
        self.pc = next + shift;
        true
    }
}

impl Actor {
    /// `LDA $00ff,X; TAY; [BEQ +9;] LDA #s; JSR $E703`: the part in field
    /// `ff` goes on at `s` in the bank (`$93:DC6A`, `$93:E0D5`). Returns
    /// whether it was that.
    fn direct_part(&mut self, around: &mut Surroundings<'_>) -> bool {
        let at = self.pc;
        let Some(code) = around.image.get(at..at + 12) else {
            return false;
        };
        let (field, rest) = match code {
            [0xBD, field, 0, 0xA8, rest @ ..] => (u16::from(*field), rest),
            _ => return false,
        };
        let tell = super::shadowkeeper::local(around.image, 0xE703).to_le_bytes();
        let (script, length, skip) = match rest {
            [0xA9, low, high, 0x20, a, b, ..] if [*a, *b] == tell => {
                (u16::from_le_bytes([*low, *high]), 10, 10)
            }
            [0xF0, 0x09, 0xA9, low, high, 0x20, a, b] if [*a, *b] == tell => {
                (u16::from_le_bytes([*low, *high]), 12, 15)
            }
            _ => return false,
        };
        let id = self.own_word(field);
        if id == 0 {
            self.pc = at + skip;
            return true;
        }
        let pc = (at & 0xFF_0000) | usize::from(script);
        around.globals.pokes.push(Poke::Script { id, pc });
        self.pc = at + length;
        true
    }

    /// `$93:E717`: the parts in `+$14` and `+$16` put at the parent's place
    /// `dy` lower (`$7F:102E`). Returns the place.
    fn follow_parent(&mut self, around: &mut Surroundings<'_>, dy: u16) -> Option<(u16, u16)> {
        let (x, y) = self.parent_place(around)?;
        let at = (x, y.wrapping_add(dy));
        for field in [0x14, 0x16] {
            let id = self.own_word(field);
            if id != 0 {
                around.globals.pokes.push(Poke::Place { id, at });
            }
        }
        Some(at)
    }
}

/// Ark's y kept in the camera's span (`$93:D992`), if it moves him.
fn clamp(y: u16, camera: u16) -> Option<u16> {
    let (low, high) = (
        camera.wrapping_add(CLAMP_SPAN.0),
        camera.wrapping_add(CLAMP_SPAN.1),
    );
    if low >= y {
        Some(low)
    } else if high < y {
        Some(high - 1)
    } else {
        None
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
    fn the_clamp_keeps_ark_in_the_cameras_span() {
        assert_eq!(clamp(90, 0), Some(0x60));
        assert_eq!(clamp(0x60, 0), Some(0x60));
        assert_eq!(clamp(0x61, 0), None);
        assert_eq!(clamp(0xE0, 0), None);
        assert_eq!(clamp(0xE1, 0), Some(0xDF));
    }

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
