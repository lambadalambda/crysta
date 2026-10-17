//! Which handler runs each `COP` service, for [`super::Actor::service`] and
//! for the inventory of what the runtime does with each one
//! (`docs/cop-inventory.md`).

use super::{
    ANIMATE_AND_END, ARK_BUSY, ARK_POSE_REPEAT, BLOCK, BLOCKED_RIGHT, BLOCKED_UP, BODY,
    BRANCH_ON_FLAG, BRANCH_ON_GLOBAL, BRANCH_ON_MAP, CALL, CHAINED_BRANCH, CHAINED_DESPAWN, CHOICE,
    CIRCLE, COUNT, COUNT_BRANCH, DELETE_ON_MAP, EASE_START, EASE_STEP, FACE_PLAYER_POSED,
    FADE_TO_TRACK, FLIPS, GIVE_ITEM, GRANT_ITEM, HELD, HELD_BRANCH, HIT_RETURN, HIT_TARGET,
    LINE_START, LINE_STEP, LOAD_COLOURS, LOOP_AGAIN, LOOP_END, LOOP_START, MUSIC_WAIT, NO_ROOM,
    PAN, PAN_WAIT, PATCH, PATCH_AT, PLACE, PLAYER_AXIS, PLAYER_FRONT, PLAYER_HEIGHT, PLAYER_NEAR,
    PLAYER_POSE_MOVING, PLAYER_SERVICES, PLAYER_SIDE, PLAY_SELECTION, PLAY_TRACK, POSE_MOVING,
    POSE_SELECTOR, PPU_WRITE, PROFILE, RANDOMIZE, RANDOM_STEP, REPEAT_MOVING, REPEAT_POSE,
    REPEAT_POSE_MOVING, RETURN, SCRIPT_SERVICES, SEAL, SELECT_POSE, SHOW_TEXT, SHOW_TEXT_BANKED,
    SIDE_CALLBACKS, SLEEP_OFF_SCREEN, SOUND_PORT2, SOUND_PORT3, SOUND_WORD, SPAWNS, SPEED, SPIN,
    STAMP, STOP_FOR_PLAYER, SWITCH, TAKE_ITEM, TEXT_STEP, TEXT_WAIT, TILE_BRANCH, TIMED_WAIT,
    UNSEAL, UNSTAMP, WAIT, WAIT_STEP, WALK_TO_COLUMN, WALK_TO_ROW, YIELD,
};
use assets::maps::actor_script;

/// The handler a service runs in [`super::Actor::service`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Text,
    Script,
    Ease,
    Parameter,
    ArkPose,
    MovingPose,
    Audio,
    Call,
    PlayerTest,
    Engine,
    Return,
    Item,
    Stage,
    Tile,
    Cell,
    Door,
    Spawn,
    WalkToward,
    SelectPose,
    Line,
    Profile,
    ArkBusy,
    Flip,
    Wait,
    WaitStep,
    RandomStep,
    Player,
    BranchOnGlobal,
    Body,
    Pan,
    LoopStart,
    LoopEnd,
    LoopAgain,
    BranchOnMap,
    TimedWait,
    StopForPlayer,
    FacePlayerPosed,
    BranchOnFlag,
    Chain,
    Display,
    /// `COP 91`, which [`super::Actor::run`] takes as the frame's end.
    EndFrame,
    /// No handler: stepped over by its operand length.
    Skip,
}

/// The handler for `service`; Ark's own pose services only for Ark.
pub(super) fn route(service: u8, ark: bool) -> Route {
    match service {
        SHOW_TEXT | SHOW_TEXT_BANKED | TEXT_WAIT | TEXT_STEP | CHOICE => Route::Text,
        _ if SCRIPT_SERVICES.contains(&service) => Route::Script,
        EASE_START | EASE_STEP => Route::Ease,
        SWITCH | SPEED => Route::Parameter,
        PLAYER_POSE_MOVING | ARK_POSE_REPEAT if ark => Route::ArkPose,
        POSE_MOVING | POSE_SELECTOR | REPEAT_POSE_MOVING | REPEAT_MOVING => Route::MovingPose,
        PLAY_TRACK | FADE_TO_TRACK | PLAY_SELECTION | SOUND_PORT3 | SOUND_PORT2 | SOUND_WORD => {
            Route::Audio
        }
        CALL => Route::Call,
        PLAYER_NEAR | PLAYER_AXIS | PLAYER_SIDE | PLAYER_HEIGHT | PLAYER_FRONT => Route::PlayerTest,
        RANDOMIZE | SLEEP_OFF_SCREEN => Route::Engine,
        RETURN => Route::Return,
        GIVE_ITEM | GRANT_ITEM | TAKE_ITEM | NO_ROOM => Route::Item,
        PLACE | DELETE_ON_MAP | REPEAT_POSE | COUNT | YIELD => Route::Stage,
        TILE_BRANCH | PATCH => Route::Tile,
        STAMP | UNSTAMP | SEAL | BLOCK | UNSEAL | PATCH_AT | HELD | BLOCKED_UP..=BLOCKED_RIGHT => {
            Route::Cell
        }
        HIT_TARGET | HIT_RETURN | COUNT_BRANCH | HELD_BRANCH | MUSIC_WAIT | SPIN | PPU_WRITE => {
            Route::Door
        }
        _ if SPAWNS.contains(&service) => Route::Spawn,
        WALK_TO_ROW | WALK_TO_COLUMN => Route::WalkToward,
        SELECT_POSE => Route::SelectPose,
        LINE_START | LINE_STEP => Route::Line,
        PROFILE | SIDE_CALLBACKS => Route::Profile,
        ARK_BUSY => Route::ArkBusy,
        _ if FLIPS.contains(&service) => Route::Flip,
        WAIT => Route::Wait,
        WAIT_STEP => Route::WaitStep,
        RANDOM_STEP => Route::RandomStep,
        _ if PLAYER_SERVICES.contains(&service) => Route::Player,
        BRANCH_ON_GLOBAL => Route::BranchOnGlobal,
        _ if BODY.contains(&service) => Route::Body,
        PAN | PAN_WAIT => Route::Pan,
        LOOP_START => Route::LoopStart,
        LOOP_END => Route::LoopEnd,
        LOOP_AGAIN => Route::LoopAgain,
        BRANCH_ON_MAP => Route::BranchOnMap,
        TIMED_WAIT => Route::TimedWait,
        STOP_FOR_PLAYER => Route::StopForPlayer,
        FACE_PLAYER_POSED => Route::FacePlayerPosed,
        BRANCH_ON_FLAG => Route::BranchOnFlag,
        CHAINED_BRANCH | CHAINED_DESPAWN => Route::Chain,
        LOAD_COLOURS | CIRCLE => Route::Display,
        ANIMATE_AND_END => Route::EndFrame,
        _ => Route::Skip,
    }
}

/// What the runtime does with a service in an actor's script (not Ark's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handling {
    /// A handler models it; see `meta/issues/partial-cop-services.md` for
    /// the cases some handlers refuse.
    Modelled,
    /// Stepped over by the operand length its native handler shows. A
    /// sound-free cosmetic step is right; anything else is missing.
    SteppedOver,
    /// Its length is not known, so the actor stops there.
    Stops,
}

/// What the runtime does with `service`.
#[must_use]
pub fn handling(image: &[u8], service: u8) -> Handling {
    if route(service, false) != Route::Skip {
        Handling::Modelled
    } else if actor_script::operand_length(image, service).is_some() {
        Handling::SteppedOver
    } else {
        Handling::Stops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ark_pose_services_route_only_for_ark() {
        assert_eq!(route(PLAYER_POSE_MOVING, true), Route::ArkPose);
        assert_eq!(route(PLAYER_POSE_MOVING, false), Route::Skip);
        assert_eq!(route(TILE_BRANCH, false), Route::Tile);
        assert_eq!(route(ANIMATE_AND_END, false), Route::EndFrame);
    }

    #[test]
    fn handling_steps_over_a_service_with_a_derived_length() {
        // COP 7B's handler: INC $36; LDA $36; STA $02,S; RTI.
        let mut image = vec![0; 0x1_0000];
        image[0x83B2 + 0x7B * 2..][..2].copy_from_slice(&0x9000u16.to_le_bytes());
        image[0x9000..0x9007].copy_from_slice(&[0xE6, 0x36, 0xA5, 0x36, 0x83, 0x02, 0x40]);
        assert_eq!(handling(&image, 0x7B), Handling::SteppedOver);
        assert_eq!(handling(&image, 0x7C), Handling::Stops);
        assert_eq!(handling(&image, TILE_BRANCH), Handling::Modelled);
    }
}
