//! The Crysta slice in a browser: the native app's session, renderer and
//! music, driven a frame at a time by the page (`www/`). The ROM comes from
//! the player's own file and stays in memory; nothing is fetched or stored.

use crysta_app::frame::{Canvas, CLASSIC_WIDTH, VIEW_HEIGHT};
use crysta_app::music::{Player, Synth};
use crysta_app::music_data::{extract_driver, extract_track};
use crysta_app::session::Session;
use room_core::Direction;

/// Pad bits the page sends each frame.
pub mod buttons {
    /// Up.
    pub const UP: u32 = 1;
    /// Down.
    pub const DOWN: u32 = 2;
    /// Left.
    pub const LEFT: u32 = 4;
    /// Right.
    pub const RIGHT: u32 = 8;
    /// A: talk, confirm, lift.
    pub const CONFIRM: u32 = 16;
    /// B: cancel a choice.
    pub const CANCEL: u32 = 32;
}

/// One game from one ROM.
pub struct Game {
    cartridge: rom::Rom,
    session: Session,
    canvas: Canvas,
    /// Last frame's buttons, so that A and B act on the press.
    held: u32,
    player: Option<Player>,
}

impl Game {
    /// Authenticates the ROM and starts a new game, as the native app does.
    ///
    /// # Errors
    /// A ROM that is not the Japanese reference.
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let cartridge = rom::Rom::load(bytes).map_err(|error| error.to_string())?;
        if cartridge.revision() != rom::Revision::Japan {
            return Err("the slice is qualified only for the Japanese reference".into());
        }
        // The world borrows the image for the page's lifetime.
        let image: &'static [u8] = Box::leak(cartridge.image().to_vec().into_boxed_slice());
        Ok(Self {
            session: Session::new(image),
            cartridge,
            canvas: Canvas::new(CLASSIC_WIDTH),
            held: 0,
            player: None,
        })
    }

    /// One frame with the buttons held now; the music hears the world's
    /// requests.
    pub fn frame(&mut self, held: u32) {
        let pressed = held & !self.held;
        self.held = held;
        let direction = [
            (buttons::UP, Direction::Up),
            (buttons::DOWN, Direction::Down),
            (buttons::LEFT, Direction::Left),
            (buttons::RIGHT, Direction::Right),
        ]
        .into_iter()
        .find(|&(bit, _)| held & bit != 0)
        .map(|(_, direction)| direction);
        self.session.advance(
            direction,
            pressed & buttons::CONFIRM != 0,
            pressed & buttons::CANCEL != 0,
        );
        let cues = self.session.world.take_cues();
        if let Some(player) = &mut self.player {
            for cue in cues {
                if let Err(error) = player.cue(cue) {
                    self.session.fault.get_or_insert(error);
                }
            }
        }
    }

    /// The view as RGBA bytes, `width() x height()`.
    pub fn draw(&mut self) -> Vec<u8> {
        self.session.compose(&self.cartridge, &mut self.canvas);
        self.canvas
            .pixels
            .iter()
            .flat_map(|pixel| {
                let [_, r, g, b] = pixel.to_be_bytes();
                [r, g, b, 0xFF]
            })
            .collect()
    }

    /// The view's width in pixels.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.canvas.width
    }

    /// The view's height in pixels.
    #[must_use]
    pub const fn height() -> usize {
        VIEW_HEIGHT
    }

    /// Boots the sound driver; the page asks on the click that starts the
    /// game, as browsers want, before the first frame asks for its track.
    ///
    /// # Errors
    /// A driver that does not boot.
    pub fn start_audio(&mut self) -> Result<(), String> {
        let driver = extract_driver(&self.cartridge).map_err(|error| error.to_string())?;
        let cartridge = self.cartridge.clone();
        let player = Player::new(&driver, move |track| Ok(extract_track(&cartridge, track)?))
            .map_err(|error| error.to_string())?;
        self.player = Some(player);
        Ok(())
    }

    /// Renders interleaved stereo at 32 kHz into `out`, as `f32` in -1..1.
    pub fn audio(&mut self, out: &mut [f32]) {
        let Some(player) = &mut self.player else {
            out.fill(0.0);
            return;
        };
        let mut samples = vec![0i16; out.len()];
        if player.render(&mut samples).is_err() {
            self.player = None;
            out.fill(0.0);
            return;
        }
        for (out, sample) in out.iter_mut().zip(samples) {
            *out = f32::from(sample) / 32768.0;
        }
    }

    /// The fatal error that stopped the world, if any.
    #[must_use]
    pub fn fault(&self) -> Option<String> {
        self.session.fault.clone()
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    /// The page's handle on a game.
    #[wasm_bindgen]
    pub struct WebGame(super::Game);

    #[wasm_bindgen]
    impl WebGame {
        /// Starts a game from the ROM's bytes.
        ///
        /// # Errors
        /// A ROM that is not the Japanese reference.
        #[wasm_bindgen(constructor)]
        pub fn new(bytes: &[u8]) -> Result<WebGame, JsError> {
            super::Game::new(bytes)
                .map(WebGame)
                .map_err(|error| JsError::new(&error))
        }

        /// One frame with the buttons held now.
        pub fn frame(&mut self, held: u32) {
            self.0.frame(held);
        }

        /// The view as RGBA bytes.
        pub fn draw(&mut self) -> Vec<u8> {
            self.0.draw()
        }

        /// The view's width.
        pub fn width(&self) -> usize {
            self.0.width()
        }

        /// The view's height.
        pub fn height() -> usize {
            super::Game::height()
        }

        /// Boots the sound driver.
        ///
        /// # Errors
        /// A driver that does not boot.
        pub fn start_audio(&mut self) -> Result<(), JsError> {
            self.0.start_audio().map_err(|error| JsError::new(&error))
        }

        /// Fills interleaved stereo at 32 kHz.
        pub fn audio(&mut self, out: &mut [f32]) {
            self.0.audio(out);
        }

        /// The fatal error that stopped the world, if any.
        pub fn fault(&self) -> Option<String> {
            self.0.fault()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_rom_is_refused() {
        assert!(Game::new(&[0; 1024]).is_err());
    }

    #[test]
    fn the_intro_draws_and_plays_from_the_rom() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../local/Tenchi Souzou (Japan).sfc"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let mut game = Game::new(&bytes).unwrap();
        game.start_audio().unwrap();
        for _ in 0..120 {
            game.frame(0);
        }
        let pixels = game.draw();
        assert_eq!(pixels.len(), game.width() * Game::height() * 4);
        assert!(pixels.chunks(4).any(|pixel| pixel[..3] != [0, 0, 0]));
        // Rendered into memory, never played.
        let mut sound = vec![0.0; 32_000];
        game.audio(&mut sound);
        assert!(sound.iter().any(|&sample| sample != 0.0));
        assert!(game.fault().is_none());
    }

    #[test]
    fn a_and_b_act_on_the_press_not_while_held() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../local/Tenchi Souzou (Japan).sfc"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let mut game = Game::new(&bytes).unwrap();
        // Elle speaks some 120 frames in; held A turns only the first page.
        for _ in 0..130 {
            game.frame(0);
        }
        assert!(game.session.world.dialogue().is_some());
        let page = |game: &Game| {
            game.session
                .world
                .dialogue()
                .map(|view| view.page.indexed().to_vec())
        };
        let first = page(&game);
        for _ in 0..30 {
            game.frame(buttons::CONFIRM);
        }
        let second = page(&game);
        assert_ne!(first, second, "the press turns a page");
        for _ in 0..30 {
            game.frame(buttons::CONFIRM);
        }
        assert_eq!(page(&game), second, "holding does not");
    }
}
