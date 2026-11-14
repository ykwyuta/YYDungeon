pub struct Adventurer {
    pub name: &'static str,
    pub health: i32,
    pub retaliation: u16,
    pub trap_armor: i32,
    pub resist: u8,
    pub bounty: u32,
}

impl Adventurer {
    const fn new(
        name: &'static str,
        health: i32,
        retaliation: u16,
        trap_armor: i32,
        resist: u8,
        bounty: u32,
    ) -> Self {
        Self {
            name,
            health,
            retaliation,
            trap_armor,
            resist,
            bounty,
        }
    }
}

// Indices 0..=2 also identify the three adventurers in old saves.
pub const ADVENTURERS: [Adventurer; 12] = [
    Adventurer::new("チンピラ", 0, 0, 0, 0, 0),
    Adventurer::new("盗賊", 4, 0, 0, 0, 0),
    Adventurer::new("猟師", 8, 0, 0, 0, 0),
    Adventurer::new("見習い冒険者", 6, 1, 0, 0, 2),
    Adventurer::new("傭兵", 12, 3, 0, 0, 4),
    Adventurer::new("巡礼僧", 5, 0, 1, 1, 4),
    Adventurer::new("騎士", 16, 3, 2, 0, 6),
    Adventurer::new("魔術師", 4, 2, 0, 1, 6),
    Adventurer::new("レンジャー", 10, 2, 3, 0, 7),
    Adventurer::new("聖騎士", 20, 4, 2, 1, 9),
    Adventurer::new("剣聖", 14, 7, 0, 0, 10),
    Adventurer::new("勇者", 24, 5, 2, 1, 14),
];

pub const MAX_RAID_LEVEL: u32 = 1_000_000_000;
pub const MAX_ADVENTURER_HP: i32 = 600;

pub fn combat_power(level: u32) -> u32 {
    level.min(12) + level.saturating_sub(12).min(240) / 4
}

pub fn strongest_class(level: u32) -> usize {
    match level {
        0..=3 => 2,
        4..=8 => 5,
        9..=15 => 8,
        _ => 11,
    }
}

pub fn raid_class(level: u32, wave: u32, index: usize, leader: bool) -> u8 {
    let strongest = strongest_class(level);
    if leader && strongest > 2 {
        return strongest as u8;
    }
    let candidates = [strongest - 2, strongest - 1, strongest, 0];
    let rotation = index + (wave - 1) as usize;
    if strongest == 2 {
        (rotation % 3) as u8
    } else {
        candidates[rotation % candidates.len()] as u8
    }
}

#[cfg(any(windows, target_arch = "wasm32"))]
pub const ADVENTURER_ART: [&[u8]; ADVENTURERS.len()] = [
    include_bytes!("../assets/adventurers/00.png"),
    include_bytes!("../assets/adventurers/01.png"),
    include_bytes!("../assets/adventurers/02.png"),
    include_bytes!("../assets/adventurers/03.png"),
    include_bytes!("../assets/adventurers/04.png"),
    include_bytes!("../assets/adventurers/05.png"),
    include_bytes!("../assets/adventurers/06.png"),
    include_bytes!("../assets/adventurers/07.png"),
    include_bytes!("../assets/adventurers/08.png"),
    include_bytes!("../assets/adventurers/09.png"),
    include_bytes!("../assets/adventurers/10.png"),
    include_bytes!("../assets/adventurers/11.png"),
];

#[cfg(any(windows, target_arch = "wasm32"))]
pub const DUEL_ADVENTURER_ART: [&[u8]; ADVENTURERS.len()] = [
    include_bytes!("../assets/duel-adventurers/00.png"),
    include_bytes!("../assets/duel-adventurers/01.png"),
    include_bytes!("../assets/duel-adventurers/02.png"),
    include_bytes!("../assets/duel-adventurers/03.png"),
    include_bytes!("../assets/duel-adventurers/04.png"),
    include_bytes!("../assets/duel-adventurers/05.png"),
    include_bytes!("../assets/duel-adventurers/06.png"),
    include_bytes!("../assets/duel-adventurers/07.png"),
    include_bytes!("../assets/duel-adventurers/08.png"),
    include_bytes!("../assets/duel-adventurers/09.png"),
    include_bytes!("../assets/duel-adventurers/10.png"),
    include_bytes!("../assets/duel-adventurers/11.png"),
];
