//! Walk the Crysta slice in a native window, on a gamepad.
//!
//! The simulation is [`crysta_runtime`]; this owns a window, a framebuffer and
//! input and music playback. Backgrounds come from the qualified renderer rather
//! than a second decode path, and sprites from the runtime's art module.

mod background;
mod diagnostics;
mod frame;
mod music;
mod music_controls;
mod music_data;
mod music_output;

use assets::text::{Acknowledgement, DialoguePage};
use crysta_runtime::art::{residents_art, Animation, ArkAtlas, Body, Placeholder};
use crysta_runtime::residents::Conversation;
use crysta_runtime::world::{Step, World};
use frame::{VIEW_HEIGHT, VIEW_WIDTH};
use gilrs::{Axis, Button, Gilrs};
use room_core::Direction;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Map the player starts in, and where: fresh startup places them at
/// `(304,112)` in bedroom `$000F` (`docs/new-game-bootstrap.md`).
const START: (u16, u16, u16) = (0x000F, 304, 112);

/// What a resident whose art was refused is drawn as: a block, so that
/// someone is visibly there and visibly not right.
const PLACEHOLDER: u32 = 0x00C0_50C0;

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
    let mode = arguments.next();
    if mode.as_deref() == Some("--screenshot") {
        let path = arguments.next().unwrap_or_else(|| "frame.ppm".into());
        let script = arguments.next().unwrap_or_default();
        screenshot(&cartridge, image, &path, &script);
        return;
    }
    if mode.as_deref().is_some_and(|mode| mode != "--no-music") {
        eprintln!("unknown option; use --no-music or --screenshot <path> <script>");
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
    event_loop.set_control_flow(ControlFlow::Poll);
    let log = start_diagnostics(&cartridge, music.is_some());
    let mut app = App::new(cartridge, image, music, log);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn start_diagnostics(cartridge: &rom::Rom, music: bool) -> Option<diagnostics::SessionLog> {
    let executable_sha256 = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| rom::digests(&bytes).sha256);
    let metadata = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"), "executable_sha256": executable_sha256,
        "rom_sha256": cartridge.digests().sha256, "music_available": music,
        "initial": {"map": START.0, "x": START.1, "y": START.2},
        "movement_policy": "interactive-refusal-reset-v1",
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
    let data = music_data::extract_crysta_music(cartridge).map_err(|error| error.to_string())?;
    music_output::Music::start(move || {
        let mut player = music::initialize(&data).map_err(|error| error.to_string())?;
        Ok(move |samples: &mut [i16]| {
            player
                .render(samples)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })
    })
}

/// Runs a step script and writes the composed view as a PPM.
///
/// The script is comma-separated: `down:400` walks 400 frames down, `wait:5`
/// stands for 5, `talk` presses the interact button once, and `at:D:200:700`
/// re-enters map `$000D` at (200,700) to look at a room directly.
fn screenshot(cartridge: &rom::Rom, image: &'static [u8], path: &str, script: &str) {
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
            "talk" => {
                session.advance(None, true);
                continue;
            }
            other => {
                eprintln!("unknown step {other:?}");
                std::process::exit(2);
            }
        };
        for _ in 0..count {
            session.advance(direction, false);
        }
    }
    if let Some(error) = &session.fault {
        eprintln!("screenshot run stopped: {error}");
        std::process::exit(1);
    }
    let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
    let camera = session.compose(cartridge, &mut frame);
    let mut out = format!("P6\n{VIEW_WIDTH} {VIEW_HEIGHT}\n255\n").into_bytes();
    for pixel in &frame {
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
        session
            .dialogue
            .as_ref()
            .map_or("closed".to_string(), |open| format!(
                "page {} of {}",
                open.index + 1,
                open.pages.len()
            )),
    );
}

/// Resident art for one roster: the map, the records present, the flags in
/// force, and their rasters. The flags are part of the key because a
/// resident's pose comes from their walked script, which branches on them.
type RosterArt = (u16, Vec<usize>, Vec<u8>, Vec<Result<Body, Placeholder>>);

/// A conversation being shown, one page at a time.
struct Dialogue {
    pages: Vec<DialoguePage>,
    index: usize,
}

/// The running world plus what it needs to draw.
struct Session {
    world: World<'static>,
    image: &'static [u8],
    atlas: ArkAtlas,
    backgrounds: HashMap<u16, frame::Background>,
    /// Resident art for the roster it was computed for, keyed by map and by
    /// which records were present, since the flags can change the roster.
    art: Option<RosterArt>,
    /// Rasterized sequences by record, selector and mirror; `None` when the
    /// packet has no such sequence.
    sprites: HashMap<(usize, u8, bool), Option<Animation>>,
    dialogue: Option<Dialogue>,
    /// Frames simulated so far, which drives resident animation.
    tick: u64,
    last_step: Option<Step>,
    last_interaction: Option<Step>,
    last_refusal: Option<room_core::Unqualified>,
    /// Checked build failures are fatal to this world, not retryable input refusals.
    fault: Option<String>,
}

impl Session {
    fn new(image: &'static [u8]) -> Self {
        Self {
            world: World::enter(image, START.0, START.1, START.2).expect("the opening house"),
            image,
            atlas: ArkAtlas::from_rom(image).expect("the player's frames"),
            backgrounds: HashMap::new(),
            art: None,
            sprites: HashMap::new(),
            dialogue: None,
            tick: 0,
            last_step: None,
            last_interaction: None,
            last_refusal: None,
            fault: None,
        }
    }

    /// One frame of simulation: walking, or paging through dialogue.
    ///
    /// While a conversation is open the player stands still and the button
    /// turns pages; the last page's acknowledgement closes it.
    fn advance(&mut self, direction: Option<Direction>, interact: bool) {
        if self.fault.is_some() {
            return;
        }
        self.tick += 1;
        self.last_step = None;
        self.last_interaction = None;
        if let Some(open) = &mut self.dialogue {
            if interact {
                let last = open.index + 1 >= open.pages.len();
                let closes =
                    last || open.pages[open.index].acknowledgement() == Acknowledgement::End;
                if closes {
                    self.dialogue = None;
                } else {
                    open.index += 1;
                }
            }
            return;
        }
        match self.world.step_interactive(direction) {
            Ok(step) => {
                self.last_step = Some(step);
                if let Step::Refused(reason) = step {
                    if self.last_refusal != Some(reason) {
                        eprintln!("movement refused at map {:#06x} {:?}: {reason:?}; input reset, choose another direction", self.world.map(), self.world.position());
                    }
                    self.last_refusal = Some(reason);
                } else if matches!(step, Step::Walked | Step::Entered { .. }) {
                    self.last_refusal = None;
                }
            }
            Err(error) => {
                self.fail_world(&error);
                return;
            }
        }
        if !interact {
            return;
        }
        // Talking first: a resident standing in a doorway should be spoken
        // to rather than walked past.
        match self.world.talk() {
            Some(Conversation::Speaks { pages, .. }) if !pages.is_empty() => {
                // Stand, rather than hold whatever stride the step left.
                self.world.face(self.world.facing());
                self.dialogue = Some(Dialogue { pages, index: 0 });
            }
            Some(Conversation::Speaks { .. }) => {}
            Some(Conversation::Unsupported { source }) => {
                eprintln!("the resident's line at ${source:04x} does not decode as text");
            }
            Some(Conversation::Unaccounted { service }) => {
                eprintln!("the resident's script stops at COP ${service:02x}");
            }
            Some(Conversation::Silent) => eprintln!("..."),
            None => match self.world.interact_checked() {
                Ok(step) => self.last_interaction = Some(step),
                Err(error) => self.fail_world(&error),
            },
        }
    }

    fn trace_frame(
        &self,
        before: (u16, (u16, u16)),
        direction: Option<Direction>,
        interact: bool,
    ) -> serde_json::Value {
        let direction = direction.map(|direction| match direction {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        });
        let describe = |step: Option<Step>| match step {
            Some(Step::Stayed) => serde_json::json!({"kind":"stayed"}),
            Some(Step::Walked) => serde_json::json!({"kind":"walked"}),
            Some(Step::Entered { from, to }) => {
                serde_json::json!({"kind":"entered", "from":from, "to":to})
            }
            Some(Step::Refused(reason)) => {
                serde_json::json!({"kind":"refused", "reason":format!("{reason:?}")})
            }
            None => serde_json::Value::Null,
        };
        let outcome = if self.fault.is_some() {
            serde_json::json!({"kind":"stopped"})
        } else if self.last_step.is_none() {
            serde_json::json!({"kind":"dialogue"})
        } else {
            describe(
                self.last_interaction
                    .filter(|step| *step != Step::Stayed)
                    .or(self.last_step),
            )
        };
        let (x, y) = self.world.position();
        serde_json::json!({"kind":"frame", "tick":self.tick,
            "input":{"direction":direction,"interact":interact},
            "before":{"map":before.0,"x":before.1.0,"y":before.1.1},
            "after":{"map":self.world.map(),"x":x,"y":y},
            "outcome":outcome, "movement":describe(self.last_step),
            "interaction":describe(self.last_interaction), "error":self.fault,
            "dialogue_page":self.dialogue.as_ref().map(|open|open.index+1)})
    }

    fn fail_world(&mut self, error: &crysta_runtime::world::WorldError) {
        let message = format!(
            "map {:#06x} {:?}: {error}",
            self.world.map(),
            self.world.position()
        );
        eprintln!("world stopped: {message}; restart the app (Escape still exits)");
        self.fault = Some(message);
    }

    /// Renders and caches the current map's background and its priority mask.
    fn ensure_background(&mut self, cartridge: &rom::Rom) {
        let map = self.world.map();
        if let std::collections::hash_map::Entry::Vacant(slot) = self.backgrounds.entry(map) {
            let Ok(decoded) = background::load(cartridge, map) else {
                return;
            };
            slot.insert(decoded);
        }
    }

    /// Decodes bodies for the current roster, recomputed when it changes.
    fn ensure_art(&mut self) {
        let map = self.world.map();
        let records: Vec<usize> = self
            .world
            .residents()
            .iter()
            .map(|resident| resident.record)
            .collect();
        let stale = !matches!(
            &self.art,
            Some((cached_map, cached, flags, _))
                if *cached_map == map && *cached == records && flags == self.world.events()
        );
        if stale {
            let events = assets::maps::scripts::EventFlags::Bitmap(self.world.events());
            let art = residents_art(self.image, map, self.world.residents(), events);
            self.art = Some((map, records, self.world.events().to_vec(), art));
        }
    }

    /// Bodies for the current roster, aligned with the world's residents.
    fn resident_art(&mut self) -> &[Result<Body, Placeholder>] {
        self.ensure_art();
        self.art
            .as_ref()
            .map_or(&[], |(_, _, _, art)| art.as_slice())
    }

    /// Composes the view: background, depth-sorted sprites, then dialogue.
    ///
    /// Depth is world Y, ties broken by spawn order with later records first
    /// and the player last, which is what every frozen tie rank encodes. The
    /// count includes residents that draw nothing, which keeps the relative
    /// order and only inflates the player's rank.
    fn compose(&mut self, cartridge: &rom::Rom, frame: &mut [u32]) -> (usize, usize) {
        frame.fill(0);
        self.ensure_background(cartridge);
        self.ensure_art();
        let Session {
            world,
            backgrounds,
            art,
            atlas,
            sprites,
            dialogue,
            ..
        } = self;
        let position = world.position();
        let player = atlas.frame(world.animation());
        let residents = world.residents();
        let bodies: &[Result<Body, Placeholder>] =
            art.as_ref().map_or(&[], |(_, _, _, art)| art.as_slice());
        let Some(background) = backgrounds.get(&world.map()) else {
            return (0, 0);
        };
        let camera = frame::camera(position, (background.width, background.height));
        frame::draw_background(frame, background, camera);
        let count = residents.len();
        let mut order: Vec<(u16, usize, usize)> = residents
            .iter()
            .enumerate()
            .map(|(index, resident)| (resident.position.1, count - 1 - index, index))
            .collect();
        order.push((position.1, count, usize::MAX));
        order.sort_unstable();
        for (_, _, index) in order {
            if index == usize::MAX {
                frame::draw_sprite(frame, background, camera, player, position);
                continue;
            }
            let resident = &residents[index];
            let placeholder = |frame: &mut [u32]| {
                let (x, y) = (
                    i32::from(resident.position.0) - i32::try_from(camera.0).unwrap_or(0),
                    i32::from(resident.position.1) - i32::try_from(camera.1).unwrap_or(0),
                );
                frame::fill(frame, (x - 8, y - 16), (16, 16), PLACEHOLDER);
            };
            match bodies.get(index) {
                Some(Ok(body)) => {
                    let key = (resident.record, resident.selector, resident.hflip);
                    let animation = sprites.entry(key).or_insert_with(|| {
                        // A sequence the packet lacks falls back to the setup
                        // one rather than a block.
                        body.animation(key.1, key.2)
                            .or_else(|_| body.animation(body.initial(), key.2))
                            .ok()
                    });
                    match animation {
                        Some(animation) => {
                            let raster = animation.frame_at(u64::from(resident.pose_age));
                            frame::draw_sprite(
                                frame,
                                background,
                                camera,
                                raster,
                                resident.position,
                            );
                        }
                        None => placeholder(frame),
                    }
                }
                Some(Err(Placeholder::Invisible)) | None => {}
                Some(Err(Placeholder::Refused(_) | Placeholder::PredecessorRefused)) => {
                    placeholder(frame);
                }
            }
        }
        if let Some(open) = dialogue {
            let page = &open.pages[open.index];
            let dimensions = (usize::from(page.width()), usize::from(page.height()));
            let player_screen_y = usize::from(position.1).saturating_sub(camera.1);
            let origin = frame::page_origin(page.placement(), dimensions, player_screen_y);
            frame::draw_page(
                frame,
                page.indexed(),
                dimensions,
                page.background_index(),
                origin,
            );
        }
        camera
    }
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
    frame: Vec<u32>,
    /// Edge-triggered, so holding the button does not re-talk every frame.
    interact_down: bool,
    interact_was_down: bool,
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
            frame: vec![0; VIEW_WIDTH * VIEW_HEIGHT],
            interact_down: false,
            interact_was_down: false,
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

    fn update_music(&mut self) {
        if let Some(music) = &self.music {
            if let Err(error) = music.update(self.music_controls) {
                eprintln!("music unavailable: {error}");
                self.music = None;
            }
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
            window.set_title(&format!(
                "Crysta — music {status} | M: pause, -/+: volume{suffix}"
            ));
        }
    }

    fn session(&mut self) -> &mut Session {
        let image = self.image;
        self.state.get_or_insert_with(|| Session::new(image))
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("Crysta")
            .with_inner_size(winit::dpi::LogicalSize::new(
                u32::try_from(VIEW_WIDTH * 3).unwrap_or(768),
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
                        self.interact_down = pressed;
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
        self.poll_pad();
        self.advance();
        if self.session().fault.is_some() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl App {
    fn poll_pad(&mut self) {
        let mut interact = false;
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
                if pad.is_pressed(Button::South) || pad.is_pressed(Button::East) {
                    interact = true;
                }
            }
        }
        self.held = direction;
        // The pad is level-polled; the keyboard sets the flag on its edges.
        // Either way one press is one interaction.
        let down = interact || self.interact_down;
        let pressed = down && !self.interact_was_down;
        self.interact_was_down = down;
        self.interact_down = pressed;
    }

    fn advance(&mut self) {
        let direction = self.held;
        let interact = std::mem::take(&mut self.interact_down);
        let session = self.session();
        if session.fault.is_some() {
            return;
        }
        let before = (session.world.map(), session.world.position());
        session.advance(direction, interact);
        let urgent = session.fault.is_some() || matches!(session.last_step, Some(Step::Refused(_)));
        if self.log.is_some() {
            let event = self.session().trace_frame(before, direction, interact);
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
                frame,
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
    fn exterior_transparency_uses_source_backdrop_not_inspector_checkerboard() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let image = Box::leak(rom.image().to_vec().into_boxed_slice());
        let mut session = Session::new(image);
        session.world = World::enter(image, 0xA, 504, 769).unwrap();
        session.ensure_background(&rom);
        let background = &session.backgrounds[&0xA];
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
            session.advance(Some(Direction::Right), false);
        }
        assert_eq!(
            session.world.position(),
            (360, 472),
            "refused step must not move"
        );
        for _ in 0..8 {
            session.advance(None, false);
        }
        for _ in 0..16 {
            session.advance(Some(Direction::Left), false);
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
        let mut app = App::new(rom, image, None, Some(log));
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
        let mut app = App::new(rom, image, None, Some(log));
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
        app.interact_down = true;
        app.advance();
        assert!(app.session().fault.is_some());
        let tick = app.session().tick;
        let position = app.session().world.position();
        let recorded = std::fs::read(&path).unwrap(); // Failure is flushed.
        assert!(String::from_utf8_lossy(&recorded).contains("stopped"));
        for _ in 0..100 {
            app.advance();
        }
        app.log.as_mut().unwrap().flush().unwrap();
        assert_eq!(app.session().tick, tick);
        assert_eq!(app.session().world.position(), position);
        assert_eq!(std::fs::read(&path).unwrap(), recorded);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
