//! Thread-local, audio-only SPC/DSP playback at 32,000 stereo frames/second.
//!
//! Host writes and SPC replies use separate port banks. Upload through the IPL
//! protocol, discarding setup audio with [`Apu::run_cycles`], then [`Apu::render`].
//! No SNES CPU, frame clock, capture loading, resampling, or audio device is used.

use std::{ffi::c_void, fmt, marker::PhantomData, ptr::NonNull, rc::Rc};

#[cfg(target_arch = "wasm32")]
mod wasm_libc;

/// Native DSP stereo frames per second (not interleaved sample count).
pub const SAMPLE_RATE: u32 = 32_000;
/// Maximum SPC cycles requested per setup call (one nominal second).
pub const MAX_RUN_CYCLES: u32 = 1_024_000;
/// Maximum stereo frames per render call; larger requests must be chunked.
pub const MAX_RENDER_FRAMES: usize = 32_000;
/// Size of the SPC's physical RAM, including RAM hidden by I/O and IPL ROM.
pub const RAM_SIZE: usize = 65_536;

/// A rejected request or backend failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Allocation failed before any upstream code was called.
    Allocation,
    /// Only host ports 0 through 3 exist.
    PortIndex,
    /// Setup request exceeds [`MAX_RUN_CYCLES`].
    CycleLimit,
    /// Render requires an even sample count, at most twice [`MAX_RENDER_FRAMES`].
    RenderSize,
    /// Physical RAM range extends beyond [`RAM_SIZE`].
    RamRange,
    /// The pinned core violated its bounded opcode/progress contract.
    BackendProgress,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Allocation => "SPC allocation failed",
            Self::PortIndex => "SPC port index must be 0..4",
            Self::CycleLimit => "SPC cycle request exceeds limit",
            Self::RenderSize => "SPC render requires a bounded stereo sample buffer",
            Self::RamRange => "SPC physical RAM range is out of bounds",
            Self::BackendProgress => "SPC backend failed its bounded progress check",
        })
    }
}

impl std::error::Error for Error {}

// Private ABI: the Rust side never sees or dereferences upstream structures.
mod ffi {
    use super::c_void;
    extern "C" {
        pub(super) fn spc_player_new() -> *mut c_void;
        pub(super) fn spc_player_free(player: *mut c_void);
        pub(super) fn spc_player_write_port(player: *mut c_void, index: usize, value: u8);
        pub(super) fn spc_player_read_port(player: *const c_void, index: usize) -> u8;
        pub(super) fn spc_player_run_cycles(player: *mut c_void, cycles: u32) -> i32;
        pub(super) fn spc_player_render(player: *mut c_void, out: *mut i16, frames: usize) -> i32;
        pub(super) fn spc_player_read_ram(
            player: *const c_void,
            start: usize,
            out: *mut u8,
            len: usize,
        );
    }
}

/// An exclusively owned APU. Construct, upload, and render on the owning thread.
///
/// Intentionally neither `Send` nor `Sync` (no unsafe thread-trait overrides).
///
/// ```compile_fail
/// let apu = spc_player::Apu::new().unwrap();
/// std::thread::spawn(move || drop(apu));
/// ```
pub struct Apu {
    raw: NonNull<c_void>,
    _thread_local: PhantomData<Rc<()>>,
}

impl Apu {
    /// Allocate/reset an APU with the physical IPL enabled. Advance it with
    /// [`Self::run_cycles`] to receive the IPL's `AA`/`BB` ready replies.
    pub fn new() -> Result<Self, Error> {
        // SAFETY: no inputs; shim returns a fresh allocation or null.
        let raw = NonNull::new(unsafe { ffi::spc_player_new() }).ok_or(Error::Allocation)?;
        Ok(Self {
            raw,
            _thread_local: PhantomData,
        })
    }

    /// Write a CPU-to-APU input latch, without touching the SPC's output latch
    /// or its RAM mirror. Does not advance emulation.
    pub fn write_port(&mut self, index: usize, value: u8) -> Result<(), Error> {
        if index >= 4 {
            return Err(Error::PortIndex);
        }
        // SAFETY: live exclusive owner, validated port index.
        unsafe { ffi::spc_player_write_port(self.raw.as_ptr(), index, value) };
        Ok(())
    }

    /// Read an APU-to-CPU output latch without advancing emulation.
    pub fn read_port(&self, index: usize) -> Result<u8, Error> {
        if index >= 4 {
            return Err(Error::PortIndex);
        }
        // SAFETY: live allocation, read-only call, validated port index.
        Ok(unsafe { ffi::spc_player_read_port(self.raw.as_ptr(), index) })
    }

    /// Run at least `cycles` SPC cycles, stopping on an opcode boundary, and
    /// discard all generated/pending audio. Returns actual cycles advanced;
    /// may overshoot by up to 31 cycles. Zero advances nothing but still discards
    /// pending audio. For bounded host-protocol polling, use small requests.
    pub fn run_cycles(&mut self, cycles: u32) -> Result<u32, Error> {
        if cycles > MAX_RUN_CYCLES {
            return Err(Error::CycleLimit);
        }
        // SAFETY: live exclusive owner, bounded request; no caller buffers.
        let actual = unsafe { ffi::spc_player_run_cycles(self.raw.as_ptr(), cycles) };
        u32::try_from(actual).map_err(|_| Error::BackendProgress)
    }

    /// Fill interleaved signed 16-bit `[left, right, ...]` output at native rate.
    /// Returns the number of stereo frames written. Empty buffers are a no-op.
    /// Unconsumed ring samples survive across arbitrary render chunk sizes.
    /// Validation errors do not advance state or change the buffer. A backend
    /// progress failure may leave a partially filled buffer; discard it.
    pub fn render(&mut self, interleaved_stereo: &mut [i16]) -> Result<usize, Error> {
        if !interleaved_stereo.len().is_multiple_of(2)
            || interleaved_stereo.len() / 2 > MAX_RENDER_FRAMES
        {
            return Err(Error::RenderSize);
        }
        let frames = interleaved_stereo.len() / 2;
        // SAFETY: live exclusive owner and writable buffer of exactly frames*2
        // i16s. Shim does not retain the pointer; empty buffers aren't accessed.
        let result = unsafe {
            ffi::spc_player_render(self.raw.as_ptr(), interleaved_stereo.as_mut_ptr(), frames)
        };
        if result < 0 {
            return Err(Error::BackendProgress);
        }
        Ok(frames)
    }

    /// Copy bounded physical RAM without I/O side effects or IPL ROM overlay.
    /// This verifies host uploads; it does not read timer counters or DSP ports.
    pub fn read_ram(&self, start: usize, out: &mut [u8]) -> Result<(), Error> {
        if start > RAM_SIZE || out.len() > RAM_SIZE - start {
            return Err(Error::RamRange);
        }
        // SAFETY: live allocation, validated physical range, writable output
        // length. Shim neither retains the pointer nor accesses an empty slice.
        unsafe { ffi::spc_player_read_ram(self.raw.as_ptr(), start, out.as_mut_ptr(), out.len()) };
        Ok(())
    }
}

impl Drop for Apu {
    fn drop(&mut self) {
        // SAFETY: unique allocation from new, released exactly once, no aliases.
        unsafe { ffi::spc_player_free(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_port(apu: &mut Apu, port: usize, expected: u8) {
        for _ in 0..2048 {
            if apu.read_port(port).unwrap() == expected {
                return;
            }
            apu.run_cycles(16).unwrap();
        }
        panic!("port {port} never reached {expected:02x}");
    }

    // The real 64-byte IPL protocol, not a direct RAM or register shortcut.
    fn upload(program: &[u8]) -> Apu {
        assert!(!program.is_empty() && program.len() < 254);
        let mut apu = Apu::new().unwrap();
        wait_port(&mut apu, 0, 0xaa);
        wait_port(&mut apu, 1, 0xbb);
        apu.write_port(2, 0).unwrap(); // destination $0200
        apu.write_port(3, 2).unwrap();
        apu.write_port(1, 1).unwrap(); // transfer, not jump
        apu.write_port(0, 0xcc).unwrap();
        wait_port(&mut apu, 0, 0xcc);
        for (counter, &byte) in program.iter().enumerate() {
            apu.write_port(1, byte).unwrap();
            apu.write_port(0, counter as u8).unwrap();
            wait_port(&mut apu, 0, counter as u8);
        }
        apu.write_port(2, 0).unwrap();
        apu.write_port(3, 2).unwrap();
        apu.write_port(1, 0).unwrap(); // jump to $0200
        let token = (program.len() + 1) as u8;
        apu.write_port(0, token).unwrap();
        wait_port(&mut apu, 0, token);
        let mut actual = vec![0; program.len()];
        apu.read_ram(0x200, &mut actual).unwrap();
        assert_eq!(actual, program);
        apu
    }

    fn noise_player() -> Apu {
        let mut program = Vec::new();
        // MOV dp,#imm to $F2/$F3: unmute, disable echo writes, noise channel 0,
        // direct gain, asymmetric stereo volumes, key on. Entirely synthetic.
        for (reg, value) in [
            (0x6c, 0x3f),
            (0x0c, 0x7f),
            (0x1c, 0x7f),
            (0x00, 0x60),
            (0x01, 0x30),
            (0x05, 0x00),
            (0x07, 0x7f),
            (0x3d, 0x01),
            (0x4c, 0x01),
        ] {
            program.extend_from_slice(&[0x8f, reg, 0xf2, 0x8f, value, 0xf3]);
        }
        program.extend_from_slice(&[0x2f, 0xfe]); // BRA self
        upload(&program)
    }

    fn check_negative_brr(shift: u8, expected: i16) {
        let mut program = Vec::new();
        // SPC code writes directory entry 0 at $0300 and a looping, filter-0
        // BRR block at $0400. Every synthetic nibble is $F (signed -1).
        for (address, value) in [
            (0x0300_u16, 0x00),
            (0x0301, 0x04),
            (0x0302, 0x00),
            (0x0303, 0x04),
            (0x0400, (shift << 4) | 3),
        ]
        .into_iter()
        .chain((0x0401..=0x0408).map(|address| (address, 0xff)))
        {
            let [lo, hi] = address.to_le_bytes();
            program.extend_from_slice(&[0xe8, value, 0xc5, lo, hi]); // MOV A,#imm; MOV abs,A
        }
        // BRR (not noise), pitch $1000, direct gain $7F, full L/R volumes.
        for (reg, value) in [
            (0x6c, 0x20),
            (0x5d, 0x03),
            (0x04, 0x00),
            (0x02, 0x00),
            (0x03, 0x10),
            (0x05, 0x00),
            (0x07, 0x7f),
            (0x00, 0x7f),
            (0x01, 0x7f),
            (0x0c, 0x7f),
            (0x1c, 0x7f),
            (0x4c, 0x01),
        ] {
            program.extend_from_slice(&[0x8f, reg, 0xf2, 0x8f, value, 0xf3]);
        }
        program.extend_from_slice(&[0x2f, 0xfe]);
        let mut apu = upload(&program);
        let mut samples = [0; 512 * 2];
        assert_eq!(apu.render(&mut samples), Ok(512));
        // After startup/interpolation settle, the constant block must sustain
        // the exact negative PCM in BOTH channels, across repeated BRR loops.
        assert!(
            samples[128 * 2..].iter().all(|&s| s == expected),
            "BRR range {shift}: expected {expected}, tail {:?}",
            &samples[1008..]
        );
    }

    #[test]
    fn negative_brr_normal_ranges() {
        // Golden PCM: decoded -1 is -2 at range 0, otherwise -(1 << range).
        // Constant interpolation uses Gaussian weights 370,1305,374,0 / 2048;
        // then direct gain 2032/2048 and channel/master volumes 127/128 each,
        // with the core's arithmetic rounding/even-sample masking at each stage.
        for (shift, expected) in [
            -4, -4, -6, -10, -18, -34, -66, -128, -253, -504, -1004, -2004, -4004,
        ]
        .into_iter()
        .enumerate()
        {
            check_negative_brr(shift as u8, expected);
        }
    }

    #[test]
    fn negative_brr_reserved_ranges() {
        // Ranges 13..15 collapse every negative nibble to decoded -8192;
        // same gain/interpolation/volume calculation as the normal-range test.
        for shift in 13..=15 {
            check_negative_brr(shift, -8006);
        }
    }

    #[test]
    fn ipl_ready_and_separate_host_ports() {
        let mut apu = Apu::new().unwrap();
        assert_eq!(apu.read_port(0), Ok(0));
        wait_port(&mut apu, 0, 0xaa);
        wait_port(&mut apu, 1, 0xbb);
        let mut before = [0; 4];
        apu.read_ram(0xf4, &mut before).unwrap();
        for i in 0..4 {
            let reply = apu.read_port(i).unwrap();
            apu.write_port(i, 0x42).unwrap();
            assert_eq!(apu.read_port(i), Ok(reply));
        }
        let mut after = [0; 4];
        apu.read_ram(0xf4, &mut after).unwrap();
        assert_eq!(before, after, "host writes must not use apu_write");
    }

    #[test]
    fn ipl_upload_executes_loop_and_reads_input_latch() {
        // MOV A,$F6; MOV $F7,A; BRA $0200
        let mut apu = upload(&[0xe4, 0xf6, 0xc4, 0xf7, 0x2f, 0xfa]);
        for byte in [0x12, 0x34, 0xab] {
            apu.write_port(2, byte).unwrap();
            wait_port(&mut apu, 3, byte);
        }
    }

    #[test]
    fn reset_render_is_deterministic_silence_and_counts_frames() {
        let mut a = Apu::new().unwrap();
        let mut b = Apu::new().unwrap();
        for frames in [0, 1, 17, 1023, 1024, 1025, MAX_RENDER_FRAMES] {
            let mut x = vec![123; frames * 2];
            let mut y = vec![456; frames * 2];
            assert_eq!(a.render(&mut x), Ok(frames));
            assert_eq!(b.render(&mut y), Ok(frames));
            assert_eq!(x, y);
            assert!(x.iter().all(|&s| s == 0));
        }
    }

    #[test]
    fn nonzero_stream_is_identical_across_chunks_and_ring_counter_wrap() {
        let mut a = noise_player();
        let mut b = noise_player();
        // Exceed both the 1024-frame ring and the 16-bit producer counter.
        let frames = 70_013;
        let mut whole = vec![0; frames * 2];
        for chunk in whole.chunks_mut(MAX_RENDER_FRAMES * 2) {
            assert_eq!(a.render(chunk), Ok(chunk.len() / 2));
        }
        let mut split = vec![0; frames * 2];
        let sizes = [1, 7, 31, 32, 33, 1023, 1024, 1025, 4097];
        let mut offset = 0;
        for size in sizes.into_iter().cycle() {
            b.render(&mut []).unwrap();
            let end = (offset + size * 2).min(split.len());
            assert_eq!(b.render(&mut split[offset..end]), Ok((end - offset) / 2));
            offset = end;
            if offset == split.len() {
                break;
            }
        }
        assert!(
            whole.iter().any(|&s| s != 0),
            "test must exercise real DSP output"
        );
        assert!(whole.chunks_exact(2).any(|s| s[0] != s[1]));
        assert_eq!(whole, split);
    }

    #[test]
    fn setup_discards_audio_instead_of_replaying_stale_ring() {
        let mut a = noise_player();
        let mut b = noise_player();
        // Finish the DSP setup before comparing a fixed-cycle BRA loop.
        a.run_cycles(1024).unwrap();
        b.run_cycles(1024).unwrap();
        // Exact render boundary: both instances have identical opcode phase.
        a.render(&mut [0; 2]).unwrap();
        b.render(&mut [0; 2]).unwrap();
        // BRA costs 4 cycles. One second of output far exceeds the ring size.
        assert_eq!(a.run_cycles(MAX_RUN_CYCLES), Ok(MAX_RUN_CYCLES));
        b.render(&mut vec![0; MAX_RENDER_FRAMES * 2]).unwrap();
        let mut x = [0; 64];
        let mut y = [0; 64];
        a.render(&mut x).unwrap();
        b.render(&mut y).unwrap();
        assert_eq!(x, y);
    }

    #[test]
    fn rejects_invalid_requests_without_mutating_state_or_output() {
        let mut apu = Apu::new().unwrap();
        assert_eq!(apu.write_port(4, 1), Err(Error::PortIndex));
        assert_eq!(apu.read_port(usize::MAX), Err(Error::PortIndex));
        assert_eq!(apu.run_cycles(MAX_RUN_CYCLES + 1), Err(Error::CycleLimit));
        let mut odd = [7; 3];
        assert_eq!(apu.render(&mut odd), Err(Error::RenderSize));
        assert_eq!(odd, [7; 3]);
        assert_eq!(
            apu.render(&mut vec![7; (MAX_RENDER_FRAMES + 1) * 2]),
            Err(Error::RenderSize)
        );
        assert_eq!(apu.read_ram(usize::MAX, &mut []), Err(Error::RamRange));
        assert_eq!(apu.read_ram(RAM_SIZE, &mut [0]), Err(Error::RamRange));
        assert_eq!(apu.read_ram(RAM_SIZE, &mut []), Ok(()));
        assert_eq!(apu.read_ram(RAM_SIZE - 1, &mut [0]), Ok(()));
        assert_eq!(apu.read_port(0), Ok(0));
        assert_eq!(apu.run_cycles(0), Ok(0));
        assert_eq!(apu.read_port(0), Ok(0));
    }

    #[test]
    fn bounded_cycle_requests_include_stopped_spc() {
        let mut apu = upload(&[0xff]); // STOP still clocks the DSP/timers
        let actual = apu.run_cycles(MAX_RUN_CYCLES).unwrap();
        assert!((MAX_RUN_CYCLES..MAX_RUN_CYCLES + 32).contains(&actual));
        assert_eq!(apu.render(&mut [0; 128]), Ok(64));
    }
}
