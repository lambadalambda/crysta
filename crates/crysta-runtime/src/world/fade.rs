//! The screen's fades around a script transfer (`COP 14`), by the transfer's
//! mode (`$0484`). `$8D:86F8` fades out (dispatch `$8D:89DA`), loads, and
//! fades in (dispatch `$8D:8A67`); each step is a whole game frame
//! (`$80:80DF`), so the actors go on moving.
//!
//! | Mode | Out | In |
//! |---|---|---|
//! | 0, 8+ | 16 frames, a brightness step each (`$89F4`) | 16 frames (`$8A81`) |
//! | 1 | 80 frames, a step each 5 (`$8A01`) | 80 frames (`$8A90`) |
//! | 2, 3 | a cut after one frame (`$8A15`) | 2: 16 frames; 3: a cut |
//! | 4, 5 | 32 frames, a step each 2, mosaic growing (`$8A1D`) | 4: the reverse (`$8AB0`); 5: a cut |
//! | 6, 7 | 37 whitening steps of 3 frames, then the slow fade (`$8A3B`) | 6: slow; 7: slow from white, then 33 steps of 3 frames back to colour (`$8AD0`) |
//!
//! Measured on the native route (`departure/journey.jsonl`): mode 0 and
//! mode 4 fade out in 16 and 32 frames. Not modelled: the load's own dark
//! frames (3 natively, 49 and 68 around the box's tour), and that each
//! whitening step runs only one game frame of its three.

use super::{Step, World, WorldError};
use crate::scene::Transfer;

/// A transfer's fades under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fading {
    /// Fading out, `frame` frames in; the map loads at the end.
    Out { transfer: Transfer, frame: u16 },
    /// Fading in after the load.
    In { mode: u8, frame: u16 },
}

/// How the screen is drawn: the effects the fades set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// `INIDISP`: 0 dark to 15 full.
    pub brightness: u8,
    /// `MOSAIC` on the first two layers: blocks of `mosaic + 1` pixels.
    pub mosaic: u8,
    /// A palette change toward white.
    pub tint: Tint,
}

/// A palette change toward white, per 5-bit colour channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    /// The colours as they are.
    None,
    /// Each channel raised by this much, up to 31 (`$86:AA96`).
    Raise(u8),
    /// Each channel at least this (`$86:AAFE` fills white, `$86:AA2B`
    /// steps back down).
    Floor(u8),
}

impl Screen {
    /// The screen at full brightness, with no effect.
    pub const FULL: Self = Self::lit(15);

    const fn lit(brightness: u8) -> Self {
        Self {
            brightness,
            mosaic: 0,
            tint: Tint::None,
        }
    }
}

const WHITENING: u16 = 37 * 3;
const SLOW: u16 = 80;
const COLOURING: u16 = 33 * 3;

/// Frames the fade out of `mode` lasts.
pub(super) const fn out_frames(mode: u8) -> u16 {
    match mode {
        1 => SLOW,
        2 | 3 => 1,
        4 | 5 => 32,
        6 | 7 => WHITENING + SLOW,
        _ => 16,
    }
}

/// Frames the fade in of `mode` lasts.
pub(super) const fn in_frames(mode: u8) -> u16 {
    match mode {
        1 | 6 => SLOW,
        3 | 5 => 1,
        4 => 32,
        7 => SLOW + COLOURING,
        _ => 16,
    }
}

/// The screen `frame` frames into the fade out (0, just queued, to
/// [`out_frames`]; the load shows instead of the last).
pub(super) fn out(mode: u8, frame: u16) -> Screen {
    if frame == 0 {
        return Screen::FULL;
    }
    let down = |per: u16, from: u16| dark(15u16.saturating_sub((frame - from - 1) / per));
    match mode {
        1 => down(5, 0),
        2 | 3 => Screen::lit(0),
        4 | 5 => {
            let screen = down(2, 0);
            Screen {
                mosaic: 15 - screen.brightness,
                ..screen
            }
        }
        6 | 7 if frame <= WHITENING => Screen {
            tint: Tint::Raise(step(frame, 3)),
            ..Screen::FULL
        },
        6 | 7 => Screen {
            tint: Tint::Raise(step(WHITENING, 3)),
            ..down(5, WHITENING)
        },
        _ => down(1, 0),
    }
}

/// The screen `frame` frames into the fade in (0, just loaded, to
/// [`in_frames`]).
pub(super) fn into(mode: u8, frame: u16) -> Screen {
    let up = |per: u16| dark((frame.saturating_sub(1) / per).min(15));
    match mode {
        _ if frame == 0 && mode == 7 => Screen {
            tint: Tint::Floor(31),
            ..Screen::lit(0)
        },
        _ if frame == 0 => Screen {
            mosaic: if mode == 4 { 15 } else { 0 },
            ..Screen::lit(0)
        },
        1 | 6 => up(5),
        3 | 5 => Screen::FULL,
        4 => {
            let screen = up(2);
            Screen {
                mosaic: 15 - screen.brightness,
                ..screen
            }
        }
        7 if frame <= SLOW => Screen {
            tint: Tint::Floor(31),
            ..up(5)
        },
        7 => Screen {
            tint: Tint::Floor(31u8.saturating_sub(step(frame - SLOW, 3))),
            ..Screen::FULL
        },
        _ => up(1),
    }
}

fn dark(brightness: u16) -> Screen {
    Screen::lit(u8::try_from(brightness).unwrap_or(15))
}

/// Steps of `per` frames begun by `frame`.
fn step(frame: u16, per: u16) -> u8 {
    u8::try_from(frame.div_ceil(per)).unwrap_or(u8::MAX)
}

impl World<'_> {
    /// Starts fading out for a transfer a script queued; one queued while
    /// the player walks through an exit waits for the walk to end.
    pub(super) fn queue_transfer(&mut self) {
        if self.fading.is_none() && self.leaving.is_none() && self.arriving.is_none() {
            if let Some(transfer) = self.globals.transfer.take() {
                self.fading = Some(Fading::Out { transfer, frame: 0 });
            }
        }
    }

    /// A frame of a transfer's fades, if one is under way: the actors run,
    /// the player stands, and the fade out's last frame loads the map. The
    /// load clears the pad mask, as every load does; the next map's scripts
    /// lock it again where they hold the player (the guide in the reloaded
    /// `$21`, `$88:AEC5`).
    pub(super) fn fade_frame(&mut self) -> Result<Option<Step>, WorldError> {
        let Some(fading) = self.fading.take() else {
            return Ok(None);
        };
        self.run_actors()?;
        match fading {
            Fading::Out { transfer, frame } if frame + 1 >= out_frames(transfer.mode) => {
                let (x, y) = transfer.position;
                let audio = self.globals.audio.clone();
                let mut entered = self.enter_destination(transfer.map, x, y, audio)?;
                entered.face(self.facing);
                entered.fading = Some(Fading::In {
                    mode: transfer.mode,
                    frame: 0,
                });
                let from = self.map;
                *self = entered;
                return Ok(Some(Step::Entered {
                    from,
                    to: transfer.map,
                }));
            }
            Fading::Out { transfer, frame } => {
                self.fading = Some(Fading::Out {
                    transfer,
                    frame: frame + 1,
                });
            }
            Fading::In { mode, frame } if frame + 1 < in_frames(mode) => {
                self.fading = Some(Fading::In {
                    mode,
                    frame: frame + 1,
                });
            }
            Fading::In { .. } => {}
        }
        Ok(Some(Step::Stayed))
    }

    /// How the screen is drawn now.
    #[must_use]
    pub fn screen(&self) -> Screen {
        match self.fading {
            Some(Fading::Out { transfer, frame }) => out(transfer.mode, frame),
            Some(Fading::In { mode, frame }) => into(mode, frame),
            None => Screen::lit(self.exit_brightness()),
        }
    }

    /// The screen's brightness, 0 dark to 15 full.
    #[must_use]
    pub fn brightness(&self) -> u8 {
        self.screen().brightness
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brightness(fade: impl Fn(u16) -> Screen, frames: u16) -> Vec<u8> {
        (1..=frames).map(|frame| fade(frame).brightness).collect()
    }

    #[test]
    fn mode_0_fades_a_step_a_frame_both_ways() {
        assert_eq!(out_frames(0), 16);
        assert_eq!(
            brightness(|f| out(0, f), 16),
            (0..16).rev().collect::<Vec<_>>()
        );
        assert_eq!(into(0, 0).brightness, 0);
        assert_eq!(brightness(|f| into(0, f), 16), (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn mode_1_holds_each_step_five_frames() {
        let steps = brightness(|f| out(1, f), 80);
        assert_eq!((steps[0], steps[4], steps[5], steps[79]), (15, 15, 14, 0));
        assert_eq!(into(1, 80).brightness, 15);
    }

    #[test]
    fn mode_4_grows_the_mosaic_as_it_darkens_and_shrinks_it_coming_back() {
        assert_eq!((out(4, 1).brightness, out(4, 1).mosaic), (15, 0));
        assert_eq!((out(4, 3).brightness, out(4, 3).mosaic), (14, 1));
        assert_eq!((out(4, 32).brightness, out(4, 32).mosaic), (0, 15));
        assert_eq!(into(4, 0).mosaic, 15);
        assert_eq!((into(4, 32).brightness, into(4, 32).mosaic), (15, 0));
    }

    #[test]
    fn mode_7_whitens_then_darkens_and_comes_back_from_white() {
        assert_eq!(out_frames(7), 191);
        assert_eq!(out(7, 1).tint, Tint::Raise(1));
        assert_eq!(
            (out(7, 111).brightness, out(7, 111).tint),
            (15, Tint::Raise(37))
        );
        assert_eq!(out(7, 191).brightness, 0);
        assert_eq!(in_frames(7), 179);
        assert_eq!(into(7, 0).tint, Tint::Floor(31));
        assert_eq!(
            (into(7, 80).brightness, into(7, 80).tint),
            (15, Tint::Floor(31))
        );
        assert_eq!(into(7, 83).tint, Tint::Floor(30));
        assert_eq!(into(7, 179).tint, Tint::Floor(0));
    }
}
