//! The colour math scripts set up on the PPU (`docs/scene-effects.md`): a
//! fixed colour subtracted from the background, and the spinning square of
//! `COP 6A` that keeps its inside at full colour. Scripts write the
//! registers directly, through `COP 76`, or in native code; a map load
//! starts with none of it.

use assets::graphics::Bgr555;

/// `W12SEL`: the windows of BG1 and BG2.
const WINDOWS_12: u16 = 0x2123;
/// `TM`: the layers on the main screen.
const MAIN_SCREEN: u16 = 0x212C;
/// `TS`: the layers on the sub screen.
const SUB_SCREEN: u16 = 0x212D;
/// `WOBJSEL`: the colour window's windows.
const WINDOW_SELECT: u16 = 0x2125;
/// `CGWSEL`: where colour math is prevented.
const MATH_SELECT: u16 = 0x2130;
/// `CGADSUB`: add or subtract, and on which layers.
const MATH_LAYERS: u16 = 0x2131;
/// `COLDATA`: the fixed colour, a channel at a time.
const FIXED: u16 = 0x2132;
/// Half the side of `COP 6A`'s shape 0, the square (`$8D:B038`).
const SQUARE: f64 = 48.0;

/// The colour math in force.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Display {
    /// `TM`, once a script wrote it.
    main_screen: Option<u8>,
    sub_screen: u8,
    windows_12: u8,
    /// The backdrop colour a script set (`$7F:0600`, CGRAM 0).
    backdrop: Option<u16>,
    window_select: u8,
    math_select: u8,
    /// `CGADSUB`, once a script wrote it.
    math_layers: Option<u8>,
    /// The fixed colour's red, green and blue, 0 to 31.
    fixed: [u8; 3],
    /// `COP 6A`'s square, while it turns.
    spin: Option<Spin>,
    /// The palette buffer's steps toward white since it was saved
    /// (`$8D:A8EA`, `$8D:AA96`), until it is restored (`$8D:A8FD`).
    whitening: Option<u8>,
}

/// `COP 6A 00 speed`'s square (`$8D:AFA1`): centred on the actor that
/// started it, turned by `angle` (256 a turn), `speed` more each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spin {
    /// The centre, in map pixels.
    pub centre: (u16, u16),
    /// The turn added each frame.
    pub speed: u8,
    /// The turn now.
    pub angle: u8,
}

impl Spin {
    /// Whether the map pixel `at` is inside the square.
    #[must_use]
    pub fn contains(&self, at: (f64, f64)) -> bool {
        self.window()(at)
    }

    /// [`Self::contains`] with the turn worked out once, for many pixels.
    pub fn window(&self) -> impl Fn((f64, f64)) -> bool {
        let turn = f64::from(self.angle) * std::f64::consts::TAU / 256.0;
        let (sin, cos) = turn.sin_cos();
        let centre = (f64::from(self.centre.0), f64::from(self.centre.1));
        move |at| {
            let (dx, dy) = (at.0 - centre.0, at.1 - centre.1);
            let (u, v) = (dx * cos + dy * sin, dy * cos - dx * sin);
            u.abs() <= SQUARE && v.abs() <= SQUARE
        }
    }
}

/// The background's colour math as a host draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Darkening {
    /// Subtracted from each 5-bit channel, red, green, blue.
    pub fixed: [u8; 3],
    /// The square kept at full colour, if any.
    pub spared: Option<Spin>,
}

impl Display {
    /// A write of `value` to PPU register `register`. Returns whether the
    /// register is one modelled here.
    pub fn write(&mut self, register: u16, value: u8) -> bool {
        match register {
            MAIN_SCREEN => self.main_screen = Some(value),
            SUB_SCREEN => self.sub_screen = value,
            WINDOWS_12 => self.windows_12 = value,
            WINDOW_SELECT => self.window_select = value,
            MATH_SELECT => self.math_select = value,
            MATH_LAYERS => self.math_layers = Some(value),
            FIXED => {
                for (channel, fixed) in self.fixed.iter_mut().enumerate() {
                    if value & (0x20 << channel) != 0 {
                        *fixed = value & 31;
                    }
                }
            }
            _ => return false,
        }
        true
    }

    /// The backdrop colour a script wrote into the palette buffer
    /// (`$7F:0600`, the `$11D` guardian's `$90:A14D`).
    pub fn set_backdrop(&mut self, colour: u16) {
        self.backdrop = Some(colour);
    }

    /// The backdrop colour a script set, a BGR555 word.
    #[must_use]
    pub const fn backdrop(&self) -> Option<u16> {
        self.backdrop
    }

    /// The backdrop colour the sub screen's BG1 is added to inside BG1's
    /// window 1, where the main screen drops BG1: the `$11D` guardian's
    /// light (`$90:A0C6`; `TS` BG1, `CGWSEL` bit 1, `CGADSUB` addition on
    /// the backdrop, `W12SEL` BG1 window 1). Its window is the ring
    /// ([`crate::world::Ring`]).
    #[must_use]
    pub fn backdrop_addition(&self) -> Option<Bgr555> {
        let adds = self.math_layers.is_some_and(|layers| layers & 0xA0 == 0x20);
        let sub_bg1 = self.sub_screen & 1 != 0 && self.math_select & 2 != 0;
        let windowed = self.windows_12 & 2 != 0;
        (adds && sub_bg1 && windowed)
            .then_some(self.backdrop)
            .flatten()
            .map(Bgr555::new)
    }

    /// Starts `COP 6A`'s square on `centre`.
    pub fn spin(&mut self, centre: (u16, u16), speed: u8) {
        self.spin = Some(Spin {
            centre,
            speed,
            angle: 0,
        });
    }

    /// Stops the square (`$7E:46E6 = 0`).
    pub fn stop_spin(&mut self) {
        self.spin = None;
    }

    /// A frame: the square turns.
    pub fn tick(&mut self) {
        if let Some(spin) = &mut self.spin {
            spin.angle = spin.angle.wrapping_add(spin.speed);
        }
    }

    /// `$8D:A8EA`: the palette buffer is saved, for a whitening.
    pub fn save_palette(&mut self) {
        self.whitening = Some(0);
    }

    /// `$8D:AA96`: each channel of every colour one step toward white.
    pub fn raise_palette(&mut self) {
        self.whitening = Some(self.whitening.unwrap_or(0).saturating_add(1).min(31));
    }

    /// `$8D:A8FD`: the saved palette is back.
    pub fn restore_palette(&mut self) {
        self.whitening = None;
    }

    /// The palette's steps toward white, while a whitening lasts.
    #[must_use]
    pub const fn whitening(&self) -> Option<u8> {
        self.whitening
    }

    /// Whether `TM` keeps BG1, the rooms' light rays, on.
    #[must_use]
    pub fn shows_bg1(&self) -> bool {
        self.main_screen.is_none_or(|layers| layers & 1 != 0)
    }

    /// The background's darkening, if the fixed colour is subtracted. The
    /// colour window is the square when `WOBJSEL` enables W1 for colour
    /// and `CGWSEL` prevents math inside it. Sprites are left alone: Ark's
    /// palettes never take colour math.
    #[must_use]
    pub fn darkening(&self) -> Option<Darkening> {
        let subtract = self.math_layers.is_none_or(|layers| layers & 0x80 != 0);
        if !subtract || self.fixed == [0; 3] {
            return None;
        }
        let windowed = self.window_select & 0x30 == 0x20 && self.math_select & 0x30 == 0x20;
        Some(Darkening {
            fixed: self.fixed,
            spared: self.spin.filter(|_| windowed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coldata_sets_the_channels_its_bits_name() {
        let mut display = Display::default();
        assert!(display.write(FIXED, 0xE7));
        assert_eq!(display.fixed, [7; 3]);
        display.write(FIXED, 0x20 | 2);
        assert_eq!(display.fixed, [2, 7, 7]);
        assert!(!display.write(0x2100, 0x0F));
    }

    #[test]
    fn the_box_darkens_outside_its_square() {
        // `$88:ACFA`: COP 76 32 E7, 30 20, 25 21, 27 00, then COP 6A 00 03.
        let mut display = Display::default();
        for (register, value) in [(0x32, 0xE7), (0x30, 0x20), (0x25, 0x21), (0x27, 0)] {
            display.write(0x2100 | register, value);
        }
        display.spin((136, 384), 3);
        let darkening = display.darkening().unwrap();
        assert_eq!(darkening.fixed, [7; 3]);
        let spared = darkening.spared.unwrap();
        assert!(spared.contains((136.0, 384.0)));
        assert!(spared.contains((136.0 + 47.0, 384.0 + 47.0)));
        // An eighth of a turn later the corner is out, the diagonal's end in.
        for _ in 0..11 {
            display.tick();
        }
        let spared = display.darkening().unwrap().spared.unwrap();
        assert_eq!(spared.angle, 33);
        assert!(!spared.contains((136.0 + 47.0, 384.0 + 47.0)));
        assert!(spared.contains((136.0 + 64.0, 384.0)));
    }

    #[test]
    fn the_guardian_adds_the_sub_screens_bg1_to_the_backdrop_in_window_1() {
        // `$90:A0C6`: TM $15, TS $01, CGWSEL $82, CGADSUB $20, W12SEL $22;
        // then the backdrop `$7F:0600 = n * $0421`.
        let mut display = Display::default();
        for (register, value) in [(0x2C, 0x15), (0x2D, 0x01), (0x30, 0x82), (0x31, 0x20)] {
            display.write(0x2100 | register, value);
        }
        display.set_backdrop(0x0421);
        assert_eq!(display.backdrop_addition(), None, "no window yet");
        assert!(display.write(0x2123, 0x22));
        assert_eq!(
            display.backdrop_addition(),
            Some(assets::graphics::Bgr555::new(0x0421))
        );
        display.write(MATH_LAYERS, 0xA0);
        assert_eq!(display.backdrop_addition(), None, "subtraction");
    }

    #[test]
    fn the_fade_child_hides_bg1_and_darkens_everywhere_without_its_window() {
        // `$88:9CD8`: TM $16, CGWSEL $20, CGADSUB $A3, WOBJSEL $21; then the
        // ramp, and at its end WOBJSEL 0 at fixed colour 0.
        let mut display = Display::default();
        assert!(display.shows_bg1());
        for (register, value) in [(0x2C, 0x16), (0x30, 0x20), (0x31, 0xA3), (0x25, 0x21)] {
            display.write(0x2100 | register, value);
        }
        assert!(!display.shows_bg1());
        assert_eq!(display.darkening(), None, "fixed colour 0");
        display.write(FIXED, 0xE1);
        assert_eq!(
            display.darkening(),
            Some(Darkening {
                fixed: [1; 3],
                spared: None
            })
        );
        display.write(MATH_LAYERS, 0x23);
        assert_eq!(
            display.darkening(),
            None,
            "addition is not modelled (meta/issues/colour-addition.md)"
        );
    }
}
