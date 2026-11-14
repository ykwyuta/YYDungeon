use super::*;

fn settle(game: &mut Game) {
    for _ in 0..2000 {
        if !game.running() {
            return;
        }
        if let Some(duel) = game.duel() {
            if duel.outcome.is_some() {
                game.dismiss_duel();
            } else {
                let level = game.boss_level();
                let command = [Command::Storm, Command::Frost, Command::Claw]
                    .into_iter()
                    .find(|&command| duel.available(command, level))
                    .unwrap();
                game.duel_command(command);
            }
        } else {
            game.tick(0.25);
        }
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
fn chest_count_controls_party_size_and_rarity_controls_only_rewards() {
    let mut game = Game::new();
    assert_eq!(game.forecast(), (3, 1));
    assert!(game.place(Order::Chest, Cell { x: 1, y: 2 }, 1).is_err());
    game.place(Order::Chest, Cell { x: 1, y: 1 }, 0).unwrap();
    assert_eq!(game.forecast(), (5, 1));
    assert!(game.place(Order::Chest, Cell { x: 2, y: 2 }, 0).is_err());
    game.active().expand();
    game.loot = 5;
    let mut plain = game.clone();
    game.place(Order::Chest, Cell { x: 1, y: 1 }, 1).unwrap();
    assert_eq!(game.forecast(), (5, 1));
    for workshop in [&mut game, &mut plain] {
        workshop.begin_raid();
    }
    assert_eq!(
        game.raid.as_ref().unwrap().waiting,
        plain.raid.as_ref().unwrap().waiting
    );
    game = Game::load(&game.save()).unwrap();
    settle(&mut game);
    settle(&mut plain);
    let better = game.last_report.unwrap();
    let normal = plain.last_report.unwrap();
    assert_eq!(better.stopped, normal.stopped);
    assert!(better.gold > normal.gold);
    assert!(better.loot > normal.loot);
    assert_eq!(better.bones, normal.bones);
}

#[test]
fn maze_expansion_and_delegation_keep_a_route_to_the_throne() {
    let mut game = Game::new();
    for _ in 0..20 {
        game.reward(1, 0, 0, 0);
    }
    game.finish_raid();
    assert_eq!(game.floor().size(), (17, 13));
    assert_eq!(game.floor().rarity_limit(), 3);
    assert!(game.floor().route(ENTRY, Cell { x: 15, y: 11 }).is_some());
    assert!(game.floor().monster_limit() > 3 && game.floor().chest_limit() > 2);
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 3).unwrap();
    game.boss_xp = boss_level_xp(3);
    game.descend(true).unwrap();
    game.active().expand();
    game.active().expand();
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 3).unwrap();
    assert!(game.floors[0].steward);
    assert!(game.floor().route(ENTRY, THRONE).is_some());
    game.wave = 10;
    game.begin_raid();
    assert!(game.invaders()[0].level >= game.floors[0].pass_level());
    assert!(game.log.iter().any(|line| line.contains("素通し")));
    assert!(Game::load(&game.save()).is_some());
}

#[test]
fn a_steward_stops_weak_invaders_upstairs() {
    let mut game = Game::new();
    for _ in 0..20 {
        game.reward(1, 0, 0, 0);
    }
    game.finish_raid();
    game.boss_xp = boss_level_xp(3);
    game.descend(false).unwrap();
    game.toggle_steward(0).unwrap();
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 0).unwrap();
    game.begin_raid();
    assert!(game.floors[0].earned() > 20);
    assert_eq!(game.floor().earned(), 0);
    assert!(game.running(), "the leader still reaches the active floor");
}

#[test]
fn defeat_ends_the_raid_and_mid_raid_saves_resume() {
    let mut game = Game::new();
    for site in &mut game.active().sites {
        *site = None;
    }
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 0).unwrap();
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
fn balance_training_requires_throne_wins_and_floor_twenty_is_reachable() {
    let chest = Cell { x: 10, y: 7 };
    let mut sealed = Game::new();
    sealed.active().sites[Cell { x: 1, y: 3 }.index()] = Some(Site::RankedTrap {
        frost: false,
        level: 30,
    });
    sealed.begin_raid();
    settle(&mut sealed);
    assert_eq!(sealed.last_report.as_ref().unwrap().throne, 0);
    assert_eq!(sealed.boss_level(), 1);
    assert!(!sealed.can_descend());

    let mut game = Game::new();
    // Leave the first wave to the garrison, then let some adventurers train the boss.
    game.begin_raid();
    settle(&mut game);
    game.place(Order::Remove, Cell { x: 1, y: 3 }, 0).unwrap();
    let mut unlocked_at = Vec::new();
    for _ in 0..350 {
        if game.floor().site(chest).is_none() {
            game.place(Order::Chest, chest, 0).unwrap();
        }
        while game.boss + 30 <= game.max_boss() && game.bones >= 2 {
            game.heal().unwrap();
        }
        game.begin_raid();
        settle(&mut game);
        let report = game.last_report.as_ref().unwrap();
        if game.wave <= 10 || game.can_descend() {
            eprintln!(
                "B{} wave {} Lv.{} HP {}/{} defeated {} fled {} XP +{}",
                game.floors.len(),
                game.wave,
                game.boss_level(),
                game.boss,
                game.max_boss(),
                report.stopped,
                report.fled,
                report.boss_xp
            );
        }
        assert!(!game.defeated, "wave {}, kills {}", game.wave, game.kills);
        // Reserve a replacement chest and dig a bounded set of branches each wave.
        for _ in 0..5 {
            if game.gold < Order::Chest.price() + Order::Dig.price() {
                break;
            }
            let (width, height) = game.floor().size();
            let next = (1..height - 1)
                .flat_map(|y| (1..width - 1).map(move |x| Cell { x, y }))
                .filter_map(|cell| {
                    game.floor()
                        .dig_development(cell)
                        .map(|score| (score, cell))
                })
                .max();
            let Some((_, cell)) = next else { break };
            game.place(Order::Dig, cell, 0).unwrap();
        }
        if game.can_descend() {
            unlocked_at.push(game.wave);
            game.descend(true).unwrap();
            if game.floors.len() == 20 {
                eprintln!("B2 through B20 unlocked on waves {unlocked_at:?}, development {}, supplies {}/{}/{}", game.development, game.gold, game.bones, game.loot);
                let dragon = &MONSTERS[39];
                assert!(game.monster_unlocked(dragon));
                assert!(
                    game.gold >= dragon.gold
                        && game.bones >= dragon.bones
                        && game.loot >= dragon.loot
                );
                return;
            }
        }
    }
    panic!("only reached B{}", game.floors.len());
}

#[test]
fn digging_branches_and_loops_unlocks_monsters_and_survives_old_saves() {
    let mut game = Game::new();
    game.boss_xp = boss_level_xp(3);
    game.descend(false).unwrap();
    let branch = Cell { x: 3, y: 2 };
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
    assert!(restored.development >= score);
    assert_eq!(
        restored.floors[0]
            .site(Cell { x: 1, y: 3 })
            .unwrap()
            .monster(),
        Some((GOBLIN, 38))
    );
}

#[test]
fn development_and_depth_gate_ranked_monsters_and_all_materials_are_charged() {
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
    assert!(game.development >= MONSTERS.last().unwrap().unlock);
    let cell = Cell { x: 4, y: 3 };
    for floor in 1..=20 {
        let unlocked = (2 * floor + 1).min(40);
        assert_eq!(
            MONSTERS.iter().filter(|m| game.monster_unlocked(m)).count(),
            unlocked
        );
        if floor < 20 {
            assert!(game.place(Order::Recruit(39), cell, 0).is_err());
            game.boss_xp = boss_level_xp(game.descent_level());
            game.descend(false).unwrap();
        }
    }
    let dragon = MONSTERS[39];
    assert!(dragon.hp >= 8 * MONSTERS[GOBLIN as usize].hp);
    assert!(dragon.gold >= 10 * Order::Recruit(GOBLIN).price());
    game.bones = dragon.bones;
    game.loot = dragon.loot - 1;
    assert!(game.place(Order::Recruit(39), cell, 0).is_err());
    game.loot += 1;
    let gold = game.gold;
    game.place(Order::Recruit(39), cell, 0).unwrap();
    assert_eq!(
        (game.gold, game.bones, game.loot),
        (gold - dragon.gold, 0, 0)
    );
    assert!(Game::load(&game.save()).is_some());
}

#[test]
fn a_hero_arrives_in_later_waves_even_with_plain_treasure() {
    let mut game = Game::new();
    game.wave = 15;
    game.begin_raid();
    for _ in 0..16 {
        game.tick(0.25);
    }
    let hero = game
        .invaders()
        .iter()
        .find(|invader| invader.class == 11)
        .unwrap();
    assert_eq!(hero.level, 16);
    assert_eq!(hero.max_hp, 115);
    assert_eq!(Game::load(&game.save()).unwrap().save(), game.save());
}

#[test]
fn smaller_raids_rotate_through_each_rank_including_its_leader() {
    use crate::adventurers::strongest_class;
    for (level, expected) in [(1, 0..3), (4, 3..6), (9, 6..9), (16, 9..12)] {
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
    let goblin = Cell { x: 1, y: 3 };
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

#[test]
fn opening_the_last_chest_stops_new_raids_until_it_is_replaced() {
    let mut game = Game::new();
    let chest = Cell { x: 10, y: 7 };
    for site in &mut game.active().sites {
        if !matches!(site, Some(Site::Chest { .. })) {
            *site = None;
        }
    }
    game.begin_raid();
    for _ in 0..150 {
        game.tick(0.25);
        if game.floor().site(chest).is_none() {
            break;
        }
    }
    assert_eq!(game.floor().site(chest), None);
    assert_eq!(game.forecast(), (0, 0));
    assert!(game.running());
    assert_eq!(
        game.invaders()
            .iter()
            .filter(|invader| invader.looted)
            .count(),
        1
    );

    let mut game = Game::load(&game.save()).unwrap();
    settle(&mut game);
    assert!(!game.defeated);
    let report = game.last_report.as_ref().unwrap();
    assert_eq!(report.stopped + report.fled, 3);
    assert_eq!(report.throne, 3);
    assert_eq!(report.boss_xp, game.boss_xp);
    assert!(report.boss_xp > 0);
    assert_eq!(report.chests, 1);
    assert_eq!(game.kills, report.stopped);
    assert_eq!(game.gold, 170 + RAID_RECOVERY_GOLD + report.gold);
    game.begin_raid();
    for _ in 0..(PREPARE * 4.0) as usize + 4 {
        game.tick(0.25);
    }
    assert_eq!(game.wave, 1);
    assert_eq!(game.countdown, PREPARE);
    assert!(!game.running());

    game.place(Order::Chest, chest, 0).unwrap();
    for _ in 0..(PREPARE * 4.0) as usize {
        game.tick(0.25);
    }
    assert_eq!(game.wave, 2);
    assert!(game.running());
}

#[test]
fn adventurers_retarget_an_unopened_chest_and_leave_upper_floors_out_of_the_forecast() {
    let mut game = Game::new();
    let first = Cell { x: 10, y: 7 };
    let second = Cell { x: 9, y: 7 };
    game.place(Order::Chest, second, 0).unwrap();
    game.begin_raid();
    let mut adventurer = game.invaders()[0].clone();
    adventurer.chest = Some(first);
    adventurer.route = vec![first];
    adventurer.cell = first;
    assert!(game.move_invader(&mut adventurer));
    assert!(adventurer.looted);
    assert!(game.floor().site(first).is_none());
    assert!(matches!(
        game.floor().site(second),
        Some(Site::Chest { .. })
    ));

    adventurer.looted = false;
    adventurer.chest = Some(first);
    adventurer.route = vec![first];
    assert!(game.move_invader(&mut adventurer));
    assert!(!adventurer.looted);
    assert_eq!(adventurer.chest, Some(second));
    assert_eq!(adventurer.route.last(), Some(&second));

    let new_floor = Floor::new(1);
    game.floors.push(new_floor);
    assert_eq!(game.forecast(), (0, 0));
}

#[test]
fn built_rocks_can_be_removed_at_a_cost_without_farming_development() {
    let mut game = Game::new();
    let rock = Cell { x: 1, y: 1 };
    let gold = game.gold;
    let score = game.development;

    assert!(game.place(Order::Wall, ENTRY, 0).is_err());
    assert!(game.place(Order::Wall, THRONE, 0).is_err());
    assert!(game.place(Order::Wall, Cell { x: 10, y: 7 }, 0).is_err());
    // Remove the entrance defender to test the reachability guard on empty floor.
    assert!(game.place(Order::Wall, Cell { x: 1, y: 3 }, 0).is_err());
    assert_eq!(game.gold, gold);
    game.active().sites[Cell { x: 1, y: 3 }.index()] = None;
    assert!(game.place(Order::Wall, Cell { x: 1, y: 3 }, 0).is_err());
    assert_eq!(game.gold, gold);
    assert!(game.floor().open(Cell { x: 1, y: 3 }));

    game.place(Order::Wall, rock, 0).unwrap();
    assert!(!game.floor().open(rock));
    assert_eq!(game.floor().dig_development(rock), None);
    assert_eq!(game.gold, gold - Order::Wall.price());
    let mut game = Game::load(&game.save()).unwrap();
    assert!(game.floor().has_placed_rock(rock));
    assert!(game.place(Order::Dig, rock, 0).is_err());
    let removal_budget = game.gold;
    game.gold = Order::Remove.price() - 1;
    assert!(game.place(Order::Remove, rock, 0).is_err());
    assert!(game.floor().has_placed_rock(rock));
    game.gold = removal_budget;
    game.place(Order::Remove, rock, 0).unwrap();
    assert_eq!(game.gold, removal_budget - Order::Remove.price());
    assert!(!game.floor().has_placed_rock(rock));
    assert!(game.floor().open(rock));
    assert_eq!(game.development, score);
}

#[test]
fn adventurers_choose_safe_detours_but_can_cross_unavoidable_defenses() {
    let mut floor = Floor::new(0);
    floor.open.fill(false);
    let start = Cell { x: 2, y: 2 };
    let goal = Cell { x: 5, y: 2 };
    let trap = Cell { x: 3, y: 2 };
    let monster = Cell { x: 4, y: 2 };
    for y in 2..=3 {
        for x in 2..=5 {
            floor.open[Cell { x, y }.index()] = true;
        }
    }
    floor.sites[trap.index()] = Some(Site::Spikes);
    floor.sites[monster.index()] = Some(Site::Goblin { hp: 38 });
    let safe = floor.adventurer_route(start, goal).unwrap();
    assert_eq!(floor.route(start, goal).unwrap().len(), 4);
    assert_eq!(safe.len(), 6);
    assert!(!safe.contains(&trap) && !safe.contains(&monster));

    floor.open[Cell { x: 3, y: 3 }.index()] = false;
    assert!(floor.adventurer_route(start, goal).unwrap().contains(&trap));
    floor.sites[monster.index()] = Some(Site::Goblin { hp: 0 });
    assert_eq!(
        floor.adventurer_route(trap, goal).unwrap(),
        [trap, monster, goal]
    );
}

#[test]
fn legacy_save_preserves_boss_level_and_missing_hp() {
    let mut game = Game::new();
    game.floors.push(Floor::new(1));
    game.active().expand();
    let legacy_max = game.max_boss();
    game.boss = legacy_max - 20;
    let mut legacy = serde_json::from_slice::<serde_json::Value>(&game.save()).unwrap();
    legacy[1].as_object_mut().unwrap().remove("boss_xp");
    let saved = Game::load(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(saved.boss_level(), 4);
    assert_eq!(saved.max_boss() - saved.boss, 20);
}

#[test]
fn only_boss_level_unlocks_each_new_floor_and_wave_pressure_is_global() {
    let mut game = Game::new();
    game.wave = 80;
    assert_eq!(game.forecast(), (3, 81));
    assert!(game.descend(false).is_err());
    game.boss_xp = boss_level_xp(3);
    game.gold = 0;
    assert_eq!(game.development, 0);
    assert!(Game::load(&game.save()).unwrap().can_descend());
    game.descend(false).unwrap();
    assert_eq!(game.boss_level(), 3);
    assert_eq!(game.gold, 0);
    assert_eq!(game.floors[0].tier, 1);
    game.place(Order::Chest, Cell { x: 10, y: 7 }, 0).unwrap();
    assert_eq!(game.forecast(), (3, 81));
    assert!(game.descend(false).is_err());
    game.boss_xp = boss_level_xp(5);
    assert!(game.can_descend());
}

#[test]
fn throne_duels_pause_the_maze_and_resume_after_a_saved_turn() {
    let mut game = Game::new();
    game.active().sites.iter_mut().for_each(|site| {
        if !matches!(site, Some(Site::Chest { .. })) {
            *site = None;
        }
    });
    game.begin_raid();
    for _ in 0..200 {
        game.tick(0.25);
        if game.duel().is_some() {
            break;
        }
    }
    assert!(game.duel().is_some());
    let before = game.save();
    game.tick(0.25);
    assert_eq!(before, game.save());
    game.duel_command(Command::Storm);
    assert_eq!(before, game.save(), "storm is locked at boss level one");
    game.duel_command(Command::Guard);
    assert_eq!(game.duel().unwrap().turn, 1);
    let bytes = game.save();
    let mut restored = Game::load(&bytes).unwrap();
    assert_eq!(restored.save(), bytes);
    settle(&mut restored);
    let report = restored.last_report.as_ref().unwrap();
    assert_eq!(report.throne, 3);
    assert_eq!(report.boss_xp, restored.boss_xp);
    assert!(report.boss_xp > 0);
    assert_eq!(report.stopped + report.fled, 3);
}

#[test]
fn advanced_duelists_use_stronger_magic_and_injured_adventurers_can_flee() {
    let mut hp = 110;
    let mut duel = Duel::new(11, 7, 100, 100, 4);
    duel.act(Command::Storm, 4, &mut hp, 1);
    assert_eq!(duel.hp, 56);
    assert_eq!(duel.mp, 4);
    assert!(duel.lines.iter().any(|line| line.contains("光雷の魔法")));
    assert!(hp < 110);

    let escaped = (1..=32).any(|wave| {
        let mut duel = Duel::new(1, 1, 2, 40, 1);
        duel.act(Command::Guard, 1, &mut 110, wave);
        duel.outcome == Some(Outcome::Fled)
    });
    assert!(escaped);
    let mut game = Game::new();
    game.begin_raid();
    let mut fled = Duel::new(1, 1, 2, 40, 1);
    fled.outcome = Some(Outcome::Fled);
    game.raid.as_mut().unwrap().duel = Some(fled);
    game.dismiss_duel();
    assert_eq!(game.boss_experience(), (0, 10));
}

#[test]
fn a_broke_workshop_can_reopen_without_selling_subsidized_chests_for_profit() {
    let mut game = Game::new();
    let chest = Cell { x: 10, y: 7 };
    game.place(Order::Remove, chest, 0).unwrap();
    game.gold = 0;
    assert_eq!(game.chest_cost(0), (0, 0));
    game.place(Order::Chest, chest, 0).unwrap();
    assert_eq!(game.forecast(), (3, 1));
    assert_eq!(game.chest_cost(0), (20, 0));
    assert!(game.place(Order::Remove, chest, 0).is_err());
    assert_eq!(game.gold, 0);
    assert_eq!(game.forecast(), (3, 1));
}

#[test]
fn mp_carries_between_duels_after_saving_and_guard_recovers_it() {
    let mut game = Game::new();
    for site in &mut game.active().sites {
        if !matches!(site, Some(Site::Chest { .. })) {
            *site = None;
        }
    }
    game.begin_raid();
    for _ in 0..200 {
        game.tick(0.25);
        if game.duel().is_some() {
            break;
        }
    }
    let max_mp = boss_mp(game.boss_level());
    game.duel_command(Command::Frost).unwrap();
    if game.duel().unwrap().outcome.is_none() {
        game.duel_command(Command::Frost).unwrap();
    }
    let spent = max_mp - game.duel().unwrap().boss_mp;
    assert!(spent >= 3);
    game.dismiss_duel();
    game = Game::load(&game.save()).unwrap();
    for _ in 0..200 {
        game.tick(0.25);
        if game.duel().is_some() {
            break;
        }
    }
    assert_eq!(
        game.duel().unwrap().boss_mp,
        max_mp - spent + DUEL_MP_RECOVERY
    );
    game.duel_command(Command::Guard).unwrap();
    assert_eq!(
        game.duel().unwrap().boss_mp,
        (max_mp - spent + 4).min(boss_mp(game.boss_level()))
    );
}

#[test]
fn the_throne_intercepts_adventurers_on_the_way_to_a_chest() {
    let approach = Cell { x: 10, y: 5 };
    let chest = Cell { x: 11, y: 6 };
    for (step, cell) in [(0, approach), (1, THRONE)] {
        let mut game = Game::new();
        game.place(Order::Chest, chest, 0).unwrap();
        game.begin_raid();
        let raid = game.raid.as_mut().unwrap();
        let invader = &mut raid.invaders[0];
        invader.chest = Some(chest);
        invader.route = vec![approach, THRONE, chest];
        invader.step = step;
        invader.cell = cell;
        raid.waiting.clear();

        let mut game = Game::load(&game.save()).unwrap();
        game.tick(0.25);
        game.tick(0.25);

        assert!(game.duel().is_some());
        assert_eq!(game.floor().site(chest), Some(Site::Chest { rarity: 0 }));
        assert_eq!(game.raid.as_ref().unwrap().tally.throne, 1);
        assert_eq!(game.raid.as_ref().unwrap().tally.chests, 0);
    }
}

#[test]
fn each_trap_type_keeps_its_level_and_price_progression_after_removal_and_reload() {
    let mut game = Game::new();
    game.bones = 2;
    let cell = Cell { x: 1, y: 1 };
    for order in [Order::Spikes, Order::Frost] {
        assert_eq!(game.next_trap_level(order), Some(1));
        let initial_price = game.construction_price(order);
        game.place(order, cell, 0).unwrap();
        assert_eq!(game.floor().site(cell).unwrap().trap_level(), Some(1));
        assert_eq!(game.next_trap_level(order), Some(2));
        assert!(game.construction_price(order) > initial_price);
        game.place(Order::Remove, cell, 0).unwrap();
        game = Game::load(&game.save()).unwrap();
        assert_eq!(game.next_trap_level(order), Some(2));
        let price = game.construction_price(order);
        let gold = game.gold;
        game.place(order, cell, 0).unwrap();
        assert_eq!(game.gold, gold - price);
        assert_eq!(game.floor().site(cell).unwrap().trap_level(), Some(2));
        game.place(Order::Remove, cell, 0).unwrap();
    }
}
