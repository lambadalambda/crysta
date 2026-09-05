use crate::{Direction, Room, Unqualified, WalkingState};

/// Version of the fixed walking-component encoding (not a game/asset schema).
pub const SNAPSHOT_VERSION: u16 = 3;
/// Byte length of a walking-component snapshot.
pub const SNAPSHOT_SIZE: usize = 16;

fn encode_direction(direction: Option<Direction>) -> u8 {
    direction.map_or(0, |d| d as u8 + 1)
}
fn decode_direction(byte: u8) -> Result<Option<Direction>, Unqualified> {
    match byte {
        0 => Ok(None),
        1 => Ok(Some(Direction::Down)),
        2 => Ok(Some(Direction::Up)),
        3 => Ok(Some(Direction::Left)),
        4 => Ok(Some(Direction::Right)),
        _ => Err(Unqualified::Snapshot),
    }
}

impl WalkingState {
    /// Encodes 16 bytes: `RWK\0`, version/u16 LE, x/u16 LE, y/u16 LE,
    /// active/u8, delayed/u8, phase/u8, last activation/u8, onset remaining/u8,
    /// one reserved zero byte.
    /// Directions encode None=0, Down=1, Up=2, Left=3, Right=4.
    /// Room identity and top-level simulation policy must be encoded by the caller.
    #[must_use]
    pub fn encode_snapshot(&self) -> [u8; SNAPSHOT_SIZE] {
        let mut bytes = [0; SNAPSHOT_SIZE];
        bytes[..4].copy_from_slice(b"RWK\0");
        bytes[4..6].copy_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.x.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.y.to_le_bytes());
        bytes[10] = encode_direction(self.active);
        bytes[11] = encode_direction(self.delayed);
        bytes[12] = self.phase;
        bytes[13] = encode_direction(self.last_activation);
        bytes[14] = self.onset_remaining;
        bytes
    }

    /// Restores structural walking invariants and verifies player bounds against a room.
    ///
    /// Does not authenticate room identity, history provenance, materials under a
    /// stationary player, or gameplay mode. Those remain the caller's responsibility.
    ///
    /// # Errors
    /// Rejects malformed/unknown encodings, invalid phases/history relationships and
    /// positions whose fixed player bounds are outside the supplied room.
    pub fn decode_snapshot(room: &Room, bytes: &[u8]) -> Result<Self, Unqualified> {
        if bytes.len() != SNAPSHOT_SIZE
            || &bytes[..4] != b"RWK\0"
            || u16::from_le_bytes([bytes[4], bytes[5]]) != SNAPSHOT_VERSION
            || bytes[15] != 0
        {
            return Err(Unqualified::Snapshot);
        }
        let state = Self {
            x: u16::from_le_bytes([bytes[6], bytes[7]]),
            y: u16::from_le_bytes([bytes[8], bytes[9]]),
            active: decode_direction(bytes[10])?,
            delayed: decode_direction(bytes[11])?,
            phase: bytes[12],
            last_activation: decode_direction(bytes[13])?,
            onset_remaining: bytes[14],
        };
        let phase_valid = match state.active {
            None => state.phase == 0,
            Some(d) if d.horizontal() => state.phase < 54,
            Some(_) => state.phase <= 2,
        };
        let history_valid = match (state.last_activation, state.delayed, state.active) {
            (None, None, None) => state.onset_remaining == 0,
            (Some(last), Some(delayed), active) => {
                last == delayed
                    && if active == Some(delayed) {
                        state.onset_remaining < 11
                    } else {
                        state.onset_remaining == 11
                    }
            }
            (Some(last), None, Some(active)) => last == active && state.onset_remaining < 11,
            (Some(_), None, None) => state.onset_remaining < 10,
            _ => false,
        };
        if !phase_valid || !history_valid {
            return Err(Unqualified::Snapshot);
        }
        room.validate_position(state.x, state.y)?;
        Ok(state)
    }
}
