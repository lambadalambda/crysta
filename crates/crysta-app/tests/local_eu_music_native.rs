//! Native-only, ROM-optional European track-4 tempo qualification.
//!
//! This compares log-energy timing, not PCM bits, waveform phase, or a shared
//! start boundary. The ares oracle runs once in an isolated child and no audio
//! device or temporary capture file is involved.
#![cfg(not(target_arch = "wasm32"))]

use assets::text::HouseDialogue;
use crysta_app::music::Player;
use crysta_app::music_data::{extract_driver, extract_track};
use crysta_runtime::audio::Cue;
use oracle::{Button, Session, MAX_AUDIO_FRAMES_PER_FRAME};
use rom::{Revision, Rom};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const TEST: &str = "european_bedroom_track_4_tempo_matches_source_player";
const CHILD: &str = "CRYSTA_EU_MUSIC_CHILD";
const RATE: usize = 32_000;
const ENV_HOP: usize = 128;
const ENV_WINDOW: usize = 1_024;
const ANCHOR_FRAMES: usize = RATE * 3 / 2;
const ANCHOR_CENTERS: [usize; 3] = [RATE, RATE * 6, RATE * 11];
const FINE_RADIUS: usize = RATE / 2;
const MIN_ACQUISITION_SCORE: f64 = 0.85;
const MIN_ANCHOR_SCORE: f64 = 0.80;
const MAX_DRIFT_STEREO_FRAMES: usize = 1_279; // Less than two 640-frame PAL periods.

#[derive(Debug, Clone)]
struct Anchor {
    near_second: usize,
    lag: isize,
    score: f64,
}

#[derive(Debug, Clone)]
struct MatchReport {
    acquisition_score: f64,
    anchors: Vec<Anchor>,
    drift: usize,
}

#[derive(Debug)]
enum Rejection {
    Correlation(MatchReport),
    Drift(MatchReport),
}

impl Rejection {
    fn report(&self) -> &MatchReport {
        match self {
            Self::Correlation(report) | Self::Drift(report) => report,
        }
    }
}

fn as_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).expect("bounded metric length fits u32"))
}

fn envelope(pcm: &[i16]) -> Vec<f64> {
    assert_eq!(pcm.len() % 2, 0, "PCM must be interleaved stereo");
    let frames = pcm.len() / 2;
    assert!(frames >= ENV_WINDOW, "PCM must hold one energy window");
    (0..=frames - ENV_WINDOW)
        .step_by(ENV_HOP)
        .map(|at| {
            let window = &pcm[at * 2..(at + ENV_WINDOW) * 2];
            let energy: f64 = window
                .chunks_exact(2)
                .map(|frame| {
                    let left = f64::from(frame[0]);
                    let right = f64::from(frame[1]);
                    f64::midpoint(left * left, right * right)
                })
                .sum::<f64>()
                / as_f64(ENV_WINDOW);
            energy.ln_1p()
        })
        .collect()
}

fn correlation(left: &[f64], right: &[f64]) -> f64 {
    assert_eq!(left.len(), right.len());
    let count = as_f64(left.len());
    let left_mean = left.iter().sum::<f64>() / count;
    let right_mean = right.iter().sum::<f64>() / count;
    let (numerator, left_power, right_power) = left.iter().zip(right).fold(
        (0.0, 0.0, 0.0),
        |(numerator, left_power, right_power), (&left, &right)| {
            let left = left - left_mean;
            let right = right - right_mean;
            (
                numerator + left * right,
                left_power + left * left,
                right_power + right * right,
            )
        },
    );
    let scale = (left_power * right_power).sqrt();
    if scale > f64::EPSILON {
        numerator / scale
    } else {
        -1.0
    }
}

fn best(reference: &[f64], needle: &[f64], low: usize, high: usize) -> (usize, f64) {
    assert!(!needle.is_empty() && high + needle.len() <= reference.len());
    (low..=high)
        .map(|at| (at, correlation(&reference[at..at + needle.len()], needle)))
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .expect("nonempty bounded lag search")
}

fn tempo_match(native: &[i16], direct: &[i16]) -> Result<MatchReport, Rejection> {
    let native = envelope(native);
    let direct = envelope(direct);
    assert!(
        direct.len() >= native.len(),
        "direct render must bound the native capture"
    );

    // The whole capture finds an arbitrary track offset. Three short local
    // searches then measure timing without requiring common sample phase.
    let (acquired, acquisition_score) = best(&direct, &native, 0, direct.len() - native.len());
    let width = ANCHOR_FRAMES / ENV_HOP;
    let radius = FINE_RADIUS / ENV_HOP;
    let max_start = direct.len() - width;
    let anchors: Vec<_> = ANCHOR_CENTERS
        .into_iter()
        .map(|center| {
            let start = (center - ANCHOR_FRAMES / 2) / ENV_HOP;
            let predicted = acquired + start;
            let low = predicted.saturating_sub(radius);
            let high = (predicted + radius).min(max_start);
            let (at, score) = best(&direct, &native[start..start + width], low, high);
            Anchor {
                near_second: center / RATE,
                lag: isize::try_from(at * ENV_HOP).expect("bounded direct render")
                    - isize::try_from(start * ENV_HOP).expect("bounded native capture"),
                score,
            }
        })
        .collect();
    let drift = anchors[0].lag.abs_diff(anchors[2].lag);
    let report = MatchReport {
        acquisition_score,
        anchors,
        drift,
    };
    if report.acquisition_score < MIN_ACQUISITION_SCORE
        || report
            .anchors
            .iter()
            .any(|anchor| anchor.score < MIN_ANCHOR_SCORE)
    {
        Err(Rejection::Correlation(report))
    } else if report.drift > MAX_DRIFT_STEREO_FRAMES {
        Err(Rejection::Drift(report))
    } else {
        Ok(report)
    }
}

fn evidence(report: &MatchReport) -> String {
    let anchors = report
        .anchors
        .iter()
        .map(|anchor| {
            format!(
                "{}s: lag={} stereo frames score={:.3}",
                anchor.near_second, anchor.lag, anchor.score
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "acquire={:.3}; {anchors}; endpoint drift={} stereo frames",
        report.acquisition_score, report.drift
    )
}

/// Deterministic stereo resampling to 101% duration (one percent slower).
fn stretch_one_percent(pcm: &[i16]) -> Vec<i16> {
    assert_eq!(pcm.len() % 2, 0);
    let input_frames = pcm.len() / 2;
    let output_frames = input_frames * 101 / 100;
    let mut output = Vec::with_capacity(output_frames * 2);
    for frame in 0..output_frames {
        let source = frame * 100;
        let at = source / 101;
        let fraction = i64::try_from(source % 101).expect("fraction fits");
        for channel in 0..2 {
            let first = i64::from(pcm[at * 2 + channel]);
            let second = i64::from(pcm[(at + 1).min(input_frames - 1) * 2 + channel]);
            let sample = (first * (101 - fraction) + second * fraction) / 101;
            output.push(i16::try_from(sample).expect("interpolation remains i16"));
        }
    }
    output
}

fn synthetic_music(seconds: usize, phase: f64) -> Vec<i16> {
    fn sample(value: f64) -> i16 {
        // Every generated amplitude is strictly inside the i16 range.
        #[allow(clippy::cast_possible_truncation)]
        let sample = value.round() as i16;
        sample
    }

    let frames = seconds * RATE;
    (0..frames)
        .flat_map(|frame| {
            let time = as_f64(frame) / as_f64(RATE);
            let shape = 0.58
                + 0.17 * (time * 2.0 * std::f64::consts::PI * 0.37).sin()
                + 0.13 * (time * 2.0 * std::f64::consts::PI * 1.73).sin()
                + 0.09 * (time * 2.0 * std::f64::consts::PI * 3.11).cos();
            let pulse = if (frame / (RATE / 7)) % 9 < 3 {
                1.0
            } else {
                0.72
            };
            let amplitude = 16_000.0 * shape * pulse;
            let carrier = time * 2.0 * std::f64::consts::PI * 997.0;
            [
                sample(amplitude * (carrier + phase).sin()),
                sample(amplitude * (carrier * 1.003 + phase + 0.4).sin()),
            ]
        })
        .collect()
}

fn synthetic_decoy(seconds: usize) -> Vec<i16> {
    let mut pcm = synthetic_music(seconds, 0.37);
    let block_frames = RATE / 11;
    for (frame, channels) in pcm.chunks_exact_mut(2).enumerate() {
        let block = frame / block_frames;
        let gain = i32::try_from(10 + (block * 73 + block * block * 19) % 91)
            .expect("bounded synthetic gain");
        for sample in channels {
            *sample =
                i16::try_from(i32::from(*sample) * gain / 100).expect("attenuation remains i16");
        }
    }
    pcm
}

#[test]
fn synthetic_envelope_match_guards_correlation_and_tempo() {
    const LATENCY: usize = RATE * 2 + RATE / 4;
    let native = synthetic_music(12, 0.0);
    let mut direct = vec![0; LATENCY * 2];
    direct.extend(synthetic_music(14, 0.83));

    let matched = tempo_match(&native, &direct).expect("constant latency and carrier phase pass");
    let latency = isize::try_from(LATENCY).expect("synthetic latency fits isize");
    assert!(
        matched
            .anchors
            .iter()
            .all(|anchor| anchor.lag.abs_diff(latency) <= ENV_HOP * 2),
        "latency should be recovered: {}",
        evidence(&matched)
    );

    let decoy = synthetic_decoy(14);
    let rejection = tempo_match(&native, &decoy).expect_err("unrelated envelope must fail");
    eprintln!("synthetic decoy: {}", evidence(rejection.report()));
    assert!(
        matches!(rejection, Rejection::Correlation(_)),
        "the unrelated envelope must fail specifically on correlation"
    );

    let slower = stretch_one_percent(&direct);
    let rejection = tempo_match(&native, &slower).expect_err("one percent slowdown must fail");
    eprintln!("synthetic 1% slower: {}", evidence(rejection.report()));
    assert!(
        matches!(rejection, Rejection::Drift(_)),
        "the slower signal must fail specifically on drift, not correlation"
    );
}

#[test]
fn european_bedroom_track_4_tempo_matches_source_player() {
    let path = owned_rom_path();
    let image = match std::fs::read(&path) {
        Ok(image) => image,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: owned European ROM absent");
            return;
        }
        Err(error) => panic!("cannot read owned European ROM: {error}"),
    };
    let rom = Rom::load(&image).expect("authenticated European dump");
    assert_eq!(rom.revision(), Revision::EuropeEnglish);
    if std::env::var_os(CHILD).is_some() {
        run_child(&rom);
    }

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", TEST, "--nocapture"])
        .env(CHILD, "1")
        .env("CRYSTA_EU_ROM", &path)
        .output()
        .expect("spawn isolated native-audio test");
    assert!(
        output.status.success(),
        "European music child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("EU bedroom track 4 tempo qualified"),
        "child must report bounded tempo evidence:\n{stderr}"
    );
    eprint!("{stderr}");
}

fn owned_rom_path() -> PathBuf {
    std::env::var_os("CRYSTA_EU_ROM").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc"),
        PathBuf::from,
    )
}

fn run(session: &mut Session, frames: usize, held: Option<Button>) {
    for button in [Button::Start, Button::Down, Button::A] {
        session.set_button(button, held == Some(button));
    }
    session.run_frames(frames);
}

fn render_direct(rom: &Rom) -> Vec<i16> {
    let owned = rom.clone();
    let mut player = Player::new(
        &extract_driver(rom).expect("European music driver"),
        rom.revision(),
        move |track| Ok(extract_track(&owned, track)?),
    )
    .expect("source-only player");
    player
        .take(Cue::Track {
            track: 4,
            fade: false,
        })
        .expect("take bedroom track");
    player.settle().expect("settle track upload");
    let mut pcm = vec![0; RATE * 42 * 2];
    player.play(&mut pcm).expect("render 42 seconds");
    assert!(pcm.iter().any(|&sample| sample != 0));
    pcm
}

fn run_child(rom: &Rom) -> ! {
    let mut native = Session::new(rom).expect("empty-SRAM native session");
    run(&mut native, 1_800, None);
    run(&mut native, 10, Some(Button::Start));
    run(&mut native, 150, None);
    for _ in 0..3 {
        run(&mut native, 12, Some(Button::Down));
        run(&mut native, 18, None);
    }
    run(&mut native, 100, None);
    run(&mut native, 12, Some(Button::A));
    run(&mut native, 300, None);
    run(&mut native, 12, Some(Button::Start));
    run(&mut native, 800, None);
    run(&mut native, 12, Some(Button::A));
    run(&mut native, 600, None);

    let wram = native.wram_image();
    let word = |offset: usize| u16::from_le_bytes([wram[offset], wram[offset + 1]]);
    assert_eq!(word(0x047e), 0x000f, "first bedroom map");
    assert_eq!((word(0x1000), word(0x1002)), (304, 112));
    assert_eq!(wram[0x06c4] & 1, 0, "event $20 remains clear");
    let page = &HouseDialogue::decode_at(rom.image(), 0x88_9c15).expect("first Elle text")[0];
    let boundary = page.boundary_source();
    let text_at = |session: &Session| {
        u32::from(session.wram(0x0dc2)) << 16
            | u32::from(u16::from_le_bytes([
                session.wram(0x0dc0),
                session.wram(0x0dc1),
            ]))
    };
    // At this exact current-build endpoint the page can still be typing. Keep
    // input neutral, with a hard bound, so the music-only window starts at D5.
    let mut d5_wait = 0;
    while text_at(&native) != boundary && d5_wait < 600 {
        run(&mut native, 1, None);
        d5_wait += 1;
    }
    assert_eq!(text_at(&native), boundary, "first page's bounded D5 wait");

    let mut native_pcm = Vec::with_capacity(384_000 * 2);
    let mut frame_counts = BTreeSet::new();
    for _ in 0..600 {
        native.run_frame();
        assert_eq!(native.samples().len() % 2, 0);
        let frames = native.samples().len() / 2;
        assert!(frames <= MAX_AUDIO_FRAMES_PER_FRAME);
        frame_counts.insert(frames);
        native_pcm.extend_from_slice(native.samples());
    }
    let native_frames = native_pcm.len() / 2;
    assert!(
        native_frames.abs_diff(383_946) <= 4,
        "600 PAL frames yielded {native_frames} stereo frames"
    );
    assert!(
        frame_counts.contains(&639) && frame_counts.contains(&640),
        "capture must preserve variable PAL frame lengths: {frame_counts:?}"
    );
    assert!(native_pcm.iter().any(|&sample| sample != 0));
    let wram = native.wram_image();
    let word = |offset: usize| u16::from_le_bytes([wram[offset], wram[offset + 1]]);
    assert_eq!(word(0x047e), 0x000f, "capture remains in the bedroom");
    assert_eq!((word(0x1000), word(0x1002)), (304, 112));
    assert_eq!(wram[0x06c4] & 1, 0, "event $20 remains clear");
    assert_eq!(
        text_at(&native),
        boundary,
        "capture remains at first-page D5"
    );

    let direct_pcm = render_direct(rom);

    let matched = tempo_match(&native_pcm, &direct_pcm).unwrap_or_else(|rejection| {
        panic!("real tempo mismatch: {}", evidence(rejection.report()))
    });
    eprintln!("EU track 4 envelope: {}", evidence(&matched));

    let slower = stretch_one_percent(&direct_pcm);
    let rejection = tempo_match(&native_pcm, &slower)
        .expect_err("stretched real direct render must fail tempo qualification");
    eprintln!(
        "EU track 4 direct 1% slower: {}",
        evidence(rejection.report())
    );
    assert!(
        matches!(rejection, Rejection::Drift(_)),
        "stretched real render must fail specifically by drift"
    );
    eprintln!(
        "EU bedroom track 4 tempo qualified: D5 after {d5_wait} additional neutral frames; 600 PAL frames -> {native_frames} stereo frames; counts {frame_counts:?}"
    );
    std::process::exit(0);
}
