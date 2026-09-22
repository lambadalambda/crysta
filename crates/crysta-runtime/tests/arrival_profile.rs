//! Portable replay of the measured completed-frame profiles; no ROM required.
use room_core::arrival::{Arrival, ArrivalPhase, ReturnRoute};
use serde_json::Value;

#[test]
fn every_arrival_sample_and_handoff_matches_pinned_native_profiles() {
    let evidence: Value = serde_json::from_str(include_str!(
        "../../../tools/arrival-qualification/evidence.json"
    ))
    .unwrap();
    for (key, route) in [("1e", ReturnRoute::Town), ("19", ReturnRoute::Stairs)] {
        let edge = &evidence["edges"][key];
        let mut arrival = Arrival::new(route);
        let end = edge["advances"].as_u64().unwrap();
        for elapsed in 0..=end {
            let position = edge["positions"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|row| row[0].as_u64().unwrap() <= elapsed)
                .unwrap();
            assert_eq!(
                arrival.position(),
                (
                    u16::try_from(position[1].as_u64().unwrap()).unwrap(),
                    u16::try_from(position[2].as_u64().unwrap()).unwrap()
                ),
                "{key} cursor{elapsed}"
            );
            assert_eq!(u64::from(arrival.elapsed()), elapsed);
            let phase = if elapsed == end {
                ArrivalPhase::Free
            } else if elapsed >= edge["phases"]["recovery"].as_u64().unwrap() {
                ArrivalPhase::Recovery
            } else if elapsed >= edge["phases"]["ownership"].as_u64().unwrap() {
                ArrivalPhase::Forced
            } else {
                ArrivalPhase::Initialized
            };
            assert_eq!(arrival.phase(), phase, "{key} cursor{elapsed}");
            assert_eq!(arrival.owns_player(), elapsed < end);
            if elapsed < end {
                arrival.advance();
            }
        }
        let complete = arrival;
        arrival.advance();
        assert_eq!(
            arrival, complete,
            "completion must not create a trailing tick"
        );
    }
}
