type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}
fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut digest = String::with_capacity(64);
    for byte in rom::digests(bytes).sha256 {
        write!(digest, "{byte:02x}").expect("writing to String");
    }
    digest
}
fn main() {
    eprintln!("Use run.sh: offline compiler/continuation qualification, not live host enablement.");
}

#[cfg(test)]
mod pandora_progression_itinerary {
    use super::*;
    use room_core::{
        slice::{FrameOutput, GameData, GameState, Policy, SliceError},
        Direction, FrameInput,
    };

    // Frozen input prefix from house_progression's existing 1701-action qualification.
    // Commands are public inputs only: neutral,L,R,U,D,interact,ack,choice0..2.
    // No native checkpoint, grid, position, story flag or pot ledger initializes Runner.
    #[rustfmt::skip]
const HOUSE_PREFIX: &[(u8, u16)] = &[
    (2,62),(0,38),(4,67),(0,35),(4,42),(1,78),(0,90),(1,12),(0,100),
    (1,55),(3,70),(0,100),(5,1),(3,13),(0,35),(3,45),(0,60),(5,1),
    (2,60),(6,1),(8,1),(6,1),(6,1),(6,1),(5,1),(7,1),(6,1),(6,1),
    (4,60),(0,100),(1,10),(4,82),(0,100),(4,70),(0,100),(4,32),
    (0,60),(2,24),(0,90),
];

    fn apply(
        data: &GameData,
        state: &mut GameState,
        command: u8,
    ) -> std::result::Result<FrameOutput, SliceError> {
        match command {
            5 => state.interact(data),
            6 => state.acknowledge(data),
            7..=9 => state.choose(data, command - 7),
            10 => state.pot_action(data),
            0..=4 => state.step(
                data,
                FrameInput {
                    direction: match command {
                        1 => Some(Direction::Left),
                        2 => Some(Direction::Right),
                        3 => Some(Direction::Up),
                        4 => Some(Direction::Down),
                        _ => None,
                    },
                },
            ),
            _ => Err(SliceError::Interaction),
        }
    }

    fn semantic(data: &GameData, state: &GameState) -> serde_json::Value {
        if !data.pandora_enabled() {
            return serde_json::Value::Null;
        }
        let output = state.pandora_output(data).unwrap();
        let flags = [0x26, 0x28, 0x27, 0x2e, 0x292, 0x22, 0x243, 0x244]
            .into_iter()
            .filter(|&b| state.story_flags().contains(b).unwrap())
            .collect::<Vec<_>>();
        serde_json::json!({"map":state.output().map_id,"scene":format!("{:?}",output.scene),
            "owner":format!("{:?}",output.owner),"invocation":output.invocation.map(|v|format!("{v:?}")),
            "cue":output.cue.map(|v|format!("{v:?}")),"motion":output.motion.map(|(v,_)|format!("{v:?}")),
            "counter":output.door_counter,"locals":output.locals,"town_open":output.town_open,"wooden":state.wooden_door_open(),
            "sheet":{"resident":output.sheet.resident,"cellar":format!("{:?}",output.sheet.cellar),"consumed":output.sheet.consumed},
            "pot":state.pot_state().map(|p|format!("{:?}",p.phase())),
            "flags":flags})
    }

    struct Runner<'a> {
        data: &'a GameData,
        state: GameState,
        restored_actions: u64,
        recorded: Vec<(u8, u16)>,
        events: Vec<serde_json::Value>,
    }
    impl<'a> Runner<'a> {
        fn new(data: &'a GameData) -> Self {
            Self {
                data,
                state: GameState::new_game(data, Policy::SemanticPreview),
                restored_actions: 0,
                recorded: Vec::new(),
                events: Vec::new(),
            }
        }
        fn run(&mut self, command: u8, count: u16) -> Result<()> {
            if command > 10 || count == 0 {
                return Err(invalid("invalid itinerary command/count").into());
            }
            for _ in 0..count {
                let before = self.state.snapshot();
                let before_semantic = semantic(self.data, &self.state);
                let mut twin = GameState::restore(self.data, &before)?;
                let result = apply(self.data, &mut self.state, command);
                let other = apply(self.data, &mut twin, command);
                if result != other || self.state.snapshot() != twin.snapshot() {
                    return Err(invalid("per-action restored continuation diverged").into());
                }
                if let Err(error) = result {
                    if self.state.snapshot() != before {
                        return Err(invalid("rejected action mutated state").into());
                    }
                    return Err(error.into());
                }
                // The continuing branch itself is restored AFTER every successful action.
                let after = self.state.snapshot();
                self.state = GameState::restore(self.data, &after)?;
                if self.state.snapshot() != after || self.state.output() != twin.output() {
                    return Err(invalid("post-action restoration diverged").into());
                }
                if self.state.effective_room(self.data)? != twin.effective_room(self.data)?
                    || (self.data.pandora_enabled()
                        && self.state.pandora_output(self.data)?
                            != twin.pandora_output(self.data)?)
                {
                    return Err(invalid("restored effective room or story output diverged").into());
                }
                self.restored_actions += 1;
                if let Some((_, count)) = self
                    .recorded
                    .last_mut()
                    .filter(|(last, _)| *last == command)
                {
                    *count += 1;
                } else {
                    self.recorded.push((command, 1));
                }
                let after_semantic = semantic(self.data, &self.state);
                if before_semantic != after_semantic {
                    self.events.push(serde_json::json!({"tick":self.state.output().tick,"position":self.state.output().position,"state":after_semantic}));
                }
            }
            Ok(())
        }
    }

    fn assert_semantics(events: &[serde_json::Value]) {
        let at = |tick| &events.iter().find(|e| e["tick"] == tick).unwrap()["state"];
        assert_eq!(at(2618)["town_open"], 1);
        assert_eq!(at(5520)["town_open"], 2);
        assert_eq!(at(2649)["town_open"], 0);
        assert_eq!(at(5551)["town_open"], 0);
        assert_eq!(at(6431)["counter"], 0);
        assert_eq!(at(6431)["pot"], "Empty");
        assert_eq!(at(6431)["sheet"]["consumed"], 4);
        assert_eq!(at(7657)["counter"], 1);
        assert_eq!(at(7657)["sheet"]["consumed"], 5);
        assert_eq!(at(9128)["counter"], 2);
        assert_eq!(at(9128)["sheet"]["consumed"], 7);
        for tick in [9353, 9558] {
            assert_eq!(
                at(tick)["sheet"],
                serde_json::json!({"resident":true,"cellar":"Open","consumed":7})
            );
            assert_eq!(at(tick)["locals"], 0);
        }
        assert_eq!(at(9763)["sheet"]["consumed"], 0);
        assert_eq!(at(10087)["invocation"], "BoxWarning");
        assert_eq!(at(10087)["locals"], 2);
        assert_eq!(at(10121)["locals"], 6);
        assert_eq!(at(10129)["cue"], "BoxAcquireControl");
        assert_eq!(at(10130)["cue"], "BoxReload");
        assert_eq!(at(10131)["locals"], 0);
        let mut grants = Vec::new();
        let mut previous = Vec::new();
        let mut invocations = Vec::new();
        for event in events {
            let state = &event["state"];
            let flags: Vec<u16> = serde_json::from_value(state["flags"].clone()).unwrap();
            for flag in &flags {
                if !previous.contains(flag) {
                    grants.push((event["tick"].as_u64().unwrap(), *flag));
                }
            }
            previous = flags;
            if let Some(invocation) = state["invocation"].as_str() {
                if invocations.last() != Some(&invocation) {
                    invocations.push(invocation);
                }
            }
        }
        assert_eq!(
            grants,
            [
                (965, 0x26),
                (3187, 0x28),
                (5706, 0x27),
                (5778, 0x2e),
                (9189, 0x0292),
                (10130, 0x22),
                (11218, 0x0243),
                (11222, 0x0244)
            ]
        );
        assert_eq!(
            invocations,
            room_core::slice::Invocation::ALL
                .iter()
                .map(|i| format!("{i:?}"))
                .collect::<Vec<_>>()
        );
    }

    const ROUTE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/pandora-runtime-qualification/route.json"
    ));
    const CHECKPOINTS: &[(u64, u16, (u16, u16))] = &[
        (1701, 0x000a, (538, 815)),
        (2617, 0x000a, (472, 304)),
        (2710, 0x0013, (392, 207)),
        (3187, 0x0013, (360, 144)),
        (4333, 0x000a, (472, 400)),
        (4517, 0x000a, (504, 400)),
        (5520, 0x000a, (504, 768)),
        (10078, 0x0021, (136, 360)),
        (10087, 0x0021, (136, 359)),
        (10129, 0x0021, (136, 368)),
        (11222, 0x0041, (136, 208)),
        (11314, 0x0041, (120, 208)),
        (11406, 0x0041, (120, 192)),
        (11498, 0x0041, (136, 192)),
        (11590, 0x0041, (136, 208)),
    ];
    fn replay<'a>(data: &'a GameData, actions: &[(u8, u16)], probes: bool) -> Result<Runner<'a>> {
        let mut runner = Runner::new(data);
        for &(command, count) in actions {
            if count == 0 {
                return Err(invalid("zero-count route command").into());
            }
            for _ in 0..count {
                runner.run(command, 1)?;
                let out = runner.state.output();
                if CHECKPOINTS.iter().any(|&(tick, map, pos)| {
                    out.tick == tick && (out.map_id != map || out.position != pos)
                }) {
                    return Err(invalid(&format!("checkpoint mismatch at {}", out.tick)).into());
                }
                if probes && matches!(out.tick, 2617 | 2618 | 10129 | 11590) {
                    let mut branch = runner.state.clone();
                    if out.tick == 2617 {
                        apply(data, &mut branch, 2)?;
                        for _ in 0..80 {
                            apply(data, &mut branch, 0)?;
                        }
                        assert_eq!(branch.output().position, runner.state.output().position);
                    }
                    let before = branch.snapshot();
                    assert_eq!(
                        apply(data, &mut branch, if out.tick == 11590 { 6 } else { 5 }),
                        Err(SliceError::Interaction)
                    );
                    assert_eq!(branch.snapshot(), before);
                }
            }
        }
        Ok(runner)
    }
    fn owned_rom() -> rom::Rom {
        rom::Rom::load(&std::fs::read(std::env::var("PANDORA_ROM").unwrap()).unwrap()).unwrap()
    }
    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn input_only_pandora_journey() {
        let rom = owned_rom();
        let fixture: serde_json::Value = serde_json::from_str(ROUTE).unwrap();
        assert_eq!(fixture["schema"], 1);
        assert_eq!(fixture["rom_sha256"], sha256(rom.image()));
        let actions: Vec<(u8, u16)> = serde_json::from_value(fixture["actions"].clone()).unwrap();
        let data = pandora_progression::compile(&rom).unwrap();
        let runner = replay(&data, &actions, true).unwrap();
        let out = runner.state.output();
        assert_eq!(runner.recorded, actions);
        assert_eq!(runner.restored_actions, out.tick);
        assert_eq!(fixture["expected"]["tick"], out.tick);
        assert_eq!(fixture["expected"]["map"], out.map_id);
        assert_eq!(
            fixture["expected"]["position"],
            serde_json::json!(out.position)
        );
        assert_eq!(
            fixture["expected"]["snapshot_sha256"],
            sha256(&runner.state.snapshot())
        );
        assert_eq!(
            fixture["expected"]["semantic_sha256"],
            sha256(&serde_json::to_vec(&runner.events).unwrap())
        );
        assert_eq!(
            runner.state.pandora_output(&data).unwrap().owner,
            room_core::slice::ControlOwner::Player
        );
        assert!(runner.state.dialogue(&data).unwrap().is_none());
        assert_semantics(&runner.events);
        let report = serde_json::json!({"schema":1,"rom_sha256":sha256(rom.image()),"core_profile":room_core::slice::PROFILE_VERSION,
            "route_sha256":sha256(ROUTE.as_bytes()),"actions":out.tick,"restored_actions":runner.restored_actions,
            "final":fixture["expected"],"events":runner.events});
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        eprintln!(
            "qualified {} input actions/restores; final41 control; report.json",
            out.tick
        );
    }
    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn missing_real_town_interaction_cannot_replay_the_route() {
        let fixture: serde_json::Value = serde_json::from_str(ROUTE).unwrap();
        let actions: Vec<(u8, u16)> = serde_json::from_value(fixture["actions"].clone()).unwrap();
        let mut expanded: Vec<(u8, u16)> = actions
            .into_iter()
            .flat_map(|(cmd, n)| std::iter::repeat_n((cmd, 1), usize::from(n)))
            .collect();
        assert_eq!(expanded[2617], (5, 1));
        expanded[2617] = (0, 1);
        let data = pandora_progression::compile(&owned_rom()).unwrap();
        let Err(error) = replay(&data, &expanded, false) else {
            panic!("missing interaction accepted")
        };
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().to_string(),
            "checkpoint mismatch at 2710"
        );
    }

    #[test]
    #[ignore = "owned ROM; run tools/pandora-runtime-qualification/run.sh"]
    fn input_runner_replays_house_prefix_with_restore_after_every_action() {
        let rom =
            rom::Rom::load(&std::fs::read(std::env::var("PANDORA_ROM").unwrap()).unwrap()).unwrap();
        let data = house_progression::compile(&rom).unwrap();
        let mut runner = Runner::new(&data);
        for &(command, count) in HOUSE_PREFIX {
            runner.run(command, count).unwrap();
        }
        let output = runner.state.output();
        assert_eq!(
            (output.tick, output.map_id, output.position),
            (1701, 0xa, (538, 815))
        );
        assert_eq!(runner.restored_actions, 1701);
        assert!(runner.state.event_flags().contains(0x26).unwrap());
        assert!(runner.state.dialogue(&data).unwrap().is_none());
        // Errors also restore identically and must not consume an action/state.
        let before = runner.state.snapshot();
        let rejected = runner.run(6, 1).unwrap_err();
        assert_eq!(
            rejected.downcast_ref::<SliceError>(),
            Some(&SliceError::Interaction)
        );
        assert_eq!(runner.state.snapshot(), before);
        assert_eq!(runner.restored_actions, 1701);
        assert!(runner.run(11, 1).is_err());
        assert_eq!(runner.state.snapshot(), before);
    }
}
