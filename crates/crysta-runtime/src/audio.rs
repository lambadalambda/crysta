//! What the host asks of the sound driver: tracks and sound effects.
//!
//! Tracks come from the track table `$96:F2A0`. A map's loading script
//! selects one (`08 FC n` plays track `n + 1`, `$86:8F37`), and the load
//! restarts nothing when that track is the one loaded (`$86:9145` compares
//! the pointer with `$044E`). Scripts start one outright (`COP 30`, and
//! `COP 60`'s fanfare), after a fade (`COP 31`), or go back to the map's
//! (`COP 32 FF`); each loads the track even when it plays (`$8D:950A`).
//! After `COP 60`'s fanfare, the player's presentation (`$84:BEA2`) waits
//! out the grant's word in frames and goes back to the map's
//! (`$84:BF0A`). The native trace has the spear's return 405 frames after
//! its fanfare, not 420; the difference is not traced.
//!
//! Sound effects go through the latch `$04B6` / `$04B7`: `COP 37` writes
//! the low byte, for port 2, `COP 36` the high byte, for port 3, `COP 38`
//! both. The NMI hands the latch to the ports and clears it; the host
//! paces the ports (see the app's player).

use assets::maps::scripts::{resolve_map_with_events, Command, EventFlags, Limits};

/// A request for the driver, in the order the frame made them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Load and play a track, fading the playing one out first (`F1`) when
    /// `fade` is set.
    Track {
        /// The track table's index.
        track: u8,
        /// Whether the playing track fades out first.
        fade: bool,
    },
    /// A latch word: port 2 its low byte, port 3 its high byte.
    Sound(u16),
}

/// The host's audio state: it outlives map loads, as WRAM does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Audio {
    /// `$04B2`: the selection the last map load chose.
    selection: u8,
    /// `$044E`: the track loaded.
    track: Option<u8>,
    /// `$04B6` / `$04B7`.
    latch: u16,
    /// Frames until a fanfare gives way to the map's track.
    fanfare: Option<u16>,
    cues: Vec<Cue>,
}

impl Audio {
    /// A map load, with the selection its script makes. The track restarts
    /// only when another is loaded; a fanfare's return goes with the
    /// player's presentation.
    pub fn load_map(&mut self, selection: Option<u8>) {
        self.fanfare = None;
        let Some(selection) = selection else {
            return;
        };
        self.selection = selection;
        let track = selection.wrapping_add(1);
        if self.track != Some(track) {
            self.play(track, false);
        }
    }

    /// `COP 30`, `COP 31` (`fade`) and `COP 60`'s fanfare.
    pub fn play(&mut self, track: u8, fade: bool) {
        self.track = Some(track);
        self.cues.push(Cue::Track { track, fade });
    }

    /// `COP 60`'s track, the map's again `frames` frames later.
    pub fn fanfare(&mut self, track: u8, frames: u16) {
        self.play(track, false);
        self.fanfare = Some(frames);
    }

    /// `COP 32`: `FF` plays the map's selection again; any other operand
    /// becomes the selection and plays.
    pub fn select(&mut self, operand: u8) {
        if operand != 0xFF {
            self.selection = operand;
        }
        self.play(self.selection.wrapping_add(1), false);
    }

    /// `COP 37`: port 2's effect.
    pub fn sound_port2(&mut self, id: u8) {
        self.latch = (self.latch & 0xFF00) | u16::from(id);
    }

    /// `COP 36`: port 3's effect.
    pub fn sound_port3(&mut self, id: u8) {
        self.latch = (self.latch & 0x00FF) | (u16::from(id) << 8);
    }

    /// `COP 38`: both ports.
    pub fn sound_word(&mut self, word: u16) {
        self.latch = word;
    }

    /// The frame's end: a latch written this frame goes out, and clears; a
    /// fanfare counts down.
    pub fn end_frame(&mut self) {
        self.flush();
        match self.fanfare {
            Some(0 | 1) => {
                self.fanfare = None;
                self.select(0xFF);
            }
            Some(frames) => self.fanfare = Some(frames - 1),
            None => {}
        }
    }

    /// A latch written so far goes out, and clears.
    pub fn flush(&mut self) {
        if self.latch != 0 {
            self.cues.push(Cue::Sound(self.latch));
            self.latch = 0;
        }
    }

    /// Takes the requests made since the last call.
    pub fn take(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }
}

/// The selection a map's loading script makes with these flags: its last
/// `08 FC` from the first music list (`$86:959C`), or `None` when it makes
/// none and the music plays on.
#[must_use]
pub fn map_selection(image: &[u8], map: u16, events: &[u8]) -> Option<u8> {
    let program =
        resolve_map_with_events(image, map, Limits::default(), EventFlags::Bitmap(events)).ok()?;
    program
        .instructions
        .iter()
        .rev()
        .find(|instruction| instruction.command == Command::AudioSelection)
        .and_then(|instruction| match instruction.bytes[2..] {
            [selection, 0, ..] => Some(selection),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_load_plays_its_selections_track_unless_it_is_loaded() {
        let mut audio = Audio::default();
        audio.load_map(Some(3));
        audio.load_map(Some(3));
        audio.load_map(None);
        audio.load_map(Some(5));
        assert_eq!(
            audio.take(),
            [
                Cue::Track {
                    track: 4,
                    fade: false
                },
                Cue::Track {
                    track: 6,
                    fade: false
                }
            ]
        );
        assert!(audio.take().is_empty());
    }

    #[test]
    fn a_scene_track_plays_even_when_loaded_and_cop_32_goes_back() {
        let mut audio = Audio::default();
        audio.load_map(Some(3));
        audio.play(1, true);
        audio.select(0xFF);
        audio.select(0x1B);
        audio.play(0x1C, false);
        let tracks: Vec<(u8, bool)> = audio
            .take()
            .into_iter()
            .map(|cue| match cue {
                Cue::Track { track, fade } => (track, fade),
                Cue::Sound(_) => unreachable!(),
            })
            .collect();
        assert_eq!(
            tracks,
            [
                (4, false),
                (1, true),
                (4, false),
                (0x1C, false),
                (0x1C, false)
            ]
        );
        // The map's track is loaded again: the next load of it goes on.
        audio.load_map(Some(0x1B));
        assert!(audio.take().is_empty());
    }

    #[test]
    fn a_fanfare_gives_way_to_the_maps_track_after_its_frames() {
        let mut audio = Audio::default();
        audio.load_map(Some(0x1B));
        audio.fanfare(0x34, 2);
        audio.end_frame();
        assert_eq!(audio.take().len(), 2);
        audio.end_frame();
        assert_eq!(
            audio.take(),
            [Cue::Track {
                track: 0x1C,
                fade: false
            }]
        );
    }

    #[test]
    fn the_latch_combines_a_frames_writes_and_goes_out_once() {
        let mut audio = Audio::default();
        audio.flush();
        audio.sound_port2(0x1A);
        audio.sound_port3(0x13);
        audio.flush();
        audio.flush();
        audio.sound_word(0x3737);
        audio.flush();
        assert_eq!(audio.take(), [Cue::Sound(0x131A), Cue::Sound(0x3737)]);
    }
}
