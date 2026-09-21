//! Walk the Crysta slice in a native window, on a gamepad.
//!
//! The simulation is [`crysta_runtime`]; this owns a window, a framebuffer and
//! input, and nothing else. Backgrounds come from the qualified renderer rather
//! than a second decode path.

mod frame;

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

/// Map the player starts in, and where. The opening house's first room.
const START: (u16, u16, u16) = (0x000B, 120, 128);

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
    // Headless: compose one frame after N steps and write it out, so the
    // renderer can be inspected without a window.
    if arguments.next().as_deref() == Some("--screenshot") {
        let path = arguments.next().unwrap_or_else(|| "frame.ppm".into());
        let steps: usize = arguments
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let direction = match arguments.next().as_deref() {
            Some("up") => Some(Direction::Up),
            Some("down") => Some(Direction::Down),
            Some("left") => Some(Direction::Left),
            Some("right") => Some(Direction::Right),
            _ => None,
        };
        screenshot(&cartridge, &path, steps, direction);
        return;
    }
    let event_loop = EventLoop::new().expect("an event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(cartridge);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

/// Runs the world for `steps` frames and writes the composed view as a PPM.
fn screenshot(cartridge: &rom::Rom, path: &str, steps: usize, direction: Option<Direction>) {
    let image: &'static [u8] = Box::leak(cartridge.image().to_vec().into_boxed_slice());
    let mut session = Session {
        world: World::enter(image, START.0, START.1, START.2).expect("the opening house"),
        backgrounds: HashMap::new(),
        said: None,
    };
    for _ in 0..steps {
        session.world.step(direction);
    }
    let mut frame = vec![0u32; VIEW_WIDTH * VIEW_HEIGHT];
    let position = session.world.position();
    let residents: Vec<_> = session
        .world
        .residents()
        .iter()
        .map(|resident| resident.position)
        .collect();
    let camera = match session.background(cartridge) {
        Some(background) => {
            let camera = frame::camera(position, (background.width, background.height));
            frame::draw_background(&mut frame, background, camera);
            camera
        }
        None => (0, 0),
    };
    draw_actors(&mut frame, camera, position, &residents);
    let mut out = format!("P6\n{VIEW_WIDTH} {VIEW_HEIGHT}\n255\n").into_bytes();
    for pixel in &frame {
        // Truncation is the point: the low byte of each channel.
        let channel = |shift: u32| u8::try_from((pixel >> shift) & 0xFF).unwrap_or(0);
        out.extend_from_slice(&[channel(16), channel(8), channel(0)]);
    }
    std::fs::write(path, out).expect("writing the frame");
    println!(
        "map {:#06x} at {position:?}, camera {camera:?}, {} residents -> {path}",
        session.world.map(),
        residents.len()
    );
}

/// Draws the player and the residents into a composed frame.
fn draw_actors(
    frame: &mut [u32],
    camera: (usize, usize),
    position: (u16, u16),
    residents: &[(u16, u16)],
) {
    let to_view = |(x, y): (u16, u16)| {
        let origin = (
            i32::try_from(camera.0).unwrap_or(i32::MAX),
            i32::try_from(camera.1).unwrap_or(i32::MAX),
        );
        (i32::from(x) - origin.0, i32::from(y) - origin.1)
    };
    for resident in residents {
        let (x, y) = to_view(*resident);
        frame::fill(frame, (x - 8, y - 16), (16, 16), 0x0000_C8FF);
    }
    let (x, y) = to_view(position);
    frame::fill(frame, (x - 8, y - 16), (16, 16), 0x00FF_FFFF);
    frame::fill(frame, (x - 4, y - 12), (8, 8), 0x00C8_2020);
}

struct App {
    cartridge: rom::Rom,
    /// The ROM image, leaked **once** so the world can borrow it for the run.
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
}

/// The running world plus what it needs to draw.
struct Session {
    world: World<'static>,
    backgrounds: HashMap<u16, frame::Background>,
    said: Option<String>,
}

impl App {
    fn new(cartridge: rom::Rom) -> Self {
        let image: &'static [u8] = Box::leak(cartridge.image().to_vec().into_boxed_slice());
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
        }
    }

    fn session(&mut self) -> &mut Session {
        let image = self.image;
        self.state.get_or_insert_with(|| Session {
            world: World::enter(image, START.0, START.1, START.2).expect("the opening house"),
            backgrounds: HashMap::new(),
            said: None,
        })
    }
}

impl Session {
    /// Renders and caches the current map's background.
    fn background(&mut self, cartridge: &rom::Rom) -> Option<&frame::Background> {
        let map = self.world.map();
        if let std::collections::hash_map::Entry::Vacant(slot) = self.backgrounds.entry(map) {
            let rendered = map_inspector::render_static_background(cartridge, map).ok()?;
            slot.insert(frame::decode_bmp(&rendered.bitmap)?);
        }
        self.backgrounds.get(&map)
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
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                let direction = match event.physical_key {
                    PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::KeyW) => Some(Direction::Up),
                    PhysicalKey::Code(KeyCode::ArrowDown | KeyCode::KeyS) => Some(Direction::Down),
                    PhysicalKey::Code(KeyCode::ArrowLeft | KeyCode::KeyA) => Some(Direction::Left),
                    PhysicalKey::Code(KeyCode::ArrowRight | KeyCode::KeyD) => {
                        Some(Direction::Right)
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
        if interact {
            self.interact_down = true;
        }
    }

    fn advance(&mut self) {
        let direction = self.held;
        let interact = std::mem::take(&mut self.interact_down);
        let session = self.session();
        session.world.step(direction);
        if interact {
            // Talking first: a resident standing in a doorway should be spoken
            // to rather than walked past.
            session.said = match session.world.talk() {
                Some(Conversation::Speaks { pages, .. }) => {
                    Some(format!("{} page(s)", pages.len()))
                }
                Some(Conversation::Unsupported { source }) => {
                    Some(format!("choice prompt at ${source:04x}"))
                }
                Some(Conversation::Unaccounted { service }) => {
                    Some(format!("script stops at COP ${service:02x}"))
                }
                Some(Conversation::Silent) => Some("...".to_string()),
                None => {
                    session.world.interact();
                    None
                }
            };
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
        frame.fill(0);
        let session = self.state.as_mut().expect("just created");
        let position = session.world.position();
        let residents: Vec<_> = session
            .world
            .residents()
            .iter()
            .map(|resident| resident.position)
            .collect();
        let camera = match session.background(&cartridge) {
            Some(background) => {
                let camera = frame::camera(position, (background.width, background.height));
                frame::draw_background(frame, background, camera);
                camera
            }
            None => (0, 0),
        };
        draw_actors(frame, camera, position, &residents);

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
