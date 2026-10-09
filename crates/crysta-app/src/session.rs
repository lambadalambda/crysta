//! The running world and what it needs to draw: the host-independent half
//! of the app, shared by the native window and the web page.

use crate::{background, frame};
use assets::sprites::{PandoraCarryMotion, PandoraSprites};
use assets::text::window::WindowArt;
use crysta_runtime::art::{
    residents_art, Animation, ArkAtlas, Body, CarryArt, Placeholder, Raster,
};
use crysta_runtime::colours::ObjColours;
use crysta_runtime::records::View;
use crysta_runtime::restart::{Outcome, Restart};
use crysta_runtime::scene::Presses;
use crysta_runtime::sram::Sram;
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
    /// Rasterized sequences by record, selector, mirror and palette field;
    /// `None` when the packet has no such sequence.
    pub sprites: HashMap<SpriteKey, Option<Animation>>,
    /// The OBJ colours `sprites` were rasterized in; a `COP 5A` load
    /// clears them.
    pub sprite_colours: ObjColours,
    /// Ark's carry poses and the pots; `None` when the decoder refused them,
    /// and then Ark carries in his ordinary frames and no pot is drawn.
    pub carry_art: Option<CarryArt>,
    /// Rasterized carry lists by art, selector and mirror.
    pub carry_frames: HashMap<(u32, u8, bool), Option<Animation>>,
    /// The shop display's art.
    pub shop_art: crate::shop::ShopArtCache,
    /// The desk's Records screen art, decoded on first use.
    pub records_art: crate::records::RecordsArtCache,
    /// The Restart file select while it runs, before the world plays.
    pub restart: Option<Restart>,
    /// Its art, decoded on first use.
    pub restart_art: crate::restart::RestartArtCache,
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

/// The world a new game starts with.
fn new_game(image: &'static [u8]) -> World<'static> {
    World::enter_with_events(
        image,
        START.0,
        START.1,
        START.2,
        crysta_runtime::world::fresh_game_flags(),
    )
    .expect("the opening house")
}

impl Session {
    /// A new game in the bedroom, before Elle wakes Ark.
    pub fn new(image: &'static [u8]) -> Self {
        Self {
            world: new_game(image),
            image,
            atlas: ArkAtlas::from_rom(image).expect("the player's frames"),
            backgrounds: HashMap::new(),
            background_clock: background::VisitClock::new(START.0),
            art: None,
            sprites: HashMap::new(),
            sprite_colours: ObjColours::default(),
            carry_art: CarryArt::from_rom(image)
                .inspect_err(|error| eprintln!("carry poses unavailable: {error}"))
                .ok(),
            carry_frames: HashMap::new(),
            shop_art: crate::shop::ShopArtCache::default(),
            records_art: crate::records::RecordsArtCache::default(),
            restart: None,
            restart_art: crate::restart::RestartArtCache::default(),
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
        if let Some(restart) = &mut self.restart {
            if let Some(outcome) = restart.step(direction, presses.confirm, presses.cancel) {
                self.finish_restart(outcome);
            }
            return;
        }
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

    /// Opens the Restart file select with the cartridge's SRAM; the world
    /// waits, a new game, until it ends.
    pub fn open_restart(&mut self, sram: Sram) {
        let europe = assets::layout::per_revision(self.image, false, true);
        self.restart = Some(Restart::open(europe, sram));
        // The waiting world's music is not the screen's.
        self.world.take_cues();
    }

    /// The screen ended: a valid slot resumes its world (the native load;
    /// one before the wake-up keeps the new game), else the new game plays.
    fn finish_restart(&mut self, outcome: Outcome) {
        let Some(mut restart) = self.restart.take() else {
            return;
        };
        let written = restart.take_sram_write();
        let sram = restart.sram().clone();
        let slot = match outcome {
            Outcome::Load(slot) => {
                let resumed = sram
                    .slot(usize::from(slot))
                    .map(|data| World::resume(self.image, &data));
                // A slot before the wake-up, or one that does not load,
                // plays a new game (`$87:8160`, `$87:820E`).
                self.world = match resumed {
                    Some(Ok(Some(world))) => world,
                    Some(Err(error)) => {
                        self.fail_world(&error);
                        new_game(self.image)
                    }
                    Some(Ok(None)) | None => new_game(self.image),
                };
                slot
            }
            Outcome::NewGame(slot) => {
                // Built afresh, so it asks for its music now. `$0496` stays
                // as the power-on's cleared WRAM left it: 0.
                self.world = new_game(self.image);
                slot.unwrap_or(0)
            }
        };
        self.world.set_sram(sram, slot);
        if written {
            self.world.mark_sram_written();
        }
    }

    /// The SRAM as the screen or the world holds it.
    #[must_use]
    pub fn sram(&self) -> &Sram {
        self.restart
            .as_ref()
            .map_or_else(|| self.world.sram(), Restart::sram)
    }

    /// Puts in the SRAM a host kept: into the Restart screen, opened again,
    /// while it runs, else into the world.
    pub fn set_sram(&mut self, sram: Sram) {
        if self.restart.is_some() {
            self.open_restart(sram);
        } else {
            // `$0496` is the world's: an import loads no slot.
            let last = self.world.last_slot();
            self.world.set_sram(sram, last);
        }
    }

    /// Whether the SRAM changed since the host last kept it.
    pub fn take_sram_write(&mut self) -> bool {
        let screen = self.restart.as_mut().is_some_and(Restart::take_sram_write);
        self.world.take_sram_write() || screen
    }

    /// Music and sound requests since the last call.
    pub fn take_cues(&mut self) -> Vec<crysta_runtime::audio::Cue> {
        let mut cues = self
            .restart
            .as_mut()
            .map(Restart::take_cues)
            .unwrap_or_default();
        cues.extend(self.world.take_cues());
        cues
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
        cached.apply_patches(
            cartridge.image(),
            map,
            self.world.patched_cells(),
            self.world.second_patched_cells(),
        );
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

    /// Drops the sprites rasterized in other OBJ colours than the world's.
    fn ensure_sprite_colours(&mut self) {
        if self.world.obj_colours() != &self.sprite_colours {
            self.sprite_colours = self.world.obj_colours().clone();
            self.sprites.clear();
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
        // A state's own pose of Ark: a fall, a drop, the rope, the burn, a
        // level up, a lift, his script's list (`World::ark_pose`).
        if let Some(pose) = self.world.ark_pose() {
            let art = (pose.art(), pose.list, pose.hflip);
            let ark = self.carry_frame(art, u64::from(pose.age), pose.once);
            return (ark, self.flying_pot());
        }
        // The spear's thrust plays its list once (`docs/combat-graphics.md`).
        if let Some((list, age, hflip)) = self.world.attack_pose() {
            let thrust = (crysta_runtime::art::THRUST, list, hflip);
            return (self.carry_frame(thrust, u64::from(age), true), None);
        }
        // Stairs play their list of Ark's resource 1 once, unmirrored.
        if let Some((list, age)) = self.world.stairs_pose() {
            return (
                self.carry_frame((0x80_a255, list, false), u64::from(age), true),
                None,
            );
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
        let pot = self.flying_pot().or_else(|| {
            let pot = self.world.pot()?;
            let (pose, tick, once) = carry?;
            Some((
                self.world.position(),
                self.carry_frame((pot.art, pose.pot_selector, pose.pot_hflip), tick, once)?,
            ))
        });
        (ark, pot)
    }

    /// A thrown or dropped pot in the air, drawn at its height above the
    /// ground point (`$0999`).
    fn flying_pot(&mut self) -> Option<Placed> {
        let pot = self.world.pot()?;
        let (x, y, height) = pot.flight?;
        Some((
            (x, y.wrapping_add_signed(height)),
            self.carry_frame((pot.art, CarryArt::FLIGHT, false), self.tick, false)?,
        ))
    }

    /// Composes the view: background, depth-sorted sprites, then dialogue.
    ///
    /// Depth is world Y as `+$06`'s depth bits move it
    /// (`Resident::draw_depth`), ties broken by spawn order with later
    /// records first and the player last, which is what every frozen tie
    /// rank encodes. The
    /// count includes residents that draw nothing, which keeps the relative
    /// order and only inflates the player's rank. Whatever lies outside the
    /// map's region is blanked before the dialogue goes on top.
    pub fn compose(&mut self, cartridge: &rom::Rom, frame: &mut Canvas) -> (i32, i32) {
        if self.compose_records(frame) {
            return (0, 0);
        }
        self.ensure_background(cartridge);
        self.ensure_art();
        self.ensure_sprite_colours();
        let (carried, pot) = self.carry_sprites();
        let Session {
            world,
            backgrounds,
            art,
            atlas,
            sprites,
            image,
            ..
        } = self;
        let image: &[u8] = image;
        let position = world.position();
        let player = carried
            .as_ref()
            .unwrap_or_else(|| atlas.frame(world.animation()));
        let (residents, obj) = (world.residents(), world.obj_colours());
        let bodies: &[Result<Body, Placeholder>] =
            art.as_ref().map_or(&[], |(_, _, _, art)| art.as_slice());
        let Some(background) = backgrounds.get(&world.map()) else {
            return (0, 0);
        };
        let camera = background.camera(world.camera_focus(), frame.width);
        background.draw(frame, camera, position);
        let screen = world.screen();
        frame::mosaic(frame, screen.mosaic);
        let backdrop = frame::Backdrop::keep(frame, world.display().darkening());
        let clouds = background;
        let background = &background.frame;
        let count = residents.len();
        let mut order: Vec<(i32, usize, usize)> = residents
            .iter()
            .enumerate()
            .map(|(index, resident)| (resident.draw_depth(), count - 1 - index, index))
            .collect();
        order.push((i32::from(position.1), count, usize::MAX));
        order.sort_unstable();
        for (_, _, index) in order {
            if index == usize::MAX {
                // A push after a hit blinks him every second frame; the
                // fall's end hides him (`World::ark_blinks`). A landing
                // draws him above his place, over the high tiles.
                if !world.ark_blinks() {
                    draw_ark(frame, background, camera, player, world);
                }
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
                Some(Ok(body)) => match resident_animation(sprites, image, body, resident, obj) {
                    Some(animation) => {
                        let raster = animation.frame_at(u64::from(resident.pose_age));
                        let draw = if resident.priority >= 3 {
                            frame::draw_sprite_over
                        } else {
                            frame::draw_sprite
                        };
                        draw(frame, background, camera, raster, resident.position);
                    }
                    None => placeholder(frame),
                },
                Some(Err(Placeholder::Invisible)) | None => {}
                Some(Err(Placeholder::Refused(_) | Placeholder::PredecessorRefused)) => {
                    placeholder(frame);
                }
            }
        }
        if let Some((at, raster)) = pot {
            frame::draw_sprite(frame, background, camera, &raster, at);
        }
        clouds.cover(frame, camera, &self.world.masks());
        // `TM` may take BG1, the light rays, off (`$88:9CD8`).
        if self.world.display().shows_bg1() {
            clouds.add_second_layer(frame, camera, self.background_clock.tick());
        }
        clouds.subtract_second_layer(frame, camera, self.world.darkness().as_ref());
        clouds.extend_edges(frame, camera);
        self.draw_labels(frame, camera);
        let world = &self.world;
        frame::tint_and_darken(frame, camera, screen.tint, backdrop);
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
    /// Clears the frame for the room, or draws the desk's Records screen,
    /// which replaces the room while it shows; returns whether it did.
    fn compose_records(&self, frame: &mut Canvas) -> bool {
        frame.pixels.fill(0);
        if let Some(restart) = &self.restart {
            if let Some(art) = self.restart_art.get(self.image) {
                crate::restart::draw(frame, self.image, art, restart.view(), restart.sram());
            }
            return true;
        }
        match self.world.records() {
            Some(View::Screen(page)) => {
                if let Some(art) = self.records_art.get(self.image) {
                    let current = self.world.save_slot();
                    let games = crate::records::Games {
                        sram: self.world.sram(),
                        current: &current,
                    };
                    crate::records::draw(frame, self.image, art, page, &games);
                }
                true
            }
            Some(View::Blank) => true,
            Some(View::Room(_)) | None => false,
        }
    }

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
        // The towers' HUD (`docs/combat.md` §7), under the damage digits;
        // not in the light room (`docs/light-room.md`).
        if crysta_runtime::TOWER_MAPS.contains(&world.map())
            && world.map() != crysta_runtime::LIGHT_ROOM
        {
            if let Some(art) = self.shop_art.art(self.image) {
                crate::hud::draw_digits(frame, art, camera, world.digits());
                let stats = world.stats();
                let shown = crate::hud::Shown {
                    level: stats.level,
                    life: (stats.life, stats.max_life),
                    gems: world.money(),
                };
                crate::hud::draw(frame, art, shown);
            }
        }
        // An item held over Ark's head (`COP 60`).
        if let Some((item, (dx, dy))) = world.held_item() {
            let (x, y) = world.position();
            let at = (
                i32::from(x) - camera.0 + i32::from(dx),
                i32::from(y) - camera.1 + i32::from(dy),
            );
            self.shop_art
                .draw_icon(frame, self.image, (item, false), at);
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
    let origin = crate::window::content_origin(
        image,
        page.placement(),
        (player_screen_y, world.tower_floor()),
        frame.width,
    );
    // As much of the page as has typed out.
    let typed = page.typed(image, view.glyphs);
    crate::window::draw_window(frame, art, page, (&typed, view.glyphs), origin, tick);
    if let Some(cursor) = view.cursor {
        crate::window::draw_cursor(frame, art, origin, cursor);
    }
}

/// A cached resident sequence: a body's by record, selector, mirror and
/// palette field, or a list of the helper art drawn over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpriteKey {
    /// A body's sequence: its record and descriptor (spawns share record
    /// 0), selector, mirror, vertical flip and palette.
    Body(usize, Option<usize>, u8, bool, bool, u8),
    /// The helper art's list: an explosion, a dropped gem, a wisp; mirror
    /// and vertical flip.
    Overlay(u32, u8, bool, bool),
}

/// Ark at his place, or above it over the high tiles while a landing drops
/// him (`World::ark_lift`, `World::ark_over`).
fn draw_ark(
    canvas: &mut Canvas,
    background: &frame::Background,
    camera: (i32, i32),
    raster: &Raster,
    world: &World<'_>,
) {
    let draw = if world.ark_over() {
        frame::draw_sprite_over
    } else {
        frame::draw_sprite
    };
    let mut lifted = raster.clone();
    lifted.offset.1 = lifted.offset.1.saturating_add(world.ark_lift());
    draw(canvas, background, camera, &lifted, world.position());
}

/// A resident's sequence as its body and palette field show it, or its
/// overlay, cached. A sequence the packet lacks falls back to the setup one
/// rather than a block.
fn resident_animation<'s>(
    sprites: &'s mut HashMap<SpriteKey, Option<Animation>>,
    image: &[u8],
    body: &Body,
    resident: &crysta_runtime::residents::Resident,
    colours: &ObjColours,
) -> Option<&'s Animation> {
    if let Some((base, selector)) = resident.overlay {
        let (hflip, vflip) = (resident.hflip, resident.vflip);
        return sprites
            .entry(SpriteKey::Overlay(base, selector, hflip, vflip))
            .or_insert_with(|| {
                let shown = crysta_runtime::art::overlay_animation(image, base, selector, hflip);
                shown
                    .map(|shown| {
                        if vflip {
                            shown.flipped_vertically()
                        } else {
                            shown
                        }
                    })
                    .ok()
            })
            .as_ref();
    }
    let (selector, hflip, palette) = (resident.selector, resident.hflip, resident.palette);
    sprites
        .entry(SpriteKey::Body(
            resident.record,
            resident.descriptor,
            selector,
            hflip,
            resident.vflip,
            palette,
        ))
        .or_insert_with(|| {
            let shown = body
                .shown(image, (selector, hflip), palette, colours)
                .or_else(|_| body.shown(image, (body.initial(), hflip), palette, colours))
                .ok()?;
            Some(if resident.vflip {
                shown.flipped_vertically()
            } else {
                shown
            })
        })
        .as_ref()
}
