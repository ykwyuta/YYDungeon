use crate::adventurers::ADVENTURERS;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Claw,
    Frost,
    Guard,
    Storm,
}

impl Command {
    pub const ALL: [Self; 4] = [Self::Claw, Self::Frost, Self::Guard, Self::Storm];

    pub fn name(self) -> &'static str {
        match self {
            Self::Claw => "爪撃",
            Self::Frost => "氷の魔法",
            Self::Guard => "防御",
            Self::Storm => "極寒の嵐",
        }
    }

    pub fn mp(self) -> u8 {
        match self {
            Self::Claw | Self::Guard => 0,
            Self::Frost => 3,
            Self::Storm => 6,
        }
    }

    pub fn level(self) -> u32 {
        match self {
            Self::Storm => 4,
            _ => 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Duel {
    pub class: u8,
    pub level: u8,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u8,
    pub max_mp: u8,
    pub boss_mp: u8,
    pub turn: u32,
    pub lines: Vec<String>,
    pub outcome: Option<Outcome>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Victory,
    Fled,
    Defeat,
}

impl Duel {
    pub fn new(class: u8, level: u8, hp: u32, max_hp: u32, boss_level: u32) -> Self {
        let name = ADVENTURERS[class as usize].name;
        let mp = if level >= 3 { 2 + level } else { 0 };
        Self {
            class,
            level,
            hp,
            max_hp,
            mp,
            max_mp: mp,
            boss_mp: boss_mp(boss_level),
            turn: 0,
            lines: vec![format!("Lv.{level} {name}が玉座に現れた！")],
            outcome: None,
        }
    }

    pub fn available(&self, command: Command, boss_level: u32) -> bool {
        self.outcome.is_none() && command.level() <= boss_level && command.mp() <= self.boss_mp
    }

    pub fn act(&mut self, command: Command, boss_level: u32, boss_hp: &mut u32, wave: u32) {
        if !self.available(command, boss_level) {
            return;
        }
        self.turn += 1;
        self.boss_mp -= command.mp();
        self.lines.clear();
        let enemy = &ADVENTURERS[self.class as usize];
        let damage = match command {
            Command::Claw => (10 + 3 * boss_level).saturating_sub(enemy.trap_armor as u32),
            Command::Frost => 16 + 4 * boss_level,
            Command::Storm => 24 + 5 * boss_level,
            Command::Guard => 0,
        };
        if command == Command::Guard {
            self.boss_mp = (self.boss_mp + 1).min(boss_mp(boss_level));
            self.lines.push("氷の主は身を守り、MPを1回復した。".into());
        } else {
            self.hp = self.hp.saturating_sub(damage);
            self.lines.push(format!(
                "氷の主の{}！　{}に{damage}ダメージ。",
                command.name(),
                enemy.name
            ));
        }
        if self.hp == 0 {
            self.outcome = Some(Outcome::Victory);
            self.lines.push(format!("{}を撃退した！", enemy.name));
            return;
        }
        let flee_chance = if self.class == 11 {
            14
        } else if self.class == 1 {
            55
        } else {
            30
        };
        let mut roll = wave
            ^ self.turn.wrapping_mul(0x9e3779b9)
            ^ (self.class as u32 * 1237)
            ^ self.hp.rotate_left(9);
        roll ^= roll >> 16;
        roll = roll.wrapping_mul(0x7feb352d);
        roll ^= roll >> 15;
        if self.hp * 3 <= self.max_hp && roll % 100 < flee_chance {
            self.outcome = Some(Outcome::Fled);
            self.lines.push(format!("{}は逃げ出した！", enemy.name));
            return;
        }
        let level = u32::from(self.level);
        let (attack, mut damage) = if self.level >= 7 && self.mp >= 5 && self.turn % 2 == 1 {
            self.mp -= 5;
            ("光雷の魔法", 10 + level * 2)
        } else if self.level >= 3 && self.mp >= 2 && self.turn % 2 == 1 {
            self.mp -= 2;
            ("火炎の魔法", 6 + level * 2)
        } else {
            let name = if self.level >= 5 { "連撃" } else { "攻撃" };
            (name, 4 + level + u32::from(enemy.retaliation))
        };
        if command == Command::Guard {
            damage = damage.div_ceil(2);
        }
        *boss_hp = boss_hp.saturating_sub(damage);
        self.lines.push(format!(
            "{}の{attack}！　氷の主に{damage}ダメージ。",
            enemy.name
        ));
        if *boss_hp == 0 {
            self.outcome = Some(Outcome::Defeat);
            self.lines.push("氷の主は倒れた。".into());
        }
    }

    pub fn valid(&self, boss_level: u32, boss_hp: u32) -> bool {
        (self.class as usize) < ADVENTURERS.len()
            && (1..=15).contains(&self.level)
            && (1..=180).contains(&self.max_hp)
            && self.hp <= self.max_hp
            && self.mp <= self.max_mp
            && self.max_mp <= 17
            && self.boss_mp <= boss_mp(boss_level)
            && self.turn <= 10000
            && self.lines.len() <= 3
            && (self.hp == 0) == (self.outcome == Some(Outcome::Victory))
            && (boss_hp == 0) == (self.outcome == Some(Outcome::Defeat))
    }
}

pub fn boss_mp(level: u32) -> u8 {
    (7 + level.min(10) * 2) as u8
}
