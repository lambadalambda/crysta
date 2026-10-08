//! Contact callbacks and the player's recoil from them.
//!
//! The pair pass (`$85:D30C`) matches the player's body against actors whose
//! `+$04` bit `$0200` arms a contact callback (`$7F:1010`), queues the
//! callback and starts the player's recoil (`$85:D735`): a hit's push
//! without the hit (`$84:8000`, [`super::hurt::Hurt`]) and its 43 frames
//! out of reach; the scheduler (`$80:CAD5`) installs the callback the next
//! frame. Measured on the box in `$21` (`docs/pandora-navigation.md`):
//! contact at (136,370), the callback and the first recoil pixel a frame
//! later, rest at (136,359) 27 frames after contact.

use super::hurt::{Hurt, ARK_IMMUNE};
use super::{occupied_by_others, surroundings, Scene, World};
use room_core::Direction;

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
        // The pair pass takes Ark only out of a hit's reach (`7F:1020`).
        if self.touched.is_some()
            || self.hurt.is_some()
            || self.ark_immune > 0
            || self.arrival.is_some()
        {
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
        let actor = self.actors[index].position;
        self.hurt = Some(Hurt::new(
            self.image,
            away(player, actor),
            super::attack::away(player, actor),
        ));
        self.ark_immune = ARK_IMMUNE;
    }

    /// The queued contact callback runs, a frame after the contact.
    pub(super) fn contact_frame(&mut self) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
