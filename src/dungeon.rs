use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const WIDTH: usize = 17;
pub const HEIGHT: usize = 13;
pub const ENTRY: Cell = Cell { x: 0, y: 3 };
pub const THRONE: Cell = Cell { x: 11, y: 5 };
pub const PREPARE: f32 = 42.0;
pub const MAX_FLOORS: usize = 100;
const STEP: f32 = 0.33;
const RAID_MAX: usize = 32;
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
    Spikes,
    Frost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Inspect,
    Chest,
    Goblin,
    Sentinel,
    Spikes,
    Frost,
    Dig,
    Remove,
}

impl Order {
    pub const ALL: [Self; 7] = [
        Self::Chest,
        Self::Goblin,
        Self::Sentinel,
        Self::Spikes,
        Self::Frost,
        Self::Dig,
        Self::Remove,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Inspect => "調べる",
            Self::Chest => "宝箱",
            Self::Goblin => "ゴブリン",
            Self::Sentinel => "砂の魔神",
            Self::Spikes => "針の罠",
            Self::Frost => "氷結の罠",
            Self::Dig => "掘削",
            Self::Remove => "撤去",
        }
    }

    pub fn price(self) -> u32 {
        match self {
            Self::Inspect | Self::Remove => 0,
            Self::Chest => 30,
            Self::Goblin => 28,
            Self::Sentinel => 66,
            Self::Spikes => 19,
            Self::Frost => 38,
            Self::Dig => 14,
        }
    }

    pub fn bones(self) -> u32 {
        match self {
            Self::Sentinel => 3,
            Self::Frost => 1,
            _ => 0,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Inspect => "床や壁を選ぶと、その区画の情報を確認できます。",
            Self::Chest => "宝箱が多いほど、次の襲撃に来る冒険者が増えます。",
            Self::Goblin => "通過する冒険者を攻撃します。交戦で消耗します。",
            Self::Sentinel => "ゴブリンより強く、複数の冒険者と交戦できます。",
            Self::Spikes => "踏んだ冒険者に12ダメージを与えます。",
            Self::Frost => "8ダメージを与え、冒険者を2手止めます。",
            Self::Dig => "通路に接した内壁を掘り、迷路に新しい道を作ります。",
            Self::Remove => "設置物を撤去して、配置枠を空けます。",
        }
    }

    fn site(self, rarity: u8) -> Option<Site> {
        Some(match self {
            Self::Chest => Site::Chest { rarity },
            Self::Goblin => Site::Goblin { hp: 38 },
            Self::Sentinel => Site::Sentinel { hp: 72 },
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
            Self::Sentinel { .. } => Order::Sentinel,
            Self::Spikes => Order::Spikes,
            Self::Frost => Order::Frost,
        }
    }

    fn is_monster(self) -> bool {
        matches!(self, Self::Goblin { .. } | Self::Sentinel { .. })
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
    pub sites: Vec<Option<Site>>,
    pub steward: bool,
}

impl Floor {
    fn new(depth: usize) -> Self {
        let mut floor = Self {
            tier: 1,
            earned: 0,
            open: vec![false; WIDTH * HEIGHT],
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
        if !self.open(start) || !self.open(goal) {
            return None;
        }
        let mut parents = [None; WIDTH * HEIGHT];
        let mut queue = VecDeque::from([start]);
        parents[start.index()] = Some(start);
        while let Some(cell) = queue.pop_front() {
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
                if parents[next.index()].is_none() {
                    parents[next.index()] = Some(cell);
                    queue.push_back(next);
                }
            }
        }
        None
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
pub struct Invader {
    pub level: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub class: u8,
    pub cell: Cell,
    route: Vec<Cell>,
    step: usize,
    freeze: u8,
    chest: Option<Cell>,
    looted: bool,
    fought: Vec<Cell>,
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
            (Cell { x: 1, y: 3 }, Site::Spikes),
            (Cell { x: 2, y: 3 }, Site::Spikes),
            (Cell { x: 2, y: 4 }, Site::Goblin { hp: 38 }),
        ] {
            floor.sites[cell.index()] = Some(site);
        }
        Self {
            floors: vec![floor],
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
        if order == Order::Dig {
            if cell.x == 0 || cell.y == 0 || cell.x == w - 1 || cell.y == h - 1 {
                return Err("フロアの外壁は掘れません。");
            }
            if self.floor().open(cell) || !cell.neighbors().any(|next| self.floor().open(next)) {
                return Err("通路につながっている内壁を選んでください。");
            }
            if self.gold < order.price() {
                return Err("金貨が足りません。");
            }
            self.gold -= order.price();
            self.active().open[cell.index()] = true;
            self.message("ゴブリンが岩を掘り、新しい通路を開きました。");
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
        self.active().sites[cell.index()] = order.site(rarity);
        self.message(format!("ゴブリンが{}を配置しました。", order.name()));
        Ok(())
    }

    pub fn forecast(&self) -> (usize, u8) {
        let mut count = 1;
        let mut best = 0;
        for floor in &self.floors {
            count += 2 * floor.chest_count();
            for site in floor.sites.iter().flatten() {
                if let Site::Chest { rarity } = site {
                    best = best.max(*rarity);
                }
            }
        }
        (
            count.min(RAID_MAX),
            (1 + best * 2 + (self.wave / 4).min(6) as u8).min(15),
        )
    }

    pub fn can_descend(&self) -> bool {
        !self.defeated
            && !self.running()
            && self.floors.len() < MAX_FLOORS
            && self.floor().tier == 3
            && self.floor().earned >= 20
            && self.gold >= 140
            && self.bones >= 8
    }

    pub fn descend(&mut self, steward: bool) -> Result<(), &'static str> {
        if !self.can_descend() {
            return Err("撃退20・フロア規模3・金貨140・骨材8が必要です。");
        }
        self.gold -= 140;
        self.bones -= 8;
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

    fn reward(&mut self, level: u8, depth: usize) {
        self.kills += 1;
        let gold = 9 + 6 * level as u32;
        self.gold += gold;
        self.bones += 1 + (level >= 5) as u32;
        self.loot += 1 + (level >= 4) as u32;
        self.floors[depth].earned += 1;
        self.message(format!(
            "B{}でLv.{}撃退。金貨 +{}・骨材と戦利品を回収。",
            depth + 1,
            level,
            gold
        ));
    }

    fn finish_raid(&mut self) {
        if self.defeated {
            return;
        }
        let before = self.max_boss();
        while self.floor().tier < 3 && self.floor().earned >= self.floor().next_goal() {
            self.active().expand();
            self.gold += 40;
            self.message("撃退数が増え、ゴブリンが迷宮を拡張しました。建設予算 +40");
        }
        self.boss = (self.boss + self.max_boss() - before + 8).min(self.max_boss());
        self.countdown = PREPARE;
        self.message(format!(
            "第{}波を防衛。ゴブリンが罠と守備隊を整えます。",
            self.wave
        ));
    }

    pub fn begin_raid(&mut self) {
        if self.defeated || self.running() {
            return;
        }
        let (count, best) = self.forecast();
        let rank = (self.wave / 4).min(6) as u8;
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
                let cost = 7 + 4 * level as u32;
                if *budget >= cost {
                    *budget -= cost;
                    self.reward(level, depth);
                    stopped += 1;
                    captured = true;
                    break;
                }
            }
            if !captured {
                waiting.push_back((level, ((index + self.wave as usize - 1) % 3) as u8));
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
                Site::Goblin { hp } => *hp = 38,
                Site::Sentinel { hp } => *hp = 72,
                _ => {}
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

    fn spawn(&self, raid: &mut Raid) {
        let Some((level, class)) = raid.waiting.pop_front() else {
            return;
        };
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
        let chest = chests
            .get((raid.spawn / 3) as usize % chests.len().max(1))
            .map(|&(_, cell)| cell);
        let route = floor
            .route(ENTRY, chest.unwrap_or(THRONE))
            .expect("reachable destination");
        let hp = 22 + 9 * level as i32 + 4 * class as i32;
        raid.invaders.push(Invader {
            level,
            hp,
            max_hp: hp,
            class,
            cell: ENTRY,
            route,
            step: 0,
            freeze: 0,
            chest,
            looted: false,
            fought: Vec::new(),
        });
    }

    fn move_invader(&mut self, invader: &mut Invader) -> bool {
        if invader.freeze > 0 {
            invader.freeze -= 1;
            return true;
        }
        if invader.step + 1 < invader.route.len() {
            invader.step += 1;
            invader.cell = invader.route[invader.step];
        }
        let cell = invader.cell;
        if !invader.fought.contains(&cell) {
            match &mut self.active().sites[cell.index()] {
                Some(Site::Spikes) => invader.hp -= 12,
                Some(Site::Frost) => {
                    invader.hp -= 8;
                    invader.freeze = 2;
                }
                Some(site @ (Site::Goblin { .. } | Site::Sentinel { .. })) => {
                    let (hp, strength) = match site {
                        Site::Goblin { hp } => (hp, 21),
                        Site::Sentinel { hp } => (hp, 30),
                        _ => unreachable!(),
                    };
                    if *hp > 0 {
                        invader.hp -= strength.min(8 + *hp as i32);
                        *hp = hp.saturating_sub(6 + 3 * invader.level as u16);
                    }
                }
                _ => {}
            }
            invader.fought.push(cell);
        }
        if invader.hp <= 0 {
            return false;
        }
        if invader.chest == Some(cell) && !invader.looted {
            invader.looted = true;
            invader.route = self.floor().route(cell, THRONE).expect("reachable throne");
            invader.step = 0;
            return true;
        }
        if cell == THRONE && (invader.chest.is_none() || invader.looted) {
            let damage = 4 + 4 * invader.level as u32 + invader.hp as u32 / 3;
            self.boss = self.boss.saturating_sub(damage);
            self.defeated = self.boss == 0;
            if self.defeated {
                self.message("氷の主が倒れ、迷宮は陥落しました。");
            } else {
                self.message(format!(
                    "Lv.{}が玉座に到達。氷の主に{}ダメージ。",
                    invader.level, damage
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
                } else if !self.defeated {
                    if invader.hp <= 0 {
                        if raid.fallen.len() < RAID_MAX {
                            raid.fallen.push(invader.cell);
                        }
                    }
                    self.reward(invader.level, self.floors.len() - 1);
                } else {
                    break;
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
        self.floor().route(ENTRY, destination).unwrap_or_default()
    }

    pub fn save(&self) -> Vec<u8> {
        serde_json::to_vec(&(1u8, self)).expect("serializable game")
    }

    pub fn load(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > SAVE_LIMIT {
            return None;
        }
        let (version, game): (u8, Self) = serde_json::from_slice(bytes).ok()?;
        (version == 1 && game.valid()).then_some(game)
    }

    fn valid(&self) -> bool {
        if self.floors.is_empty()
            || self.floors.len() > MAX_FLOORS
            || self.log.len() > 8
            || !(0.0..=PREPARE).contains(&self.countdown)
            || [self.gold, self.bones, self.loot, self.kills, self.wave]
                .into_iter()
                .any(|value| value > 1_000_000_000)
            || self.defeated != (self.boss == 0)
        {
            return false;
        }
        for floor in &self.floors {
            if !(1..=3).contains(&floor.tier)
                || floor.earned > self.kills
                || floor.open.len() != WIDTH * HEIGHT
                || floor.sites.len() != WIDTH * HEIGHT
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
        let valid_level = |level: u8, class: u8| (1..=15).contains(&level) && class <= 2;
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
            })
    }
}

#[cfg(test)]
mod tests;
