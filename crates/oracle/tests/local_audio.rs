//! Optional native PAL audio capture check. One oracle boot per process.
use oracle::{Button, Session, MAX_AUDIO_FRAMES_PER_FRAME};
use std::{collections::BTreeSet, path::Path, process::Command};

const BUTTONS: [(Button, &str); 6] = [
    (Button::Start, "Start"),
    (Button::A, "A"),
    (Button::Up, "Up"),
    (Button::Down, "Down"),
    (Button::Left, "Left"),
    (Button::Right, "Right"),
];

#[derive(Default)]
struct AudioStats {
    video_frames: usize,
    stereo_frames: usize,
    counts: BTreeSet<usize>,
    nonzero: bool,
}

impl AudioStats {
    fn record(&mut self, session: &Session) {
        let samples = session.samples();
        assert_eq!(samples.len() % 2, 0, "audio must be interleaved stereo");
        let frames = samples.len() / 2;
        assert!(
            frames <= MAX_AUDIO_FRAMES_PER_FRAME,
            "{frames} audio frames exceed the safe per-frame bound"
        );
        self.video_frames += 1;
        self.stereo_frames += frames;
        self.counts.insert(frames);
        self.nonzero |= samples.iter().any(|&sample| sample != 0);
    }
}

#[test]
fn european_bedroom_captures_variable_non_silent_pal_audio() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    if !path.exists() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    if std::env::var_os("ORACLE_AUDIO_CHILD").is_some() {
        run_child(&path);
    }

    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "european_bedroom_captures_variable_non_silent_pal_audio",
            "--nocapture",
        ])
        .env("ORACLE_AUDIO_CHILD", "1")
        .output()
        .expect("spawn isolated audio test");
    assert!(
        output.status.success(),
        "audio child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("PAL audio capture:"),
        "child must report audio evidence: {stderr}"
    );
}

fn run_frames(session: &mut Session, frames: usize, held: &[&str], stats: &mut AudioStats) {
    for (button, name) in BUTTONS {
        session.set_button(button, held.contains(&name));
    }
    for _ in 0..frames {
        session.run_frame();
        stats.record(session);
    }
}

fn replay_line(session: &mut Session, line: &str, stats: &mut AudioStats) {
    let mut parts = line.split_whitespace();
    let frames = parts.next().expect("frame count").parse().expect("frames");
    let held: Vec<_> = parts.collect();
    assert!(held
        .iter()
        .all(|name| BUTTONS.iter().any(|(_, known)| name == known)));
    run_frames(session, frames, &held, stats);
}

fn run_child(path: &Path) -> ! {
    let image = std::fs::read(path).expect("owned European ROM");
    let rom = rom::Rom::load(&image).expect("validated European ROM");
    assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
    let mut session = Session::new(&rom).expect("empty-SRAM native session");
    let mut all = AudioStats::default();

    run_frames(&mut session, 1_800, &[], &mut all);
    run_frames(&mut session, 10, &["Start"], &mut all);
    run_frames(&mut session, 150, &[], &mut all);
    for line in include_str!("fixtures/eu-pandora-tour.inputs")
        .lines()
        .take(12)
    {
        replay_line(&mut session, line, &mut all);
    }

    let wram = session.wram_image();
    let word = |offset: usize| u16::from_le_bytes([wram[offset], wram[offset + 1]]);
    assert_eq!(
        word(0x047e),
        0x000f,
        "input prefix must reach Ark's bedroom"
    );
    assert_eq!((word(0x1000), word(0x1002)), (304, 112));

    let mut bedroom = AudioStats::default();
    for _ in 0..120 {
        session.run_frame();
        all.record(&session);
        bedroom.record(&session);
    }
    assert!(
        bedroom.counts.contains(&639) && bedroom.counts.contains(&640),
        "PAL frame capture should show the expected variable 32 kHz cadence: {:?}",
        bedroom.counts
    );
    assert!(bedroom.nonzero, "bedroom music must produce nonzero PCM");
    assert!(
        (76_780..=76_820).contains(&bedroom.stereo_frames),
        "120 PAL video frames should contain about 76,800 audio frames"
    );
    eprintln!(
        "PAL audio capture: {} video frames -> {} stereo frames; all counts {:?}; bedroom counts {:?}, 120 -> {}, nonzero={}",
        all.video_frames,
        all.stereo_frames,
        all.counts,
        bedroom.counts,
        bedroom.stereo_frames,
        bedroom.nonzero
    );
    std::process::exit(0);
}
