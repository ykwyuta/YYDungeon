use crate::duel::{Command, EnemyMove, Outcome, Turn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Idle,
    Attack,
    Spell,
    Special,
    Hurt,
    Guard,
    Down,
    Exit,
    Charge,
    Win,
}

impl Pose {
    pub const EXTRA_COUNT: usize = 9;

    pub fn extra_index(self) -> Option<usize> {
        match self {
            Self::Idle => None,
            _ => Some(self as usize - 1),
        }
    }
}

const IMPACT: f32 = 0.42;
const REPLY: f32 = 0.92;
const COUNTER_HIT: f32 = 1.24;
pub const TURN_TIME: f32 = 1.78;

pub struct Animation {
    pub turn: Turn,
    pub time: f32,
    pub old_boss_hp: u32,
    pub old_enemy_hp: u32,
}

impl Animation {
    pub fn poses(&self, outcome: Option<Outcome>) -> (Pose, Pose) {
        let boss = match self.turn.command {
            Command::Claw => Pose::Attack,
            Command::Frost => Pose::Spell,
            Command::Guard => Pose::Guard,
            Command::Storm if self.time < IMPACT => Pose::Charge,
            Command::Storm => Pose::Special,
        };
        if self.time < IMPACT {
            return (
                boss,
                if self.turn.command == Command::Storm {
                    Pose::Guard
                } else {
                    Pose::Idle
                },
            );
        }
        if self.time < REPLY {
            return (
                boss,
                if self.turn.command == Command::Guard {
                    Pose::Idle
                } else {
                    Pose::Hurt
                },
            );
        }
        if let Some(enemy) = self.turn.enemy_move {
            if self.time < COUNTER_HIT {
                let attack = match enemy {
                    EnemyMove::Attack => Pose::Attack,
                    EnemyMove::Spell => Pose::Spell,
                    EnemyMove::Special => Pose::Charge,
                };
                return (
                    if self.turn.command == Command::Guard {
                        Pose::Guard
                    } else {
                        Pose::Idle
                    },
                    attack,
                );
            }
            return (
                match outcome {
                    Some(Outcome::Defeat) => Pose::Down,
                    _ if self.turn.command == Command::Guard => Pose::Guard,
                    _ if self.time >= 1.60 => Pose::Exit,
                    _ => Pose::Hurt,
                },
                match (enemy, outcome) {
                    (_, Some(Outcome::Defeat)) if self.time >= 1.55 => Pose::Win,
                    (EnemyMove::Attack, _) => Pose::Attack,
                    (EnemyMove::Spell, _) => Pose::Spell,
                    (EnemyMove::Special, _) => Pose::Special,
                },
            );
        }
        match outcome {
            Some(Outcome::Victory) => (Pose::Win, Pose::Down),
            Some(Outcome::Fled) => (Pose::Idle, Pose::Exit),
            _ => (Pose::Idle, Pose::Idle),
        }
    }

    pub fn hp(&self, boss_hp: u32, enemy_hp: u32) -> (u32, u32) {
        (
            if self.time < COUNTER_HIT {
                self.old_boss_hp
            } else {
                boss_hp
            },
            if self.time < IMPACT {
                self.old_enemy_hp
            } else {
                enemy_hp
            },
        )
    }

    pub fn message_count(&self) -> usize {
        if self.time < IMPACT {
            0
        } else if self.time < REPLY || (self.turn.enemy_move.is_some() && self.time < COUNTER_HIT) {
            1
        } else {
            3
        }
    }
}

pub fn resting_poses(outcome: Option<Outcome>) -> (Pose, Pose) {
    match outcome {
        Some(Outcome::Victory) => (Pose::Win, Pose::Down),
        Some(Outcome::Fled) => (Pose::Idle, Pose::Exit),
        Some(Outcome::Defeat) => (Pose::Down, Pose::Win),
        None => (Pose::Idle, Pose::Idle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_moves_charge_hit_and_show_both_reactions_in_order() {
        let mut animation = Animation {
            turn: Turn {
                command: Command::Storm,
                enemy_move: Some(EnemyMove::Special),
            },
            time: 0.0,
            old_boss_hp: 100,
            old_enemy_hp: 80,
        };
        assert_eq!(animation.poses(None), (Pose::Charge, Pose::Guard));
        assert_eq!(animation.hp(70, 36), (100, 80));
        animation.time = IMPACT;
        assert_eq!(animation.poses(None), (Pose::Special, Pose::Hurt));
        assert_eq!(animation.hp(70, 36), (100, 36));
        animation.time = REPLY;
        assert_eq!(animation.poses(None), (Pose::Idle, Pose::Charge));
        animation.time = COUNTER_HIT;
        assert_eq!(animation.poses(None), (Pose::Hurt, Pose::Special));
        assert_eq!(animation.hp(70, 36), (70, 36));
    }
}
