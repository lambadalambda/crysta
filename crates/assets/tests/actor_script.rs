//! Chained-condition semantics, checked against synthetic streams.
//!
//! The rules come from `$80:8695`: the first word's flag seeds the result,
//! `$4000` ors the next flag in, `$2000` ands it, `$1000` alone exclusive-ors
//! it, and bit 15 ends the chain and inverts the result.

use assets::maps::actor_script::{
    chained_condition_holds, chained_condition_length, walk_with_events, Stop, CHAINED_BRANCH,
    CHAINED_DESPAWN,
};
use assets::maps::scripts::EventFlags;

fn bitmap(set: &[u16]) -> Vec<u8> {
    let mut bits = vec![0u8; 512];
    for flag in set {
        bits[usize::from(*flag) / 8] |= 1 << (flag % 8);
    }
    bits
}

fn words(list: &[u16]) -> Vec<u8> {
    list.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn holds(chain: &[u16], set: &[u16]) -> bool {
    let bits = bitmap(set);
    chained_condition_holds(&words(chain), 0, &EventFlags::Bitmap(&bits)).expect("a chain")
}

#[test]
fn a_lone_word_tests_its_flag_and_bit_fifteen_inverts_it() {
    assert!(holds(&[0x0010], &[0x10]));
    assert!(!holds(&[0x0010], &[]));
    assert!(!holds(&[0x8010], &[0x10]));
    assert!(holds(&[0x8010], &[]));
}

#[test]
fn bit_fifteen_ends_the_chain_even_with_combinator_bits() {
    // $80:86A9 tests the sign before the nibble, so a negated word never
    // pulls in another; the length rule must agree with the evaluator.
    assert_eq!(
        chained_condition_length(&words(&[0x9010, 0x0011]), 0),
        Some(2)
    );
    assert!(holds(&[0x9010, 0x0011], &[]));
}

#[test]
fn or_and_and_combine_with_the_next_word() {
    assert!(holds(&[0x4010, 0x0011], &[0x11]));
    assert!(holds(&[0x4010, 0x0011], &[0x10]));
    assert!(!holds(&[0x4010, 0x0011], &[]));
    assert!(holds(&[0x2010, 0x0011], &[0x10, 0x11]));
    assert!(!holds(&[0x2010, 0x0011], &[0x10]));
    assert!(!holds(&[0x2010, 0x0011], &[0x11]));
}

#[test]
fn exclusive_or_keeps_the_low_bit_of_the_sum() {
    assert!(holds(&[0x1010, 0x0011], &[0x10]));
    assert!(holds(&[0x1010, 0x0011], &[0x11]));
    assert!(!holds(&[0x1010, 0x0011], &[0x10, 0x11]));
    assert!(!holds(&[0x1010, 0x0011], &[]));
}

#[test]
fn a_false_and_settles_the_chain_so_a_later_or_cannot_revive_it() {
    // $80:86F6 reads the rest without testing. With evaluation continuing,
    // the trailing or would make this true.
    assert!(!holds(&[0x2010, 0x4011, 0x0012], &[0x12]));
    // Whereas a true and lets the or take effect.
    assert!(holds(&[0x2010, 0x4011, 0x0012], &[0x10, 0x11]));
    // A failed and settles the chain too, even with the first flag set.
    assert!(!holds(&[0x2010, 0x4011, 0x0012], &[0x10, 0x12]));
    // The terminating word's polarity still applies to a settled result.
    assert!(holds(&[0x2010, 0x4011, 0x8012], &[0x12]));
}

#[test]
fn a_zero_exclusive_or_sum_settles_but_a_two_does_not() {
    // Both clear: the sum is zero and $80:86CC abandons the chain.
    assert!(!holds(&[0x1010, 0x4011, 0x0012], &[0x12]));
    // Both set: the sum is two, the low bit is zero, and evaluation goes on,
    // so the trailing or revives the result.
    assert!(holds(&[0x1010, 0x4011, 0x0012], &[0x10, 0x11, 0x12]));
}

#[test]
fn truncation_and_an_uncovered_flag_are_refused() {
    assert_eq!(
        chained_condition_holds(&[0x10], 0, &EventFlags::AllClear),
        None
    );
    assert_eq!(
        chained_condition_holds(&words(&[0x4010]), 0, &EventFlags::AllClear),
        None
    );
    let short = vec![0u8; 1];
    assert_eq!(
        chained_condition_holds(&words(&[0x0FFF]), 0, &EventFlags::Bitmap(&short)),
        None
    );
}

/// A one-bank image with a script at `$88:8000`.
fn image_with(script: &[u8]) -> Vec<u8> {
    let mut image = vec![0u8; 0x09_0000];
    image[0x08_8000..0x08_8000 + script.len()].copy_from_slice(script);
    image
}

#[test]
fn the_branch_form_jumps_past_its_target_when_the_chain_holds() {
    // COP 09 <0x0010> <target $8010>; at $8010 a COP 3B, which takes nothing.
    let mut script = vec![0x02, CHAINED_BRANCH, 0x10, 0x00, 0x10, 0x80];
    script.extend([0x02, 0x3B, 0x02, 0x3B]);
    script.resize(0x10, 0xEA);
    script.extend([0x02, 0x3B, 0xEA]);
    let image = image_with(&script);
    let set = bitmap(&[0x10]);
    // These synthetic handlers do not exist, so $3B is unaccounted here; the
    // walk stopping *at* $8010 rather than $8006 is what shows the branch.
    let taken = walk_with_events(&image, 0x88_8000, EventFlags::Bitmap(&set)).unwrap();
    assert_eq!(
        taken.stop,
        Stop::Unaccounted {
            offset: 0x08_8010,
            service: 0x3B
        }
    );
    let clear = bitmap(&[]);
    let fell = walk_with_events(&image, 0x88_8000, EventFlags::Bitmap(&clear)).unwrap();
    assert_eq!(
        fell.stop,
        Stop::Unaccounted {
            offset: 0x08_8006,
            service: 0x3B
        }
    );
}

#[test]
fn the_despawn_form_has_no_target_and_ends_the_walk_when_it_holds() {
    let script = [0x02, CHAINED_DESPAWN, 0x10, 0x00, 0x02, 0x3B];
    let image = image_with(&script);
    let set = bitmap(&[0x10]);
    let halted = walk_with_events(&image, 0x88_8000, EventFlags::Bitmap(&set)).unwrap();
    assert_eq!(halted.stop, Stop::Despawned { offset: 0x08_8000 });
    let clear = bitmap(&[]);
    let passed = walk_with_events(&image, 0x88_8000, EventFlags::Bitmap(&clear)).unwrap();
    assert_eq!(
        passed.stop,
        Stop::Unaccounted {
            offset: 0x08_8004,
            service: 0x3B
        }
    );
}
