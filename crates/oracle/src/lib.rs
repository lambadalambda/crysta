//! Reference execution boundary around the vendored `LakeSnes` core.
//!
//! This crate is deliberately thin: it loads a validated [`rom::Rom`], steps
//! frames deterministically, exposes selected state for comparison, and
//! keeps every artifact in memory. There is no windowing, no audio device,
//! and no filesystem access here — callers own persistence.

use std::fmt;

use rom::Rom;
use sha2::{Digest, Sha256};

mod ffi {
    #![allow(non_snake_case, non_camel_case_types, dead_code)]

    use std::os::raw::c_int;

    /// Opaque SNES instance from the vendored core.
    #[repr(C)]
    pub struct Snes {
        _private: [u8; 0],
    }

    pub const PIXEL_FORMAT_XRGB: c_int = 0;
    pub const BUTTON_SELECT: c_int = 2;
    pub const BUTTON_START: c_int = 3;
    pub const BUTTON_UP: c_int = 4;
    pub const BUTTON_DOWN: c_int = 5;
    pub const BUTTON_LEFT: c_int = 6;
    pub const BUTTON_RIGHT: c_int = 7;
    pub const BUTTON_A: c_int = 8;
    pub const BUTTON_B: c_int = 0;
    pub const BUTTON_X: c_int = 9;
    pub const BUTTON_Y: c_int = 1;
    pub const BUTTON_L: c_int = 10;
    pub const BUTTON_R: c_int = 11;

    extern "C" {
        pub fn snes_init() -> *mut Snes;
        pub fn snes_free(snes: *mut Snes);
        pub fn snes_reset(snes: *mut Snes, hard: bool);
        pub fn snes_runFrame(snes: *mut Snes);
        pub fn snes_loadRom(snes: *mut Snes, data: *const u8, length: c_int) -> bool;
        pub fn snes_setPixelFormat(snes: *mut Snes, pixelFormat: c_int);
        pub fn snes_setPixels(snes: *mut Snes, pixelData: *mut u8);
        pub fn snes_setSamples(snes: *mut Snes, sampleData: *mut i16, samplesPerFrame: c_int);
        pub fn snes_setButtonState(snes: *mut Snes, player: c_int, button: c_int, pressed: bool);
        pub fn snes_saveState(snes: *mut Snes, data: *mut u8) -> c_int;
        pub fn snes_loadState(snes: *mut Snes, data: *const u8, size: c_int) -> bool;
        // shim accessors (vendor/lakesnes/shims.c)
        pub fn snes_ram(snes: *const Snes) -> *const u8;
        pub fn snes_frames(snes: *const Snes) -> u32;
        pub fn snes_cycles(snes: *const Snes) -> u64;
    }
}

/// Framebuffer width in pixels.
pub const FRAME_WIDTH: usize = 512;
/// Framebuffer height in pixels.
pub const FRAME_HEIGHT: usize = 480;
/// Samples per frame, stereo (`LakeSnes` NTSC upper bound).
pub const SAMPLES_PER_FRAME: usize = 534;

/// A controller button for [`Session::set_button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// SNES B button.
    B,
    /// SNES Y button.
    Y,
    /// SNES Select.
    Select,
    /// SNES Start.
    Start,
    /// D-pad up.
    Up,
    /// D-pad down.
    Down,
    /// D-pad left.
    Left,
    /// D-pad right.
    Right,
    /// SNES A button.
    A,
    /// SNES X button.
    X,
    /// Left shoulder.
    L,
    /// Right shoulder.
    R,
}

impl Button {
    fn as_c(self) -> std::os::raw::c_int {
        match self {
            Self::B => ffi::BUTTON_B,
            Self::Y => ffi::BUTTON_Y,
            Self::Select => ffi::BUTTON_SELECT,
            Self::Start => ffi::BUTTON_START,
            Self::Up => ffi::BUTTON_UP,
            Self::Down => ffi::BUTTON_DOWN,
            Self::Left => ffi::BUTTON_LEFT,
            Self::Right => ffi::BUTTON_RIGHT,
            Self::A => ffi::BUTTON_A,
            Self::X => ffi::BUTTON_X,
            Self::L => ffi::BUTTON_L,
            Self::R => ffi::BUTTON_R,
        }
    }
}

/// A headless reference session over a validated ROM.
pub struct Session {
    snes: *mut ffi::Snes,
    pixels: Vec<u8>,
    samples: Vec<i16>,
}

// The core keeps no threads and we never share the pointer; the session owns
// it exclusively for its lifetime.
unsafe impl Send for Session {}

/// Errors from reference-session use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The core rejected the ROM image.
    CoreRejectedRom,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreRejectedRom => write!(f, "the emulator core rejected the ROM image"),
        }
    }
}

impl std::error::Error for SessionError {}

/// State snapshot of selected semantic fields for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameState {
    /// Frame counter reported by the core.
    pub frames: u32,
    /// Total CPU cycles elapsed.
    pub cycles: u64,
    /// SHA-256 over the current 128 KiB WRAM image.
    pub wram_sha256: [u8; 32],
}

impl Session {
    /// Creates a session and hard-resets the core.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::CoreRejectedRom`] if the vendored core
    /// refuses the image (distinct from rom-crate digest validation).
    ///
    /// # Panics
    ///
    /// Panics if the sample-buffer size does not fit an `i32`; it is a
    /// compile-time constant that always fits.
    pub fn new(rom: &Rom) -> Result<Self, SessionError> {
        unsafe {
            let snes = ffi::snes_init();
            let image = rom.image();
            let len = i32::try_from(image.len()).map_err(|_| SessionError::CoreRejectedRom)?;
            let ok = ffi::snes_loadRom(snes, image.as_ptr(), len);
            if !ok {
                ffi::snes_free(snes);
                return Err(SessionError::CoreRejectedRom);
            }
            let mut session = Self {
                snes,
                pixels: vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4],
                samples: vec![0; SAMPLES_PER_FRAME * 2],
            };
            ffi::snes_setPixelFormat(snes, ffi::PIXEL_FORMAT_XRGB);
            ffi::snes_setPixels(snes, session.pixels.as_mut_ptr());
            let samples_per_frame = i32::try_from(SAMPLES_PER_FRAME).expect("fits i32");
            ffi::snes_setSamples(snes, session.samples.as_mut_ptr(), samples_per_frame);
            ffi::snes_reset(snes, true);
            Ok(session)
        }
    }

    /// Advances exactly one frame and flushes the framebuffer.
    ///
    /// `snes_setPixels` performs the copy from the core's internal line
    /// buffer into the caller's buffer (the interactive frontend calls it
    /// once per rendered frame), so we re-invoke it here.
    pub fn run_frame(&mut self) {
        unsafe {
            ffi::snes_runFrame(self.snes);
            ffi::snes_setPixels(self.snes, self.pixels.as_mut_ptr());
        }
    }

    /// Advances `n` frames.
    pub fn run_frames(&mut self, n: usize) {
        for _ in 0..n {
            self.run_frame();
        }
    }

    /// Sets a button state for player 1; applies to subsequent frames.
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        unsafe { ffi::snes_setButtonState(self.snes, 1, button.as_c(), pressed) };
    }

    /// The last flushed framebuffer, XRGB byte order, `FRAME_WIDTH x FRAME_HEIGHT`.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The last frame's audio samples, interleaved stereo.
    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Reads a byte from WRAM (`$7E:0000`–`$7F:FFFF`).
    ///
    /// # Panics
    ///
    /// Panics if `offset` is beyond the 128 KiB WRAM image.
    #[must_use]
    pub fn wram(&self, offset: usize) -> u8 {
        assert!(offset < 0x20000, "WRAM offset out of range");
        unsafe { *ffi::snes_ram(self.snes).add(offset) }
    }

    /// Semantic state for comparison at the current frame boundary.
    #[must_use]
    pub fn frame_state(&self) -> FrameState {
        let mut wram = vec![0u8; 0x20000];
        unsafe {
            std::ptr::copy_nonoverlapping(ffi::snes_ram(self.snes), wram.as_mut_ptr(), 0x20000);
        }
        let digest = Sha256::digest(&wram);
        FrameState {
            frames: unsafe { ffi::snes_frames(self.snes) },
            cycles: unsafe { ffi::snes_cycles(self.snes) },
            wram_sha256: digest.into(),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { ffi::snes_free(self.snes) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_rom() -> Rom {
        // Smallest thing the core accepts: a headerless 4 MiB image whose
        // digests we inject, so no copyrighted content is needed.
        let image = vec![0u8; Rom::IMAGE_SIZE];
        let d = rom::digests(&image);
        let known = vec![rom::KnownRom {
            revision: rom::Revision::Japan,
            sha256: d.sha256,
            crc32: d.crc32,
        }];
        Rom::load_with_known(&image, &known).expect("synthetic rom loads")
    }

    #[test]
    fn session_boots_synthetic_rom_and_steps_deterministically() {
        let rom = synthetic_rom();
        let mut a = Session::new(&rom).expect("core accepts image");
        a.run_frames(10);
        let sa = a.frame_state();

        let mut b = Session::new(&rom).expect("core accepts image");
        b.run_frames(10);
        let sb = b.frame_state();

        assert_eq!(sa.frames, sb.frames);
        assert_eq!(sa.cycles, sb.cycles);
        assert_eq!(sa.wram_sha256, sb.wram_sha256);
    }

    #[test]
    fn wram_access_bounded() {
        let rom = synthetic_rom();
        let s = Session::new(&rom).expect("core accepts image");
        let _ = s.wram(0x1FFFF);
    }
}
