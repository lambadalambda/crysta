//! The running world and what it needs to draw: the host-independent half
//! of the app, shared by the native window and the web page.

use crate::{background, frame};
use assets::sprites::{PandoraCarryMotion, PandoraSprites};
use assets::text::window::WindowArt;
use crysta_runtime::art::{
    residents_art, Animation, ArkAtlas, Body, CarryArt, Placeholder, Raster,
};
use crysta_runtime::scene::Presses;
use crysta_runtime::world::{Step, World};
use frame::Canvas;
use room_core::Direction;
use std::collections::HashMap;

/// Map the player starts in, and where: fresh startup places them at
/// `(304,112)` in bedroom `$000F` (`docs/new-game-bootstrap.md`), where Elle
/// wakes them. The prologue title card before it is not shown.
pub const START: (u16, u16, u16) = (0x000F, 304, 112);

/// The buttons pressed this frame, besides the pad's directions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    /// A: talk, confirm, lift.
    pub confirm: bool,
    /// B: cancel.
    pub cancel: bool,
    /// L: the shop's item description.
    pub describe: bool,
}

/// What a resident whose art was refused is drawn as: a block, so that
/// someone is visibly there and visibly not right.
const PLACEHOLDER: u32 = 0x00C0_50C0;

/// Resident art for one roster: the map, the records present, the flags in
/// force, and their rasters. The flags are part of the key because a
/// resident's pose comes from their walked script, which branches on them.
pub type RosterArt = (u16, Vec<usize>, Vec<u8>, Vec<Result<Body, Placeholder>>);

/// A raster and the world point it stands on.
pub type Placed = ((u16, u16), Raster);

/// The running world plus what it needs to draw.
pub struct Session {
    /// The simulation.
    pub world: World<'static>,
    /// The ROM image it reads.
    pub image: &'static [u8],
    /// Ark's ordinary frames.
    pub atlas: ArkAtlas,
    /// Backgrounds by map, composed once.
    pub backgrounds: HashMap<u16, background::CachedBackground>,
    /// Frames into the current map's visit, for its animations.
    pub background_clock: background::VisitClock,
    /// Resident art for the roster it was computed for, keyed by map and by
    /// which records were present, since the flags can change the roster.
    pub art: Option<RosterArt>,
    /// Rasterized sequences by record, selector and mirror; `None` when the
    /// packet has no such sequence.
    pub sprites: HashMap<(usize, u8, bool), Option<Animation>>,
    /// Ark's carry poses and the pots; `None` when the decoder refused them,
    /// and then Ark carries in his ordinary frames and no pot is drawn.
    pub carry_art: Option<CarryArt>,
    /// Rasterized carry lists by art, selector and mirror.
    pub carry_frames: HashMap<(u32, u8, bool), Option<Animation>>,
    /// The shop display's art.
    pub shop_art: crate::shop::ShopArtCache,
    /// The area titles.
    pub titles: crate::title::Titles,
    /// The text window's art.
    pub window_art: WindowArt,
    /// The direction held last frame, so a new one reads as a press.
    pub last_direction: Option<Direction>,
    /// Frames simulated so far, which drives resident animation.
    pub tick: u64,
    /// The last frame's walking step.
    pub last_step: Option<Step>,
    /// The last frame's confirm action, when it opened a doorway.
    pub last_interaction: Option<Step>,
    /// The last refusal reported, so that a held refusal reports once.
    pub last_refusal: Option<room_core::Unqualified>,
    /// Checked build failures are fatal to this world, not retryable input refusals.
    pub fault: Option<String>,
}

impl Session {
    /// A new game in the bedroom, before Elle wakes Ark.
    pub fn new(image: &'static [u8]) -> Self {
        Self {
            // A new game, before Elle wakes Ark.
            world: World::enter_with_events(
                image,
                START.0,
                START.1,
                START.2,
                crysta_runtime::world::fresh_game_flags(),
            )
            .expect("the opening house"),
            image,
            atlas: ArkAtlas::from_rom(image).expect("the player's frames"),
            backgrounds: HashMap::new(),
            background_clock: background::VisitClock::new(START.0),
            art: None,
            sprites: HashMap::new(),
            carry_art: CarryArt::from_rom(image)
                .inspect_err(|error| eprintln!("carry poses unavailable: {error}"))
                .ok(),
            carry_frames: HashMap::new(),
            shop_art: crate::shop::ShopArtCache::default(),
            titles: crate::title::Titles::default(),
            window_art: WindowArt::from_rom(image).expect("the text window's art"),
            last_direction: None,
            tick: 0,
            last_step: None,
            last_interaction: None,
            last_refusal: None,
            fault: None,
        }
    }

    /// One frame of simulation from the pad: the world's scripts decide
    /// whether it walks, pages through text or answers a choice.
    pub fn advance(&mut self, direction: Option<Direction>, confirm: bool, cancel: bool) {
        self.advance_with(
            direction,
            Buttons {
                confirm,
                cancel,
                describe: false,
            },
        );
    }

    /// As [`Self::advance`], with every button the world reads.
    pub fn advance_with(&mut self, direction: Option<Direction>, buttons: Buttons) {
        let newly = |wanted| direction == Some(wanted) && self.last_direction != Some(wanted);
        let presses = Presses {
            confirm: buttons.confirm,
            cancel: buttons.cancel,
            describe: buttons.describe,
            up: newly(Direction::Up),
            down: newly(Direction::Down),
            left: newly(Direction::Left),
            right: newly(Direction::Right),
        };
        self.last_direction = direction;
        self.advance_world(direction, presses);
        if self.fault.is_none() {
            self.background_clock.advance(self.world.map());
        }
    }

    /// One frame of the world with already-edged presses.
    pub fn advance_world(&mut self, direction: Option<Direction>, presses: Presses) {
        if self.fault.is_some() {
            return;
        }
        self.tick += 1;
        self.last_step = None;
        self.last_interaction = None;
        match self.world.update(direction, presses) {
            Ok((step, interaction)) => {
                self.last_step = Some(step);
                self.last_interaction = interaction;
                if let Step::Refused(reason) = step {
                    if self.last_refusal != Some(reason) {
                        eprintln!("movement refused at map {:#06x} {:?}: {reason:?}; input reset, choose another direction", self.world.map(), self.world.position());
                    }
                    self.last_refusal = Some(reason);
                } else if matches!(step, Step::Walked | Step::Entered { .. }) {
                    self.last_refusal = None;
                }
            }
            Err(error) => self.fail_world(&error),
        }
    }

    /// A diagnostic record of the last frame.
    pub fn trace_frame(
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
        } else if self.world.dialogue().is_some() {
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
            "background_tick":self.background_clock.tick(),
            "input":{"direction":direction,"interact":interact},
            "before":{"map":before.0,"x":before.1.0,"y":before.1.1},
            "after":{"map":self.world.map(),"x":x,"y":y},
            "outcome":outcome, "movement":describe(self.last_step),
            "interaction":describe(self.last_interaction), "error":self.fault,
            "dialogue_open":self.world.dialogue().is_some(), "scene":self.world.in_scene()})
    }

    /// Stops the world after a fatal error, reporting it once.
    pub fn fail_world(&mut self, error: &crysta_runtime::world::WorldError) {
        let message = format!(
            "map {:#06x} {:?}: {error}",
            self.world.map(),
            self.world.position()
        );
        eprintln!("world stopped: {message}; restart the app (Escape still exits)");
        self.fault = Some(message);
    }

    /// Renders and caches the current map's background and its priority mask.
    pub fn ensure_background(&mut self, cartridge: &rom::Rom) {
        let map = self.world.map();
        if let std::collections::hash_map::Entry::Vacant(slot) = self.backgrounds.entry(map) {
            let Ok(decoded) = background::load(cartridge, map, self.world.spawn_events()) else {
                return;
            };
            slot.insert(decoded);
        }
        let cached = self.backgrounds.get_mut(&map).expect("loaded background");
        cached.update(self.background_clock.tick());
        cached.apply_patches(cartridge.image(), map, self.world.patched_cells());
    }

    /// Decodes bodies for the current roster, recomputed when it changes.
    pub fn ensure_art(&mut self) {
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
            let spawned = assets::maps::scripts::EventFlags::Bitmap(self.world.spawn_events());
            let art = residents_art(self.image, map, self.world.residents(), spawned, events);
            self.art = Some((map, records, self.world.events().to_vec(), art));
        }
    }

    /// Bodies for the current roster, aligned with the world's residents.
    pub fn resident_art(&mut self) -> &[Result<Body, Placeholder>] {
        self.ensure_art();
        self.art
            .as_ref()
            .map_or(&[], |(_, _, _, art)| art.as_slice())
    }

    /// A carry list's frame `tick` frames in, decoded once per list; a
    /// `once` list holds its last frame rather than looping.
    pub fn carry_frame(
        &mut self,
        (art, selector, hflip): (u32, u8, bool),
        tick: u64,
        once: bool,
    ) -> Option<Raster> {
        let carry_art = &self.carry_art;
        self.carry_frames
            .entry((art, selector, hflip))
            .or_insert_with(|| carry_art.as_ref()?.animation(art, selector, hflip).ok())
            .as_ref()
            .map(|animation| {
                if once {
                    animation.frame_once(tick)
                } else {
                    animation.frame_at(tick)
                }
                .clone()
            })
    }

    /// Ark's carry pose while he lifts, holds or throws a pot, and the pot:
    /// in hand at Ark's origin (its frames carry the height), then along its
    /// flight.
    pub fn carry_sprites(&mut self) -> (Option<Raster>, Option<Placed>) {
        // A bought item held up: the lift's standing pose facing Down
        // (`$84:B4BF`, `COP 84 03`); the shop draws the item.
        let holding = self
            .world
            .shop()
            .and_then(crysta_runtime::shop::Shop::display)
            .is_some_and(|display| display.holding);
        if holding {
            let pose = PandoraSprites::carry_pose(PandoraCarryMotion::Standing, 0);
            let ark = pose.and_then(|pose| {
                self.carry_frame(
                    (pose.ark_art, pose.ark_selector, pose.ark_hflip),
                    self.tick,
                    false,
                )
            });
            return (ark, None);
        }
        // Dashing loops its list; the brake holds its one frame.
        if let Some((motion, facing, age)) = self.world.run_pose() {
            let ark = PandoraSprites::run_pose(motion, facing)
                .and_then(|pose| self.carry_frame(pose, u64::from(age), false));
            return (ark, None);
        }
        let carry = self.world.carry().and_then(|carry| {
            let pose = PandoraSprites::carry_pose(carry.motion, carry.facing)?;
            // The lift and the throw run once from their start; holding loops.
            let once = matches!(
                carry.motion,
                PandoraCarryMotion::Lifting | PandoraCarryMotion::Throwing
            );
            let tick = if once {
                u64::from(carry.tick)
            } else {
                self.tick
            };
            Some((pose, tick, once))
        });
        let ark = carry.and_then(|(pose, tick, once)| {
            self.carry_frame(
                (pose.ark_art, pose.ark_selector, pose.ark_hflip),
                tick,
                once,
            )
        });
        let pot = self.world.pot().and_then(|pot| match (pot.flight, carry) {
            (Some(at), _) => Some((
                at,
                self.carry_frame((pot.art, CarryArt::FLIGHT, false), self.tick, false)?,
            )),
            (None, Some((pose, tick, once))) => Some((
                self.world.position(),
                self.carry_frame((pot.art, pose.pot_selector, pose.pot_hflip), tick, once)?,
            )),
            (None, None) => None,
        });
        (ark, pot)
    }

    /// Composes the view: background, depth-sorted sprites, then dialogue.
    ///
    /// Depth is world Y, ties broken by spawn order with later records first
    /// and the player last, which is what every frozen tie rank encodes. The
    /// count includes residents that draw nothing, which keeps the relative
    /// order and only inflates the player's rank. Whatever lies outside the
    /// map's region is blanked before the dialogue goes on top.
    pub fn compose(&mut self, cartridge: &rom::Rom, frame: &mut Canvas) -> (i32, i32) {
        frame.pixels.fill(0);
        self.ensure_background(cartridge);
        self.ensure_art();
        let (carried, pot) = self.carry_sprites();
        let Session {
            world,
            backgrounds,
            art,
            atlas,
            sprites,
            ..
        } = self;
        let position = world.position();
        let player = carried
            .as_ref()
            .unwrap_or_else(|| atlas.frame(world.animation()));
        let residents = world.residents();
        let bodies: &[Result<Body, Placeholder>] =
            art.as_ref().map_or(&[], |(_, _, _, art)| art.as_slice());
        let Some(background) = backgrounds.get(&world.map()) else {
            return (0, 0);
        };
        let region = background.region;
        let camera = background.camera(position, frame.width);
        frame::draw_background(frame, &background.frame, camera);
        let screen = world.screen();
        frame::mosaic(frame, screen.mosaic);
        let clouds = background;
        let background = &background.frame;
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
            if resident.hidden {
                continue;
            }
            let placeholder = |frame: &mut Canvas| {
                let (x, y) = (
                    i32::from(resident.position.0) - camera.0,
                    i32::from(resident.position.1) - camera.1,
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
        if let Some((at, raster)) = pot {
            frame::draw_sprite(frame, background, camera, &raster, at);
        }
        clouds.add_second_layer(frame, camera, self.background_clock.tick());
        frame::extend_edges(frame, camera, region.bounds);
        self.draw_labels(frame, camera);
        let world = &self.world;
        frame::tint(frame, screen.tint);
        frame::dim(frame, screen.brightness);
        draw_dialogue(
            frame,
            world,
            (position, camera),
            (self.image, &self.window_art),
            self.tick,
        );
        camera
    }
}

impl Session {
    /// The sprites over the scene that nothing adds onto: the shop's
    /// display and the area title (priority 3).
    fn draw_labels(&mut self, frame: &mut Canvas, camera: (i32, i32)) {
        let world = &self.world;
        if let Some(display) = world.shop().and_then(crysta_runtime::shop::Shop::display) {
            let on_screen = |(x, y): (u16, u16)| (i32::from(x) - camera.0, i32::from(y) - camera.1);
            self.shop_art.draw(
                frame,
                self.image,
                &display,
                (on_screen(display.position), on_screen(world.position())),
                world.money(),
            );
        }
        self.titles.draw(
            frame,
            self.image,
            (world.map(), world.spawn_events()),
            world.since_arrival(),
        );
    }
}

/// The dialogue window over the view, placed by the player's screen row.
fn draw_dialogue(
    frame: &mut Canvas,
    world: &World<'_>,
    (position, camera): ((u16, u16), (i32, i32)),
    (image, art): (&[u8], &WindowArt),
    tick: u64,
) {
    let Some(view) = world.dialogue() else {
        return;
    };
    let page = view.page;
    let player_screen_y = usize::try_from(i32::from(position.1) - camera.1).unwrap_or(0);
    let origin = crate::window::content_origin(page.placement(), player_screen_y, frame.width);
    // As much of the page as has typed out.
    let typed = page.typed(image, view.glyphs);
    crate::window::draw_window(frame, art, page, (&typed, view.glyphs), origin, tick);
    if let Some(cursor) = view.cursor {
        crate::window::draw_cursor(frame, art, origin, cursor);
    }
}
