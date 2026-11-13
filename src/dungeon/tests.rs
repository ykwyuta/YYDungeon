use super::*;

fn settle(game: &mut Game) {
    for _ in 0..2000 {
        if !game.running() {
            return;
        }
        game.tick(0.25);
    }
    panic!("raid did not settle");
}

#[test]
fn timer_starts_a_raid_and_the_original_defenses_pay_out() {
    let mut game = Game::new();
    for _ in 0..(PREPARE * 4.0) as usize {
        game.tick(0.25);
    }
    assert_eq!(game.wave, 1);
    assert!(game.running());
    settle(&mut game);
    assert!(!game.defeated);
    assert_eq!(game.kills, 3);
    assert!(game.bones >= 3 && game.loot >= 3 && game.gold > 170);
    assert_eq!(game.countdown, PREPARE);
}

#[test]
fn chest_count_and_rarity_change_the_forecast_and_reward() {
    let mut game = Game::new();
    assert_eq!(game.forecast(), (3, 1));
    assert!(game.place(Order::Chest, Cell { x: 1, y: 2 }, 1).is_err());
    game.place(Order::Chest, Cell { x: 1, y: 2 }, 0).unwrap();
    assert_eq!(game.forecast(), (5, 1));
    assert!(game.place(Order::Chest, Cell { x: 2, y: 2 }, 0).is_err());
    game.active().expand();
    game.loot = 5;
    game.place(Order::Chest, Cell { x: 1, y: 2 }, 1).unwrap();
    assert_eq!(game.forecast(), (5, 3));
    let gold = game.gold;
    game.reward(3, 0, 0);
    assert_eq!(game.gold - gold, 27);
}

#[test]
fn maze_expansion_and_delegation_keep_a_route_to_the_throne() {
    let mut game = Game::new();
    for _ in 0..20 {
        game.reward(1, 0, 0);
    }
    game.finish_raid();
    assert_eq!(game.floor().size(), (17, 13));
    assert_eq!(game.floor().rarity_limit(), 3);
    assert!(game.floor().route(ENTRY, Cell { x: 15, y: 11 }).is_some());
    assert!(game.floor().monster_limit() > 3 && game.floor().chest_limit() > 2);
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 3).unwrap();
    game.descend(true).unwrap();
    assert!(game.floors[0].steward);
    assert!(game.floor().route(ENTRY, THRONE).is_some());
    game.begin_raid();
    assert!(game.invaders()[0].level >= game.floors[0].pass_level());
    assert!(game.log.iter().any(|line| line.contains("素通し")));
    assert!(Game::load(&game.save()).is_some());
}

#[test]
fn a_steward_stops_weak_invaders_upstairs() {
    let mut game = Game::new();
    for _ in 0..20 {
        game.reward(1, 0, 0);
    }
    game.finish_raid();
    game.descend(false).unwrap();
    game.toggle_steward(0).unwrap();
    game.begin_raid();
    assert!(game.floors[0].earned() > 20);
    assert_eq!(game.floor().earned(), 0);
}

#[test]
fn defeat_ends_the_raid_and_mid_raid_saves_resume() {
    let mut game = Game::new();
    for site in &mut game.active().sites {
        *site = None;
    }
    game.boss = 1;
    game.begin_raid();
    for _ in 0..7 {
        game.tick(0.25);
    }
    let bytes = game.save();
    let mut loaded = Game::load(&bytes).unwrap();
    assert_eq!(bytes, loaded.save());
    settle(&mut loaded);
    assert!(loaded.defeated);
    assert_eq!(loaded.boss, 0);
    assert!(!loaded.running());
    assert_eq!(loaded.kills, 0);
    assert!(Game::load(b"{}").is_none());
}

#[test]
fn construction_checks_walls_resources_and_running_raids() {
    let mut game = Game::new();
    let cell = Cell { x: 3, y: 3 };
    assert!(game.place(Order::Spikes, cell, 0).is_err());
    assert!(game.place(Order::Dig, ENTRY, 0).is_err());
    assert!(game
        .place(Order::Recruit(SENTINEL), Cell { x: 1, y: 2 }, 0)
        .is_err());
    game.place(Order::Dig, cell, 0).unwrap();
    game.place(Order::Spikes, cell, 0).unwrap();
    game.begin_raid();
    assert!(game.place(Order::Remove, cell, 0).is_err());
}

#[test]
fn a_prepared_first_floor_can_earn_and_open_the_second_floor() {
    let mut game = Game::new();
    game.place(Order::Spikes, Cell { x: 2, y: 5 }, 0).unwrap();
    game.place(Order::Spikes, Cell { x: 3, y: 5 }, 0).unwrap();
    game.place(Order::Goblin, Cell { x: 4, y: 5 }, 0).unwrap();
    game.place(Order::Chest, Cell { x: 9, y: 7 }, 0).unwrap();
    for _ in 0..7 {
        game.begin_raid();
        settle(&mut game);
        assert!(!game.defeated);
        if game.can_descend() {
            game.descend(true).unwrap();
            assert_eq!(game.floors.len(), 2);
            assert!(game.floors[0].steward);
            assert!(game.kills >= 20);
            return;
        }
    }
    panic!("only {} adventurers defeated", game.kills);
}

#[test]
fn digging_branches_and_loops_unlocks_monsters_and_survives_old_saves() {
    let mut game = Game::new();
    let branch = Cell { x: 3, y: 3 };
    let room = Cell { x: 4, y: 3 };
    assert!(game.place(Order::Recruit(4), room, 0).is_err());
    game.place(Order::Dig, branch, 0).unwrap();
    assert!(
        game.development >= 3,
        "loop should be worth extra development"
    );
    game.place(Order::Recruit(4), room, 0).unwrap();
    assert_eq!(game.floor().site(room).unwrap().monster().unwrap().0, 4);
    let score = game.development;
    assert!(game.place(Order::Dig, branch, 0).is_err());
    assert_eq!(game.development, score);
    assert_eq!(Game::load(&game.save()).unwrap().development, score);

    game.active().sites[room.index()] = None;
    let mut legacy = serde_json::from_slice::<serde_json::Value>(&game.save()).unwrap();
    legacy[1].as_object_mut().unwrap().remove("development");
    let restored = Game::load(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(restored.development, score);
    assert_eq!(
        restored
            .floor()
            .site(Cell { x: 2, y: 4 })
            .unwrap()
            .monster(),
        Some((GOBLIN, 38))
    );
}

#[test]
fn all_monsters_are_earnable_on_a_developed_floor() {
    let mut game = Game::new();
    game.gold = 10000;
    game.active().expand();
    game.active().expand();
    for _ in 0..WIDTH {
        for y in 1..HEIGHT - 1 {
            for x in 1..WIDTH - 1 {
                let _ = game.place(Order::Dig, Cell { x, y }, 0);
            }
        }
    }
    assert!(MONSTERS
        .iter()
        .all(|monster| game.development >= monster.unlock));
}

#[test]
fn legendary_treasure_attracts_a_hero_and_the_raid_resumes_from_save() {
    let mut game = Game::new();
    game.begin_raid();
    let original = game.raid.as_ref().unwrap();
    assert_eq!(original.invaders[0].class, 0);
    assert_eq!(
        original.waiting.iter().copied().collect::<Vec<_>>(),
        [(1, 1), (1, 2)]
    );
    assert!(Game::load(&game.save()).is_some());

    let mut game = Game::new();
    game.active().expand();
    game.active().expand();
    game.loot = 3;
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 3).unwrap();
    game.begin_raid();
    for _ in 0..16 {
        game.tick(0.25);
    }
    let hero = game
        .invaders()
        .iter()
        .find(|invader| invader.class == 11)
        .unwrap();
    assert_eq!(ADVENTURERS[hero.class as usize].name, "勇者");
    assert_eq!(hero.level, 7);
    assert_eq!(hero.max_hp, 109);
    assert_eq!(Game::load(&game.save()).unwrap().save(), game.save());
    let before = game.gold;
    game.reward(7, 0, 11);
    assert_eq!(game.gold - before, 65);
}

#[test]
fn smaller_raids_rotate_through_each_rank_including_its_leader() {
    use crate::adventurers::strongest_class;
    for (level, expected) in [(1, 0..3), (3, 3..6), (5, 6..9), (7, 9..12)] {
        let mut encountered = [false; ADVENTURERS.len()];
        for wave in 1..=4 {
            for index in 0..3 {
                encountered[raid_class(level, wave, index, index == 2) as usize] = true;
            }
        }
        assert!(expected.clone().all(|class| encountered[class]));
        assert_eq!(strongest_class(level), expected.end - 1);
    }
}

#[test]
fn melee_paces_attacks_and_preserves_health_when_resuming_mid_fight() {
    let mut game = Game::new();
    let goblin = Cell { x: 2, y: 4 };
    game.begin_raid();
    while !game.invaders()[0].fighting() {
        game.tick(0.25);
    }
    let enemy_hp = game.invaders()[0].hp;
    assert_eq!(game.invaders()[0].cell, goblin);
    assert_eq!(
        game.floor().site(goblin).unwrap().monster(),
        Some((GOBLIN, 38))
    );
    let mut restored = Game::load(&game.save()).unwrap();

    for game in [&mut game, &mut restored] {
        game.tick(0.25);
        assert_eq!(game.invaders()[0].hp, enemy_hp);
        for _ in 0..2 {
            game.tick(0.25);
        }
        assert!(game.invaders()[0].fighting());
        assert_eq!(game.invaders()[0].cell, goblin);
        assert!(game.invaders()[0].hp < enemy_hp);
        assert!(game.floor().site(goblin).unwrap().monster().unwrap().1 < 38);
    }
    assert_eq!(game.save(), restored.save());
    settle(&mut restored);
    assert_eq!(restored.kills, 3);
}
