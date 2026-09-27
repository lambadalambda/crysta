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
    #[derive(Clone, Copy, Default)]
    pub struct SnesApuPortWrite {
        pub frame: u32,
        pub scanline: u16,
        pub cycle: u16,
        pub port: u8,
        pub value: u8,
        pub reserved: [u8; 2],
    }

    const _: () = assert!(std::mem::size_of::<SnesApuPortWrite>() == 12);
    const _: () = assert!(std::mem::offset_of!(SnesApuPortWrite, port) == 8);

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
        pub fn snes_clearApuPortWrites(snes: *mut Snes);
        pub fn snes_takeApuPortWrites(
            snes: *mut Snes,
            out: *mut SnesApuPortWrite,
            capacity: c_int,
            overflow: *mut bool,
        ) -> c_int;
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
        pub fn snes_copySamples(
            snes: *mut Snes,
            sampleData: *mut i16,
            frameCapacity: c_int,
        ) -> c_int;
        pub fn snes_testConvertAudioSample(sample: f64) -> i16;
        pub fn snes_setButtonState(snes: *mut Snes, player: c_int, button: c_int, pressed: bool);
        pub fn snes_saveState(snes: *mut Snes, data: *mut u8) -> c_int;
        pub fn snes_loadState(snes: *mut Snes, data: *const u8, size: c_int) -> bool;
        // project-authored ares shim accessors (vendor/ares/shims.cpp)
        pub fn snes_ram(snes: *const Snes) -> *const u8;
        pub fn snes_frames(snes: *const Snes) -> u32;
        pub fn snes_cycles(snes: *const Snes) -> u64;
        pub fn snes_vram(snes: *const Snes) -> *const u16;
        pub fn snes_cgram(snes: *const Snes) -> *const u16;
        pub fn snes_bg3_state(snes: *const Snes, output: *mut u16);
        pub fn snes_sprite_state(snes: *const Snes, output: *mut u8);
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
/// Maximum stereo audio frames captured for one video frame.
///
/// The DSP output is resampled to exactly 32,000 stereo frames per second.
/// Ordinary captures are approximately 532–533 frames on NTSC and 639–641
/// on PAL; scheduler boundaries can shift a neighboring sample, and startup or
/// synchronization transitions can be shorter (495 was observed during PAL
/// boot). The aggregate cadence, not an exact per-frame count, is contractual.
pub const MAX_AUDIO_FRAMES_PER_FRAME: usize = 1024;
/// Size in bytes of a cartridge SRAM image accepted by [`Session::new_with_sram`].
pub const SRAM_SIZE: usize = 8 * 1024;
/// Maximum records accepted by one bounded CPU trace.
pub const MAX_CPU_TRACE_INSTRUCTIONS: usize = 2_000_000;
/// Maximum retained CPU-origin APU port writes between explicit takes/clears.
pub const MAX_APU_PORT_WRITES: usize = 4096;
const _: () = assert!(MAX_APU_PORT_WRITES <= i32::MAX as usize);

/// Current read-only BG3 configuration (not a scanline rendering latch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bg3State {
    /// VRAM word address of the first BG3 screen (tilemap).
    pub screen_address: u16,
    /// VRAM word address of the BG3 character data.
    pub tiledata_address: u16,
    /// BG3 screen size register bits (0 = 32×32 tiles).
    pub screen_size: u16,
    /// ares BG3 mode (0 = 2bpp, 4 = inactive).
    pub mode: u16,
    /// Enabled on the main screen.
    pub above_enable: bool,
}

/// Current read-only OBJ hardware state for reference inspection.
///
/// This is not a claim about the latches that produced a completed framebuffer.
/// It is deliberately separate from the stable frame/trace digest schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteState {
    /// All 544 physical OAM bytes: 512 low-table bytes followed by 32 high-table bytes.
    pub oam: [u8; 544],
    /// Reconstructed `$2101` OBJSEL: tile base, name gap and small/large size mode.
    pub obsel: u8,
    /// Current first-sprite priority index, in 0..128; not the scanline latch.
    pub first_sprite: u8,
}

/// One CPU instruction bus write to `$2140`–`$2143` in banks `$00`–`$3f`
/// or `$80`–`$bf`. Excludes APU address aliases `$2144`–`$217f`,
/// DMA/HDMA bus transfers, and writes from the SPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApuPortWrite {
    /// Completed video-frame events at the time of the write (zero-based).
    /// This event boundary is not the beam scanline wrap.
    pub frame: u32,
    /// CPU beam scanline at the write (not a completed frame index).
    pub scanline: u16,
    /// CPU beam master-clock position within the scanline, not a global cycle count.
    /// Scanline/cycle may wrap before the next frame event; vector order is authoritative.
    pub cycle: u16,
    /// Physical port index, 0–3.
    pub port: u8,
    /// Byte placed on the CPU-to-SPC bus.
    pub value: u8,
}

/// Chronological, bounded CPU-to-APU writes since the last take or clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApuPortWrites {
    /// First [`MAX_APU_PORT_WRITES`] writes, in bus order.
    pub entries: Vec<ApuPortWrite>,
    /// At least one later write was dropped; this interval is incomplete.
    pub overflow: bool,
}

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
const _: () = assert!(
    MAX_AUDIO_FRAMES_PER_FRAME <= i32::MAX as usize,
    "audio frame capacity fits the core's i32 length"
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
    /// Panics only if the native audio boundary violates its bounded capture
    /// contract. The ROM and SRAM length conversions are guaranteed by const
    /// assertions.
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
    /// Panics under the same native audio contract violation as [`Self::new`].
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
                samples: vec![0; MAX_AUDIO_FRAMES_PER_FRAME * 2],
            };
            ffi::snes_setPixelFormat(snes, ffi::PIXEL_FORMAT_XRGB);
            ffi::snes_setPixels(snes, session.pixels.as_mut_ptr());
            session.flush_samples();
            Ok(session)
        }
    }

    fn flush_samples(&mut self) {
        self.samples.resize(MAX_AUDIO_FRAMES_PER_FRAME * 2, 0);
        let capacity = i32::try_from(MAX_AUDIO_FRAMES_PER_FRAME).expect("fits i32");
        let frames =
            unsafe { ffi::snes_copySamples(self.snes, self.samples.as_mut_ptr(), capacity) };
        assert!(frames >= 0, "core rejected the audio capture buffer");
        let frames = usize::try_from(frames).expect("nonnegative audio frame count");
        assert!(
            frames <= MAX_AUDIO_FRAMES_PER_FRAME,
            "core audio frame count exceeded allocation"
        );
        self.samples.truncate(frames * 2);
    }

    /// Advances exactly one frame, then flushes the framebuffer and that
    /// frame's variable-length 32 kHz stereo PCM into the session buffers.
    ///
    /// Ordinary captures are approximately 532–533 stereo frames on NTSC and
    /// 639–641 on PAL. Scheduler and synchronization boundaries may produce
    /// neighboring or shorter counts; [`Self::samples`] exposes the actual
    /// complete frame without padding.
    ///
    /// # Panics
    ///
    /// Panics if the native core rejects the bounded audio buffer or violates
    /// its frame-count contract.
    pub fn run_frame(&mut self) {
        unsafe {
            ffi::snes_runFrame(self.snes);
        }
        self.flush_samples();
        unsafe {
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

    /// Discards pending CPU-to-APU writes and resets the sticky overflow flag.
    /// Does not affect the emulated machine or the other capture buffers.
    pub fn clear_apu_port_writes(&mut self) {
        unsafe { ffi::snes_clearApuPortWrites(self.snes) };
    }

    /// Takes CPU instruction writes (not DMA or HDMA transfers) since boot,
    /// the last take, or clear, including writes made during mid-frame trace exits. A full buffer retains its first 4096
    /// entries and sets `overflow` until a take/clear. Saving retains pending
    /// writes; successful load clears them (the old timeline is discarded).
    ///
    /// # Panics
    ///
    /// Panics if the native core violates the fixed-size capture contract.
    #[must_use]
    pub fn take_apu_port_writes(&mut self) -> ApuPortWrites {
        let mut raw = vec![ffi::SnesApuPortWrite::default(); MAX_APU_PORT_WRITES];
        let mut overflow = false;
        let count = unsafe {
            ffi::snes_takeApuPortWrites(
                self.snes,
                raw.as_mut_ptr(),
                i32::try_from(MAX_APU_PORT_WRITES).expect("bounded capacity fits i32"),
                &raw mut overflow,
            )
        };
        let count = usize::try_from(count).expect("core rejected APU capture buffer");
        assert!(
            count <= MAX_APU_PORT_WRITES,
            "core exceeded APU capture buffer"
        );
        raw.truncate(count);
        ApuPortWrites {
            entries: raw
                .into_iter()
                .map(|write| ApuPortWrite {
                    frame: write.frame,
                    scanline: write.scanline,
                    cycle: write.cycle,
                    port: write.port,
                    value: write.value,
                })
                .collect(),
            overflow,
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

    /// The last video frame's audio at 32,000 stereo frames per second,
    /// interleaved left/right.
    ///
    /// The slice has an even, variable length. Ordinary captures are around
    /// 1,064–1,066 `i16` samples on NTSC and 1,278–1,282 on PAL, with possible
    /// neighboring scheduler effects and shorter startup/synchronization
    /// frames. It is empty before the first call to [`Self::run_frame`].
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

    /// Reads current BG3 tilemap/character configuration without PPU I/O side effects.
    #[must_use]
    pub fn bg3_state(&self) -> Bg3State {
        let mut fields = [0u16; 5];
        // SAFETY: the session owns the initialized core; shim fills five words.
        unsafe { ffi::snes_bg3_state(self.snes, fields.as_mut_ptr()) };
        Bg3State {
            screen_address: fields[0],
            tiledata_address: fields[1],
            screen_size: fields[2],
            mode: fields[3],
            above_enable: fields[4] != 0,
        }
    }

    /// Reads physical OAM and current OBJ configuration without advancing the
    /// CPU, mutating memory, or reading side-effecting PPU I/O registers.
    #[must_use]
    pub fn sprite_state(&self) -> SpriteState {
        let mut bytes = [0; 546];
        // SAFETY: the initialized singleton is owned by this session. The shim
        // fills exactly 544 OAM bytes plus two registers in this 546-byte buffer.
        unsafe { ffi::snes_sprite_state(self.snes, bytes.as_mut_ptr()) };
        let mut oam = [0; 544];
        oam.copy_from_slice(&bytes[..544]);
        SpriteState {
            oam,
            obsel: bytes[544],
            first_sprite: bytes[545],
        }
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
/// Saving synchronizes/mutates the native core. After serialization, the shim
/// resets the DSP resampler to 32 kHz and clears both native capture
/// accumulators. Pending APU writes are retained across save. The currently
/// exposed [`Session::samples`] remains unchanged;
/// the next frame replaces it from the fresh resampler phase.
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
/// After a successful load, the shim resets the DSP resampler to 32 kHz and
/// clears both native capture accumulators and pending APU writes/overflow.
/// The next frame therefore starts at
/// the same fresh audio phase as the first frame after [`save_state`].
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
    fn apu_port_writes_are_bounded_and_survive_trace_steps() {
        if let Some(mode) = std::env::var_os("ORACLE_APU_CHILD") {
            let mut image = vec![0u8; Rom::IMAGE_SIZE];
            // SEI; CLC; XCE; JML $80:8000 (native mode, 8-bit accumulator).
            image[0xffc0..0xffc7].copy_from_slice(&[0x78, 0x18, 0xfb, 0x5c, 0, 0x80, 0x80]);
            // LDA #$11; STA $2140; LDA #$22; STA $2141; BRA to self.
            image[0x8000..0x800c].copy_from_slice(&[
                0xa9, 0x11, 0x8d, 0x40, 0x21, 0xa9, 0x22, 0x8d, 0x41, 0x21, 0x80, 0xfe,
            ]);
            if mode == "overflow" {
                image[0x800b] = 0xf4; // BRA back to $8000.
            }
            image[0xfffc..0xfffe].copy_from_slice(&[0xc0, 0xff]);
            let d = rom::digests(&image);
            let known = [rom::KnownRom {
                revision: rom::Revision::Japan,
                sha256: d.sha256,
                crc32: d.crc32,
            }];
            let rom = Rom::load_with_known(&image, &known).unwrap();
            let mut session = Session::new(&rom).unwrap();
            assert_eq!(session.take_apu_port_writes().entries.len(), 0);
            assert_eq!(
                session.trace_until_pc(0x80_8005, 1000, 2).unwrap().stop,
                CpuTraceStop::TargetReached
            );
            if mode == "quiet" {
                let first = session.take_apu_port_writes();
                assert!(!first.overflow);
                assert_eq!(first.entries.len(), 1);
                assert_eq!((first.entries[0].port, first.entries[0].value), (0, 0x11));
                assert!(session.take_apu_port_writes().entries.is_empty());
            }
            assert_eq!(
                session.trace_until_pc(0x80_800a, 1000, 2).unwrap().stop,
                CpuTraceStop::TargetReached
            );
            let writes = session.take_apu_port_writes();
            assert!(!writes.overflow);
            assert_eq!(writes.entries.len(), if mode == "quiet" { 1 } else { 2 });
            assert_eq!(
                (
                    writes.entries.last().unwrap().port,
                    writes.entries.last().unwrap().value
                ),
                (1, 0x22)
            );
            if mode == "overflow" {
                assert_eq!((writes.entries[0].port, writes.entries[0].value), (0, 0x11));
                assert!(writes.entries[1].cycle >= writes.entries[0].cycle);
            }
            session.clear_apu_port_writes();
            assert!(session.take_apu_port_writes().entries.is_empty());
            session.run_frame();
            let writes = session.take_apu_port_writes();
            if mode == "overflow" {
                assert!(writes.overflow);
                assert_eq!(writes.entries.len(), MAX_APU_PORT_WRITES);
                assert_eq!((writes.entries[0].port, writes.entries[0].value), (0, 0x11));
                assert_eq!((writes.entries[1].port, writes.entries[1].value), (1, 0x22));
                assert!(!session.take_apu_port_writes().overflow);
                session.run_frame();
                let state = save_state(&session);
                // Saving does not erase pending writes.
                let saved_writes = session.take_apu_port_writes();
                assert!(saved_writes.overflow);
                assert!(saved_writes.entries.iter().any(|w| w.frame >= 1));
                assert!(saved_writes
                    .entries
                    .iter()
                    .all(|w| w.scanline < 262 && w.cycle < 1364));
                session.run_frame();
                load_state(&mut session, &state);
                assert_eq!(
                    session.take_apu_port_writes(),
                    ApuPortWrites {
                        entries: vec![],
                        overflow: false
                    }
                );
            } else {
                // The spin loop is quiet; clear/take must not invent a write.
                assert_eq!(
                    writes,
                    ApuPortWrites {
                        entries: vec![],
                        overflow: false
                    }
                );
                let state = save_state(&session);
                load_state(&mut session, &state);
                assert!(session.take_apu_port_writes().entries.is_empty());
            }
            std::process::exit(0);
        }
        for mode in ["quiet", "overflow"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tests::apu_port_writes_are_bounded_and_survive_trace_steps",
                    "--nocapture",
                ])
                .env("ORACLE_APU_CHILD", mode)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn apu_hook_excludes_other_addresses_and_dma_hdma() {
        if std::env::var_os("ORACLE_APU_BOUNDARY_CHILD").is_some() {
            fn store(code: &mut Vec<u8>, value: u8, address: u16) {
                code.extend([0xa9, value, 0x8d, address as u8, (address >> 8) as u8]);
            }
            fn store_long(code: &mut Vec<u8>, value: u8, bank: u8, address: u16) {
                code.extend([0xa9, value, 0x8f, address as u8, (address >> 8) as u8, bank]);
            }
            let mut code = Vec::new();
            store(&mut code, 0x32, 0x2142);
            store(&mut code, 0x43, 0x2143);
            store(&mut code, 0x21, 0x213f); // Adjacent PPU register, not APU I/O.
            store(&mut code, 0x54, 0x2144); // APU mirror: deliberately outside the hook's range.
            store_long(&mut code, 0x65, 0x40, 0x2140); // ROM bank, not APU I/O.
            store_long(&mut code, 0x76, 0x7e, 0x2140); // WRAM bank, not APU I/O.
            store_long(&mut code, 0x87, 0x80, 0x2142); // Valid mirrored CPU I/O bank.
            let before_dma = 0x80_8000 + code.len() as u32;

            // DMA channel 0: one byte from WRAM $7e:0000 to B-bus $2140.
            store_long(&mut code, 0xa5, 0x7e, 0x0000);
            for (value, address) in [
                (0, 0x4300),
                (0x40, 0x4301),
                (0, 0x4302),
                (0, 0x4303),
                (0x7e, 0x4304),
                (1, 0x4305),
                (0, 0x4306),
            ] {
                store(&mut code, value, address);
            }
            store(&mut code, 1, 0x420b);
            // $4302 reads the advanced DMA source pointer: the transfer really ran.
            code.extend([0xea, 0xad, 0x02, 0x43, 0x8f, 0x10, 0x00, 0x7e]);
            let after_dma = 0x80_8000 + code.len() as u32;

            // HDMA channel 1: table at $7e:0020, one $2140 byte on the next scanline.
            for (offset, byte) in [0x81, 0x5a, 0].into_iter().enumerate() {
                store_long(&mut code, byte, 0x7e, 0x0020 + offset as u16);
            }
            for (value, address) in [
                (0, 0x4310),
                (0x40, 0x4311),
                (0x20, 0x4312),
                (0, 0x4313),
                (0x7e, 0x4314),
            ] {
                store(&mut code, value, address);
            }
            store(&mut code, 2, 0x420c);
            // Sample HDMA's A2A pointer while spinning. It advances beyond $0021
            // only when a scanline transfer has consumed the table byte.
            code.extend([0xad, 0x18, 0x43, 0x8f, 0x11, 0x00, 0x7e, 0x80, 0xf7]);

            let mut image = vec![0u8; Rom::IMAGE_SIZE];
            image[0xffc0..0xffc7].copy_from_slice(&[0x78, 0x18, 0xfb, 0x5c, 0, 0x80, 0x80]);
            image[0x8000..0x8000 + code.len()].copy_from_slice(&code);
            image[0xfffc..0xfffe].copy_from_slice(&[0xc0, 0xff]);
            let d = rom::digests(&image);
            let rom = Rom::load_with_known(
                &image,
                &[rom::KnownRom {
                    revision: rom::Revision::Japan,
                    sha256: d.sha256,
                    crc32: d.crc32,
                }],
            )
            .unwrap();
            let mut session = Session::new(&rom).unwrap();
            assert_eq!(
                session.trace_until_pc(before_dma, 1000, 2).unwrap().stop,
                CpuTraceStop::TargetReached
            );
            assert_eq!(
                session.wram(0x2140),
                0x76,
                "excluded WRAM-bank write took effect"
            );
            let writes = session.take_apu_port_writes();
            assert!(!writes.overflow);
            assert_eq!(
                writes
                    .entries
                    .iter()
                    .map(|w| (w.port, w.value))
                    .collect::<Vec<_>>(),
                [(2, 0x32), (3, 0x43), (2, 0x87)]
            );
            assert!(writes
                .entries
                .iter()
                .all(|w| w.frame == 0 && w.scanline < 262 && w.cycle < 1364));
            assert!(writes
                .entries
                .windows(2)
                .all(|pair| pair[0].scanline < pair[1].scanline
                    || (pair[0].scanline == pair[1].scanline && pair[0].cycle < pair[1].cycle)));

            assert_eq!(
                session.trace_until_pc(after_dma, 1000, 2).unwrap().stop,
                CpuTraceStop::TargetReached
            );
            assert_eq!(
                session.wram(0x10),
                1,
                "DMA source pointer must have advanced"
            );
            assert!(
                session.take_apu_port_writes().entries.is_empty(),
                "DMA must not enter CPU write queue"
            );
            session.run_frames(2);
            assert!(
                session.wram(0x11) >= 0x22,
                "HDMA table must have transferred"
            );
            assert_eq!(
                session.take_apu_port_writes(),
                ApuPortWrites {
                    entries: vec![],
                    overflow: false
                }
            );
            std::process::exit(0);
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::apu_hook_excludes_other_addresses_and_dma_hdma",
                "--nocapture",
            ])
            .env("ORACLE_APU_BOUNDARY_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
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
    fn native_pcm_conversion_is_finite_saturating_and_truncating() {
        let convert = |sample| unsafe { ffi::snes_testConvertAudioSample(sample) };
        assert_eq!(convert(f64::NAN), 0);
        assert_eq!(convert(f64::INFINITY), 0);
        assert_eq!(convert(f64::NEG_INFINITY), 0);
        assert_eq!(convert(-2.0), i16::MIN);
        assert_eq!(convert(2.0), i16::MAX);
        assert_eq!(convert(1.9 / 32768.0), 1);
        assert_eq!(convert(-1.9 / 32768.0), -1);
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

    fn assert_audio_boundary(session: &mut Session) {
        assert!(
            session.samples().is_empty(),
            "instruction tracing must not publish a partial audio frame"
        );

        session.run_frame();
        let captured_frames = session.samples().len() / 2;
        assert!(
            matches!(captured_frames, 532 | 533),
            "normal run after a mid-frame trace must publish one complete NTSC frame"
        );
        let capacity = i32::try_from(MAX_AUDIO_FRAMES_PER_FRAME).unwrap();
        let mut raw = vec![i16::MIN; MAX_AUDIO_FRAMES_PER_FRAME * 2];
        assert_eq!(
            unsafe { ffi::snes_copySamples(session.snes, raw.as_mut_ptr(), capacity) },
            i32::try_from(captured_frames).unwrap()
        );
        assert_eq!(&raw[..session.samples().len()], session.samples());
        assert!(raw[session.samples().len()..]
            .iter()
            .all(|&sample| sample == i16::MIN));

        let mut rejected = vec![i16::MAX; MAX_AUDIO_FRAMES_PER_FRAME * 2];
        assert_eq!(
            unsafe {
                ffi::snes_copySamples(
                    session.snes,
                    rejected.as_mut_ptr(),
                    i32::try_from(captured_frames - 1).unwrap(),
                )
            },
            -1
        );
        assert!(rejected.iter().all(|&sample| sample == i16::MAX));
        assert_eq!(
            unsafe { ffi::snes_copySamples(session.snes, std::ptr::null_mut(), capacity) },
            -1
        );
        assert_eq!(
            unsafe { ffi::snes_copySamples(std::ptr::null_mut(), raw.as_mut_ptr(), capacity) },
            -1
        );
        assert_eq!(
            unsafe { ffi::snes_copySamples(session.snes, raw.as_mut_ptr(), -1) },
            -1
        );

        let snapshot = save_state(session);
        session.run_frame();
        let after_save = session.samples().to_vec();
        load_state(session, &snapshot);
        session.run_frame();
        assert_eq!(
            session.samples(),
            after_save,
            "save/reset and load/reset must replay exact per-frame PCM"
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

        let midframe_trace = s
            .trace_until_pc(0x80_9000, 5_000, 1)
            .expect("trace a partial frame");
        assert_eq!(midframe_trace.stop, CpuTraceStop::InstructionLimit);
        assert_audio_boundary(&mut s);

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

        // The vendored ares engine does not tear down cleanly at process exit;
        // exit before static destructors run. The parent test verifies this
        // marker and the child status.
        eprintln!("synthetic ares checks passed");
        std::process::exit(0);
    }
}
