type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn invalid(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}
fn sha256(bytes: &[u8]) -> String {
    rom::digests(bytes)
        .sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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

    struct Runner<'a> {
        data: &'a GameData,
        state: GameState,
        restored_actions: u64,
    }
    impl<'a> Runner<'a> {
        fn new(data: &'a GameData) -> Self {
            Self {
                data,
                state: GameState::new_game(data, Policy::SemanticPreview),
                restored_actions: 0,
            }
        }
        fn run(&mut self, command: u8, count: u16) -> Result<()> {
            if command > 9 || count == 0 {
                return Err(invalid("invalid itinerary command/count").into());
            }
            let action = |state: &mut GameState| -> std::result::Result<FrameOutput, SliceError> {
                match command {
                    5 => state.interact(self.data),
                    6 => state.acknowledge(self.data),
                    7..=9 => state.choose(self.data, command - 7),
                    _ => state.step(
                        self.data,
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
                }
            };
            for _ in 0..count {
                let before = self.state.snapshot();
                let mut twin = GameState::restore(self.data, &before)?;
                let result = action(&mut self.state);
                let other = action(&mut twin);
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
                self.restored_actions += 1;
            }
            Ok(())
        }
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
        assert!(runner.run(10, 1).is_err());
        assert_eq!(runner.state.snapshot(), before);
    }
}
