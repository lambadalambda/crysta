//! Native audio output, kept entirely outside the simulation.

use crate::music_controls::Controls;
use crysta_app::music::Synth;
use crysta_runtime::audio::Cue;
use rodio::{OutputStream, Sink, Source};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError, TrySendError};
use std::thread::JoinHandle;
use std::time::Duration;

const SAMPLE_RATE: u32 = 32_000;
const BLOCK_SAMPLES: usize = 1024; // 16ms of interleaved stereo.

/// One continuous source: the device resampler does not restart at block seams.
/// Its callback never waits for the producer. A late producer yields silence,
/// always a whole stereo frame, rather than blocking the audio device thread.
struct PcmSource {
    receiver: Receiver<Vec<i16>>,
    current: std::vec::IntoIter<i16>,
    right: Option<i16>,
}

impl PcmSource {
    fn new(receiver: Receiver<Vec<i16>>) -> Self {
        Self {
            receiver,
            current: Vec::new().into_iter(),
            right: None,
        }
    }
}

impl Iterator for PcmSource {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        if self.current.len() == 0 {
            match self.receiver.try_recv() {
                Ok(block) => self.current = block.into_iter(),
                Err(TryRecvError::Disconnected) => return None,
                Err(TryRecvError::Empty) => {
                    self.right = Some(0);
                    return Some(0);
                }
            }
        }
        let left = self.current.next()?;
        self.right = self.current.next();
        Some(left)
    }
}

impl Source for PcmSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn apply_controls(sink: &Sink, controls: Controls) {
    sink.set_volume(controls.volume());
    if controls.playing() {
        sink.play();
    } else {
        sink.pause();
    }
}

/// Messages to the worker.
enum Message {
    Controls(Controls),
    Cue(Cue),
}

/// Owns the device and generator worker, not a gameplay clock. The factory runs
/// on that worker so even a non-Send SPC instance never crosses a thread boundary.
/// Only owned PCM blocks cross into rodio's callback.
pub struct Music {
    controls: Option<Sender<Message>>,
    worker: Option<JoinHandle<()>>,
}

impl Music {
    pub fn start<F, R>(factory: F) -> Result<Self, String>
    where
        F: FnOnce() -> Result<R, String> + Send + 'static,
        R: Synth,
    {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("crysta-music".into())
            .spawn(move || {
                let result = run(factory, &rx, &ready_tx);
                if let Err(error) = result {
                    // Before startup this goes to the caller; after startup it is a
                    // diagnostic. Either way gameplay keeps running without audio.
                    let _ = ready_tx.send(Err(error.clone()));
                    eprintln!("music stopped: {error}");
                }
            })
            .map_err(|error| error.to_string())?;
        match ready_rx
            .recv()
            .map_err(|error| error.to_string())
            .and_then(|ready| ready)
        {
            Ok(()) => Ok(Self {
                controls: Some(tx),
                worker: Some(worker),
            }),
            Err(error) => {
                drop(tx);
                let _ = worker.join();
                Err(error)
            }
        }
    }

    pub fn update(&self, controls: Controls) -> Result<(), String> {
        self.send(Message::Controls(controls))
    }

    /// Passes a request from the game on to the synth.
    pub fn cue(&self, cue: Cue) -> Result<(), String> {
        self.send(Message::Cue(cue))
    }

    fn send(&self, message: Message) -> Result<(), String> {
        self.controls
            .as_ref()
            .ok_or("music is closed")?
            .send(message)
            .map_err(|_| "music worker is no longer running".into())
    }
}

impl Drop for Music {
    fn drop(&mut self) {
        self.controls.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run<F, R>(
    factory: F,
    controls_rx: &Receiver<Message>,
    ready: &Sender<Result<(), String>>,
) -> Result<(), String>
where
    F: FnOnce() -> Result<R, String>,
    R: Synth,
{
    let (_stream, handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
    let sink = Sink::try_new(&handle).map_err(|e| e.to_string())?;
    let mut synth = factory()?;
    let (pcm_tx, pcm_rx) = mpsc::sync_channel(2);
    // Prime before playback, then keep at most two queued blocks plus one pending.
    for _ in 0..2 {
        let mut block = vec![0; BLOCK_SAMPLES];
        synth.render(&mut block)?;
        pcm_tx.send(block).map_err(|e| e.to_string())?;
    }
    let mut controls = Controls::default();
    apply_controls(&sink, controls);
    sink.append(PcmSource::new(pcm_rx));
    ready.send(Ok(())).map_err(|e| e.to_string())?;
    let mut pending = None;
    loop {
        match controls_rx.recv_timeout(Duration::from_millis(2)) {
            Ok(Message::Controls(next)) => {
                controls = next;
                apply_controls(&sink, controls);
            }
            // A request the synth cannot follow is skipped, not fatal.
            Ok(Message::Cue(cue)) => {
                if let Err(error) = synth.cue(cue) {
                    eprintln!("music request {cue:x?} skipped: {error}");
                }
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {}
        }
        if !controls.playing() {
            continue;
        }
        let block = if let Some(block) = pending.take() {
            block
        } else {
            let mut block = vec![0; BLOCK_SAMPLES];
            synth.render(&mut block)?;
            block
        };
        match pcm_tx.try_send(block) {
            Ok(()) => {}
            Err(TrySendError::Full(block)) => pending = Some(block),
            Err(TrySendError::Disconnected(_)) => return Err("audio source disconnected".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    /// Counts the blocks it renders.
    struct Counted(
        crate::music::Player,
        std::sync::Arc<std::sync::atomic::AtomicUsize>,
    );

    impl Synth for Counted {
        fn cue(&mut self, cue: Cue) -> Result<(), String> {
            self.0.cue(cue)
        }
        fn render(&mut self, samples: &mut [i16]) -> Result<(), String> {
            self.0.render(samples)?;
            self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    #[ignore = "requires owned JP ROM (CRYSTA_JP_ROM), a real output device and \
                CRYSTA_PLAY_AUDIO=1; plays music"]
    fn native_device_play_pause_resume_and_shutdown() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        // Audible: never as a side effect of `--include-ignored`.
        if std::env::var_os("CRYSTA_PLAY_AUDIO").is_none_or(|value| value != "1") {
            eprintln!("skipped: set CRYSTA_PLAY_AUDIO=1 to play music on the output device");
            return;
        }
        let rom = rom::Rom::load(&std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap())
            .unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let rendered = Arc::clone(&count);
        let music =
            Music::start(move || Ok(Counted(crate::start_player(&rom)?, rendered))).unwrap();
        music
            .cue(Cue::Track {
                track: 4,
                fade: false,
            })
            .unwrap();
        std::thread::sleep(Duration::from_secs(2));
        assert!(
            count.load(Ordering::Relaxed) > 8,
            "device must consume beyond the bounded prebuffer"
        );
        let mut controls = Controls::default();
        controls.toggle();
        music.update(controls).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let paused = count.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            count.load(Ordering::Relaxed),
            paused,
            "pause must stop rendering, not just mute"
        );
        controls.adjust_volume(-10);
        controls.toggle();
        music.update(controls).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        assert!(count.load(Ordering::Relaxed) > paused + 8);
        drop(music); // joins worker and releases device, no detached playback
        let finished = count.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(count.load(Ordering::Relaxed), finished);
        eprintln!(
            "native device: {finished} PCM blocks consumed/generated; pause held at {paused}"
        );
    }

    #[test]
    fn pcm_source_keeps_stereo_order_across_blocks() {
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        tx.send(vec![1, 2, 3, 4]).unwrap();
        tx.send(vec![5, 6]).unwrap();
        let mut source = PcmSource::new(rx);
        assert_eq!(source.channels(), 2);
        assert_eq!(source.sample_rate(), 32_000);
        assert_eq!(
            source.by_ref().take(6).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
        drop(tx);
        assert_eq!(source.next(), None);
    }

    #[test]
    fn underrun_is_nonblocking_and_preserves_stereo_alignment() {
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let mut source = PcmSource::new(rx);
        assert_eq!(source.next(), Some(0));
        tx.send(vec![9, 10]).unwrap();
        // Complete the silent stereo frame before admitting a new block.
        assert_eq!(source.next(), Some(0));
        assert_eq!(source.next(), Some(9));
        assert_eq!(source.next(), Some(10));
    }

    #[test]
    fn sink_controls_do_not_discard_queued_audio() {
        let (sink, _) = rodio::Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(
            2,
            32_000,
            vec![1_i16; 16],
        ));
        let mut controls = crate::music_controls::Controls::default();
        controls.toggle();
        apply_controls(&sink, controls);
        assert!(sink.is_paused());
        assert_eq!(sink.len(), 1);
        assert_eq!(sink.volume().to_bits(), 0.5_f32.to_bits());
        controls.toggle();
        controls.adjust_volume(10);
        apply_controls(&sink, controls);
        assert!(!sink.is_paused());
        assert_eq!(sink.len(), 1);
        assert_eq!(sink.volume().to_bits(), 0.6_f32.to_bits());
    }
}
