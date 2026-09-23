//! Contact callbacks, the player's recoil from them, and script transfers.
//!
//! The pair pass (`$85:D30C`) matches the player's body against actors whose
//! `+$04` bit `$0200` arms a contact callback (`$7F:1010`), queues the
//! callback and starts the player's recoil (`$85:D735`); the scheduler
//! (`$80:CAD5`) installs the callback the next frame. Measured on the box in
//! `$21` (`docs/pandora-navigation.md`): contact at (136,370), the callback
//! and the first recoil pixel a frame later, rest at (136,359) 27 frames
//! after contact.

use super::{facing_delta, occupied_by_others, surroundings, Scene, Step, World, WorldError};
use room_core::{Direction, WalkingState};

/// Recoil frames that move one pixel, then frames at rest, then one pixel.
const PUSH: u16 = 10;
const REST: u16 = 16;

/// The player pushed back from a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Recoil {
    direction: Direction,
    /// Frames since the contact.
    frame: u16,
}

impl Recoil {
    /// Whether frame `frame` after the contact moves the player a pixel.
    const fn moves(frame: u16) -> bool {
        frame <= PUSH || frame == PUSH + REST + 1
    }

    /// Whether the recoil has ended after frame `frame`.
    const fn over(frame: u16) -> bool {
        frame > PUSH + REST
    }
}

/// Which way a contact pushes the player: away from the actor along the
/// longer axis, horizontally on a tie (`$85:F8D1`).
pub(super) fn away(player: (u16, u16), actor: (u16, u16)) -> Direction {
    let dx = i32::from(player.0) - i32::from(actor.0);
    let dy = i32::from(player.1) - i32::from(actor.1);
    match (dy.abs() > dx.abs(), dx < 0, dy < 0) {
        (true, _, true) => Direction::Up,
        (true, _, false) => Direction::Down,
        (false, true, _) => Direction::Left,
        (false, false, _) => Direction::Right,
    }
}

/// Whether the player's walking body touches an actor's: `(x-5..x+5,
/// y-16..y-2)` against a 16-pixel body `(x-8..x+8, y-16..y)`, edges
/// included (`$85:F835`). The box's and the walking player's frames.
pub(super) fn touches(player: (u16, u16), actor: (u16, u16)) -> bool {
    let (px, py) = (i32::from(player.0), i32::from(player.1));
    let (ax, ay) = (i32::from(actor.0), i32::from(actor.1));
    px - 5 <= ax + 8 && ax - 8 <= px + 5 && py - 16 <= ay && ay - 16 <= py - 2
}

impl World<'_> {
    /// Queues the contact callback of an actor the player touches, and starts
    /// the recoil. Only one contact at a time.
    pub(super) fn touch(&mut self) {
        if self.touched.is_some() || self.recoil.is_some() || self.arrival.is_some() {
            return;
        }
        let player = self.position();
        let Some(index) = self
            .actors
            .iter()
            .position(|actor| actor.contact().is_some() && touches(player, actor.position))
        else {
            return;
        };
        self.touched = Some(index);
        self.recoil = Some(Recoil {
            direction: away(player, self.actors[index].position),
            frame: 0,
        });
        self.globals.player_action = true;
    }

    /// A frame the contact owns: the queued callback runs, and the recoil
    /// moves the player instead of the pad. Returns the step when it did.
    pub(super) fn contact_frame(&mut self) -> Result<Option<Step>, WorldError> {
        if let Some(index) = self.touched.take() {
            let player = self.position();
            let occupied = occupied_by_others(&self.actors, &self.residents, index, player);
            let mut around = surroundings(
                self.image,
                &mut self.globals,
                &self.base,
                &occupied,
                player,
                self.facing,
            );
            if let Some(actor) = self.actors.get_mut(index) {
                self.scene = actor
                    .run_contact(&mut around)
                    .map(|(pc, wait)| Scene::Callback {
                        actor: index,
                        pc,
                        wait,
                    });
            }
        }
        let Some(mut recoil) = self.recoil else {
            return Ok(None);
        };
        recoil.frame += 1;
        let before = self.position();
        if Recoil::moves(recoil.frame) {
            // Collision and exits are not consulted: north of the box is
            // free, and other recoils are not measured.
            let (dx, dy) = facing_delta(recoil.direction);
            let (x, y) = before;
            self.walking = WalkingState::new(x.wrapping_add_signed(dx), y.wrapping_add_signed(dy));
        }
        self.recoil = (!Recoil::over(recoil.frame)).then_some(recoil);
        self.globals.player_action = self.recoil.is_some();
        if self.scene.is_none() {
            self.run_actors()?;
        }
        Ok(Some(if self.position() == before {
            Step::Stayed
        } else {
            Step::Walked
        }))
    }

    /// Follows a transfer a script queued (`COP 14`): the map loads at the
    /// queued position, and the pad mask the script set stays.
    pub(super) fn follow_transfer(&mut self) -> Result<Option<Step>, WorldError> {
        let Some((map, x, y)) = self.globals.transfer.take() else {
            return Ok(None);
        };
        let mut entered = self.enter_destination(map, x, y)?;
        entered.globals.input_mask = self.globals.input_mask;
        entered.face(self.facing);
        let from = self.map;
        *self = entered;
        Ok(Some(Step::Entered { from, to: map }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recoil_pushes_ten_pixels_rests_sixteen_frames_then_one_more() {
        let moved: Vec<u16> = (1..=30).filter(|&frame| Recoil::moves(frame)).collect();
        let mut expected: Vec<u16> = (1..=10).collect();
        expected.push(27);
        assert_eq!(moved, expected);
        assert!(!Recoil::over(26) && Recoil::over(27));
    }

    #[test]
    fn a_contact_pushes_away_along_the_longer_axis_and_sideways_on_a_tie() {
        assert_eq!(away((136, 370), (136, 384)), Direction::Up);
        assert_eq!(away((136, 400), (136, 384)), Direction::Down);
        assert_eq!(away((120, 380), (136, 384)), Direction::Left);
        assert_eq!(away((146, 374), (136, 384)), Direction::Right, "a tie");
    }

    #[test]
    fn the_box_is_touched_from_x_123_to_149_and_y_370_to_400() {
        let box_origin = (136, 384);
        for (player, touching) in [
            ((136, 370), true),
            ((136, 369), false),
            ((123, 380), true),
            ((122, 380), false),
            ((149, 400), true),
            ((150, 380), false),
            ((136, 401), false),
        ] {
            assert_eq!(touches(player, box_origin), touching, "{player:?}");
        }
    }
}
