//! Ark's own poses that the world's states show (`docs/ark-poses.md`): a
//! list of one of his resources (`$80:A24F + 6r`), mirrored or not, and
//! the frames into it. The hosts draw it in place of his walk.

use super::World;
use room_core::Direction;

/// A list of Ark's resource `resource` that a state shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArkPose {
    /// His resource, 0 to 5.
    pub resource: u8,
    /// The list.
    pub list: u8,
    /// Mirrored.
    pub hflip: bool,
    /// Frames into the list.
    pub age: u16,
    /// Played once and held on its last frame; else looped.
    pub once: bool,
}

impl ArkPose {
    pub(crate) const fn new(resource: u8, list: u8, hflip: bool, age: u16, once: bool) -> Self {
        Self {
            resource,
            list,
            hflip,
            age,
            once,
        }
    }

    /// The art the hosts know the resource by: its entry in `$80:A24F`.
    #[must_use]
    pub const fn art(self) -> u32 {
        0x80_A24F + 6 * self.resource as u32
    }
}

/// A list chosen by facing with `COP 5F`: Down, Up, Left (mirrored) and
/// Right take `lists` 0, 1, 2 and 2.
pub(super) const fn by_facing(
    resource: u8,
    lists: [u8; 3],
    facing: Direction,
    age: u16,
) -> ArkPose {
    let (list, hflip) = match facing {
        Direction::Down => (lists[0], false),
        Direction::Up => (lists[1], false),
        Direction::Left => (lists[2], true),
        Direction::Right => (lists[2], false),
    };
    ArkPose::new(resource, list, hflip, age, true)
}

impl World<'_> {
    /// The pose a state shows of Ark: his script's own list (the
    /// Guardner's sleep), a landing, a fall, a drop, the rope, the burn, a
    /// level up, a chest or a Magirock; `None` when he walks or stands.
    #[must_use]
    pub fn ark_pose(&self) -> Option<ArkPose> {
        self.player_actor
            .as_ref()
            .and_then(crate::actors::Actor::ark_shown)
            .or_else(|| self.landing.and_then(super::landing::Landing::pose))
            .or_else(|| self.fall.and_then(super::fall::Fall::pose))
            .or_else(|| self.jump.and_then(|jump| jump.pose(self.facing)))
            .or_else(|| self.burn.and_then(super::status::Burn::pose))
            .or_else(|| {
                let walking = self.walking.active_direction();
                self.rope
                    .map(|rope| rope.pose(self.facing, walking, self.globals.frames))
            })
            .or_else(|| self.level_up.as_ref().map(super::levelup::LevelUp::pose))
            .or_else(|| self.chest.map(|chest| chest.pose(self.facing)))
            .or_else(|| self.pickup.map(|pickup| pickup.pose(self.facing)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pose_names_its_resources_art_and_facing_picks_the_list() {
        assert_eq!(ArkPose::new(1, 9, false, 0, true).art(), 0x80_A255);
        assert_eq!(ArkPose::new(5, 4, false, 0, true).art(), 0x80_A26D);
        let lift = |facing| {
            let pose = by_facing(3, [0x18, 0x19, 0x1A], facing, 2);
            (pose.list, pose.hflip)
        };
        assert_eq!(lift(Direction::Down), (0x18, false));
        assert_eq!(lift(Direction::Up), (0x19, false));
        assert_eq!(lift(Direction::Left), (0x1A, true));
        assert_eq!(lift(Direction::Right), (0x1A, false));
    }
}
