//! Walk the Crysta slice in a native window, on a gamepad.
//!
//! The simulation is [`crysta_runtime`]; this owns a window, a framebuffer and
//! input and music playback. Backgrounds come from the qualified renderer rather
//! than a second decode path, and sprites from the runtime's art module.

mod diagnostics;
mod input;
mod music_controls;
mod music_output;

use crysta_app::session::{Buttons, Session, START};
use crysta_app::{background, clock, frame, music, music_data};

use crysta_runtime::art::Placeholder;
use crysta_runtime::residents::Conversation;
use crysta_runtime::world::{Step, World};
use frame::{Canvas, CLASSIC_WIDTH, VIEW_HEIGHT, WIDE_WIDTH};
use gilrs::{Axis, Button, Gilrs};
use room_core::Direction;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: crysta-app <japanese-rom>");
        std::process::exit(2);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("reading {path}: {error}");
            std::process::exit(1);
        }
    };
    let cartridge = match rom::Rom::load(&bytes) {
        Ok(cartridge) => cartridge,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    };
    if cartridge.revision() != rom::Revision::Japan {
        eprintln!("the slice is qualified only for the Japanese reference");
        std::process::exit(1);
    }
    // Headless: run a step script and write the composed frame out, so the
    // renderer can be inspected without a window.
    // The image is leaked once, so the world can borrow it for the run.
    let image: &'static [u8] = Box::leak(cartridge.image().to_vec().into_boxed_slice());
    // `--wide` combines with either mode, so it is taken out first.
    let mut rest: Vec<String> = arguments.collect();
    let width = if rest.iter().any(|argument| argument == "--wide") {
        WIDE_WIDTH
    } else {
        CLASSIC_WIDTH
    };
    rest.retain(|argument| argument != "--wide");
    let mut arguments = rest.into_iter();
    let mode = arguments.next();
    if mode.as_deref() == Some("--screenshot") {
        let path = arguments.next().unwrap_or_else(|| "frame.ppm".into());
        let script = arguments.next().unwrap_or_default();
        screenshot(&cartridge, image, (&path, &script), width);
        return;
    }
    if mode.as_deref().is_some_and(|mode| mode != "--no-music") {
        eprintln!("unknown option; use --wide, --no-music or --screenshot <path> <script>");
        std::process::exit(2);
    }
    let music = if mode.is_none() {
        match start_music(&cartridge) {
            Ok(music) => {
                eprintln!(
                    "Crysta music: M pauses/resumes, -/+ changes volume; unfocusing pauses music."
                );
                Some(music)
            }
            Err(error) => {
                eprintln!("music unavailable (continuing silently): {error}");
                None
            }
        }
    } else {
        None
    };
    let event_loop = EventLoop::new().expect("an event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let log = start_diagnostics(&cartridge, music.is_some(), width);
    let mut app = App::new(cartridge, image, music, log, width);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn start_diagnostics(
    cartridge: &rom::Rom,
    music: bool,
    width: usize,
) -> Option<diagnostics::SessionLog> {
    let executable_sha256 = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| rom::digests(&bytes).sha256);
    let metadata = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"), "executable_sha256": executable_sha256,
        "rom_sha256": cartridge.digests().sha256, "music_available": music, "view_width": width,
        "initial": {"map": START.0, "x": START.1, "y": START.2},
        "movement_policy": "interactive-refusal-reset-v1",
        "timing_policy": "ntsc-mean-fixed-step-v1",
        "tick_period_ns": clock::FRAME_PERIOD.as_nanos(), "max_catch_up":clock::MAX_CATCH_UP,
        "limits": "portable-host diagnostics, not native qualification; rotated history may be incomplete"
    });
    match diagnostics::SessionLog::start(std::path::Path::new("local/crysta-app/logs"), metadata) {
        Ok(log) => {
            eprintln!(
                "Diagnostic log: {} (send both events-*.jsonl files in this session folder)",
                log.path().display()
            );
            Some(log)
        }
        Err(error) => {
            eprintln!("diagnostic logging unavailable (continuing): {error}");
            None
        }
    }
}

fn start_music(cartridge: &rom::Rom) -> Result<music_output::Music, String> {
    let cartridge = cartridge.clone();
    music_output::Music::start(move || start_player(&cartridge))
}

/// Boots the driver; tracks come from the cartridge as the game asks.
fn start_player(cartridge: &rom::Rom) -> Result<music::Player, String> {
    let driver = music_data::extract_driver(cartridge).map_err(|error| error.to_string())?;
    let cartridge = cartridge.clone();
    music::Player::new(&driver, move |track| {
        Ok(music_data::extract_track(&cartridge, track)?)
    })
    .map_err(|error| error.to_string())
}

/// Runs a step script and writes the composed view as a PPM.
///
/// The script is comma-separated: `down:400` walks 400 frames down, `wait:5`
/// stands for 5, `talk` presses the interact button once, and `at:D:200:700`
/// re-enters map `$000D` at (200,700) to look at a room directly.
fn screenshot(
    cartridge: &rom::Rom,
    image: &'static [u8],
    (path, script): (&str, &str),
    width: usize,
) {
    let mut session = Session::new(image);
    for step in script.split(',').filter(|step| !step.is_empty()) {
        if let Some(rest) = step.strip_prefix("at:") {
            let mut parts = rest.split(':');
            let parsed = (
                parts.next().and_then(|v| u16::from_str_radix(v, 16).ok()),
                parts.next().and_then(|v| v.parse::<u16>().ok()),
                parts.next().and_then(|v| v.parse::<u16>().ok()),
            );
            let (Some(map), Some(x), Some(y)) = parsed else {
                eprintln!("bad placement in {step:?}");
                std::process::exit(2);
            };
            session.world = World::enter(image, map, x, y).unwrap_or_else(|error| {
                eprintln!("cannot enter {map:#06x}: {error}");
                std::process::exit(1);
            });
            session.background_clock = background::VisitClock::new(map);
            continue;
        }
        let (what, count) = step.split_once(':').unwrap_or((step, "1"));
        let Ok(count) = count.parse::<usize>() else {
            eprintln!("bad step count in {step:?}");
            std::process::exit(2);
        };
        let direction = match what {
            "up" => Some(Direction::Up),
            "down" => Some(Direction::Down),
            "left" => Some(Direction::Left),
            "right" => Some(Direction::Right),
            "wait" => None,
            "talk" | "cancel" => {
                session.advance(None, what == "talk", what == "cancel");
                continue;
            }
            other => {
                eprintln!("unknown step {other:?}");
                std::process::exit(2);
            }
        };
        for _ in 0..count {
            session.advance(direction, false, false);
        }
    }
    if let Some(error) = &session.fault {
        eprintln!("screenshot run stopped: {error}");
        std::process::exit(1);
    }
    let mut canvas = Canvas::new(width);
    let camera = session.compose(cartridge, &mut canvas);
    let mut out = format!("P6\n{width} {VIEW_HEIGHT}\n255\n").into_bytes();
    for pixel in &canvas.pixels {
        // Truncation is the point: the low byte of each channel.
        let channel = |shift: u32| u8::try_from((pixel >> shift) & 0xFF).unwrap_or(0);
        out.extend_from_slice(&[channel(16), channel(8), channel(0)]);
    }
    std::fs::write(path, out).expect("writing the frame");
    let roster = session.world.residents().to_vec();
    let statuses: Vec<String> = session
        .resident_art()
        .iter()
        .map(|art| match art {
            Ok(_) => "drawn".to_string(),
            Err(Placeholder::Invisible) => "invisible".to_string(),
            Err(other) => format!("{other:?}"),
        })
        .collect();
    for (resident, status) in roster.iter().zip(&statuses) {
        let events = assets::maps::scripts::EventFlags::Bitmap(session.world.events());
        let says = match crysta_runtime::residents::talk_to(session.image, resident, events) {
            Conversation::Speaks { pages, .. } => format!("speaks {} page(s)", pages.len()),
            Conversation::Silent => "silent".to_string(),
            Conversation::Unsupported { source } => format!("undecodable text at ${source:04x}"),
            Conversation::Unaccounted { service } => format!("stops at COP ${service:02x}"),
        };
        println!(
            "  resident {:#08x} at {:?} pose {}{}: {status}, {says}",
            resident.record,
            resident.position,
            resident.selector,
            if resident.hflip { "m" } else { "" }
        );
    }
    println!(
        "map {:#06x} at {:?}, camera {camera:?}, {} residents, dialogue {} -> {path}",
        session.world.map(),
        session.world.position(),
        session.world.residents().len(),
        match (session.world.dialogue(), session.world.in_scene()) {
            (Some(view), _) if view.cursor.is_some() => "choice",
            (Some(_), true) => "open, world held",
            (Some(_), false) => "open",
            (None, _) => "closed",
        },
    );
}

struct App {
    cartridge: rom::Rom,
    /// The ROM image, leaked once in `main`.
    image: &'static [u8],
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    state: Option<Session>,
    pads: Option<Gilrs>,
    held: Option<Direction>,
    keys: Vec<Direction>,
    /// Classic or wide; `V` switches between them.
    frame: Canvas,
    /// Edge-triggered, so holding the button does not re-talk every frame.
    interaction: input::Interaction,
    /// B: cancels a choice. Edge-triggered like the confirm button.
    cancel: input::Interaction,
    describe: input::Interaction,
    started: std::time::Instant,
    clock: clock::Clock,
    suspended: bool,
    music: Option<music_output::Music>,
    music_controls: music_controls::Controls,
    log: Option<diagnostics::SessionLog>,
}

impl App {
    fn new(
        cartridge: rom::Rom,
        image: &'static [u8],
        music: Option<music_output::Music>,
        log: Option<diagnostics::SessionLog>,
        width: usize,
    ) -> Self {
        Self {
            cartridge,
            image,
            window: None,
            surface: None,
            state: None,
            pads: Gilrs::new().ok(),
            held: None,
            keys: Vec::new(),
            frame: Canvas::new(width),
            interaction: input::Interaction::default(),
            cancel: input::Interaction::default(),
            describe: input::Interaction::default(),
            started: std::time::Instant::now(),
            clock: clock::Clock::new(std::time::Duration::ZERO),
            suspended: false,
            music,
            music_controls: music_controls::Controls::default(),
            log,
        }
    }

    fn record_diagnostic(&mut self, event: &serde_json::Value, urgent: bool) {
        if let Some(log) = &mut self.log {
            if let Err(error) = log.record(event, urgent) {
                eprintln!("diagnostic logging disabled (game continues): {error}");
                self.log = None;
            }
        }
    }

    /// Passes the game's music and sound requests on to the driver.
    fn play(&mut self, cues: &[crysta_runtime::audio::Cue]) {
        let Some(music) = &self.music else {
            return;
        };
        if let Some(error) = cues.iter().find_map(|&cue| music.cue(cue).err()) {
            self.lose_music(&error);
            self.update_title();
        }
    }

    fn lose_music(&mut self, error: &str) {
        eprintln!("music unavailable: {error}");
        self.music = None;
    }

    fn update_music(&mut self) {
        if let Some(Err(error)) = self
            .music
            .as_ref()
            .map(|music| music.update(self.music_controls))
        {
            self.lose_music(&error);
        }
        self.record_diagnostic(
            &serde_json::json!({"kind":"host", "event":"music_state",
            "playing":self.music_controls.playing(), "volume":self.music_controls.volume(),
            "available":self.music.is_some()}),
            true,
        );
        self.update_title();
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let status = if self.music.is_none() {
                "unavailable".to_string()
            } else if !self.music_controls.playing() {
                "paused".to_string()
            } else {
                format!("{:.0}%", self.music_controls.volume() * 100.0)
            };
            let fault = self.state.as_ref().and_then(|state| state.fault.as_deref());
            let suffix =
                fault.map_or_else(String::new, |error| format!(" | WORLD STOPPED: {error}"));
            let view = if self.frame.width == WIDE_WIDTH {
                "16:9"
            } else {
                "classic"
            };
            window.set_title(&format!(
                "Crysta — music {status} | M: pause, -/+: volume | V: view {view}{suffix}"
            ));
        }
    }

    /// Switches between the classic and the wide view, keeping the window's
    /// height and fitting its width unless the window is maximized.
    fn toggle_view(&mut self) {
        let width = if self.frame.width == WIDE_WIDTH {
            CLASSIC_WIDTH
        } else {
            WIDE_WIDTH
        };
        self.frame = Canvas::new(width);
        if let Some(window) = &self.window {
            if !window.is_maximized() && window.fullscreen().is_none() {
                let height = window.inner_size().height;
                let across = frame::fitted_width(width, height);
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(across, height));
            }
            window.request_redraw();
        }
        self.record_diagnostic(
            &serde_json::json!({"kind":"host", "event":"view", "width":width}),
            true,
        );
        self.update_title();
    }

    fn session(&mut self) -> &mut Session {
        let image = self.image;
        self.state.get_or_insert_with(|| Session::new(image))
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.clock.reset(self.started.elapsed());
        self.suspended = false;
        let attributes = Window::default_attributes()
            .with_title("Crysta")
            .with_inner_size(winit::dpi::LogicalSize::new(
                u32::try_from(self.frame.width * 3).unwrap_or(768),
                u32::try_from(VIEW_HEIGHT * 3).unwrap_or(672),
            ));
        let window = Rc::new(event_loop.create_window(attributes).expect("a window"));
        let context = softbuffer::Context::new(window.clone()).expect("a drawing context");
        self.surface = Some(softbuffer::Surface::new(&context, window.clone()).expect("a surface"));
        self.window = Some(window);
        self.music_controls.set_focused(true);
        self.update_music();
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        self.interaction = input::Interaction::default();
        self.cancel = input::Interaction::default();
        self.describe = input::Interaction::default();
        self.keys.clear();
        self.held = None;
        self.music_controls.set_focused(false);
        self.update_music();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.record_diagnostic(
                    &serde_json::json!({"kind":"shutdown", "reason":"window_close"}),
                    true,
                );
                event_loop.exit();
            }
            // OS auto-repeat would page through a whole conversation on a
            // held button and shuffle held directions; a repeat is not a press.
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                let pressed = event.state == ElementState::Pressed;
                let direction = match event.physical_key {
                    PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::KeyW) => Some(Direction::Up),
                    PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::KeyS) => Some(Direction::Down),
                    PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => Some(Direction::Left),
                    PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => {
                        Some(Direction::Right)
                    }
                    PhysicalKey::Code(KeyCode::KeyV) if pressed => {
                        self.toggle_view();
                        None
                    }
                    PhysicalKey::Code(KeyCode::KeyM) if pressed => {
                        self.music_controls.toggle();
                        self.update_music();
                        None
                    }
                    PhysicalKey::Code(KeyCode::Minus | KeyCode::NumpadSubtract) if pressed => {
                        self.music_controls.adjust_volume(-10);
                        self.update_music();
                        None
                    }
                    PhysicalKey::Code(KeyCode::Equal | KeyCode::NumpadAdd) if pressed => {
                        self.music_controls.adjust_volume(10);
                        self.update_music();
                        None
                    }
                    PhysicalKey::Code(KeyCode::Escape) if pressed => {
                        self.record_diagnostic(
                            &serde_json::json!({"kind":"shutdown", "reason":"escape"}),
                            true,
                        );
                        event_loop.exit();
                        None
                    }
                    PhysicalKey::Code(KeyCode::Space | KeyCode::Enter) => {
                        self.interaction.keyboard(pressed);
                        None
                    }
                    PhysicalKey::Code(KeyCode::KeyX | KeyCode::Backspace) => {
                        self.cancel.keyboard(pressed);
                        None
                    }
                    PhysicalKey::Code(KeyCode::KeyQ) => {
                        self.describe.keyboard(pressed);
                        None
                    }
                    _ => None,
                };
                if let Some(direction) = direction {
                    self.keys.retain(|held| *held != direction);
                    if pressed {
                        self.keys.push(direction);
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                self.music_controls.set_focused(focused);
                self.update_music();
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.suspended
            || self
                .state
                .as_ref()
                .is_some_and(|session| session.fault.is_some())
        {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        self.poll_pad();
        let batch = self.clock.poll(self.started.elapsed());
        for _ in 0..batch.steps {
            self.advance();
            if self.session().fault.is_some() {
                break;
            }
        }
        if batch.dropped_backlog {
            let tick = self.session().tick;
            self.record_diagnostic(
                &serde_json::json!({"kind":"host", "event":"timing_backlog_dropped",
                "tick":tick}),
                true,
            );
        }
        if self
            .state
            .as_ref()
            .is_some_and(|session| session.fault.is_some())
        {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if batch.steps != 0 {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.started + self.clock.deadline()));
    }
}

impl App {
    fn poll_pad(&mut self) {
        let (mut interact, mut cancel, mut describe) = (false, false, false);
        let mut direction = self.keys.last().copied();
        if let Some(pads) = &mut self.pads {
            while pads.next_event().is_some() {}
            for (_, pad) in pads.gamepads() {
                if pad.is_pressed(Button::DPadUp) {
                    direction = Some(Direction::Up);
                } else if pad.is_pressed(Button::DPadDown) {
                    direction = Some(Direction::Down);
                } else if pad.is_pressed(Button::DPadLeft) {
                    direction = Some(Direction::Left);
                } else if pad.is_pressed(Button::DPadRight) {
                    direction = Some(Direction::Right);
                } else {
                    // A stick past half deflection counts as a direction.
                    let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
                    if x.abs() > 0.5 || y.abs() > 0.5 {
                        direction = Some(if x.abs() > y.abs() {
                            if x > 0.0 {
                                Direction::Right
                            } else {
                                Direction::Left
                            }
                        } else if y > 0.0 {
                            Direction::Up
                        } else {
                            Direction::Down
                        });
                    }
                }
                // SNES A confirms and B cancels: the pad's South and East.
                interact |= pad.is_pressed(Button::South);
                cancel |= pad.is_pressed(Button::East);
                describe |= pad.is_pressed(Button::LeftTrigger);
            }
        }
        self.held = direction;
        // Level-polled pad edges and keyboard events remain latched until a
        // simulation tick, even if several redraw/input wakeups happen first.
        self.interaction.gamepad(interact);
        self.cancel.gamepad(cancel);
        self.describe.gamepad(describe);
    }

    fn advance(&mut self) {
        let direction = self.held;
        let interact = self.interaction.take();
        let cancel = self.cancel.take();
        let describe = self.describe.take();
        let session = self.session();
        if session.fault.is_some() {
            return;
        }
        let before = (session.world.map(), session.world.position());
        session.advance_with(
            direction,
            Buttons {
                confirm: interact,
                cancel,
                describe,
            },
        );
        let urgent = session.fault.is_some() || matches!(session.last_step, Some(Step::Refused(_)));
        let cues = session.world.take_cues();
        self.play(&cues);
        if self.log.is_some() {
            let mut event = self.session().trace_frame(before, direction, interact);
            event["host_elapsed_ns"] = serde_json::json!(self.started.elapsed().as_nanos());
            if !cues.is_empty() {
                event["cues"] = serde_json::json!(format!("{cues:x?}"));
            }
            self.record_diagnostic(&event, urgent);
        }
        if self.session().fault.is_some() {
            self.update_title();
        }
    }

    fn draw(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        let cartridge = self.cartridge.clone();
        // A redraw can arrive before the first wait, so the session is created
        // here rather than assumed.
        self.session();
        let frame = &mut self.frame;
        let session = self.state.as_mut().expect("just created");
        if session.fault.is_some() {
            return;
        } // Do not render/reuse a partially advanced world.
        session.compose(&cartridge, frame);
        if let Some(surface) = &mut self.surface {
            surface.resize(width, height).expect("resize");
            let mut buffer = surface.buffer_mut().expect("a buffer");
            frame::present(
                &*frame,
                &mut buffer,
                (size.width as usize, size.height as usize),
            );
            buffer.present().expect("present");
        }
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn river_changes_while_standing_without_requiring_intermediate_redraws() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let mut session = Session::new(image);
        session.world = World::enter(image, 0xA, 640, 400).unwrap();
        session.background_clock = background::VisitClock::new(0xA);
        session.ensure_background(&rom);
        let river = |session: &Session| {
            let bg = &session.backgrounds[&0xA].frame;
            (352..600)
                .map(|y| bg.pixels[y * bg.width + 672])
                .collect::<Vec<_>>()
        };
        let first = river(&session);
        let mut changed = false;
        for count in [1, 3, 7, 21] {
            // Screenshot scripts can run multiple updates before first composition.
            for _ in 0..count {
                session.advance(None, false, false);
            }
            session.ensure_background(&rom);
            changed |= river(&session) != first;
        }
        assert_eq!(session.world.position(), (640, 400));
        assert!(changed, "the river must flow while Ark stands still");
        assert_eq!(session.background_clock.tick(), 32);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn the_camera_stays_in_the_map_region_not_the_shared_layer() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let mut session = Session::new(image);
        // `$0C` is the second page of a layer it shares with `$0B` and `$0D`;
        // the exterior's 1280-pixel sheet has a 1024-pixel region. Wide, the
        // room is centred between sides that repeat its edge pixels, and the
        // exterior shows more.
        for (width, map, position, camera) in [
            (CLASSIC_WIDTH, 0xC, (136, 300), (0, 256)),
            (CLASSIC_WIDTH, 0xA, (504, 1000), (376, 768)),
            (WIDE_WIDTH, 0xC, (136, 300), (-72, 256)),
            (WIDE_WIDTH, 0xA, (504, 1000), (304, 768)),
        ] {
            let mut frame = Canvas::new(width);
            session.world = World::enter(image, map, position.0, position.1).unwrap();
            session.background_clock = background::VisitClock::new(map);
            assert_eq!(session.compose(&rom, &mut frame), camera, "map {map:#x}");
            let side = (WIDE_WIDTH - CLASSIC_WIDTH) / 2;
            let inside: Vec<_> = frame
                .pixels
                .chunks(width)
                .flat_map(|row| row.iter().enumerate())
                .filter(|(x, _)| !(camera.0 < 0 && (*x < side || *x >= side + CLASSIC_WIDTH)))
                .collect();
            for row in frame.pixels.chunks(width).filter(|_| camera.0 < 0) {
                assert!(row[..side].iter().all(|pixel| *pixel == row[side]));
                let right = side + CLASSIC_WIDTH - 1;
                assert!(row[right..].iter().all(|pixel| *pixel == row[right]));
            }
            assert!(inside.iter().filter(|(_, pixel)| **pixel != 0).count() > inside.len() / 2);
        }
        for map in 0xA..=0x21 {
            assert!(
                background::load(&rom, map, &crysta_runtime::world::new_game_flags()).is_ok(),
                "map {map:#x}"
            );
        }
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn exterior_transparency_uses_source_backdrop_not_inspector_checkerboard() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let mut session = Session::new(image);
        session.world = World::enter(image, 0xA, 504, 769).unwrap();
        session.ensure_background(&rom);
        let background = &session.backgrounds[&0xA].frame;
        let source = assets::maps::visual::StaticBackground::from_rom(image, 0xA).unwrap();
        assert_eq!(source.palette()[32].raw(), 0x15ed);
        let [r, g, b] = source.palette()[32].rgb8();
        let backdrop = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        let mut transparent = 0;
        // Tree rectangle in the qualified landed-A capture, away from leaf effects.
        for y in 701..762 {
            for x in 384..414 {
                if source.pixel(x, y).unwrap() == assets::graphics::IndexedPixel::Transparent {
                    transparent += 1;
                    assert_eq!(background.pixels[y * background.width + x], backdrop);
                    assert!(!background.occludes(x, y));
                }
            }
        }
        assert_eq!(transparent, 47);
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn outdoor_tree_refusal_does_not_lock_native_session() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let mut session = Session::new(image);
        session.world = World::enter(image, 0xA, 360, 472).unwrap();
        for _ in 0..3 {
            session.advance(Some(Direction::Right), false, false);
        }
        assert_eq!(
            session.world.position(),
            (360, 472),
            "refused step must not move"
        );
        for _ in 0..8 {
            session.advance(None, false, false);
        }
        for _ in 0..16 {
            session.advance(Some(Direction::Left), false, false);
        }
        assert!(
            session.world.position().0 < 360,
            "must be able to turn away from the refused tree edge"
        );
        assert!(
            session.world.residents()[0].pose_age > 2,
            "residents must not remain frozen"
        );
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn outdoor_trace_records_refusal_escape_and_survives_logging_failure() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let root = std::env::temp_dir().join(format!(
            "crysta-app-trace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let log = diagnostics::SessionLog::start(&root, serde_json::json!({"test":true})).unwrap();
        let path = log.path().to_owned();
        let mut app = App::new(rom, image, None, Some(log), CLASSIC_WIDTH);
        app.session().world = World::enter(image, 0xA, 360, 472).unwrap();
        for (direction, count) in [
            (Some(Direction::Right), 3),
            (None, 8),
            (Some(Direction::Left), 16),
        ] {
            app.held = direction;
            for _ in 0..count {
                app.advance();
            }
        }
        app.log.as_mut().unwrap().flush().unwrap();
        let records: Vec<serde_json::Value> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 28); // Header + 27 host updates.
        for (index, record) in records[1..].iter().enumerate() {
            assert_eq!(record["tick"], index + 1);
            assert_eq!(record["before"]["map"], 10);
            assert_eq!(record["after"]["map"], 10);
            assert_eq!(record["input"]["interact"], false);
            assert!(record["error"].is_null());
            if index > 0 {
                assert_eq!(record["before"], records[index]["after"]);
            }
            let direction = match index {
                0..=2 => serde_json::json!("right"),
                3..=10 => serde_json::Value::Null,
                _ => serde_json::json!("left"),
            };
            assert_eq!(record["input"]["direction"], direction);
        }
        assert_eq!(
            records[3]["outcome"],
            serde_json::json!({"kind":"refused", "reason":"UnsupportedType(6)"})
        );
        assert_eq!(
            records[3]["before"],
            serde_json::json!({"map":10,"x":360,"y":472})
        );
        assert_eq!(records[3]["before"], records[3]["after"]);
        assert!(records.last().unwrap()["after"]["x"].as_u64().unwrap() < 360);
        // A record too large for the bounded logger follows the same error path
        // as failed I/O: the host disables logging and keeps simulating.
        app.record_diagnostic(&serde_json::json!("x".repeat(8 * 1024 * 1024)), true);
        assert!(app.log.is_none());
        let before = app.session().world.position();
        app.held = Some(Direction::Up);
        for _ in 0..32 {
            app.advance();
        }
        assert_ne!(app.session().world.position(), before);
        assert!(app.session().fault.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn fatal_frame_is_logged_once_and_outcomes_preserve_each_action() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let mut damaged = rom.image().to_vec();
        let exits = assets::maps::exits::ExitList::from_rom(&damaged, 0xB).unwrap();
        let record = exits
            .records()
            .iter()
            .find(|record| {
                record
                    .direct_destination()
                    .is_ok_and(|map| map != 0xB && crysta_runtime::MAPS.contains(&map))
            })
            .unwrap();
        let destination = record.direct_destination().unwrap();
        let position = (
            u16::from(record.x()) * 16 + 8,
            u16::from(record.y()) * 16 + 16,
        );
        let pointer = 0x18000 + usize::from(destination) * 2;
        damaged[pointer..pointer + 2].copy_from_slice(&1u16.to_le_bytes());
        let image = Box::leak(damaged.into_boxed_slice());
        let root = std::env::temp_dir().join(format!("crysta-fatal-trace-{}", std::process::id()));
        let log = diagnostics::SessionLog::start(&root, serde_json::json!({"test":true})).unwrap();
        let path = log.path().to_owned();
        let mut app = App::new(rom, image, None, Some(log), CLASSIC_WIDTH);
        app.session().world = World::enter(image, 0xB, position.0, position.1).unwrap();
        // Exercise trace precedence without needing a naturally coincident
        // refused movement + successful interaction at the same doorway.
        let session = app.session();
        session.last_step = Some(Step::Refused(room_core::Unqualified::UnsupportedType(6)));
        session.last_interaction = Some(Step::Entered {
            from: 0xB,
            to: destination,
        });
        let frame = session.trace_frame((0xB, position), Some(Direction::Right), true);
        assert_eq!(frame["movement"]["kind"], "refused");
        assert_eq!(frame["interaction"]["kind"], "entered");
        assert_eq!(frame["outcome"]["kind"], "entered");
        let error = session.world.interact_checked().unwrap_err();
        session.last_step = Some(Step::Walked);
        session.fail_world(&error);
        let frame = session.trace_frame((0xB, position), None, true);
        assert_eq!(frame["outcome"]["kind"], "stopped");
        assert_eq!(frame["movement"]["kind"], "walked");
        assert!(frame["error"].as_str().unwrap().contains("exits"));
        // Now drive a real checked destination failure through the host.
        session.fault = None;
        app.interaction.keyboard(true);
        app.advance();
        // The doorway walks Ark out first; the load at its end fails.
        for _ in 0..20 {
            if app.session().fault.is_some() {
                break;
            }
            app.advance();
        }
        assert!(app.session().fault.is_some());
        let tick = app.session().tick;
        let background_tick = app.session().background_clock.tick();
        let position = app.session().world.position();
        let recorded = std::fs::read(&path).unwrap(); // Failure is flushed.
        assert!(String::from_utf8_lossy(&recorded).contains("stopped"));
        for _ in 0..100 {
            app.advance();
        }
        app.log.as_mut().unwrap().flush().unwrap();
        assert_eq!(app.session().tick, tick);
        assert_eq!(app.session().background_clock.tick(), background_tick);
        assert_eq!(app.session().world.position(), position);
        assert_eq!(std::fs::read(&path).unwrap(), recorded);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
