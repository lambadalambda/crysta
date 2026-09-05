//! Reference execution boundary around the vendored ares core.
//!
//! This crate is deliberately thin: it loads a validated [`rom::Rom`], steps
//! frames deterministically, exposes selected state for comparison, and
//! keeps every artifact in memory. There is no windowing, no audio device,
//! and no filesystem access here — callers own persistence.

use std::fmt;
use std::sync::Mutex;

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

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct SnesCpuTraceEntry {
        pub address: u32,
        pub direct_page: u16,
        pub status: u8,
        pub data_bank: u8,
        pub emulation: u8,
        pub reserved: [u8; 3],
    }

    const _: () = assert!(std::mem::size_of::<SnesCpuTraceEntry>() == 12);
    const _: () = assert!(std::mem::offset_of!(SnesCpuTraceEntry, direct_page) == 4);
    const _: () = assert!(std::mem::offset_of!(SnesCpuTraceEntry, status) == 6);
    const _: () = assert!(std::mem::offset_of!(SnesCpuTraceEntry, data_bank) == 7);
    const _: () = assert!(std::mem::offset_of!(SnesCpuTraceEntry, emulation) == 8);

    #[repr(C)]
    #[derive(Default)]
    pub struct SnesCpuRegisters {
        pub address: u32,
        pub accumulator: u16,
        pub x: u16,
        pub y: u16,
        pub stack: u16,
        pub direct_page: u16,
        pub status: u8,
        pub data_bank: u8,
        pub emulation: u8,
        pub reserved: [u8; 3],
    }

    const _: () = assert!(std::mem::size_of::<SnesCpuRegisters>() == 20);
    const _: () = assert!(std::mem::align_of::<SnesCpuRegisters>() == 4);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, address) == 0);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, accumulator) == 4);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, x) == 6);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, y) == 8);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, stack) == 10);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, direct_page) == 12);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, status) == 14);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, data_bank) == 15);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, emulation) == 16);
    const _: () = assert!(std::mem::offset_of!(SnesCpuRegisters, reserved) == 17);

    pub const TRACE_TARGET_REACHED: c_int = 0;
    pub const TRACE_INSTRUCTION_LIMIT: c_int = 1;
    pub const TRACE_FRAME_LIMIT: c_int = 2;

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
        pub fn snes_traceUntil(
            snes: *mut Snes,
            target: u32,
            instructionLimit: u32,
            frameLimit: u32,
            entries: *mut SnesCpuTraceEntry,
            count: *mut u32,
        ) -> c_int;
        pub fn snes_loadRomWithSram(
            snes: *mut Snes,
            data: *const u8,
            length: c_int,
            sramData: *const u8,
            sramLength: c_int,
        ) -> bool;
        pub fn snes_setPixelFormat(snes: *mut Snes, pixelFormat: c_int);
        pub fn snes_setPixels(snes: *mut Snes, pixelData: *mut u8);
        pub fn snes_setSamples(snes: *mut Snes, sampleData: *mut i16, samplesPerFrame: c_int);
        pub fn snes_setButtonState(snes: *mut Snes, player: c_int, button: c_int, pressed: bool);
        pub fn snes_saveState(snes: *mut Snes, data: *mut u8) -> c_int;
        pub fn snes_loadState(snes: *mut Snes, data: *const u8, size: c_int) -> bool;
        // project-authored ares shim accessors (vendor/ares/shims.cpp)
        pub fn snes_ram(snes: *const Snes) -> *const u8;
        pub fn snes_frames(snes: *const Snes) -> u32;
        pub fn snes_cycles(snes: *const Snes) -> u64;
        pub fn snes_vram(snes: *const Snes) -> *const u16;
        pub fn snes_cgram(snes: *const Snes) -> *const u16;
        pub fn snes_cpu_registers(snes: *const Snes, registers: *mut SnesCpuRegisters);
        pub fn snes_cpu_pc(snes: *const Snes) -> u16;
        pub fn snes_cpu_bank(snes: *const Snes) -> u8;
        pub fn snes_apu_ram(snes: *const Snes) -> *const u8;
    }
}

/// Framebuffer width in pixels.
pub const FRAME_WIDTH: usize = 512;
/// Framebuffer height in pixels.
pub const FRAME_HEIGHT: usize = 480;
/// Samples per frame, stereo (legacy shim boundary capacity).
pub const SAMPLES_PER_FRAME: usize = 534;
/// Size in bytes of a cartridge SRAM image accepted by [`Session::new_with_sram`].
pub const SRAM_SIZE: usize = 8 * 1024;
/// Maximum records accepted by one bounded CPU trace.
pub const MAX_CPU_TRACE_INSTRUCTIONS: usize = 2_000_000;

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
/// ROM and SRAM lengths use `i32`; both validated images therefore fit.
const _: () = assert!(
    rom::Rom::IMAGE_SIZE <= i32::MAX as usize,
    "validated ROM image fits the core's i32 length"
);
const _: () = assert!(
    SRAM_SIZE <= i32::MAX as usize,
    "validated SRAM image fits the core's i32 length"
);

static ZEROED_SRAM: [u8; SRAM_SIZE] = [0; SRAM_SIZE];

// Serializes constructors and prevents safe Rust from entering ares's
// process-global initialization more than once.
static SESSION_CLAIMED: Mutex<bool> = Mutex::new(false);

/// A headless reference session over a validated ROM.
pub struct Session {
    snes: *mut ffi::Snes,
    pixels: Vec<u8>,
    samples: Vec<i16>,
}

// Deliberately NOT `Send`: the ares core keeps a process-global platform
// pointer, so a session may only be used from the thread that created it.
// The Rust type system enforces this (raw pointer members are `!Send`).

/// Errors from reference-session use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The SRAM image was not exactly [`SRAM_SIZE`] bytes.
    InvalidSramLength {
        /// Required SRAM image size.
        expected: usize,
        /// Supplied SRAM image size.
        actual: usize,
    },
    /// A session already owns the process-global ares core.
    AlreadyBooted,
    /// The core rejected the ROM image.
    CoreRejectedRom,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSramLength { expected, actual } => {
                write!(
                    f,
                    "SRAM image is {actual} bytes; expected exactly {expected} bytes"
                )
            }
            Self::AlreadyBooted => write!(f, "an ares session is already booted in this process"),
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

/// Read-only snapshot of the reference CPU's current registers.
///
/// A, X, Y and S retain their full 16-bit register contents, regardless of mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuRegisters {
    /// Actual 24-bit execution address (`PBR:PC`), without mirror normalization.
    pub address: u32,
    /// Full accumulator (including the hidden high byte in 8-bit mode).
    pub accumulator: u16,
    /// X index register.
    pub x: u16,
    /// Y index register.
    pub y: u16,
    /// Stack pointer.
    pub stack: u16,
    /// Direct-page register.
    pub direct_page: u16,
    /// Processor status register.
    pub status: u8,
    /// Data-bank register.
    pub data_bank: u8,
    /// Whether the CPU is in emulation mode.
    pub emulation: bool,
}

/// CPU state captured immediately before one instruction executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTraceEntry {
    /// Actual 24-bit execution address (`PBR:PC`), without mirror normalization.
    pub address: u32,
    /// Processor status register.
    pub status: u8,
    /// Whether the CPU is in emulation mode.
    pub emulation: bool,
    /// Direct-page register.
    pub direct_page: u16,
    /// Data-bank register.
    pub data_bank: u8,
}

/// Why a bounded CPU trace stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuTraceStop {
    /// The requested target instruction was reached and included in the trace.
    TargetReached,
    /// The requested maximum number of instructions was captured.
    InstructionLimit,
    /// The requested maximum number of video frames elapsed.
    FrameLimit,
}

/// Version of the canonical structured CPU-trace digest stream.
pub const CPU_TRACE_FORMAT_VERSION: u32 = 1;

/// Result of one bounded CPU trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTrace {
    /// Structured pre-instruction CPU states, including the target when reached.
    pub entries: Vec<CpuTraceEntry>,
    /// Why tracing stopped.
    pub stop: CpuTraceStop,
}

impl CpuTrace {
    /// Returns the canonical lowercase SHA-256 for this trace's structured
    /// records. The stream is domain-separated and includes its format version
    /// and record count before the fixed-width entries.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(b"terranigma.cpu-trace\0");
        digest.update(CPU_TRACE_FORMAT_VERSION.to_le_bytes());
        digest.update((self.entries.len() as u64).to_le_bytes());
        for entry in &self.entries {
            digest.update(entry.address.to_le_bytes());
            digest.update([entry.status, u8::from(entry.emulation)]);
            digest.update(entry.direct_page.to_le_bytes());
            digest.update([entry.data_bank]);
        }
        export::bytes_to_hex(&digest.finalize())
    }
}

/// Invalid bounds or core failures from [`Session::trace_until_pc`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    /// A 24-bit target address was exceeded.
    TargetOutOfRange(u32),
    /// At least one instruction must be allowed.
    ZeroInstructionLimit,
    /// The requested instruction capacity does not fit the shim ABI.
    InstructionLimitTooLarge(usize),
    /// At least one frame must be allowed.
    ZeroFrameLimit,
    /// The reference core rejected otherwise validated tracing parameters.
    CoreFailure(i32),
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetOutOfRange(target) => {
                write!(
                    f,
                    "CPU trace target ${target:X} exceeds 24-bit address space"
                )
            }
            Self::ZeroInstructionLimit => write!(f, "CPU trace instruction limit must be nonzero"),
            Self::InstructionLimitTooLarge(limit) => {
                write!(
                    f,
                    "CPU trace instruction limit {limit} exceeds maximum {MAX_CPU_TRACE_INSTRUCTIONS}"
                )
            }
            Self::ZeroFrameLimit => write!(f, "CPU trace frame limit must be nonzero"),
            Self::CoreFailure(code) => write!(f, "reference core trace failed with code {code}"),
        }
    }
}

impl std::error::Error for TraceError {}

fn validate_sram(sram: &[u8]) -> Result<&[u8; SRAM_SIZE], SessionError> {
    sram.try_into()
        .map_err(|_| SessionError::InvalidSramLength {
            expected: SRAM_SIZE,
            actual: sram.len(),
        })
}

impl Session {
    /// Creates a session with zero-initialized 8 KiB cartridge SRAM.
    /// The core hard-resets as part of ROM load.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::AlreadyBooted`] if any session construction has
    /// already entered the process-global ares core, or
    /// [`SessionError::CoreRejectedRom`] if the core refuses the image
    /// (distinct from rom-crate digest validation).
    ///
    /// # Panics
    ///
    /// Panics if the sample-buffer size does not fit an `i32`; it is a
    /// compile-time constant that always fits. The ROM and SRAM length
    /// conversions are guaranteed by their const assertions.
    pub fn new(rom: &Rom) -> Result<Self, SessionError> {
        Self::new_with_sram(rom, &ZEROED_SRAM)
    }

    /// Creates a session with the supplied 8 KiB cartridge SRAM image.
    /// SRAM is copied synchronously before the emulated system powers on.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::InvalidSramLength`] if `sram` is not exactly
    /// [`SRAM_SIZE`] bytes. This validation occurs before claiming the
    /// process-global core. Otherwise, returns the same errors as [`Self::new`].
    ///
    /// # Panics
    ///
    /// Panics only for compile-time-bounded length conversions; see [`Self::new`].
    pub fn new_with_sram(rom: &Rom, sram: &[u8]) -> Result<Self, SessionError> {
        let sram = validate_sram(sram)?;
        Self::boot(rom, sram)
    }

    fn boot(rom: &Rom, sram: &[u8; SRAM_SIZE]) -> Result<Self, SessionError> {
        let mut claimed = SESSION_CLAIMED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *claimed {
            return Err(SessionError::AlreadyBooted);
        }
        // ares cannot safely recover or reinitialize after construction starts.
        *claimed = true;

        unsafe {
            let snes = ffi::snes_init();
            let image = rom.image();
            let image_len = i32::try_from(image.len()).expect("validated ROM image fits i32");
            let sram_len = i32::try_from(sram.len()).expect("validated SRAM image fits i32");
            let ok =
                ffi::snes_loadRomWithSram(snes, image.as_ptr(), image_len, sram.as_ptr(), sram_len);
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

    /// Captures bounded pre-instruction CPU state until `target` is reached.
    ///
    /// The target entry is included, but that instruction has not executed when
    /// this method returns. Addresses preserve the core's actual execution
    /// mirror rather than being normalized to the canonical disassembly bank.
    ///
    /// # Errors
    ///
    /// Returns [`TraceError`] for a target above 24 bits, zero bounds, an
    /// instruction limit that does not fit the shim ABI, or an unexpected core
    /// failure.
    pub fn trace_until_pc(
        &mut self,
        target: u32,
        instruction_limit: usize,
        frame_limit: u32,
    ) -> Result<CpuTrace, TraceError> {
        if target > 0xFF_FFFF {
            return Err(TraceError::TargetOutOfRange(target));
        }
        if instruction_limit == 0 {
            return Err(TraceError::ZeroInstructionLimit);
        }
        if instruction_limit > MAX_CPU_TRACE_INSTRUCTIONS {
            return Err(TraceError::InstructionLimitTooLarge(instruction_limit));
        }
        let instruction_limit = u32::try_from(instruction_limit)
            .map_err(|_| TraceError::InstructionLimitTooLarge(instruction_limit))?;
        if frame_limit == 0 {
            return Err(TraceError::ZeroFrameLimit);
        }

        let mut raw = vec![ffi::SnesCpuTraceEntry::default(); instruction_limit as usize];
        let mut count = 0u32;
        let stop = unsafe {
            ffi::snes_traceUntil(
                self.snes,
                target,
                instruction_limit,
                frame_limit,
                raw.as_mut_ptr(),
                &raw mut count,
            )
        };
        if count > instruction_limit {
            return Err(TraceError::CoreFailure(stop));
        }
        raw.truncate(count as usize);

        let stop = match stop {
            ffi::TRACE_TARGET_REACHED => CpuTraceStop::TargetReached,
            ffi::TRACE_INSTRUCTION_LIMIT => CpuTraceStop::InstructionLimit,
            ffi::TRACE_FRAME_LIMIT => CpuTraceStop::FrameLimit,
            code => return Err(TraceError::CoreFailure(code)),
        };
        let entries = raw
            .into_iter()
            .map(|entry| CpuTraceEntry {
                address: entry.address,
                status: entry.status,
                emulation: entry.emulation != 0,
                direct_page: entry.direct_page,
                data_bank: entry.data_bank,
            })
            .collect();
        Ok(CpuTrace { entries, stop })
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

    /// Reads the current CPU registers without advancing execution or changing memory.
    ///
    /// After [`Self::trace_until_pc`] reaches its target, this describes the CPU
    /// before that instruction executes. This snapshot is separate from the
    /// stable trace record ABI and digest format.
    #[must_use]
    pub fn cpu_registers(&self) -> CpuRegisters {
        let mut raw = ffi::SnesCpuRegisters::default();
        // SAFETY: this session owns the initialized core on its creating thread;
        // the shim only reads registers and fills the layout-checked output.
        unsafe { ffi::snes_cpu_registers(self.snes, &raw mut raw) };
        CpuRegisters {
            address: raw.address,
            accumulator: raw.accumulator,
            x: raw.x,
            y: raw.y,
            stack: raw.stack,
            direct_page: raw.direct_page,
            status: raw.status,
            data_bank: raw.data_bank,
            emulation: raw.emulation != 0,
        }
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
        // The shim returns the used size; a NULL probe is not offered, so
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
        // A minimum valid cart image with a reset prefix that enters native
        // mode, long-jumps to bank $80, and then spins at $80:8000.
        let mut image = vec![0u8; Rom::IMAGE_SIZE];
        // SEI; CLC; XCE; JML $80:8000.
        let reset = [0x78, 0x18, 0xFB, 0x5C, 0x00, 0x80, 0x80];
        image[0xFFC0..0xFFC0 + reset.len()].copy_from_slice(&reset);
        // BRA $80:8000.
        image[0x8000..0x8002].copy_from_slice(&[0x80, 0xFE]);
        // NMI/IRQ/RESET vectors all land on the spin loop.
        image[0xFFEA] = 0xC0;
        image[0xFFEB] = 0xFF;
        image[0xFFEC] = 0xC0;
        image[0xFFED] = 0xFF;
        image[0xFFEE] = 0xC0;
        image[0xFFEF] = 0xFF;
        image[0xFFFC] = 0xC0;
        image[0xFFFD] = 0xFF;
        let d = rom::digests(&image);
        let known = vec![rom::KnownRom {
            revision: rom::Revision::Japan,
            sha256: d.sha256,
            crc32: d.crc32,
        }];
        Rom::load_with_known(&image, &known).expect("synthetic rom loads")
    }

    #[test]
    fn sram_length_validation_is_rom_free_and_does_not_claim() {
        for actual in [0, SRAM_SIZE - 1, SRAM_SIZE + 1] {
            let sram = vec![0; actual];
            assert!(matches!(
                validate_sram(&sram),
                Err(SessionError::InvalidSramLength {
                    expected: SRAM_SIZE,
                    actual: found
                }) if found == actual
            ));
        }

        let claimed = SESSION_CLAIMED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            !*claimed,
            "SRAM validation must not claim the ares singleton"
        );
    }

    #[test]
    fn supplied_sram_is_visible_at_boot_and_default_is_zeroed() {
        if let Ok(mode) = std::env::var("ORACLE_SRAM_CHILD") {
            run_sram_child(&mode);
        }

        let exe = std::env::current_exe().expect("current test executable");
        for mode in ["supplied", "zeroed"] {
            let output = std::process::Command::new(&exe)
                .args([
                    "--exact",
                    "tests::supplied_sram_is_visible_at_boot_and_default_is_zeroed",
                    "--nocapture",
                ])
                .env("ORACLE_SRAM_CHILD", mode)
                .output()
                .expect("spawn isolated SRAM test");
            assert!(
                output.status.success(),
                "isolated {mode} SRAM test failed\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("synthetic SRAM check passed"),
                "{mode} child must reach the end of its assertions"
            );
        }
    }

    fn run_sram_child(mode: &str) -> ! {
        let mut image = synthetic_rom().image().to_vec();
        // LDA $20:6000; STA $7E:0000; BRA $80:8000.
        image[0x8000..0x800A]
            .copy_from_slice(&[0xAF, 0x00, 0x60, 0x20, 0x8F, 0x00, 0x00, 0x7E, 0x80, 0xF6]);
        let digest = rom::digests(&image);
        let rom = Rom::load_with_known(
            &image,
            &[rom::KnownRom {
                revision: rom::Revision::Japan,
                sha256: digest.sha256,
                crc32: digest.crc32,
            }],
        )
        .expect("synthetic SRAM ROM loads");

        let expected = 0xA5;
        let mut sram = vec![0; SRAM_SIZE];
        sram[0] = expected;
        let mut session = match mode {
            "supplied" => {
                for actual in [0, SRAM_SIZE - 1] {
                    assert!(matches!(
                        Session::new_with_sram(&rom, &sram[..actual]),
                        Err(SessionError::InvalidSramLength {
                            expected: SRAM_SIZE,
                            actual: found
                        }) if found == actual
                    ));
                }
                Session::new_with_sram(&rom, &sram).expect("core accepts supplied SRAM")
            }
            "zeroed" => Session::new(&rom).expect("core accepts zeroed SRAM"),
            _ => panic!("unexpected SRAM child mode"),
        };
        // The constructor promises to copy SRAM synchronously.
        sram.fill(0x5A);
        session.run_frame();
        assert_eq!(
            session.wram(0),
            if mode == "supplied" { expected } else { 0 }
        );

        eprintln!("synthetic SRAM check passed");
        std::process::exit(0);
    }

    #[test]
    fn cpu_registers_at_instruction_stop_are_read_only() {
        if std::env::var("ORACLE_REGISTERS_CHILD").is_ok() {
            run_registers_child();
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::cpu_registers_at_instruction_stop_are_read_only",
                "--nocapture",
            ])
            .env("ORACLE_REGISTERS_CHILD", "1")
            .output()
            .expect("spawn isolated register test");
        assert!(
            output.status.success(),
            "register child failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("synthetic register checks passed")
        );
    }

    fn run_registers_child() -> ! {
        let mut image = synthetic_rom().image().to_vec();
        // The reset prefix already enters native mode and jumps to $80:8000.
        let program = [
            0xC2, 0x30, // REP #$30: 16-bit A/X/Y
            0xA9, 0x34, 0x12, 0x5B, // LDA #$1234; TCD
            0xA9, 0xED, 0x1F, 0x1B, // LDA #$1FED; TCS
            0xE2, 0x20, 0xA9, 0x7E, 0x48, 0xAB, // SEP #$20; LDA #$7E; PHA; PLB
            0xC2, 0x20, 0xA9, 0x5B, 0xA6, // REP #$20; LDA #$A65B
            0xA2, 0x78, 0xC3, // LDX #$C378
            0xA0, 0xBC, 0x9A, // LDY #$9ABC
            0xE2, 0xC9, // SEP #$C9: N/V/D/C set, I remains set, M/X clear
        ];
        image[0x8000..0x8000 + program.len()].copy_from_slice(&program);
        // Stop BEFORE this store, then resume to prove it had not executed.
        image[0x8000 + program.len()..0x8000 + program.len() + 6]
            .copy_from_slice(&[0x8F, 0x00, 0x00, 0x7E, 0x80, 0xFE]);
        let digest = rom::digests(&image);
        let rom = Rom::load_with_known(
            &image,
            &[rom::KnownRom {
                revision: rom::Revision::Japan,
                sha256: digest.sha256,
                crc32: digest.crc32,
            }],
        )
        .expect("synthetic register ROM loads");
        let mut session = Session::new(&rom).expect("core accepts image");
        let target = 0x80_8000 + u32::try_from(program.len()).unwrap();
        let trace = session.trace_until_pc(target, 64, 1).unwrap();
        assert_eq!(trace.stop, CpuTraceStop::TargetReached);
        let frame = session.frame_state();
        let memory = session.wram_image();
        let pc = (session.cpu_bank(), session.cpu_pc());
        let expected = CpuRegisters {
            address: target,
            accumulator: 0xA65B,
            x: 0xC378,
            y: 0x9ABC,
            stack: 0x1FED,
            direct_page: 0x1234,
            status: 0xCD,
            data_bank: 0x7E,
            emulation: false,
        };
        for _ in 0..3 {
            let registers = session.cpu_registers();
            assert_eq!(registers, expected);
            assert_eq!(session.frame_state(), frame);
            assert_eq!(session.wram_image(), memory);
            assert_eq!((session.cpu_bank(), session.cpu_pc()), pc);
            assert_eq!(registers.address, (u32::from(pc.0) << 16) | u32::from(pc.1));
            assert_eq!(
                trace.entries.last(),
                Some(&CpuTraceEntry {
                    address: registers.address,
                    status: registers.status,
                    emulation: registers.emulation,
                    direct_page: registers.direct_page,
                    data_bank: registers.data_bank,
                })
            );
        }
        assert_ne!(&memory[..2], &[0x5B, 0xA6]);
        let resumed = session.trace_until_pc(target + 4, 2, 1).unwrap();
        assert_eq!(resumed.stop, CpuTraceStop::TargetReached);
        assert_eq!(&session.wram_image()[..2], &[0x5B, 0xA6]);
        eprintln!("synthetic register checks passed");
        // Avoid the vendored core's static destructors, as in other child tests.
        std::process::exit(0);
    }

    #[test]
    fn trace_digest_v1_is_stable() {
        let trace = CpuTrace {
            entries: vec![CpuTraceEntry {
                address: 0x12_3456,
                status: 0xA5,
                emulation: true,
                direct_page: 0x789A,
                data_bank: 0xBC,
            }],
            stop: CpuTraceStop::TargetReached,
        };
        assert_eq!(
            trace.digest_hex(),
            "8daa7f434165e6b0d5bf06879937036f89f73713171fcf34589ac5681b6f04ad"
        );
    }

    #[test]
    fn session_accessors_and_replay() {
        if std::env::var("ORACLE_UNIT_CHILD").is_ok() {
            run_session_child();
        }
        let exe = std::env::current_exe().expect("current test executable");
        let output = std::process::Command::new(exe)
            .args([
                "--exact",
                "tests::session_accessors_and_replay",
                "--nocapture",
            ])
            .env("ORACLE_UNIT_CHILD", "1")
            .output()
            .expect("spawn isolated ares test");
        assert!(
            output.status.success(),
            "isolated ares test failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("synthetic ares checks passed"),
            "child must reach the end of its assertions"
        );
    }

    fn run_session_child() -> ! {
        let rom = synthetic_rom();
        let mut s = Session::new(&rom).expect("core accepts image");
        assert!(matches!(
            Session::new(&rom),
            Err(SessionError::AlreadyBooted)
        ));
        assert_eq!(s.frame_state().frames, 0, "first session remains valid");

        assert_eq!(
            s.trace_until_pc(0x100_0000, 1, 1),
            Err(TraceError::TargetOutOfRange(0x100_0000))
        );
        assert_eq!(
            s.trace_until_pc(0x80_8000, 0, 1),
            Err(TraceError::ZeroInstructionLimit)
        );
        assert_eq!(
            s.trace_until_pc(0x80_8000, MAX_CPU_TRACE_INSTRUCTIONS + 1, 1),
            Err(TraceError::InstructionLimitTooLarge(
                MAX_CPU_TRACE_INSTRUCTIONS + 1
            ))
        );
        assert_eq!(
            s.trace_until_pc(0x80_8000, 1, 0),
            Err(TraceError::ZeroFrameLimit)
        );

        let reset_trace = s
            .trace_until_pc(0x80_8000, 8, 1)
            .expect("valid trace bounds");
        assert_eq!(reset_trace.stop, CpuTraceStop::TargetReached);
        assert_eq!(
            reset_trace
                .entries
                .iter()
                .map(|entry| entry.address)
                .collect::<Vec<_>>(),
            [0x00_FFC0, 0x00_FFC1, 0x00_FFC2, 0x00_FFC3, 0x80_8000]
        );
        assert_eq!(reset_trace.entries[0].status, 0x34);
        assert!(reset_trace.entries[0].emulation);
        assert_eq!(reset_trace.entries[3].status, 0x35);
        assert!(!reset_trace.entries[3].emulation);
        assert_eq!(reset_trace.entries[0].direct_page, 0);
        assert_eq!(reset_trace.entries[0].data_bank, 0);
        assert_eq!(s.cpu_bank(), 0x80);
        assert_eq!(s.frame_state().frames, 0);

        let loop_trace = s
            .trace_until_pc(0x80_9000, 3, 1)
            .expect("valid trace bounds");
        assert_eq!(loop_trace.stop, CpuTraceStop::InstructionLimit);
        assert_eq!(loop_trace.entries.len(), 3);
        assert!(loop_trace
            .entries
            .iter()
            .all(|entry| entry.address == 0x80_8000));

        let frame_trace = s
            .trace_until_pc(0x80_9000, 1_000_000, 1)
            .expect("valid frame bound");
        assert_eq!(frame_trace.stop, CpuTraceStop::FrameLimit);
        assert!(!frame_trace.entries.is_empty());
        assert_eq!(s.frame_state().frames, 1);

        // Memory-model accessor shapes.
        assert_eq!(s.vram().len(), 0x8000);
        assert_eq!(s.cgram().len(), 0x100);
        assert_eq!(s.apu_ram().len(), 0x10000);
        // The registers are readable after boot.
        let _ = (s.cpu_pc(), s.cpu_bank());
        let _ = s.wram(0x1FFFF);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = s.wram(0x20000);
        }));
        assert!(panic.is_err(), "wram past the image must panic");

        // Snapshot-resume determinism is covered by the ROM-backed integration
        // suites. Here we only verify that save/load round-trips without error
        // on the boot frame (no run_frames needed).
        let snap = save_state(&s);
        assert!(!snap.is_empty());
        load_state(&mut s, &snap);
        // The vendored ares engine does not tear down cleanly at process exit;
        // exit before static destructors run. The parent test verifies this
        // marker and the child status.
        eprintln!("synthetic ares checks passed");
        std::process::exit(0);
    }
}
