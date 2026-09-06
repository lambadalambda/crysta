//! Wider semantic projection required by the qualified Pandora source operands.
//! Synthetic sequences below test the component, not the actual story request order.
use room_core::events::{
    EventError, EventFlags, EventOp, EventSequence, StoryFlags, StorySequence,
};

#[test]
fn high_story_flags_wait_for_acknowledgement_and_roundtrip_as_bytes() {
    let sequence = StorySequence::new(vec![
        EventOp::ShowPage(10),
        EventOp::SetFlag(0x243),
        EventOp::SetFlag(0x244),
        EventOp::ShowPage(20),
        EventOp::SetFlag(0x292),
    ])
    .unwrap();
    let mut bytes = [0; 128];
    bytes[4] = 1; // Retain startup $20 and $FB without captured memory.
    bytes[31] = 8;
    let mut flags = StoryFlags::new(bytes);
    let mut cursor = sequence.start(&mut flags);
    assert_eq!(flags.bytes(), &bytes);
    assert_eq!(sequence.choose(&mut cursor, 1), Err(EventError::NotWaiting));
    assert_eq!(flags.bytes(), &bytes);
    sequence.acknowledge(&mut cursor, &mut flags).unwrap();
    bytes[72] = 0x18; // $243/$244: low-bit-first within this semantic projection.
    assert_eq!(flags.bytes(), &bytes);
    assert!(!flags.contains(0x292).unwrap());
    let mut restored = StoryFlags::new(*flags.bytes());
    let mut restored_cursor = sequence.restore_cursor(cursor.position()).unwrap();
    sequence.acknowledge(&mut cursor, &mut flags).unwrap();
    sequence
        .acknowledge(&mut restored_cursor, &mut restored)
        .unwrap();
    bytes[82] = 4; // $292.
    assert_eq!(flags.bytes(), &bytes);
    assert_eq!(restored, flags);
    assert_eq!(restored_cursor, cursor);
    assert_eq!(sequence.page(cursor), None);
    assert_eq!(
        sequence.acknowledge(&mut cursor, &mut flags),
        Err(EventError::NotWaiting)
    );
    assert_eq!(flags.bytes(), &bytes);
}

#[test]
fn projection_limits_are_explicit_and_do_not_expand_the_house_contract() {
    for flag in [512, 0x243, 0x244, 0x292, 1023] {
        assert_eq!(
            EventFlags::new([0; 64]).contains(flag),
            Err(EventError::Flag)
        );
        assert_eq!(
            EventSequence::new(vec![EventOp::ShowPage(1), EventOp::SetFlag(flag)]),
            Err(EventError::Flag)
        );
        assert!(!StoryFlags::new([0; 128]).contains(flag).unwrap());
        assert!(StorySequence::new(vec![EventOp::ShowPage(1), EventOp::SetFlag(flag)]).is_ok());
    }
    for flag in [1024, u16::MAX] {
        assert_eq!(
            StoryFlags::new([0; 128]).contains(flag),
            Err(EventError::Flag)
        );
        assert_eq!(
            StorySequence::new(vec![EventOp::ShowPage(1), EventOp::SetFlag(flag)]),
            Err(EventError::Flag)
        );
    }
}

#[test]
fn wider_choice_cursor_keeps_atomic_rejection_and_explicit_results() {
    let sequence = StorySequence::new(vec![EventOp::Choose {
        catalog: 1,
        branches: [7, 8, 7],
    }])
    .unwrap();
    let mut flags = StoryFlags::new([0; 128]);
    let mut cursor = sequence.start(&mut flags);
    let before = cursor;
    assert_eq!(
        sequence.acknowledge(&mut cursor, &mut flags),
        Err(EventError::NotWaiting)
    );
    assert_eq!(sequence.choose(&mut cursor, 3), Err(EventError::Selection));
    assert_eq!(cursor, before);
    assert_eq!(flags.bytes(), &[0; 128]);
    assert_eq!(sequence.choose(&mut cursor, 0), Ok(7));
    assert_eq!(sequence.choose(&mut cursor, 1), Err(EventError::NotWaiting));
    assert_eq!(cursor, sequence.restore_cursor(1).unwrap());
}

#[test]
fn final_bit_of_wider_projection_is_written_without_aliasing_low_flags() {
    let sequence = StorySequence::new(vec![EventOp::ShowPage(1), EventOp::SetFlag(1023)]).unwrap();
    let mut flags = StoryFlags::new([0; 128]);
    let mut cursor = sequence.start(&mut flags);
    sequence.acknowledge(&mut cursor, &mut flags).unwrap();
    let mut expected = [0; 128];
    expected[127] = 0x80;
    assert_eq!(flags.bytes(), &expected);
    assert!(flags.contains(1023).unwrap());
}
