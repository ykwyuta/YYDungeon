use crate::adventurers::{raid_class, ADVENTURERS};
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
pub const FLOOR_DEVELOPMENT_GOAL: u32 = 24;
pub const FLOOR_GOLD_COST: u32 = 100;
pub const FLOOR_BONE_COST: u32 = 6;
const RAID_RECOVERY_GOLD: u32 = 10;
const STEP: f32 = 0.33;
const COMBAT_ROUNDS: u8 = 3;
const COMBAT_INTERVAL: u8 = 2;
const SPIKE_DAMAGE: i32 = 7;
const FROST_DAMAGE: i32 = 5;
const RAID_MAX: usize = 32;

fn missing_development() -> u32 {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Inspect,
    Chest,
    Goblin,
    Bestiary,
    Recruit(u8),
    Spikes,
    Frost,
    Dig,
    Remove,
    Wall,
}

impl Order {
    pub const ALL: [Self; 8] = [
        Self::Chest,
        Self::Goblin,
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
            Self::Goblin => "ゴブリン",
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
            Self::Inspect | Self::Remove => 0,
            Self::Chest => 20,
            Self::Goblin => 28,
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

    pub fn description(self) -> &'static str {
        match self {
            Self::Inspect => "床や壁を選ぶと、その区画の情報を確認できます。",
            Self::Chest => "冒険者が開けると宝箱は消えます。宝箱がある間は襲撃が来ます。",
            Self::Goblin => "通路で冒険者を足止めし、数回攻防します。",
            Self::Bestiary => "迷路を掘って、新しい魔物を仲間にできます。",
            Self::Recruit(_) => "空いている通路を選ぶと、仲間を配置します。",
            Self::Spikes => "踏んだ冒険者に7ダメージを与えます。",
            Self::Frost => "5ダメージを与え、冒険者を2手止めます。",
            Self::Dig => "通路に接した内壁を掘り、迷路に新しい道を作ります。",
            Self::Remove => "設置物を撤去して、配置枠を空けます。",
            Self::Wall => "空き床を岩でふさぎます。置いた岩は掘削で取り除けます。",
        }
    }

    fn site(self, rarity: u8) -> Option<Site> {
        Some(match self {
            Self::Chest => Site::Chest { rarity },
            Self::Goblin => Site::Goblin { hp: 38 },
            Self::Recruit(id) => Site::Monster {
                id,
                hp: MONSTERS.get(id as usize)?.hp,
            },
            Self::Spikes => Site::Spikes,
            Self::Frost => Site::Frost,
            _ => return None,
        })
    }
}

impl Site {
    pub fn order(self) -> Order {
        match self {
            Self::Chest { .. } => Order::Chest,
            Self::Goblin { .. } => Order::Goblin,
            Self::Sentinel { .. } => Order::Recruit(SENTINEL),
            Self::Monster { id, .. } => Order::Recruit(id),
            Self::Spikes => Order::Spikes,
            Self::Frost => Order::Frost,
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
        matches!(self, Self::Spikes | Self::Frost)
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
        if cell.x == 0 || cell.y == 0 || cell.x >= w - 1 || cell.y >= h - 1 || self.open(cell) {
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
        Some(if self.has_placed_rock(cell) {
            0
        } else {
            1 + new_branches + 2 * (links - 1)
        })
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

    pub fn pass_level(&self) -> u8 {
        // Small parties stay upstairs. Strong ones always pass through.
        (2 + self.tier as usize + self.monster_count() / 2).min(10) as u8
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Encounter {
    rounds: u8,
    delay: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invader {
    pub level: u8,
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
    waiting: VecDeque<(u8, u8)>,
    pub invaders: Vec<Invader>,
    clock: f32,
    spawn: u32,
    fallen: Vec<Cell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub floors: Vec<Floor>,
    #[serde(default = "missing_development")]
    pub development: u32,
    pub gold: u32,
    pub bones: u32,
    pub loot: u32,
    pub kills: u32,
    pub wave: u32,
    pub boss: u32,
    pub countdown: f32,
    pub paused: bool,
    pub log: Vec<String>,
    pub defeated: bool,
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
            countdown: PREPARE,
            paused: false,
            log: vec!["親方、宝箱に釣られた冒険者が来ます。通り道を守りましょう。".into()],
            defeated: false,
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
        110 + 20 * (self.floors.len() - 1) as u32 + 12 * (self.floor().tier - 1) as u32
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

    pub fn chest_cost(rarity: u8) -> (u32, u32) {
        (Order::Chest.price() + 12 * rarity as u32, rarity as u32)
    }

    pub fn place(&mut self, order: Order, cell: Cell, rarity: u8) -> Result<(), &'static str> {
        if self.defeated || self.running() {
            return Err("建設は襲撃と襲撃の間にできます。");
        }
        if let Order::Recruit(id) = order {
            let monster = MONSTERS
                .get(id as usize)
                .ok_or("その魔物は見つかりません。")?;
            if monster.unlock > self.development {
                return Err("迷路を開発して、この魔物を仲間にしてください。");
            }
        }
        if !self.floor().contains(cell) || cell == ENTRY || cell == THRONE {
            return Err("ここは工事できません。");
        }
        let (w, h) = self.floor().size();
        let old = self.floor().site(cell);
        if order == Order::Remove {
            if old.is_none() {
                return Err("この区画に撤去できるものはありません。");
            }
            self.active().sites[cell.index()] = None;
            self.gold += 5;
            self.message("ゴブリンが設置物を片付けました。金貨 +5");
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
            self.message("ゴブリンが岩壁を築きました。掘削すると通路に戻せます。");
            return Ok(());
        }
        if order == Order::Dig {
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
            if gain == 0 {
                floor.placed_rocks.retain(|&rock| rock != cell);
                self.message("ゴブリンが置いた岩を取り除き、通路に戻しました。");
                return Ok(());
            }
            let unlocked = MONSTERS
                .iter()
                .filter(|monster| {
                    monster.unlock > self.development && monster.unlock <= self.development + gain
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
        let (gold, loot) = if order == Order::Chest {
            if rarity > self.floor().rarity_limit() {
                return Err("フロアを拡張すると、このレア度が解放されます。");
            }
            let (gold, loot) = Self::chest_cost(rarity);
            if let Some(old_rarity) = old_rarity {
                if rarity == old_rarity {
                    return Err("すでに同じレア度の品が入っています。");
                }
                // A new offering replaces the old one, including when lowering danger.
                (8 + 12 * rarity as u32, loot)
            } else {
                if self.floor().chest_count() >= self.floor().chest_limit() {
                    return Err("宝箱の配置上限です。");
                }
                (gold, loot)
            }
        } else {
            let site = order.site(rarity).ok_or("建設指示を選んでください。")?;
            if site.is_monster() && self.floor().monster_count() >= self.floor().monster_limit() {
                return Err("モンスターの配置上限です。");
            }
            if site.is_trap() && self.floor().trap_count() >= self.floor().trap_limit() {
                return Err("罠の配置上限です。");
            }
            (order.price(), 0)
        };
        if self.gold < gold || self.bones < order.bones() || self.loot < loot {
            return Err("金貨、骨材、または戦利品が足りません。");
        }
        self.gold -= gold;
        self.bones -= order.bones();
        self.loot -= loot;
        if order == Order::Chest && self.floor().chest_count() == 0 {
            self.countdown = PREPARE;
        }
        self.active().sites[cell.index()] = order.site(rarity);
        self.message(format!("ゴブリンが{}を配置しました。", order.name()));
        Ok(())
    }

    pub fn forecast(&self) -> (usize, u8) {
        let mut rarities = self.floor().sites.iter().filter_map(|site| match site {
            Some(Site::Chest { rarity }) => Some(*rarity),
            _ => None,
        });
        let Some(first) = rarities.next() else {
            return (0, 0);
        };
        let (count, best) = rarities.fold((1, first), |(count, best), rarity| {
            (count + 1, best.max(rarity))
        });
        (
            (1 + 2 * count).min(RAID_MAX),
            1 + best * 2 + self.local_raid_rank(),
        )
    }

    fn local_raid_rank(&self) -> u8 {
        (self.floor().earned / 8).min(2) as u8
    }

    pub fn descent_blocker(&self) -> Option<&'static str> {
        let floor = self.floor();
        if self.defeated || self.running() {
            Some("新フロアは襲撃と襲撃の間に掘れます。")
        } else if self.floors.len() >= MAX_FLOORS {
            Some("これ以上深いフロアは掘れません。")
        } else if floor.tier < 3 || floor.earned < floor.next_goal() {
            Some("今の階を規模3まで拡張し、その階で冒険者を20人撃退してください。")
        } else if floor.development(self.floors.len() - 1) < FLOOR_DEVELOPMENT_GOAL {
            Some("今の階で内壁を掘り、開発24に達すると次の階を掘れます。")
        } else if self.gold < FLOOR_GOLD_COST || self.bones < FLOOR_BONE_COST {
            Some("新フロアには金貨100・骨材6が必要です。")
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
        self.gold -= FLOOR_GOLD_COST;
        self.bones -= FLOOR_BONE_COST;
        self.active().steward = steward;
        self.floors.push(Floor::new(self.floors.len()));
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
        self.message("骨材2で氷の主を回復しました。");
        Ok(())
    }

    fn reward(&mut self, level: u8, depth: usize, class: u8) {
        self.kills += 1;
        let adventurer = &ADVENTURERS[class as usize];
        let gold = 9 + 6 * level as u32 + adventurer.bounty;
        self.gold += gold;
        self.bones += 1 + (level >= 5) as u32;
        self.loot += 1 + (level >= 4) as u32;
        self.floors[depth].earned += 1;
        self.message(format!(
            "B{}でLv.{} {}を撃退。金貨 +{}・骨材と戦利品を回収。",
            depth + 1,
            level,
            adventurer.name,
            gold
        ));
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
        if self.defeated || self.running() {
            return;
        }
        let (count, best) = self.forecast();
        if count == 0 {
            return;
        }
        let rank = self.local_raid_rank();
        self.wave += 1;
        self.paused = false;
        self.countdown = 0.0;
        let mut waiting = VecDeque::new();
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
            let level = if index + 1 == count {
                best
            } else {
                best.saturating_sub((index % 3) as u8).max(1 + rank)
            };
            let leader = index + 1 == count;
            let class = raid_class(level, self.wave, index, leader);
            // The leader always reaches the active floor so delegating does not stall development.
            if leader {
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
                let cost = 7 + 4 * level as u32 + ADVENTURERS[class as usize].bounty;
                if *budget >= cost {
                    *budget -= cost;
                    self.reward(level, depth, class);
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
            waiting,
            invaders: Vec::new(),
            clock: 0.0,
            spawn: 0,
            fallen: Vec::new(),
        };
        if raid.waiting.is_empty() {
            self.finish_raid();
        } else {
            self.spawn(&mut raid);
            self.raid = Some(raid);
        }
    }

    fn chest_target(&self, index: usize) -> Option<Cell> {
        let floor = self.floor();
        let mut chests: Vec<_> = floor
            .sites
            .iter()
            .enumerate()
            .filter_map(|(i, site)| {
                if let Some(Site::Chest { rarity }) = site {
                    Some((
                        *rarity,
                        Cell {
                            x: i % WIDTH,
                            y: i / WIDTH,
                        },
                    ))
                } else {
                    None
                }
            })
            .collect();
        chests.sort_by_key(|&(rarity, cell)| (std::cmp::Reverse(rarity), cell));
        chests
            .get(index % chests.len().max(1))
            .map(|&(_, cell)| cell)
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
        let hp = 22 + 9 * level as i32 + ADVENTURERS[class as usize].health;
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
        let mut retaliation = (6 + 3 * invader.level as u16 + adventurer.retaliation).div_ceil(3);
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
                Some(Site::Spikes) => invader.hp -= SPIKE_DAMAGE - adventurer.trap_armor,
                Some(Site::Frost) => {
                    invader.hp -= FROST_DAMAGE - adventurer.trap_armor;
                    invader.freeze = 2 - adventurer.resist;
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
        if cell == THRONE && (invader.chest.is_none() || invader.looted) {
            let damage = 4
                + 4 * invader.level as u32
                + invader.hp as u32 / 3
                + u32::from(adventurer.retaliation);
            self.boss = self.boss.saturating_sub(damage);
            self.defeated = self.boss == 0;
            if self.defeated {
                self.message("氷の主が倒れ、迷宮は陥落しました。");
            } else {
                self.message(format!(
                    "Lv.{} {}が玉座に到達。氷の主に{}ダメージ。",
                    invader.level, adventurer.name, damage
                ));
            }
            return false;
        }
        true
    }

    pub fn tick(&mut self, dt: f32) {
        if self.paused || self.defeated || !dt.is_finite() {
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
            for mut invader in std::mem::take(&mut raid.invaders) {
                if self.move_invader(&mut invader) {
                    survivors.push(invader);
                } else if self.defeated {
                    break;
                } else if invader.hp <= 0 {
                    if raid.fallen.len() < RAID_MAX {
                        raid.fallen.push(invader.cell);
                    }
                    self.reward(invader.level, self.floors.len() - 1, invader.class);
                }
            }
            raid.invaders = survivors;
            if self.defeated {
                return;
            }
            if raid.invaders.is_empty() && raid.waiting.is_empty() {
                self.finish_raid();
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
        if game.development == missing_development() {
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
            || self.defeated != (self.boss == 0)
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
        let valid_level = |level: u8, class: u8| {
            (1..=15).contains(&level) && (class as usize) < ADVENTURERS.len()
        };
        !self.defeated
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
                    && (1..=180).contains(&invader.hp)
                    && (invader.hp..=180).contains(&invader.max_hp)
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
