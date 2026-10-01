//! The combat core's arithmetic (`docs/combat.md`): an enemy's profile,
//! the damage each side deals, and the level table.

/// Ark's stats (`$064E` block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// `$0656`.
    pub level: u8,
    /// `$065D`.
    pub life: u16,
    /// `$0657`.
    pub max_life: u16,
    /// `$0662`.
    pub attack: u16,
    /// `$065F`.
    pub defense: u16,
    /// `$0666`.
    pub luck: u8,
    /// `$0659`: the equipped weapon's power.
    pub weapon: u16,
    /// `$065B`: the equipped armor's power.
    pub armor: u16,
    /// `$0690`, as a number (the game keeps BCD).
    pub exp: u32,
}

impl Stats {
    /// Level 1 (`$8D:BA61`'s first entry), the spear in hand.
    #[must_use]
    pub const fn start() -> Self {
        Self {
            level: 1,
            life: 28,
            max_life: 28,
            attack: 3,
            defense: 2,
            luck: 3,
            weapon: 3,
            armor: 0,
            exp: 0,
        }
    }
}

/// An enemy's 25-byte profile (`$8D:BDFA[n]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// +2: attack types it resists (high nibble) and by how much (2 bits each).
    pub resist: u16,
    /// +6: attack types it is weak to, the same way.
    pub weak: u16,
    /// +8.
    pub level: u8,
    /// +9.
    pub life: u16,
    /// +`$0B` (BCD in the ROM).
    pub exp: u16,
    /// +`$0D` & `$0FFF` (BCD in the ROM).
    pub gems: u16,
    /// +`$0E` bits 4-7: a drop needs `random & mask == 0`.
    pub drop_mask: u8,
    /// +`$0F` + 5k: attack of kind k (10 bits).
    pub attacks: [u16; 2],
    /// +`$11`.
    pub defense: u16,
    /// +`$13`.
    pub luck: u8,
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// A BCD word as a number.
fn decimal(bcd: u16) -> u16 {
    (0..4)
        .rev()
        .fold(0, |value, digit| value * 10 + ((bcd >> (digit * 4)) & 15))
}

impl Profile {
    /// Decodes a profile's 25 bytes.
    #[must_use]
    pub fn decode(bytes: &[u8; 25]) -> Self {
        Self {
            resist: word(bytes, 2),
            weak: word(bytes, 6),
            level: bytes[8],
            life: u16::from(bytes[9]) | u16::from(bytes[10]) << 8,
            exp: decimal(word(bytes, 0x0B)),
            gems: decimal(word(bytes, 0x0D) & 0x0FFF),
            drop_mask: bytes[0x0E] >> 4,
            attacks: [word(bytes, 0x0F) & 0x3FF, word(bytes, 0x14) & 0x3FF],
            defense: word(bytes, 0x11),
            luck: bytes[0x13],
        }
    }
}

/// Ark's attack kinds (`7F:102C`) and their types (`$066E`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 0.
    Thrust,
    /// 1, `$8400`.
    Rapid,
    /// 2, `$4400`.
    Jump,
    /// 3, `$1400`.
    Dash,
    /// 4, `$2400`.
    DashJump,
}

impl Kind {
    const fn types(self) -> u16 {
        match self {
            Self::Thrust => 0,
            Self::Rapid => 0x8400,
            Self::Jump => 0x4400,
            Self::Dash => 0x1400,
            Self::DashJump => 0x2400,
        }
    }
}

/// The game's randomness for one hit: `random & $7F` for the critical
/// roll, and the frame counter `$42` for the variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roll {
    /// `random & $7F`.
    pub critical: u8,
    /// `$42`.
    pub counter: u16,
}

/// A hit's damage, and whether it was critical (another digit colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Damage {
    /// Life taken.
    pub amount: u16,
    /// A critical hit.
    pub critical: bool,
}

/// The 2-bit factor a profile field holds for the highest type bit it
/// shares with `types` (bit 15 → bits 0-1, ... bit 12 → bits 6-7).
fn type_factor(field: u16, types: u16) -> Option<u16> {
    let common = (field & types & 0xF000) >> 12;
    // The highest common bit's place below bit 15: 0 for bit 15.
    let below = (0..4).find(|step| common & (8 >> step) != 0)?;
    Some((field >> (2 * below)) & 3)
}

/// `$42`'s variance: bit 3 set adds or takes a quarter (bit 2) or an
/// eighth, at least 1; bit 8 decides which.
fn variance(damage: u16, counter: u16) -> u16 {
    if counter & 8 == 0 {
        return damage;
    }
    let delta = (if counter & 4 == 0 {
        damage / 8
    } else {
        damage / 4
    })
    .max(1);
    if counter & 0x100 == 0 {
        damage + delta
    } else {
        damage.saturating_sub(delta).max(1)
    }
}

/// Ark's hit on an enemy (`$85:DBDF`..`DFF0`).
#[must_use]
pub fn ark_damage(stats: &Stats, kind: Kind, profile: &Profile, roll: Roll) -> Damage {
    let raw = ((u32::from(stats.level) + 7) * u32::from(stats.attack)) >> 3;
    let raw = raw + u32::from(stats.weapon);
    let defense = (u32::from(profile.level) + 11) * u32::from(profile.defense) / 2 / 6;
    let mut damage = if raw > defense {
        raw - defense
    } else {
        u32::from(stats.weapon >> 5) + 1
    };
    let quarter = damage / 4 + 1;
    damage = match kind {
        Kind::Thrust => damage,
        Kind::Rapid => damage.saturating_sub(quarter),
        Kind::Jump => damage + quarter,
        Kind::Dash => damage + 2 * quarter,
        Kind::DashJump => damage + damage / 2 + 2 * quarter,
    };
    let types = kind.types() & 0xFBFF;
    match type_factor(profile.resist, types) {
        Some(0) => damage = 0,
        Some(1) => damage /= 2,
        Some(_) => damage /= 4,
        None => {}
    }
    match type_factor(profile.weak, types) {
        Some(1) => damage = damage * 3 / 2,
        Some(2 | 3) => damage *= 2,
        _ => {}
    }
    let chance = (i32::from(stats.luck) + 8 - i32::from(profile.luck)).max(4);
    let critical = i32::from(roll.critical) < chance;
    if critical {
        damage *= 2;
    }
    let damage = u16::try_from(damage).unwrap_or(u16::MAX);
    let amount = if damage == 0 {
        0
    } else {
        variance(damage, roll.counter)
    };
    Damage {
        amount: amount.min(9999),
        critical,
    }
}

/// An enemy's hit on Ark with its attack of kind `kind` (`$85:D30C`..).
#[must_use]
pub fn enemy_damage(profile: &Profile, kind: usize, stats: &Stats, counter: u16) -> u16 {
    let attack = u32::from(profile.attacks.get(kind).copied().unwrap_or(0));
    let raw = ((u32::from(profile.level) + 7) * attack) >> 3;
    let defense =
        u32::from(stats.armor) + (u32::from(stats.level) + 7) * u32::from(stats.defense) / 4 / 3;
    let damage = if raw > defense {
        raw - defense
    } else {
        u32::from(profile.level >> 2) + 1
    };
    variance(u16::try_from(damage).unwrap_or(u16::MAX), counter).min(9999)
}

/// The stat table's pointers (`$8D:BDFA`, European `$8D:BCC3`), into bank
/// `$8D`; entry 0 is Ark's own block.
const PROFILES: usize = 0x0D_BDFA;

/// Profile `index` (`COP D9`, a descriptor's byte 4), not Ark's.
#[must_use]
pub fn profile(image: &[u8], index: u8) -> Option<Profile> {
    let index = index & 0x7F;
    if index == 0 {
        return None;
    }
    let table = assets::layout::per_revision(image, PROFILES, PROFILES - 0x137);
    let at = table + usize::from(index) * 2;
    let pointer = image.get(at..at + 2)?;
    let start = 0x0D_0000 | usize::from(u16::from_le_bytes([pointer[0], pointer[1]]));
    let bytes: &[u8; 25] = image.get(start..start + 25)?.try_into().ok()?;
    Some(Profile::decode(bytes))
}

/// One level of the table `$8D:BA61` (EU `$8D:B92A`): the EXP it needs and
/// the base stats it brings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// EXP needed.
    pub exp: u32,
    /// Base life.
    pub life: u16,
    /// Base attack.
    pub attack: u16,
    /// Base defense.
    pub defense: u16,
    /// Base luck.
    pub luck: u8,
}

/// Decodes the level table's entry `level` (1-based), 11 bytes each.
#[must_use]
pub fn level(table: &[u8], level: u8) -> Option<Level> {
    let at = usize::from(level.checked_sub(1)?) * 11;
    let entry = table.get(at..at + 11)?;
    let bcd = |byte: u8| u32::from(byte >> 4) * 10 + u32::from(byte & 15);
    Some(Level {
        exp: bcd(entry[2]) * 10000 + bcd(entry[1]) * 100 + bcd(entry[0]),
        life: word(entry, 3),
        attack: word(entry, 5),
        defense: word(entry, 7),
        luck: entry[9],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The blob (`$8D:BEC0`, `docs/combat.md`).
    const BLOB: [u8; 25] = [
        0x00, 0x00, 0x02, 0x84, 0x00, 0x00, 0x04, 0x44, 0x01, 0x04, 0x00, 0x02, 0x00, 0x03, 0x10,
        0x04, 0x00, 0x02, 0x00, 0x05, 0x04, 0x68, 0x00, 0x00, 0x05,
    ];
    const QUIET: Roll = Roll {
        critical: 0x7F,
        counter: 0,
    };

    #[test]
    fn the_blob_profile_decodes() {
        let blob = Profile::decode(&BLOB);
        assert_eq!(
            (blob.level, blob.life, blob.exp, blob.gems, blob.drop_mask),
            (1, 4, 2, 3, 1)
        );
        assert_eq!((blob.attacks[0], blob.defense, blob.luck), (4, 2, 5));
    }

    #[test]
    fn the_spear_takes_four_from_a_blob_and_its_types_count() {
        let blob = Profile::decode(&BLOB);
        let stats = Stats::start();
        let hit = |kind| ark_damage(&stats, kind, &blob, QUIET).amount;
        assert_eq!(hit(Kind::Thrust), 4, "EU measured 4");
        // Rapid: 4 - 2 = 2, then a quarter (resisted, field 2).
        assert_eq!(hit(Kind::Rapid), 0);
        // Jump: 4 + 2 = 6, then x1.5 (weak, field 1).
        assert_eq!(hit(Kind::Jump), 9);
        // JP measured 3: `$42 = $FB78` takes an eighth, at least 1.
        let roll = Roll {
            critical: 0x7F,
            counter: 0xFB78,
        };
        assert_eq!(ark_damage(&stats, Kind::Thrust, &blob, roll).amount, 3);
        let critical = Roll {
            critical: 0,
            counter: 0,
        };
        assert_eq!(
            ark_damage(&stats, Kind::Thrust, &blob, critical),
            Damage {
                amount: 8,
                critical: true
            }
        );
    }

    #[test]
    fn a_blob_takes_three_from_ark() {
        let blob = Profile::decode(&BLOB);
        assert_eq!(enemy_damage(&blob, 0, &Stats::start(), 0), 3);
        // Measured 2: the variance takes one.
        assert_eq!(enemy_damage(&blob, 0, &Stats::start(), 0x0108), 2);
    }

    #[test]
    fn levels_decode_from_the_table() {
        let mut table = vec![0u8; 22];
        table[11..22].copy_from_slice(&[0x38, 0, 0, 33, 0, 4, 0, 3, 0, 4, 0]);
        assert_eq!(
            level(&table, 2),
            Some(Level {
                exp: 38,
                life: 33,
                attack: 4,
                defense: 3,
                luck: 4
            })
        );
    }
}
