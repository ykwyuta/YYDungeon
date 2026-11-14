use crate::adventurers::{
    combat_power, raid_class, ADVENTURERS, MAX_ADVENTURER_HP, MAX_RAID_LEVEL,
};
use crate::duel::{boss_mp, Command, Duel, Outcome, Turn};
use crate::monsters::{Monster, Tactic, GOBLIN, MONSTERS, SENTINEL};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

pub const WIDTH: usize = 17;
pub const HEIGHT: usize = 13;
pub const ENTRY: Cell = Cell { x: 0, y: 3 };
pub const THRONE: Cell = Cell { x: 11, y: 5 };
pub const PREPARE: f32 = 42.0;
pub const MAX_FLOORS: usize = 100;
pub const REWARD_PERCENT: [u32; 4] = [100, 135, 175, 225];
const MAX_BOSS_XP: u32 = 1_000_000_000;

const RAID_RECOVERY_GOLD: u32 = 10;
const STEP: f32 = 0.33;
const COMBAT_ROUNDS: u8 = 3;
const COMBAT_INTERVAL: u8 = 2;
const SPIKE_DAMAGE: i32 = 7;
const FROST_DAMAGE: i32 = 5;
const MAX_TRAP_LEVEL: u32 = 100_000;
const RAID_MAX: usize = 32;
const MIN_ACTIVE_INVADERS: usize = 2;
const DUEL_MP_RECOVERY: u8 = 2;

// Inverted by Game::boss_level.
fn boss_level_xp(level: u32) -> u32 {
    2 * (level - 1) * (level + 3)
}

fn missing_progress() -> u32 {
    u32::MAX
}
pub const SAVE_LIMIT: usize = 300_000;
const INITIAL: [&str; 9] = [
    "#############",
    "#.....#.....#",
    "#.###.#.###.#",
    "...#.....#..#",
    "##.###.#.#.##",
    "#....#.#.#..#",
    "#.##.#.#.##.#",
    "#...........#",
    "#############",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}

impl Cell {
    fn index(self) -> usize {
        self.y * WIDTH + self.x
    }

    fn neighbors(self) -> impl Iterator<Item = Self> {
        let x = self.x;
        let y = self.y;
        [
            x.checked_sub(1).map(|x| Self { x, y }),
            (x + 1 < WIDTH).then_some(Self { x: x + 1, y }),
            y.checked_sub(1).map(|y| Self { x, y }),
            (y + 1 < HEIGHT).then_some(Self { x, y: y + 1 }),
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Site {
    Chest { rarity: u8 },
    Goblin { hp: u16 },
    Sentinel { hp: u16 },
    Monster { id: u8, hp: u16 },
    Spikes,
    Frost,
    RankedTrap { frost: bool, level: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Inspect,
    Chest,
    Bestiary,
    Recruit(u8),
    Spikes,
    Frost,
    Dig,
    Remove,
    Wall,
}

impl Order {
    pub const ALL: [Self; 7] = [
        Self::Chest,
        Self::Bestiary,
        Self::Spikes,
        Self::Frost,
        Self::Dig,
        Self::Remove,
        Self::Wall,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Inspect => "調べる",
            Self::Chest => "宝箱",
            Self::Bestiary => "仲間モンスター図鑑",
            Self::Recruit(id) => MONSTERS[id as usize].name,
            Self::Spikes => "針の罠",
            Self::Frost => "氷結の罠",
            Self::Dig => "掘削",
            Self::Remove => "撤去",
            Self::Wall => "岩壁を置く",
        }
    }

    pub fn price(self) -> u32 {
        match self {
            Self::Inspect => 0,
            Self::Remove => 8,
            Self::Chest => 20,
            Self::Bestiary => 0,
            Self::Recruit(id) => MONSTERS[id as usize].gold,
            Self::Spikes => 19,
            Self::Frost => 38,
            Self::Dig => 14,
            Self::Wall => 10,
        }
    }

    pub fn bones(self) -> u32 {
        match self {
            Self::Recruit(id) => MONSTERS[id as usize].bones,
            Self::Frost => 1,
            _ => 0,
        }
    }

    pub fn loot(self) -> u32 {
        match self {
            Self::Recruit(id) => MONSTERS[id as usize].loot,
            _ => 0,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Inspect => "床や壁を選ぶと、その区画の情報を確認できます。",
            Self::Chest => "冒険者が開けると宝箱は消えます。宝箱がある間は襲撃が来ます。",
            Self::Bestiary => "迷路を掘って、新しい魔物を仲間にできます。",
            Self::Recruit(_) => "空いている通路を選ぶと、仲間を配置します。",
            Self::Spikes => "ダメージを与えます。設置するたび次の針の罠が強くなります。",
            Self::Frost => "ダメージを与えて足止めします。設置するたび次の氷結の罠が強くなります。",
            Self::Dig => "通路に接した内壁を掘り、迷路に新しい道を作ります。",
            Self::Remove => "選んだ設置物や岩壁を金貨8で撤去します。",
            Self::Wall => "空き床を岩でふさぎます。置いた岩は撤去で取り除けます。",
        }
    }

    fn site(self, rarity: u8, trap_level: u32) -> Option<Site> {
        Some(match self {
            Self::Chest => Site::Chest { rarity },
            Self::Recruit(id) => Site::Monster {
                id,
                hp: MONSTERS.get(id as usize)?.hp,
            },
            Self::Spikes | Self::Frost => Site::RankedTrap {
                frost: self == Self::Frost,
                level: trap_level,
            },
            _ => return None,
        })
    }
}

impl Site {
    pub fn order(self) -> Order {
        match self {
            Self::Chest { .. } => Order::Chest,
            Self::Goblin { .. } => Order::Recruit(GOBLIN),
            Self::Sentinel { .. } => Order::Recruit(SENTINEL),
            Self::Monster { id, .. } => Order::Recruit(id),
            Self::Spikes | Self::RankedTrap { frost: false, .. } => Order::Spikes,
            Self::Frost | Self::RankedTrap { frost: true, .. } => Order::Frost,
        }
    }

    pub fn monster(self) -> Option<(u8, u16)> {
        match self {
            Self::Goblin { hp } => Some((GOBLIN, hp)),
            Self::Sentinel { hp } => Some((SENTINEL, hp)),
            Self::Monster { id, hp } => Some((id, hp)),
            _ => None,
        }
    }

    fn is_monster(self) -> bool {
        self.monster().is_some()
    }

    fn monster_hp_mut(&mut self) -> Option<(&mut u16, Monster)> {
        let id = self.monster()?.0;
        let hp = match self {
            Self::Goblin { hp } | Self::Sentinel { hp } | Self::Monster { hp, .. } => hp,
            _ => return None,
        };
        Some((hp, MONSTERS[id as usize]))
    }

    fn is_trap(self) -> bool {
        self.trap_level().is_some()
    }

    pub fn trap_level(self) -> Option<u32> {
        match self {
            Self::Spikes | Self::Frost => Some(1),
            Self::RankedTrap { level, .. } => Some(level),
            _ => None,
        }
    }
}

pub const RARITIES: [&str; 4] = ["並品", "上物", "希少", "伝説"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Floor {
    pub tier: u8,
    earned: u32,
    open: Vec<bool>,
    #[serde(default)]
    placed_rocks: Vec<Cell>,
    pub sites: Vec<Option<Site>>,
    pub steward: bool,
}

impl Floor {
    fn new(depth: usize) -> Self {
        let mut floor = Self {
            tier: 1,
            earned: 0,
            open: vec![false; WIDTH * HEIGHT],
            placed_rocks: Vec::new(),
            sites: vec![None; WIDTH * HEIGHT],
            steward: false,
        };
        for (y, row) in INITIAL.iter().enumerate() {
            for (x, tile) in row.bytes().enumerate() {
                floor.open[Cell { x, y }.index()] = tile == b'.';
            }
        }
        if depth % 2 == 1 {
            // Alternate shortcuts and branches while keeping the entrance and throne connected.
            for cell in [
                Cell { x: 3, y: 3 },
                Cell { x: 9, y: 5 },
                Cell { x: 6, y: 7 },
            ] {
                floor.open[cell.index()] = !floor.open[cell.index()];
            }
        }
        floor
    }

    pub fn size(&self) -> (usize, usize) {
        match self.tier {
            1 => (13, 9),
            2 => (15, 11),
            _ => (WIDTH, HEIGHT),
        }
    }

    pub fn contains(&self, cell: Cell) -> bool {
        let (w, h) = self.size();
        cell.x < w && cell.y < h
    }

    pub fn open(&self, cell: Cell) -> bool {
        self.contains(cell) && self.open[cell.index()]
    }

    pub fn has_placed_rock(&self, cell: Cell) -> bool {
        self.placed_rocks.contains(&cell)
    }

    pub fn site(&self, cell: Cell) -> Option<Site> {
        self.contains(cell)
            .then(|| self.sites[cell.index()])
            .flatten()
    }

    pub fn chest_count(&self) -> usize {
        self.sites
            .iter()
            .flatten()
            .filter(|site| matches!(site, Site::Chest { .. }))
            .count()
    }

    pub fn monster_count(&self) -> usize {
        self.sites
            .iter()
            .flatten()
            .filter(|site| site.is_monster())
            .count()
    }

    pub fn trap_count(&self) -> usize {
        self.sites
            .iter()
            .flatten()
            .filter(|site| site.is_trap())
            .count()
    }

    pub fn chest_limit(&self) -> usize {
        1 + self.tier as usize
    }

    pub fn monster_limit(&self) -> usize {
        1 + 2 * self.tier as usize
    }

    pub fn trap_limit(&self) -> usize {
        2 + 3 * self.tier as usize
    }

    pub fn rarity_limit(&self) -> u8 {
        if self.tier == 3 {
            3
        } else {
            self.tier - 1
        }
    }

    fn route(&self, start: Cell, goal: Cell) -> Option<Vec<Cell>> {
        self.find_route(start, goal, false)
    }

    fn adventurer_route(&self, start: Cell, goal: Cell) -> Option<Vec<Cell>> {
        self.find_route(start, goal, true)
    }

    fn find_route(&self, start: Cell, goal: Cell, avoid_danger: bool) -> Option<Vec<Cell>> {
        if !self.open(start) || !self.open(goal) {
            return None;
        }
        let mut parents = [None; WIDTH * HEIGHT];
        let mut costs = [(u16::MAX, u16::MAX); WIDTH * HEIGHT];
        let mut queue = BinaryHeap::new();
        queue.push(Reverse(((0, 0), 0, start)));
        costs[start.index()] = (0, 0);
        let mut sequence = 0;
        while let Some(Reverse((cost, _, cell))) = queue.pop() {
            if costs[cell.index()] != cost {
                continue;
            }
            if cell == goal {
                let mut route = vec![goal];
                let mut previous = goal;
                while previous != start {
                    previous = parents[previous.index()]?;
                    route.push(previous);
                }
                route.reverse();
                return Some(route);
            }
            for next in cell.neighbors().filter(|&next| self.open(next)) {
                let guarded = avoid_danger
                    && self.site(next).is_some_and(|site| {
                        site.is_trap() || site.monster().is_some_and(|(_, hp)| hp > 0)
                    });
                // Prefer fewer hazardous tiles, then the shortest route with that risk.
                let cost = (cost.0 + u16::from(guarded), cost.1 + 1);
                if cost < costs[next.index()] {
                    costs[next.index()] = cost;
                    parents[next.index()] = Some(cell);
                    sequence += 1;
                    queue.push(Reverse((cost, sequence, next)));
                }
            }
        }
        None
    }

    fn reachable_from_entrance(&self) -> [bool; WIDTH * HEIGHT] {
        let mut reachable = [false; WIDTH * HEIGHT];
        let mut queue = VecDeque::from([ENTRY]);
        reachable[ENTRY.index()] = true;
        while let Some(cell) = queue.pop_front() {
            for next in cell.neighbors().filter(|&next| self.open(next)) {
                if !reachable[next.index()] {
                    reachable[next.index()] = true;
                    queue.push_back(next);
                }
            }
        }
        reachable
    }

    pub fn dig_development(&self, cell: Cell) -> Option<u32> {
        let (w, h) = self.size();
        if cell.x == 0
            || cell.y == 0
            || cell.x >= w - 1
            || cell.y >= h - 1
            || self.open(cell)
            || self.has_placed_rock(cell)
        {
            return None;
        }
        let adjacent: Vec<_> = cell.neighbors().filter(|&next| self.open(next)).collect();
        if adjacent.is_empty() {
            return None;
        }
        let links = adjacent.len() as u32;
        let new_branches = adjacent
            .iter()
            .filter(|&&next| {
                next.neighbors()
                    .filter(|&neighbor| self.open(neighbor))
                    .count()
                    >= 2
            })
            .count() as u32
            + links.saturating_sub(2);
        Some(1 + new_branches + 2 * (links - 1))
    }

    pub fn development(&self, depth: usize) -> u32 {
        let mut natural = Self::new(depth);
        while natural.tier < self.tier {
            natural.expand();
        }
        self.complexity().saturating_sub(natural.complexity())
    }

    fn complexity(&self) -> u32 {
        let mut rooms = 0;
        let mut edges = 0;
        let mut branches = 0;
        // Built rocks temporarily close corridors. Count them as previously excavated.
        let mut excavated = self.open.clone();
        for rock in &self.placed_rocks {
            excavated[rock.index()] = true;
        }
        for (index, open) in excavated.iter().enumerate() {
            if !open {
                continue;
            }
            let cell = Cell {
                x: index % WIDTH,
                y: index / WIDTH,
            };
            let degree = cell
                .neighbors()
                .filter(|&next| excavated[next.index()])
                .count() as u32;
            rooms += 1;
            edges += degree;
            branches += degree.saturating_sub(2);
        }
        // Every extra edge beyond a spanning tree adds an independent loop.
        rooms + branches + 2 * (edges / 2 + 1).saturating_sub(rooms)
    }

    pub fn next_goal(&self) -> u32 {
        match self.tier {
            1 => 6,
            2 => 14,
            _ => 20,
        }
    }

    pub fn earned(&self) -> u32 {
        self.earned
    }

    fn expand(&mut self) {
        self.tier += 1;
        let (w, h) = self.size();
        // Extend two sides, joining each new corridor to the existing maze.
        for x in 1..w - 1 {
            self.open[Cell { x, y: h - 2 }.index()] = true;
        }
        for y in 1..h - 1 {
            self.open[Cell { x: w - 2, y }.index()] = true;
        }
        for x in [1, 7] {
            self.open[Cell { x, y: h - 3 }.index()] = true;
        }
        for y in [1, 5] {
            self.open[Cell { x: w - 3, y }.index()] = true;
        }
    }

    pub fn pass_level(&self) -> u32 {
        // Small parties stay upstairs. Strong ones always pass through.
        (2 + self.tier as usize + self.monster_count() / 2).min(10) as u32
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Encounter {
    rounds: u8,
    delay: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invader {
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub class: u8,
    pub cell: Cell,
    route: Vec<Cell>,
    step: usize,
    freeze: u8,
    #[serde(default)]
    poison: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encounter: Option<Encounter>,
    chest: Option<Cell>,
    looted: bool,
    fought: Vec<Cell>,
}

impl Invader {
    pub fn fighting(&self) -> bool {
        self.encounter.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Raid {
    #[serde(default)]
    tally: Report,
    #[serde(default)]
    reward_rarity: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duel: Option<Duel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boss_mp: Option<u8>,
    waiting: VecDeque<(u32, u8)>,
    pub invaders: Vec<Invader>,
    clock: f32,
    spawn: u32,
    fallen: Vec<Cell>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    pub wave: u32,
    pub stopped: u32,
    pub fled: u32,
    pub throne: u32,
    pub gold: u32,
    pub bones: u32,
    pub loot: u32,
    pub chests: u32,
    #[serde(default)]
    pub boss_xp: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub floors: Vec<Floor>,
    #[serde(default = "missing_progress")]
    pub development: u32,
    pub gold: u32,
    pub bones: u32,
    pub loot: u32,
    pub kills: u32,
    pub wave: u32,
    pub boss: u32,
    #[serde(default = "missing_progress")]
    boss_xp: u32,
    #[serde(default)]
    traps_built: [u32; 2],
    pub countdown: f32,
    pub paused: bool,
    pub log: Vec<String>,
    pub defeated: bool,
    #[serde(default)]
    pub last_report: Option<Report>,
    raid: Option<Raid>,
}

impl Game {
    pub fn new() -> Self {
        let mut floor = Floor::new(0);
        for (cell, site) in [
            (Cell { x: 10, y: 7 }, Site::Chest { rarity: 0 }),
            (Cell { x: 1, y: 2 }, Site::Spikes),
            (Cell { x: 2, y: 3 }, Site::Spikes),
            (Cell { x: 1, y: 3 }, Site::Goblin { hp: 38 }),
        ] {
            floor.sites[cell.index()] = Some(site);
        }
        Self {
            floors: vec![floor],
            development: 0,
            gold: 170,
            bones: 0,
            loot: 0,
            kills: 0,
            wave: 0,
            boss: 110,
            boss_xp: 0,
            traps_built: [0; 2],
            countdown: PREPARE,
            paused: false,
            log: vec!["秘書レイラ：宝箱までの予想経路を確認し、分岐の手前を守りましょう。".into()],
            defeated: false,
            last_report: None,
            raid: None,
        }
    }

    pub fn floor(&self) -> &Floor {
        self.floors.last().expect("active floor")
    }

    fn active(&mut self) -> &mut Floor {
        self.floors.last_mut().expect("active floor")
    }

    pub fn max_boss(&self) -> u32 {
        110 + 20 * (self.floors.len() - 1) as u32
            + 12 * (self.floor().tier - 1) as u32
            + 8 * (self.boss_level() - 1)
    }

    pub fn boss_level(&self) -> u32 {
        (self.boss_xp / 2 + 4).isqrt() - 1
    }

    pub fn boss_experience(&self) -> (u32, u32) {
        let level = self.boss_level();
        let start = boss_level_xp(level);
        (self.boss_xp - start, boss_level_xp(level + 1) - start)
    }

    pub fn descent_level(&self) -> u32 {
        1 + 2 * self.floors.len() as u32
    }

    pub fn monster_unlocked(&self, monster: &Monster) -> bool {
        self.floors.len() >= monster.floor && self.development >= monster.unlock
    }

    pub fn boss_experience_reward(&self, enemy_level: u32) -> u32 {
        (6 + combat_power(enemy_level)).min(MAX_BOSS_XP - self.boss_xp)
    }

    fn gain_boss_experience(&mut self, level: u32) -> u32 {
        let old_level = self.boss_level();
        let old_max = self.max_boss();
        let earned = self.boss_experience_reward(level);
        self.boss_xp += earned;
        let new_level = self.boss_level();
        self.boss += self.max_boss() - old_max;
        if new_level > old_level {
            self.message(format!(
                "魔王は経験値 +{earned} を得て、Lv.{new_level}になりました。"
            ));
        } else {
            self.message(format!("魔王の経験値 +{earned}"));
        }
        earned
    }

    pub fn duel(&self) -> Option<&Duel> {
        self.raid.as_ref()?.duel.as_ref()
    }

    pub fn duel_command(&mut self, command: Command) -> Option<Turn> {
        let level = self.boss_level();
        let duel = self.raid.as_mut()?.duel.as_mut()?;
        duel.act(command, level, &mut self.boss, self.wave)
    }

    pub fn dismiss_duel(&mut self) {
        let Some(outcome) = self.duel().and_then(|duel| duel.outcome) else {
            return;
        };
        let Some(mut raid) = self.raid.take() else {
            return;
        };
        let duel = raid.duel.take().expect("finished duel");
        match outcome {
            Outcome::Victory => {
                self.raid_reward(
                    &mut raid.tally,
                    duel.level,
                    self.floors.len() - 1,
                    duel.class,
                    raid.reward_rarity,
                );
                raid.tally.boss_xp += self.gain_boss_experience(duel.level);
            }
            Outcome::Fled => {
                raid.tally.fled += 1;
                self.message(format!(
                    "{}は玉座から逃走しました。",
                    ADVENTURERS[duel.class as usize].name
                ));
            }
            Outcome::Defeat => {
                self.defeated = true;
                self.last_report = Some(raid.tally);
                self.message("魔王が倒れ、迷宮は陥落しました。");
                return;
            }
        }
        raid.boss_mp = Some(
            duel.boss_mp
                .saturating_add(DUEL_MP_RECOVERY)
                .min(boss_mp(self.boss_level())),
        );
        self.raid = Some(raid);
    }

    pub fn running(&self) -> bool {
        self.raid.is_some()
    }

    pub fn invaders(&self) -> &[Invader] {
        self.raid
            .as_ref()
            .map_or(&[], |raid| raid.invaders.as_slice())
    }

    pub fn fallen(&self) -> &[Cell] {
        self.raid
            .as_ref()
            .map_or(&[], |raid| raid.fallen.as_slice())
    }

    fn message(&mut self, text: impl Into<String>) {
        self.log.insert(0, text.into());
        self.log.truncate(8);
    }

    pub fn chest_cost(&self, rarity: u8) -> (u32, u32) {
        let price = Order::Chest.price() + 12 * u32::from(rarity);
        let gold = if rarity == 0 && self.floor().chest_count() == 0 {
            // Keep a way to restart raids if the player has spent their last coins.
            price.min(self.gold)
        } else {
            price
        };
        (gold, u32::from(rarity))
    }

    pub fn next_trap_level(&self, order: Order) -> Option<u32> {
        let index = match order {
            Order::Spikes => 0,
            Order::Frost => 1,
            _ => return None,
        };
        Some((self.traps_built[index] + 1).min(MAX_TRAP_LEVEL))
    }

    pub fn construction_price(&self, order: Order) -> u32 {
        let step = if order == Order::Spikes { 7 } else { 10 };
        order.price()
            + self
                .next_trap_level(order)
                .map_or(0, |level| (level - 1) * step)
    }

    pub fn place(&mut self, order: Order, cell: Cell, rarity: u8) -> Result<(), &'static str> {
        if self.defeated || self.running() {
            return Err("建設は襲撃と襲撃の間にできます。");
        }
        if let Order::Recruit(id) = order {
            let monster = MONSTERS
                .get(id as usize)
                .ok_or("その魔物は見つかりません。")?;
            if monster.floor > self.floors.len() {
                return Err("必要な階に到達すると、この魔物を仲間にできます。");
            }
            if !self.monster_unlocked(monster) {
                return Err("迷路を開発して、この魔物を仲間にしてください。");
            }
        }
        if !self.floor().contains(cell) || cell == ENTRY || cell == THRONE {
            return Err("ここは工事できません。");
        }
        let (w, h) = self.floor().size();
        let old = self.floor().site(cell);
        if order == Order::Remove {
            let wall = self.floor().has_placed_rock(cell);
            if old.is_none() && !wall {
                return Err("この区画に撤去できるものはありません。");
            }
            let price = order.price();
            if self.gold < price {
                return Err("撤去するための金貨が足りません。");
            }
            self.gold -= price;
            let floor = self.active();
            if wall {
                floor.open[cell.index()] = true;
                floor.placed_rocks.retain(|&rock| rock != cell);
            } else {
                floor.sites[cell.index()] = None;
            }
            let name = if wall {
                "岩壁"
            } else {
                old.expect("removable site").order().name()
            };
            self.message(format!("ゴブリンが{name}を撤去しました。金貨 -{price}"));
            return Ok(());
        }
        if order == Order::Wall {
            if old.is_some() || !self.floor().open(cell) {
                return Err("岩壁を置くには空き床を選んでください。");
            }
            if self.gold < order.price() {
                return Err("金貨が足りません。");
            }
            let floor = self.active();
            let before = floor.reachable_from_entrance();
            floor.open[cell.index()] = false;
            let after = floor.reachable_from_entrance();
            if !after[THRONE.index()]
                || before.iter().zip(after).enumerate().any(
                    |(index, (&was_reachable, reachable))| {
                        index != cell.index() && was_reachable && !reachable
                    },
                )
            {
                floor.open[cell.index()] = true;
                return Err("入口から玉座や他の通路への道を塞げません。");
            }
            floor.placed_rocks.push(cell);
            self.gold -= order.price();
            self.message("ゴブリンが岩壁を築きました。撤去すると通路に戻せます。");
            return Ok(());
        }
        if order == Order::Dig {
            if self.floor().has_placed_rock(cell) {
                return Err("設置した岩壁には撤去を使ってください。");
            }
            if cell.x == 0 || cell.y == 0 || cell.x == w - 1 || cell.y == h - 1 {
                return Err("フロアの外壁は掘れません。");
            }
            let gain = self
                .floor()
                .dig_development(cell)
                .ok_or("通路につながっている内壁を選んでください。")?;
            if self.gold < order.price() {
                return Err("金貨が足りません。");
            }
            self.gold -= order.price();
            let floor = self.active();
            floor.open[cell.index()] = true;
            let unlocked = MONSTERS
                .iter()
                .filter(|monster| {
                    monster.floor <= self.floors.len()
                        && monster.unlock > self.development
                        && monster.unlock <= self.development + gain
                })
                .map(|monster| monster.name)
                .collect::<Vec<_>>();
            self.development += gain;
            if unlocked.is_empty() {
                self.message(format!("ゴブリンが通路を掘りました。迷宮開発 +{gain}"));
            } else {
                self.message(format!(
                    "迷宮開発 +{gain}　仲間にできる魔物：{}",
                    unlocked.join("、")
                ));
            }
            return Ok(());
        }
        if !self.floor().open(cell) {
            return Err("設置するには通路を選んでください。");
        }
        let old_rarity = match old {
            Some(Site::Chest { rarity }) if order == Order::Chest => Some(rarity),
            Some(_) => return Err("設置物を撤去してから建設してください。"),
            None => None,
        };
        let trap_level = self.next_trap_level(order);
        let (gold, loot) = if order == Order::Chest {
            if rarity > self.floor().rarity_limit() {
                return Err("フロアを拡張すると、このレア度が解放されます。");
            }
            let (gold, loot) = self.chest_cost(rarity);
            if let Some(old_rarity) = old_rarity {
                if rarity == old_rarity {
                    return Err("すでに同じレア度の品が入っています。");
                }
                // A new offering replaces the old one, including when lowering the reward bonus.
                (8 + 12 * rarity as u32, loot)
            } else {
                if self.floor().chest_count() >= self.floor().chest_limit() {
                    return Err("宝箱の配置上限です。");
                }
                (gold, loot)
            }
        } else {
            let site = order
                .site(rarity, trap_level.unwrap_or(1))
                .ok_or("建設指示を選んでください。")?;
            if site.is_monster() && self.floor().monster_count() >= self.floor().monster_limit() {
                return Err("モンスターの配置上限です。");
            }
            if site.is_trap() && self.floor().trap_count() >= self.floor().trap_limit() {
                return Err("罠の配置上限です。");
            }
            (self.construction_price(order), order.loot())
        };
        if self.gold < gold || self.bones < order.bones() || self.loot < loot {
            return Err("金貨、骨材、または戦利品が足りません。");
        }
        self.gold -= gold;
        self.bones -= order.bones();
        self.loot -= loot;
        if order == Order::Chest && self.floor().chest_count() == 0 {
            self.countdown = PREPARE;
            if rarity == 0 && gold < Order::Chest.price() {
                self.active().sites[cell.index()] = order.site(rarity, trap_level.unwrap_or(1));
                self.message("工務隊が金貨の不足分を補い、並品の宝箱を置きました。");
                return Ok(());
            }
        }
        self.active().sites[cell.index()] = order.site(rarity, trap_level.unwrap_or(1));
        if let Some(level) = trap_level {
            self.traps_built[usize::from(order == Order::Frost)] = level;
            self.message(format!(
                "ゴブリンが{} Lv.{level}を配置しました。",
                order.name()
            ));
        } else {
            self.message(format!("ゴブリンが{}を配置しました。", order.name()));
        }
        Ok(())
    }

    pub fn forecast(&self) -> (usize, u32) {
        let count = self.floor().chest_count();
        if count == 0 {
            return (0, 0);
        }
        (
            (1 + 2 * count).min(RAID_MAX),
            (self.wave + 1).min(MAX_RAID_LEVEL),
        )
    }

    pub fn reward_rarity(&self) -> u8 {
        self.floor()
            .sites
            .iter()
            .filter_map(|site| match site {
                Some(Site::Chest { rarity }) => Some(*rarity),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    pub fn descent_blocker(&self) -> Option<&'static str> {
        if self.defeated || self.running() {
            Some("新フロアは襲撃と襲撃の間に掘れます。")
        } else if self.floors.len() >= MAX_FLOORS {
            Some("これ以上深いフロアは掘れません。")
        } else if self.boss_level() < self.descent_level() {
            Some("玉座で冒険者を倒し、魔王のレベルを上げてください。")
        } else {
            None
        }
    }

    pub fn can_descend(&self) -> bool {
        self.descent_blocker().is_none()
    }

    pub fn descend(&mut self, steward: bool) -> Result<(), &'static str> {
        if let Some(reason) = self.descent_blocker() {
            return Err(reason);
        }
        self.active().steward = steward;
        self.floors.push(Floor::new(self.floors.len()));
        let unlocked: Vec<_> = MONSTERS
            .iter()
            .filter(|monster| monster.floor == self.floors.len() && self.monster_unlocked(monster))
            .map(|monster| monster.name)
            .collect();
        if !unlocked.is_empty() {
            self.message(format!(
                "新しい階で仲間にできる魔物：{}",
                unlocked.join("、")
            ));
        }
        self.boss = self.max_boss();
        self.countdown = PREPARE;
        self.message(if steward {
            "中ボスに前の階を任せ、新しいフロアへ移動しました。"
        } else {
            "新しいフロアへ移動しました。前の階の守りは停止中です。"
        });
        Ok(())
    }

    pub fn toggle_steward(&mut self, depth: usize) -> Result<(), &'static str> {
        if self.running() || self.defeated || depth + 1 >= self.floors.len() {
            return Err("準備中に上層の中ボスを切り替えられます。");
        }
        let floor = &mut self.floors[depth];
        floor.steward = !floor.steward;
        let active = floor.steward;
        self.message(if active {
            "中ボスに運営を委任しました。"
        } else {
            "中ボスの運営を停止しました。"
        });
        Ok(())
    }

    pub fn heal(&mut self) -> Result<(), &'static str> {
        if self.running() || self.defeated || self.bones < 2 || self.boss == self.max_boss() {
            return Err("回復には準備時間・骨材2・ボスの負傷が必要です。");
        }
        self.bones -= 2;
        self.boss = (self.boss + 30).min(self.max_boss());
        self.message("骨材2で魔王を回復しました。");
        Ok(())
    }

    fn reward(&mut self, level: u32, depth: usize, class: u8, rarity: u8) {
        self.kills += 1;
        let adventurer = &ADVENTURERS[class as usize];
        let gold = (9 + 6 * combat_power(level) + adventurer.bounty)
            * REWARD_PERCENT[rarity as usize]
            / 100;
        self.gold += gold;
        self.bones += 1 + u32::from(level >= 5) + u32::from(rarity >= 2);
        self.loot += 1 + u32::from(level >= 4) + u32::from(rarity);
        self.floors[depth].earned += 1;
        self.message(format!(
            "B{}でLv.{} {}を撃退。金貨 +{}・骨材と戦利品を回収。",
            depth + 1,
            level,
            adventurer.name,
            gold
        ));
    }

    fn raid_reward(&mut self, tally: &mut Report, level: u32, depth: usize, class: u8, rarity: u8) {
        let before = (self.gold, self.bones, self.loot);
        self.reward(level, depth, class, rarity);
        tally.stopped += 1;
        tally.gold += self.gold - before.0;
        tally.bones += self.bones - before.1;
        tally.loot += self.loot - before.2;
    }

    fn complete_raid(&mut self, tally: Report) {
        self.last_report = Some(tally);
        self.finish_raid();
        if let Some(report) = &self.last_report {
            self.message(format!(
                "執事セヴラン：第{}波、撃退{}、逃走{}、玉座到達{}、魔王経験値 +{}。",
                report.wave, report.stopped, report.fled, report.throne, report.boss_xp
            ));
        }
    }

    fn finish_raid(&mut self) {
        if self.defeated {
            return;
        }
        self.gold += RAID_RECOVERY_GOLD;
        let before = self.max_boss();
        while self.floor().tier < 3 && self.floor().earned >= self.floor().next_goal() {
            self.active().expand();
            self.gold += 40;
            self.message("撃退数が増え、ゴブリンが迷宮を拡張しました。建設予算 +40");
        }
        self.boss = (self.boss + self.max_boss() - before + 14).min(self.max_boss());
        self.countdown = PREPARE;
        self.message(format!(
            "第{}波が終了。守備隊を整えます。復旧費 +{RAID_RECOVERY_GOLD}金貨",
            self.wave
        ));
    }

    pub fn begin_raid(&mut self) {
        if self.defeated || self.running() || self.wave >= MAX_RAID_LEVEL {
            return;
        }
        let (count, best) = self.forecast();
        if count == 0 {
            return;
        }
        let reward_rarity = self.reward_rarity();
        self.wave += 1;
        self.paused = false;
        self.countdown = 0.0;
        let mut waiting = VecDeque::new();
        let mut tally = Report {
            wave: self.wave,
            ..Report::default()
        };
        let mut budgets: Vec<u32> = self
            .floors
            .iter()
            .map(|floor| {
                12 + 6 * floor.tier as u32
                    + floor.monster_count() as u32 * 12
                    + floor.trap_count() as u32 * 5
            })
            .collect();
        let mut bypassed = 0;
        let mut stopped = 0;
        for index in 0..count {
            let leader = index + 1 == count;
            let level = if leader {
                best
            } else {
                best.saturating_sub(1 + (index % 3) as u32).max(1)
            };
            let class = raid_class(level, self.wave, index, leader);
            // Keep the leader and a companion on the active floor even with strong stewards.
            if index + MIN_ACTIVE_INVADERS >= count {
                waiting.push_back((level, class));
                continue;
            }
            let mut captured = false;
            for (depth, budget) in budgets.iter_mut().enumerate().take(self.floors.len() - 1) {
                let floor = &self.floors[depth];
                if !floor.steward {
                    continue;
                }
                if level >= floor.pass_level() {
                    bypassed += 1;
                    continue;
                }
                let cost = 7 + 4 * combat_power(level) + ADVENTURERS[class as usize].bounty;
                if *budget >= cost {
                    *budget -= cost;
                    self.raid_reward(&mut tally, level, depth, class, reward_rarity);
                    stopped += 1;
                    captured = true;
                    break;
                }
            }
            if !captured {
                waiting.push_back((level, class));
            }
        }
        self.message(format!("第{}波。冒険者{}人が侵入。", self.wave, count));
        if stopped > 0 || bypassed > 0 {
            self.message(format!(
                "上層の中ボスが{}人撃退、強敵{}回を素通し。",
                stopped, bypassed
            ));
        }
        for site in self.active().sites.iter_mut().flatten() {
            match site {
                site => {
                    if let Some((hp, monster)) = site.monster_hp_mut() {
                        *hp = monster.hp;
                    }
                }
            }
        }
        let mut raid = Raid {
            tally,
            reward_rarity,
            duel: None,
            boss_mp: None,
            waiting,
            invaders: Vec::new(),
            clock: 0.0,
            spawn: 0,
            fallen: Vec::new(),
        };
        if raid.waiting.is_empty() {
            self.complete_raid(raid.tally);
        } else {
            self.spawn(&mut raid);
            self.raid = Some(raid);
        }
    }

    fn chest_target(&self, index: usize) -> Option<Cell> {
        let floor = self.floor();
        let count = floor.chest_count();
        if count == 0 {
            return None;
        }
        floor
            .sites
            .iter()
            .enumerate()
            .filter_map(|(i, site)| {
                matches!(site, Some(Site::Chest { .. })).then_some(Cell {
                    x: i % WIDTH,
                    y: i / WIDTH,
                })
            })
            .nth(index % count)
    }

    fn spawn(&self, raid: &mut Raid) {
        let Some((level, class)) = raid.waiting.pop_front() else {
            return;
        };
        let chest = self.chest_target((raid.spawn / 3) as usize);
        let floor = self.floor();
        let route = floor
            .adventurer_route(ENTRY, chest.unwrap_or(THRONE))
            .expect("reachable destination");
        let hp = 26 + 5 * combat_power(level) as i32 + ADVENTURERS[class as usize].health;
        raid.invaders.push(Invader {
            level,
            hp,
            max_hp: hp,
            class,
            cell: ENTRY,
            route,
            step: 0,
            freeze: 0,
            poison: 0,
            encounter: None,
            chest,
            looted: false,
            fought: Vec::new(),
        });
    }

    fn fight(&mut self, invader: &mut Invader, mut encounter: Encounter) -> Option<Encounter> {
        let (hp, monster) = self.active().sites[invader.cell.index()]
            .as_mut()?
            .monster_hp_mut()?;
        if *hp == 0 {
            return None;
        }
        if encounter.delay > 1 {
            encounter.delay -= 1;
            return Some(encounter);
        }

        let adventurer = &ADVENTURERS[invader.class as usize];
        let mut retaliation =
            (6 + 3 * combat_power(invader.level) as u16 + adventurer.retaliation).div_ceil(3);
        let mut damage = (monster.damage.min(8 + *hp as i32) + 1) / 2;
        if encounter.rounds == COMBAT_ROUNDS {
            match monster.tactic {
                Tactic::Ambush if invader.hp == invader.max_hp => damage += 8,
                Tactic::Venom => invader.poison = invader.poison.max(3 - adventurer.resist),
                Tactic::Snare => {
                    retaliation = retaliation.saturating_sub(2 - u16::from(adventurer.resist))
                }
                _ => {}
            }
        }
        if monster.tactic == Tactic::Guard {
            retaliation = retaliation.div_ceil(2);
        }
        invader.hp -= damage;
        *hp = hp.saturating_sub(retaliation);
        encounter.rounds -= 1;
        encounter.delay = COMBAT_INTERVAL;
        (invader.hp > 0 && *hp > 0 && encounter.rounds > 0).then_some(encounter)
    }

    fn move_invader(&mut self, invader: &mut Invader) -> bool {
        // An older save may already have a chest-bound adventurer on the throne.
        if invader.cell == THRONE {
            return true;
        }
        if invader.poison > 0 {
            invader.poison -= 1;
            invader.hp -= 3;
            if invader.hp <= 0 {
                return false;
            }
        }
        if let Some(encounter) = invader.encounter.take() {
            invader.encounter = self.fight(invader, encounter);
            return invader.hp > 0;
        }
        if invader.freeze > 0 {
            invader.freeze -= 1;
            return true;
        }
        if invader.step + 1 < invader.route.len() {
            invader.step += 1;
            invader.cell = invader.route[invader.step];
        }
        let cell = invader.cell;
        let adventurer = &ADVENTURERS[invader.class as usize];
        if !invader.fought.contains(&cell) {
            match self.floor().site(cell) {
                Some(site) if site.is_trap() => {
                    let level = site.trap_level().expect("trap level");
                    let frost = site.order() == Order::Frost;
                    let bonus = level as i32 - 1;
                    let damage = if frost {
                        invader.freeze =
                            (2 + (bonus / 4).min(4) as u8).saturating_sub(adventurer.resist);
                        FROST_DAMAGE + bonus * 2
                    } else {
                        SPIKE_DAMAGE + bonus * 3
                    };
                    invader.hp -= damage - adventurer.trap_armor;
                }
                Some(site) if site.monster().is_some_and(|(_, hp)| hp > 0) => {
                    invader.encounter = Some(Encounter {
                        rounds: COMBAT_ROUNDS,
                        delay: COMBAT_INTERVAL,
                    });
                }
                _ => {}
            }
            invader.fought.push(cell);
        }
        if invader.hp <= 0 {
            return false;
        }
        if invader.chest == Some(cell) && !invader.looted {
            if matches!(self.floor().site(cell), Some(Site::Chest { .. })) {
                self.active().sites[cell.index()] = None;
                invader.looted = true;
                self.message(format!(
                    "Lv.{} {}が宝箱を開け、持ち去りました。",
                    invader.level, adventurer.name
                ));
            }
            invader.chest = if invader.looted {
                None
            } else {
                self.chest_target(0)
            };
            invader.route = self
                .floor()
                .adventurer_route(cell, invader.chest.unwrap_or(THRONE))
                .expect("reachable destination");
            invader.step = 0;
            return true;
        }
        true
    }

    pub fn tick(&mut self, dt: f32) {
        if self.paused || self.defeated || self.duel().is_some() || !dt.is_finite() {
            return;
        }
        let dt = dt.clamp(0.0, 0.25);
        let Some(mut raid) = self.raid.take() else {
            if self.floor().chest_count() == 0 {
                return;
            }
            self.countdown = (self.countdown - dt).max(0.0);
            if self.countdown == 0.0 {
                self.begin_raid();
            }
            return;
        };
        raid.clock += dt;
        while raid.clock >= STEP {
            raid.clock -= STEP;
            raid.spawn += 1;
            if raid.spawn % 3 == 0 {
                self.spawn(&mut raid);
            }
            let mut survivors = Vec::new();
            let mut invaders = std::mem::take(&mut raid.invaders).into_iter();
            for mut invader in invaders.by_ref() {
                let was_looted = invader.looted;
                if self.move_invader(&mut invader) {
                    if invader.looted && !was_looted {
                        raid.tally.chests += 1;
                    }
                    if invader.cell == THRONE {
                        raid.tally.throne += 1;
                        let level = self.boss_level();
                        let mut duel = Duel::new(
                            invader.class,
                            invader.level,
                            invader.hp as u32,
                            invader.max_hp as u32,
                            level,
                        );
                        if let Some(mp) = raid.boss_mp {
                            duel.boss_mp = mp.min(boss_mp(level));
                        }
                        raid.duel = Some(duel);
                        break;
                    }
                    survivors.push(invader);
                } else if invader.hp <= 0 {
                    if raid.fallen.len() < RAID_MAX {
                        raid.fallen.push(invader.cell);
                    }
                    self.raid_reward(
                        &mut raid.tally,
                        invader.level,
                        self.floors.len() - 1,
                        invader.class,
                        raid.reward_rarity,
                    );
                }
            }
            survivors.extend(invaders);
            raid.invaders = survivors;
            if raid.duel.is_some() {
                break;
            }
            if raid.invaders.is_empty() && raid.waiting.is_empty() {
                self.complete_raid(raid.tally);
                return;
            }
        }
        self.raid = Some(raid);
    }

    pub fn preview_route(&self, destination: Cell) -> Vec<Cell> {
        self.floor()
            .adventurer_route(ENTRY, destination)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Vec<u8> {
        serde_json::to_vec(&(1u8, self)).expect("serializable game")
    }

    pub fn load(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > SAVE_LIMIT {
            return None;
        }
        let (version, mut game): (u8, Self) = serde_json::from_slice(bytes).ok()?;
        if game.boss_xp == missing_progress() {
            let tier = game.floors.last()?.tier;
            if !(1..=3).contains(&tier) || game.floors.len() > MAX_FLOORS {
                return None;
            }
            let old_level = 1 + 2 * (game.floors.len() - 1) as u32 + u32::from(tier - 1);
            game.boss_xp = boss_level_xp(old_level);
            if game.boss > 0 {
                game.boss = game.boss.checked_add(8 * (old_level - 1))?;
            }
        }
        for site in game
            .floors
            .iter_mut()
            .flat_map(|floor| floor.sites.iter_mut())
            .flatten()
        {
            if let Site::Monster { id, hp } = site {
                if let Some(monster) = MONSTERS.get(*id as usize) {
                    *hp = (*hp).min(monster.hp);
                }
            }
        }
        if game.development == missing_progress() {
            game.development = 0;
            if !game.valid() {
                return None;
            }
            game.development = game
                .floors
                .iter()
                .enumerate()
                .map(|(depth, floor)| floor.development(depth))
                .sum();
        }
        if version != 1 || !game.valid() {
            return None;
        }
        if let Some(raid) = &mut game.raid {
            if raid.tally.wave == 0 {
                raid.tally.wave = game.wave;
            }
        }
        Some(game)
    }

    fn valid(&self) -> bool {
        if self.floors.is_empty()
            || self.floors.len() > MAX_FLOORS
            || self.log.len() > 8
            || !(0.0..=PREPARE).contains(&self.countdown)
            || [self.gold, self.bones, self.loot, self.kills, self.wave]
                .into_iter()
                .any(|value| value > 1_000_000_000)
            || self.development > 100_000
            || self.boss_xp > MAX_BOSS_XP
            || self.traps_built.iter().any(|&count| count > MAX_TRAP_LEVEL)
            || self.defeated != (self.boss == 0 && self.duel().is_none())
        {
            return false;
        }
        for floor in &self.floors {
            if !(1..=3).contains(&floor.tier)
                || floor.earned > self.kills
                || floor.open.len() != WIDTH * HEIGHT
                || floor.sites.len() != WIDTH * HEIGHT
                || floor.placed_rocks.len() > WIDTH * HEIGHT
                || floor
                    .placed_rocks
                    .iter()
                    .any(|&cell| !floor.contains(cell) || floor.open(cell))
                || !floor.open(ENTRY)
                || !floor.open(THRONE)
                || floor.route(ENTRY, THRONE).is_none()
            {
                return false;
            }
            for (index, site) in floor.sites.iter().enumerate() {
                let cell = Cell {
                    x: index % WIDTH,
                    y: index / WIDTH,
                };
                if let Some(site) = site {
                    if !floor.open(cell)
                        || cell == ENTRY
                        || cell == THRONE
                        || floor.route(ENTRY, cell).is_none()
                        || site
                            .trap_level()
                            .is_some_and(|level| !(1..=MAX_TRAP_LEVEL).contains(&level))
                        || matches!(site, Site::Chest { rarity } if *rarity > floor.rarity_limit())
                        || matches!(site, Site::Monster { id, hp } if MONSTERS.get(*id as usize).is_none_or(|monster| *hp > monster.hp || monster.unlock > self.development))
                    {
                        return false;
                    }
                }
            }
        }
        if self.boss > self.max_boss() {
            return false;
        }
        let Some(raid) = &self.raid else { return true };
        let valid_level = |level: u32, class: u8| {
            (1..=MAX_RAID_LEVEL).contains(&level) && (class as usize) < ADVENTURERS.len()
        };
        !self.defeated
            && (raid.reward_rarity as usize) < REWARD_PERCENT.len()
            && raid
                .boss_mp
                .is_none_or(|mp| mp <= boss_mp(self.boss_level()))
            && raid
                .duel
                .as_ref()
                .is_none_or(|duel| duel.valid(self.boss_level(), self.boss))
            && raid.waiting.len() + raid.invaders.len() <= RAID_MAX
            && (0.0..STEP).contains(&raid.clock)
            && raid.spawn < 1000
            && raid.fallen.len() <= RAID_MAX
            && raid.fallen.iter().all(|&cell| self.floor().open(cell))
            && raid
                .waiting
                .iter()
                .all(|&(level, class)| valid_level(level, class))
            && raid.invaders.iter().all(|invader| {
                valid_level(invader.level, invader.class)
                    && (1..=MAX_ADVENTURER_HP).contains(&invader.hp)
                    && (invader.hp..=MAX_ADVENTURER_HP).contains(&invader.max_hp)
                    && invader.route.get(invader.step) == Some(&invader.cell)
                    && invader.route.len() <= WIDTH * HEIGHT
                    && invader.route.iter().all(|&cell| self.floor().open(cell))
                    && invader.route.last()
                        == Some(&if invader.looted {
                            THRONE
                        } else {
                            invader.chest.unwrap_or(THRONE)
                        })
                    && invader.fought.len() <= WIDTH * HEIGHT
                    && invader.poison <= 3
                    && invader.encounter.as_ref().is_none_or(|fight| {
                        (1..=COMBAT_ROUNDS).contains(&fight.rounds)
                            && (1..=COMBAT_INTERVAL).contains(&fight.delay)
                            && invader.fought.contains(&invader.cell)
                            && self
                                .floor()
                                .site(invader.cell)
                                .is_some_and(Site::is_monster)
                    })
            })
    }
}

#[cfg(test)]
mod tests;
