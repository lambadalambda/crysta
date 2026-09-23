//! Source-derived music and sound effects through physical APU ports, never
//! RAM injection.
//!
//! [`Player`] is the host's side of the driver, frame by frame: a track
//! change is the worker's port script (`$8D:94C5` fade, `$8D:950A` stop,
//! upload and play), polled once a frame as the game does, so a fade stays
//! audible; sound effects go out as the NMI hands the latch `$04B6` to ports
//! 2 and 3 -- on every other frame, with zeros between.
use crate::music_data::{Driver, Track, TransferGroup, Upload};
use crate::music_output::Synth;
use crysta_runtime::audio::Cue;
use spc_player::Apu;
use std::collections::VecDeque;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Stereo frames per NMI frame: 32000 Hz over the SNES's 60 Hz.
const FRAME: usize = 533;
/// SPC cycles per NMI frame, for settling without rendering.
const FRAME_CYCLES: u32 = 17_067;
/// Frames a port wait may take: a fade takes some five seconds, an echo
/// buffer's drain a third of one.
const WAIT_FRAMES: u32 = 900;
/// Sound effects queued between latch frames; more are dropped, as the
/// latch keeps only one natively.
const HELD_SOUNDS: usize = 8;

/// One step of a track change's port script.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    /// Write an input port.
    Write(usize, u8),
    /// Wait, a frame at a time, for an output port's value.
    Wait(usize, u8),
    /// Let frames pass.
    Frames(u8),
    /// Upload a track's groups through the resident receiver.
    Upload(Box<Upload>),
}

/// The port script that replaces the playing track: the fade (`F1`) when
/// asked, the stop (`F0`) with the new track's parameter, the uploads after
/// `FF`, then `F4` when `play`. Ports 2 and 3 go quiet first.
fn change(stop_parameter: u8, upload: Upload, fade: bool, play: bool) -> Vec<Op> {
    let mut ops = vec![Op::Write(2, 0), Op::Write(3, 0)];
    if fade {
        // The driver acknowledges `F1`, and answers the zero that follows
        // once the fade is over.
        ops.extend([
            Op::Write(1, stop_parameter),
            Op::Write(0, 0xf1),
            Op::Frames(1),
            Op::Wait(0, 0xf1),
            Op::Write(1, 0),
            Op::Write(0, 0),
            Op::Frames(1),
            Op::Wait(0, 0),
        ]);
    }
    // It echoes `F0`, then answers zero once its echo buffer has drained.
    ops.extend([
        Op::Write(0, 0),
        Op::Write(1, stop_parameter),
        Op::Write(0, 0xf0),
        Op::Frames(1),
        Op::Wait(0, 0xf0),
        Op::Wait(0, 0),
        Op::Frames(1),
        Op::Write(0, 0xff),
        Op::Frames(2),
        Op::Upload(Box::new(upload)),
    ]);
    if play {
        ops.extend([Op::Frames(3), Op::Write(0, 0xf4)]);
    }
    ops
}

/// The driver with its host: tracks load on request, sound effects play.
pub struct Player {
    apu: Apu,
    tracks: Box<dyn FnMut(u8) -> Result<Track> + Send>,
    script: VecDeque<Op>,
    /// Frames the script's current wait has taken.
    waited: u32,
    sounds: VecDeque<u16>,
    /// Whether the next NMI frame writes the latch rather than zeros.
    latch_frame: bool,
    /// Stereo frames rendered into the current NMI frame.
    phase: usize,
}

impl Player {
    /// Boots the driver and uploads its sound effect bank. RAM reads only
    /// verify source uploads; they never initialize state.
    ///
    /// # Errors
    /// A failed upload or a driver that does not answer.
    pub fn new(
        driver: &Driver,
        tracks: impl FnMut(u8) -> Result<Track> + Send + 'static,
    ) -> Result<Self> {
        let mut apu = Apu::new()?;
        upload(&mut apu, &driver.bootstrap)?;
        apu.run_cycles(20_000)?;
        let mut player = Self {
            apu,
            tracks: Box::new(tracks),
            script: change(0, driver.sounds.clone(), false, false).into(),
            waited: 0,
            sounds: VecDeque::new(),
            latch_frame: true,
            phase: 0,
        };
        player.settle()?;
        Ok(player)
    }

    /// Takes a request.
    ///
    /// # Errors
    /// A track the ROM does not hold.
    pub fn take(&mut self, cue: Cue) -> Result<()> {
        match cue {
            Cue::Track { track, fade } => {
                let Track {
                    stop_parameter,
                    upload,
                    ..
                } = (self.tracks)(track)?;
                self.script
                    .extend(change(stop_parameter, upload, fade, true));
                self.sounds.clear();
            }
            // The driver takes effects only a few frames into a track (its
            // `$1D`, SPC `$0AB2`): those made during a change go nowhere.
            Cue::Sound(latch) if self.script.is_empty() && self.sounds.len() < HELD_SOUNDS => {
                self.sounds.push_back(latch);
            }
            Cue::Sound(_) => {}
        }
        Ok(())
    }

    /// Runs frames without rendering until the port script is done.
    ///
    /// # Errors
    /// As [`Self::render`].
    pub fn settle(&mut self) -> Result<()> {
        while !self.script.is_empty() {
            self.apu.run_cycles(FRAME_CYCLES)?;
            self.frame()?;
        }
        Ok(())
    }

    /// Renders interleaved stereo, running the host's frames on the way.
    ///
    /// # Errors
    /// A backend failure, an upload that did not verify, or a wait the driver
    /// never answered.
    pub fn play(&mut self, out: &mut [i16]) -> Result<()> {
        if !out.len().is_multiple_of(2) {
            return Err("interleaved stereo has an even length".into());
        }
        let mut rest = out;
        while !rest.is_empty() {
            let frames = (rest.len() / 2).min(FRAME - self.phase);
            let (now, later) = rest.split_at_mut(frames * 2);
            self.apu.render(now)?;
            rest = later;
            self.phase += frames;
            if self.phase == FRAME {
                self.phase = 0;
                self.frame()?;
            }
        }
        Ok(())
    }

    /// One NMI frame: the port script as far as it goes, or else the latch.
    fn frame(&mut self) -> Result<()> {
        if self.script.is_empty() {
            let latch = if self.latch_frame {
                self.sounds.pop_front().unwrap_or(0)
            } else {
                0
            };
            let [port2, port3] = latch.to_le_bytes();
            self.apu.write_port(2, port2)?;
            self.apu.write_port(3, port3)?;
            self.latch_frame = !self.latch_frame;
            return Ok(());
        }
        while let Some(op) = self.script.front_mut() {
            match op {
                Op::Write(port, value) => self.apu.write_port(*port, *value)?,
                Op::Wait(port, value) => {
                    if self.apu.read_port(*port)? != *value {
                        self.waited += 1;
                        if self.waited > WAIT_FRAMES {
                            return Err(
                                format!("the driver never set port{port}=${value:02x}").into()
                            );
                        }
                        return Ok(());
                    }
                }
                Op::Frames(frames) if *frames > 0 => {
                    *frames -= 1;
                    return Ok(());
                }
                Op::Frames(_) => {}
                Op::Upload(groups) => load(&mut self.apu, groups)?,
            }
            self.script.pop_front();
            self.waited = 0;
        }
        Ok(())
    }
}

impl Synth for Player {
    fn cue(&mut self, cue: Cue) -> std::result::Result<(), String> {
        self.take(cue).map_err(|error| error.to_string())
    }

    fn render(&mut self, samples: &mut [i16]) -> std::result::Result<(), String> {
        self.play(samples).map_err(|error| error.to_string())
    }
}

/// The sequence and sample groups, with `FF` between them, verified in
/// driver RAM before anything plays.
fn load(apu: &mut Apu, groups: &Upload) -> Result<()> {
    upload(apu, &groups.sequence)?;
    apu.write_port(0, 0xff)?;
    upload(apu, &groups.samples)?;
    // Catch dropped/corrupted transfers before playing. In particular the
    // resident receiver echoes ACK before reading the payload input latch.
    for group in [&groups.sequence, &groups.samples] {
        for block in &group.blocks {
            let mut actual = vec![0; block.data.len()];
            apu.read_ram(usize::from(block.destination), &mut actual)?;
            if actual != block.data {
                return Err(format!("music upload mismatch at ${:04x}", block.destination).into());
            }
        }
    }
    for port in 1..=3 {
        apu.write_port(port, 0)?;
    }
    Ok(())
}

fn wait(apu: &mut Apu, port: usize, value: u8) -> Result<()> {
    for _ in 0..4096 {
        if apu.read_port(port)? == value {
            return Ok(());
        }
        apu.run_cycles(16)?;
    }
    Err(format!("music upload timed out waiting for port{port}=${value:02x}").into())
}

fn next_token(length: usize) -> u8 {
    let token = length.wrapping_add(3).to_le_bytes()[0];
    if token == 0 {
        4
    } else {
        token
    }
}

fn upload(apu: &mut Apu, group: &TransferGroup) -> Result<()> {
    if group.blocks.len() > 16
        || group.blocks.iter().any(|block| {
            block.data.is_empty() || usize::from(block.destination) + block.data.len() > 0x10000
        })
    {
        return Err("invalid bounded music transfer".into());
    }
    wait(apu, 0, 0xaa)?;
    wait(apu, 1, 0xbb)?;
    let mut token = 0xcc;
    let blocks = group
        .blocks
        .iter()
        .map(|block| (block.destination, block.data.as_slice()));
    for (destination, payload) in
        blocks.chain(std::iter::once((group.terminal_destination, &[][..])))
    {
        let [low, high] = destination.to_le_bytes();
        apu.write_port(2, low)?;
        apu.write_port(3, high)?;
        apu.write_port(1, u8::from(!payload.is_empty()))?;
        apu.write_port(0, token)?;
        wait(apu, 0, token)?;
        for (index, &value) in payload.iter().enumerate() {
            apu.write_port(1, value)?;
            let counter = index.to_le_bytes()[0];
            apu.write_port(0, counter)?;
            wait(apu, 0, counter)?;
            // SPC $0421 echoes before $0423 reads port1. Do not overwrite it.
            apu.run_cycles(32)?;
        }
        token = next_token(payload.len());
    }
    apu.write_port(2, 0)?;
    apu.write_port(3, 0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music_data::{extract_driver, extract_track, Transfer};

    #[test]
    fn header_tokens_skip_zero_and_wrap_like_the_host() {
        assert_eq!(next_token(1), 4);
        assert_eq!(next_token(253), 4);
        assert_eq!(next_token(254), 1);
        assert_eq!(next_token(65535), 2);
    }

    #[test]
    fn uploads_synthetic_program_through_physical_ipl() {
        let code = vec![0x8f, 0x42, 0xf6, 0x2f, 0xfe]; // MOV $F6,#$42; BRA self
        let group = TransferGroup {
            blocks: vec![Transfer {
                destination: 0x300,
                source_ranges: vec![],
                data: code.clone(),
            }],
            terminal_destination: 0x300,
        };
        let mut apu = Apu::new().unwrap();
        upload(&mut apu, &group).unwrap();
        apu.run_cycles(128).unwrap();
        assert_eq!(apu.read_port(2).unwrap(), 0x42);
        let mut actual = vec![0; code.len()];
        apu.read_ram(0x300, &mut actual).unwrap();
        assert_eq!(actual, code);
    }

    #[test]
    fn missing_ack_times_out_instead_of_hanging() {
        let mut apu = Apu::new().unwrap();
        assert!(wait(&mut apu, 0, 0x12).is_err());
    }

    #[test]
    fn a_change_stops_uploads_and_plays_and_a_fade_comes_first() {
        let groups = || Upload {
            sequence: TransferGroup {
                blocks: Vec::new(),
                terminal_destination: 0,
            },
            samples: TransferGroup {
                blocks: Vec::new(),
                terminal_destination: 0,
            },
        };
        let writes = |ops: &[Op]| -> Vec<(usize, u8)> {
            ops.iter()
                .filter_map(|op| match op {
                    Op::Write(port, value) => Some((*port, *value)),
                    _ => None,
                })
                .collect()
        };
        let plain = change(7, groups(), false, true);
        assert_eq!(
            writes(&plain),
            [
                (2, 0),
                (3, 0),
                (0, 0),
                (1, 7),
                (0, 0xf0),
                (0, 0xff),
                (0, 0xf4)
            ]
        );
        assert!(matches!(plain[plain.len() - 3], Op::Upload(_)));
        let faded = change(7, groups(), true, false);
        assert_eq!(
            writes(&faded)[2..6],
            [(1, 7), (0, 0xf1), (1, 0), (0, 0)],
            "the fade, then the stop"
        );
        assert!(matches!(faded.last(), Some(Op::Upload(_))), "no F4");
    }

    fn rom() -> rom::Rom {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        rom::Rom::load(&bytes).unwrap()
    }

    fn player(rom: &rom::Rom, track: u8) -> Player {
        let owned = rom.clone();
        let mut player = Player::new(&extract_driver(rom).unwrap(), move |track| {
            Ok(extract_track(&owned, track)?)
        })
        .unwrap();
        player.take(Cue::Track { track, fade: false }).unwrap();
        player.settle().unwrap();
        player
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn source_only_crysta_is_non_silent_and_chunk_deterministic() {
        let rom = rom();
        let mut first = player(&rom, 4); // verifies upload RAM before start
        let mut second = player(&rom, 4);
        let mut expected = vec![0; 64_000];
        first.play(&mut expected).unwrap();
        let mut actual = vec![0; expected.len()];
        for chunk in actual.chunks_mut(254) {
            second.play(chunk).unwrap();
        }
        assert_eq!(actual, expected);
        assert!(actual.iter().any(|&sample| sample != 0));
        // Keep running past ring and 16-bit counter wrap, without further host commands.
        for _ in 0..9 {
            first.play(&mut expected).unwrap();
        }
        assert!(expected.iter().filter(|&&s| s != 0).count() > 32_000);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_playing_driver_fades_into_another_track_while_rendering() {
        let rom = rom();
        let mut player = player(&rom, 4);
        let mut buffer = vec![0; 16_000];
        player.play(&mut buffer).unwrap();
        // The underworld's track, after a fade; the port script runs inside
        // the renders and verifies its uploads in driver RAM.
        player
            .take(Cue::Track {
                track: 2,
                fade: true,
            })
            .unwrap();
        // Track 2's parameter fades over some five seconds.
        for _ in 0..30 {
            player.play(&mut buffer).unwrap();
        }
        assert!(player.script.is_empty(), "the change is done");
        player.play(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|&sample| sample != 0),
            "the new track plays"
        );
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn a_sound_effect_changes_the_output() {
        let rom = rom();
        let (mut quiet, mut door) = (player(&rom, 4), player(&rom, 4));
        let (mut expected, mut actual) = (vec![0; 16_000], vec![0; 16_000]);
        // The track's sequence enables effects a few frames in (`$1D`,
        // SPC `$0AB2`); the driver reads ports 2 and 3 only then.
        quiet.play(&mut expected).unwrap();
        door.play(&mut actual).unwrap();
        door.take(Cue::Sound(0x001A)).unwrap();
        quiet.play(&mut expected).unwrap();
        door.play(&mut actual).unwrap();
        assert_ne!(actual, expected);
    }
}
