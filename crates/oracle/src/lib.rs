//! Reference execution boundary around the vendored `LakeSnes` core.
//!
//! This crate is deliberately thin: it loads a validated [`rom::Rom`], steps
//! frames deterministically, exposes selected state for comparison, and
//! keeps every artifact in memory. There is no windowing, no audio device,
//! and no filesystem access here — callers own persistence.

use std::fmt;

use rom::Rom;
use sha2::{Digest, Sha256};

pub mod export;
pub mod replay;

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
        pub fn snes_vram(snes: *const Snes) -> *const u16;
        pub fn snes_cgram(snes: *const Snes) -> *const u16;
        pub fn snes_cpu_pc(snes: *const Snes) -> u16;
        pub fn snes_cpu_bank(snes: *const Snes) -> u8;
        pub fn snes_apu_ram(snes: *const Snes) -> *const u8;
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

/// `Rom` validates to exactly [`rom::Rom::IMAGE_SIZE`] bytes, and the core's
/// ROM length is an `i32`; a validated image therefore always fits, and no
/// fallible path in [`Session::new`] can leak the core allocation.
const _: () = assert!(
    rom::Rom::IMAGE_SIZE <= i32::MAX as usize,
    "validated ROM image fits the core's i32 length"
);

/// A headless reference session over a validated ROM.
pub struct Session {
    snes: *mut ffi::Snes,
    pixels: Vec<u8>,
    samples: Vec<i16>,
}

// The core holds no mutable global state (a CI check pins this on the
// vendored tree) and the session owns the pointer exclusively for its
// lifetime, so moving a session across threads is sound.
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
    /// Creates a session; the core hard-resets as part of ROM load.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::CoreRejectedRom`] if the vendored core
    /// refuses the image (distinct from rom-crate digest validation).
    ///
    /// # Panics
    ///
    /// Panics if the sample-buffer size does not fit an `i32`; it is a
    /// compile-time constant that always fits. The ROM length conversion
    /// is guaranteed by the const assertion on [`rom::Rom::IMAGE_SIZE`].
    pub fn new(rom: &Rom) -> Result<Self, SessionError> {
        unsafe {
            let snes = ffi::snes_init();
            let image = rom.image();
            let len = i32::try_from(image.len()).expect("validated ROM image fits i32");
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
            Ok(session)
        }
    }

    /// Advances exactly one frame, then flushes the framebuffer and audio
    /// samples into the session buffers, mirroring upstream's per-frame
    /// `playAudio()` + `renderScreen()` sequence.
    ///
    /// # Panics
    ///
    /// Panics if the sample-buffer size does not fit an `i32`; it is a
    /// compile-time constant that always fits.
    pub fn run_frame(&mut self) {
        unsafe {
            ffi::snes_runFrame(self.snes);
            let samples_per_frame = i32::try_from(SAMPLES_PER_FRAME).expect("fits i32");
            ffi::snes_setSamples(self.snes, self.samples.as_mut_ptr(), samples_per_frame);
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

    /// The last flushed framebuffer (XRGB8888, one `u32` per pixel,
    /// `FRAME_WIDTH x FRAME_HEIGHT`).
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The last frame's audio samples, interleaved stereo.
    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// The full 64 KiB VRAM image (16-bit words).
    #[must_use]
    pub fn vram(&self) -> Vec<u16> {
        let mut out = vec![0u16; 0x8000];
        unsafe {
            std::ptr::copy_nonoverlapping(ffi::snes_vram(self.snes), out.as_mut_ptr(), 0x8000);
        }
        out
    }

    /// The 256-entry CGRAM palette (15-bit colors, one `u16` each).
    #[must_use]
    pub fn cgram(&self) -> Vec<u16> {
        let mut out = vec![0u16; 0x100];
        unsafe {
            std::ptr::copy_nonoverlapping(ffi::snes_cgram(self.snes), out.as_mut_ptr(), 0x100);
        }
        out
    }

    /// The program counter the core's CPU is currently at.
    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        unsafe { ffi::snes_cpu_pc(self.snes) }
    }

    /// The program bank the core's CPU is currently executing in.
    #[must_use]
    pub fn cpu_bank(&self) -> u8 {
        unsafe { ffi::snes_cpu_bank(self.snes) }
    }

    /// The full 64 KiB SPC RAM image (the audio driver's workspace).
    #[must_use]
    pub fn apu_ram(&self) -> Vec<u8> {
        let mut out = vec![0u8; 0x10000];
        unsafe {
            std::ptr::copy_nonoverlapping(ffi::snes_apu_ram(self.snes), out.as_mut_ptr(), 0x10000);
        }
        out
    }

    /// A full 128 KiB WRAM image snapshot.
    #[must_use]
    pub fn wram_image(&self) -> Vec<u8> {
        let mut out = vec![0u8; 0x20000];
        unsafe {
            std::ptr::copy_nonoverlapping(ffi::snes_ram(self.snes), out.as_mut_ptr(), 0x20000);
        }
        out
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
        let digest = Sha256::digest(self.wram_image());
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

/// Saves the core state to an opaque byte vector for replay resumption.
///
/// # Panics
///
/// Panics if the state exceeds an internal bound (allocation guard).
#[must_use]
pub fn save_state(session: &Session) -> Vec<u8> {
    unsafe {
        // LakeSnes returns the used size; a NULL probe is not offered, so
        // allocate the documented maximum and truncate.
        let mut buf = vec![0u8; STATE_SIZE_MAX];
        let n = ffi::snes_saveState(session.snes, buf.as_mut_ptr());
        assert!(n >= 0, "state save failed");
        let n = usize::try_from(n).expect("positive size");
        assert!(n <= STATE_SIZE_MAX, "state exceeded allocation");
        buf.truncate(n);
        buf
    }
}

/// Loads a core state previously saved by [`save_state`].
///
/// # Panics
///
/// Panics if the core rejects the payload.
pub fn load_state(session: &mut Session, data: &[u8]) {
    let len = i32::try_from(data.len()).expect("state size fits i32");
    let ok = unsafe { ffi::snes_loadState(session.snes, data.as_ptr(), len) };
    assert!(ok, "core rejected state payload");
}

/// Internal bound matching the core's documented maximum state size.
const STATE_SIZE_MAX: usize = 4 * 1024 * 1024;

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

    #[test]
    #[should_panic(expected = "WRAM offset out of range")]
    fn wram_panics_past_the_image() {
        let rom = synthetic_rom();
        let s = Session::new(&rom).expect("core accepts image");
        let _ = s.wram(0x20000);
    }

    #[test]
    fn graphics_memory_is_full_size_and_deterministic() {
        let rom = synthetic_rom();
        let run = |frames: usize| -> (Vec<u16>, Vec<u16>) {
            let mut s = Session::new(&rom).expect("core accepts image");
            s.run_frames(frames);
            (s.vram(), s.cgram())
        };
        let (vram_a, cgram_a) = run(10);
        let (vram_b, cgram_b) = run(10);
        assert_eq!(vram_a.len(), 0x8000);
        assert_eq!(cgram_a.len(), 0x100);
        assert_eq!(vram_a, vram_b);
        assert_eq!(cgram_a, cgram_b);
    }

    #[test]
    fn cpu_pc_is_exposed_and_deterministic() {
        let rom = synthetic_rom();
        let run = || -> (u16, u8) {
            let mut s = Session::new(&rom).expect("core accepts image");
            s.run_frames(10);
            (s.cpu_pc(), s.cpu_bank())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn spc_ram_is_full_size_and_deterministic() {
        let rom = synthetic_rom();
        let run = || -> Vec<u8> {
            let mut s = Session::new(&rom).expect("core accepts image");
            s.run_frames(10);
            s.apu_ram()
        };
        let a = run();
        let b = run();
        assert_eq!(a.len(), 0x10000);
        assert_eq!(a, b);
    }
}
