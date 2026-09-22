//! Walk the Crysta slice in a native window, on a gamepad.
//!
//! The simulation is [`crysta_runtime`]; this owns a window, a framebuffer and
//! input and music playback. Backgrounds come from the qualified renderer rather
//! than a second decode path, and sprites from the runtime's art module.

mod frame;
mod music;
mod music_controls;
mod music_data;
mod music_output;

use assets::text::{Acknowledgement, DialoguePage};
use crysta_runtime::art::{residents_art, Animation, ArkAtlas, Body, Placeholder};
use crysta_runtime::residents::Conversation;
use crysta_runtime::world::World;
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
    let mut app = App::new(cartridge, image, music);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("{error}");
        std::process::exit(1);
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
        }
    }

    /// One frame of simulation: walking, or paging through dialogue.
    ///
    /// While a conversation is open the player stands still and the button
    /// turns pages; the last page's acknowledgement closes it.
    fn advance(&mut self, direction: Option<Direction>, interact: bool) {
        self.tick += 1;
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
        self.world.step(direction);
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
            None => {
                self.world.interact();
            }
        }
    }

    /// Renders and caches the current map's background and its priority mask.
    fn ensure_background(&mut self, cartridge: &rom::Rom) {
        let map = self.world.map();
        if let std::collections::hash_map::Entry::Vacant(slot) = self.backgrounds.entry(map) {
            let Ok(rendered) = map_inspector::render_static_background(cartridge, map) else {
                return;
            };
            let Some(mut decoded) = frame::decode_bmp(&rendered.bitmap) else {
                return;
            };
            decoded.high = rendered.priorities.iter().map(|bit| *bit != 0).collect();
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
}

impl App {
    fn new(cartridge: rom::Rom, image: &'static [u8], music: Option<music_output::Music>) -> Self {
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
        }
    }

    fn update_music(&mut self) {
        if let Some(music) = &self.music {
            if let Err(error) = music.update(self.music_controls) {
                eprintln!("music unavailable: {error}");
                self.music = None;
            }
        }
        if let Some(window) = &self.window {
            let status = if self.music.is_none() {
                "unavailable".to_string()
            } else if !self.music_controls.playing() {
                "paused".to_string()
            } else {
                format!("{:.0}%", self.music_controls.volume() * 100.0)
            };
            window.set_title(&format!("Crysta — music {status} | M: pause, -/+: volume"));
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
            WindowEvent::CloseRequested => event_loop.exit(),
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

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.poll_pad();
        self.advance();
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
        self.session().advance(direction, interact);
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
