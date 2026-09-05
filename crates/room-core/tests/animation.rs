//! Animation tests deliberately stand apart from walking/snapshot integration.
use room_core::{AnimationSet, AnimationState, Direction};
use Direction::{Down, Left, Right, Up};

#[test]
fn six_native_records_last_nine_ticks_not_two_frames() {
    let mut state = AnimationState::standing(Down);
    for age in 0..120 {
        let frame = state.advance(Some(Right));
        assert_eq!(frame.set, AnimationSet::Walking);
        assert_eq!(frame.sequence, 2);
        assert_eq!(frame.record, (age % 54) / 9);
        assert_eq!(state.phase(), age % 54);
    }
}

#[test]
fn all_directions_turn_reset_and_mirror_only_left() {
    let mut state = AnimationState::standing(Down);
    for (direction, sequence, mirror) in [
        (Right, 2, false),
        (Left, 2, true),
        (Up, 1, false),
        (Down, 0, false),
    ] {
        for age in 0..70 {
            let frame = state.advance(Some(direction));
            assert_eq!(
                (frame.sequence, frame.record, frame.mirror_x),
                (sequence, (age % 54) / 9, mirror)
            );
            assert_eq!(state.facing(), direction);
        }
    }
}

#[test]
fn release_stands_immediately_on_delayed_neutral_and_retains_facing() {
    let mut state = AnimationState::standing(Down);
    for _ in 0..16 {
        state.advance(Some(Left));
    }
    let standing = state.advance(None);
    assert_eq!(standing.set, AnimationSet::Standing);
    assert_eq!(
        (standing.sequence, standing.record, standing.mirror_x),
        (2, 0, true)
    );
    for _ in 0..600 {
        assert_eq!(state.advance(None), standing);
    }
    assert_eq!(state.phase(), 0);
    assert!(!state.is_walking());
    assert_eq!(state.advance(Some(Left)).record, 0);
}

#[test]
fn active_direction_owns_latency_and_blocked_holds_not_displacement() {
    let mut state = AnimationState::standing(Down);
    // F+1 still neutral; caller supplies WalkingState's delayed active direction.
    assert_eq!(state.advance(None).set, AnimationSet::Standing);
    // F+2 setup has zero displacement but already displays walk record zero.
    assert_eq!(state.advance(Some(Up)).record, 0);
    // Wall-blocked positions could remain constant throughout: still animate.
    for age in 1..55 {
        assert_eq!(state.advance(Some(Up)).record, (age % 54) / 9);
    }
    // R+1 retains old active; R+2 neutral owns standing, no extra delay here.
    assert_eq!(state.advance(Some(Up)).set, AnimationSet::Walking);
    assert_eq!(state.advance(None).set, AnimationSet::Standing);
}

#[test]
fn restore_is_canonical_and_deterministic() {
    assert!(AnimationState::from_parts(Down, false, 1).is_none());
    assert!(AnimationState::from_parts(Down, true, 54).is_none());
    let mut state = AnimationState::standing(Down);
    for _ in 0..37 {
        state.advance(Some(Left));
    }
    let mut restored =
        AnimationState::from_parts(state.facing(), state.is_walking(), state.phase()).unwrap();
    for active in [Some(Left), Some(Left), Some(Up), None, None, Some(Right)] {
        assert_eq!(state.advance(active), restored.advance(active));
        assert_eq!(state, restored);
    }
}
