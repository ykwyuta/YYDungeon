use crate::adventurers::{strongest_class, ADVENTURERS, ADVENTURER_ART, DUEL_ADVENTURER_ART};
use crate::duel::{boss_mp, Command, Duel};
use crate::duel_animation::{resting_poses, Animation, Pose, TURN_TIME};
use crate::dungeon::REWARD_PERCENT;
use crate::dungeon::{Cell, Floor, Game, Order, Site, ENTRY, RARITIES, THRONE};
use crate::gfx::{Bitmap, Font, Gfx};
use crate::monsters::{GOBLIN, MONSTERS, MONSTER_ART, SENTINEL};

const W: f32 = 1240.0;
const H: f32 = 780.0;
type Color = [f32; 4];
const fn rgb(hex: u32) -> Color {
    [
        ((hex >> 16) & 255) as f32 / 255.0,
        ((hex >> 8) & 255) as f32 / 255.0,
        (hex & 255) as f32 / 255.0,
        1.0,
    ]
}

const BG: Color = rgb(0x10151e);
const PANEL: Color = rgb(0x19212c);
const INSET: Color = rgb(0x111821);
const EDGE: Color = rgb(0x394452);
const GOLD: Color = rgb(0xe4ba75);
const TEXT: Color = rgb(0xf4ebdb);
const DIM: Color = rgb(0xb7bdc5);
const TEAL: Color = rgb(0x81c9b6);
const RED: Color = rgb(0xf39682);
const RARE: [Color; 4] = [GOLD, TEAL, rgb(0x93bdf4), rgb(0xdca4ea)];

const MAP: Rect = Rect::new(26.0, 175.0, 805.0, 469.0);
const LOG: Rect = Rect::new(26.0, 655.0, 805.0, 103.0);
const SIDE: Rect = Rect::new(850.0, 118.0, 364.0, 640.0);

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn has(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn inset(self, n: f32) -> Self {
        Self::new(self.x + n, self.y + n, self.w - 2.0 * n, self.h - 2.0 * n)
    }

    fn fill(self, g: &Gfx, color: Color) {
        g.fill_rect(self.x, self.y, self.x + self.w, self.y + self.h, color);
    }

    fn edge(self, g: &Gfx, color: Color) {
        g.stroke_rect(self.x, self.y, self.x + self.w, self.y + self.h, 1.0, color);
    }

    fn panel(self, g: &Gfx, fill: Color, edge: Color) {
        self.fill(g, fill);
        self.edge(g, edge);
    }

    fn label(self, g: &mut Gfx, s: &str, font: Font, color: Color) {
        g.text(s, self.x, self.y, self.w, self.h, font, color);
    }

    fn button(self, g: &mut Gfx, s: &str, selected: bool, enabled: bool) {
        let (fill, edge, ink) = if !enabled {
            (INSET, EDGE, DIM)
        } else if selected {
            (rgb(0x51402e), GOLD, TEXT)
        } else {
            (rgb(0x263341), EDGE, TEXT)
        };
        self.panel(g, fill, edge);
        self.label(g, s, Font::NormalCenter, ink);
    }
}

pub(crate) fn viewport(pw: u32, ph: u32) -> (f32, u32, u32) {
    let scale = (pw.max(1) as f32 / W).min(ph.max(1) as f32 / H);
    (
        scale,
        (pw as f32 / scale).round() as u32,
        (ph as f32 / scale).round() as u32,
    )
}

fn health_bar(g: &Gfx, rect: Rect, hp: f32, max_hp: f32, ink: Color) {
    rect.fill(g, rgb(0x080c12));
    let ratio = (hp / max_hp).clamp(0.0, 1.0);
    if ratio > 0.0 {
        Rect::new(
            rect.x + 1.0,
            rect.y + 1.0,
            (rect.w - 2.0) * ratio,
            rect.h - 2.0,
        )
        .fill(g, ink);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rock {
    Diggable,
    Built,
    Solid,
}

impl Rock {
    const ALL: [Self; 3] = [Self::Diggable, Self::Built, Self::Solid];

    fn at(floor: &Floor, cell: Cell) -> Self {
        if floor.has_placed_rock(cell) {
            Self::Built
        } else if floor.dig_development(cell).is_some() {
            Self::Diggable
        } else {
            Self::Solid
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Diggable => "掘削できる岩",
            Self::Built => "設置した岩壁",
            Self::Solid => "掘削できない岩",
        }
    }

    fn sprite(self) -> usize {
        match self {
            Self::Diggable => ROCK_DIGGABLE,
            Self::Built => ROCK_BUILT,
            Self::Solid => ROCK_SOLID,
        }
    }

    fn background(self) -> Color {
        match self {
            Self::Diggable => rgb(0x35271d),
            Self::Built => rgb(0x1e3546),
            Self::Solid => rgb(0x151b24),
        }
    }
}

struct Grid {
    rect: Rect,
    size: f32,
    cols: usize,
    rows: usize,
}

impl Grid {
    fn new(floor: &Floor) -> Self {
        let (cols, rows) = floor.size();
        let size = ((MAP.w - 100.0) / cols as f32)
            .min((MAP.h - 54.0) / rows as f32)
            .floor();
        let (w, h) = (size * cols as f32, size * rows as f32);
        Self {
            rect: Rect::new(
                MAP.x + (MAP.w - w) / 2.0,
                MAP.y + 16.0 + (MAP.h - 54.0 - h) / 2.0,
                w,
                h,
            ),
            size,
            cols,
            rows,
        }
    }

    fn cell(&self, cell: Cell) -> Rect {
        Rect::new(
            self.rect.x + self.size * cell.x as f32,
            self.rect.y + self.size * cell.y as f32,
            self.size,
            self.size,
        )
    }

    fn hit(&self, x: f32, y: f32) -> Option<Cell> {
        self.rect.has(x, y).then(|| Cell {
            x: ((x - self.rect.x) / self.size) as usize,
            y: ((y - self.rect.y) / self.size) as usize,
        })
    }
}

const SECRETARY: usize = 12;
const BUTLER: usize = 13;
const ARENA: usize = 14;
const DUEL_BOSS: usize = 15;
const ROCK_DIGGABLE: usize = 16;
const ROCK_BUILT: usize = 17;
const ROCK_SOLID: usize = 18;
const SPRITES: [&[u8]; 19] = [
    include_bytes!("../assets/warden.png"),
    include_bytes!("../assets/sentinel.png"),
    include_bytes!("../assets/goblin.png"),
    ADVENTURER_ART[0],
    ADVENTURER_ART[1],
    ADVENTURER_ART[2],
    include_bytes!("../assets/fort.png"),
    include_bytes!("../assets/foreman.png"),
    include_bytes!("../assets/chest.png"),
    include_bytes!("../assets/spikes.png"),
    include_bytes!("../assets/frost.png"),
    include_bytes!("../assets/title.jpg"),
    include_bytes!("../assets/secretary.png"),
    include_bytes!("../assets/butler.png"),
    include_bytes!("../assets/throne-arena.jpg"),
    include_bytes!("../assets/throne-boss-back.png"),
    include_bytes!("../assets/rock-diggable.png"),
    include_bytes!("../assets/rock-built.png"),
    include_bytes!("../assets/rock-solid.png"),
];

type Sprite = (Bitmap, f32);

struct Art {
    generation: u32,
    bitmaps: Vec<Option<Sprite>>,
    duel_poses: [Option<Vec<Option<Sprite>>>; crate::duel_art::SHEETS.len()],
}

impl Art {
    fn load(&mut self, gfx: &Gfx) {
        if self.generation == gfx.d2d_gen {
            return;
        }
        self.generation = gfx.d2d_gen;
        self.bitmaps = SPRITES
            .iter()
            .chain(MONSTER_ART.iter())
            .chain(ADVENTURER_ART[3..].iter())
            .chain(DUEL_ADVENTURER_ART.iter())
            .map(|bytes| Self::bitmap(gfx, bytes))
            .collect();
        self.duel_poses.fill_with(|| None);
    }

    fn bitmap(gfx: &Gfx, bytes: &[u8]) -> Option<Sprite> {
        let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
        Self::rgba_bitmap(gfx, &rgba)
    }

    fn rgba_bitmap(gfx: &Gfx, rgba: &image::RgbaImage) -> Option<Sprite> {
        let (w, h) = rgba.dimensions();
        let pixels: Vec<_> = rgba
            .pixels()
            .map(|p| {
                let a = p[3] as u32;
                let mul = |c| (c as u32 * a + 127) / 255;
                (a << 24) | (mul(p[0]) << 16) | (mul(p[1]) << 8) | mul(p[2])
            })
            .collect();
        Some((gfx.create_bitmap(w, h, &pixels)?, w as f32 / h as f32))
    }

    fn load_duel(&mut self, gfx: &Gfx, class: u8) {
        for index in [0, class as usize + 1] {
            if self.duel_poses[index].is_none() {
                self.duel_poses[index] =
                    Some(Self::duel_sheet(gfx, crate::duel_art::SHEETS[index]));
            }
        }
    }

    fn duel_sheet(gfx: &Gfx, bytes: &[u8]) -> Vec<Option<Sprite>> {
        let sheet = image::load_from_memory(bytes)
            .expect("embedded duel sprite sheet")
            .into_rgba8();
        let size = sheet.width() / 3;
        (0..Pose::EXTRA_COUNT)
            .map(|index| {
                let x = (index as u32 % 3) * size;
                let y = (index as u32 / 3) * size;
                Self::rgba_bitmap(
                    gfx,
                    &image::imageops::crop_imm(&sheet, x, y, size, size).to_image(),
                )
            })
            .collect()
    }

    fn draw_pose(&self, gfx: &Gfx, class: Option<u8>, rect: Rect, pose: Pose, alpha: f32) {
        if let Some(index) = pose.extra_index() {
            let fighter = class.map_or(0, |class| class as usize + 1);
            if let Some(bitmap) = self.duel_poses[fighter]
                .as_ref()
                .and_then(|sheet| sheet[index].as_ref())
            {
                Self::draw_bitmap(gfx, bitmap, rect, alpha);
                return;
            }
        }
        if let Some(class) = class {
            self.draw_duel_adventurer(gfx, class, rect);
        } else {
            self.draw(gfx, DUEL_BOSS, rect, alpha);
        }
    }

    fn draw_adventurer(&self, gfx: &Gfx, class: u8, rect: Rect) {
        let index = if class < 3 {
            3 + class as usize
        } else {
            SPRITES.len() + MONSTER_ART.len() + class as usize - 3
        };
        self.draw(gfx, index, rect, 1.0);
    }

    fn draw_duel_adventurer(&self, gfx: &Gfx, class: u8, rect: Rect) {
        let index = SPRITES.len() + MONSTER_ART.len() + ADVENTURER_ART.len() - 3;
        self.draw(gfx, index + class as usize, rect, 1.0);
    }

    fn draw_monster(&self, gfx: &Gfx, id: u8, rect: Rect, alpha: f32) {
        let sprite = match id {
            GOBLIN => 2,
            SENTINEL => 1,
            _ => SPRITES.len() + id as usize - 2,
        };
        self.draw(gfx, sprite, rect, alpha);
    }

    fn draw(&self, gfx: &Gfx, id: usize, rect: Rect, alpha: f32) {
        if let Some(Some(bitmap)) = self.bitmaps.get(id) {
            Self::draw_bitmap(gfx, bitmap, rect, alpha);
        }
    }

    fn draw_bitmap(gfx: &Gfx, (bitmap, ratio): &Sprite, rect: Rect, alpha: f32) {
        let w = rect.w.min(rect.h * ratio);
        let h = w / ratio;
        let (x, y) = (rect.x + (rect.w - w) / 2.0, rect.y + (rect.h - h) / 2.0);
        gfx.draw_bitmap_alpha(bitmap, x, y, x + w, y + h, alpha);
    }
}

const fn order_row(index: usize) -> Rect {
    Rect::new(868.0, 270.0 + index as f32 * 34.0, 328.0, 30.0)
}
const fn rarity_button(index: usize) -> Rect {
    Rect::new(868.0 + index as f32 * 82.0, 574.0, 76.0, 27.0)
}
const BESTIARY_CLOSE: Rect = Rect::new(854.0, 652.0, 225.0, 45.0);
const BESTIARY_BACK: Rect = Rect::new(161.0, 652.0, 107.0, 45.0);
const BESTIARY_NEXT: Rect = Rect::new(367.0, 652.0, 107.0, 45.0);
const MONSTERS_PER_PAGE: usize = 10;

fn monster_card(index: usize) -> Rect {
    Rect::new(
        161.0 + (index % 2) as f32 * 465.0,
        232.0 + (index / 2) as f32 * 80.0,
        453.0,
        73.0,
    )
}

const ADVICE: Rect = Rect::new(867.0, 27.0, 116.0, 40.0);
const REPORT: Rect = Rect::new(989.0, 27.0, 122.0, 40.0);
const COURT_CLOSE: Rect = Rect::new(759.0, 520.0, 175.0, 44.0);
const DUEL_NEXT: Rect = Rect::new(891.0, 671.0, 275.0, 62.0);
fn duel_button(index: usize) -> Rect {
    Rect::new(
        880.0 + (index % 2) as f32 * 151.0,
        617.0 + (index / 2) as f32 * 57.0,
        144.0,
        49.0,
    )
}

const HELP: Rect = Rect::new(1118.0, 27.0, 96.0, 40.0);
const PREVIOUS: Rect = Rect::new(30.0, 124.0, 39.0, 39.0);
const NEXT: Rect = Rect::new(74.0, 124.0, 39.0, 39.0);
const PAUSE: Rect = Rect::new(722.0, 126.0, 109.0, 36.0);
const HEAL: Rect = Rect::new(868.0, 657.0, 155.0, 32.0);
const DEPTH: Rect = Rect::new(1030.0, 657.0, 166.0, 32.0);
const RAID: Rect = Rect::new(868.0, 700.0, 328.0, 42.0);
const TITLE_START: Rect = Rect::new(81.0, 433.0, 303.0, 55.0);
#[cfg(target_arch = "wasm32")]
const REFRESH_CACHE: Rect = Rect::new(470.0, 578.0, 302.0, 38.0);
const TITLE_HELP: Rect = Rect::new(81.0, 503.0, 303.0, 43.0);

#[derive(Clone, Copy)]
pub(crate) enum Event {
    Move(f32, f32),
    Click(f32, f32),
    Key(u32),
    Resize(u32, u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Help,
    Secretary,
    Butler,
    Bestiary(usize),
    Descend,
    Defeat,
    Reset,
}

pub(crate) struct Screen {
    pub gfx: Gfx,
    pub game: Game,
    title: bool,
    has_save: bool,
    #[cfg(target_arch = "wasm32")]
    refresh_requested: bool,
    art: Art,
    floor: usize,
    order: Order,
    rarity: u8,
    hover: Option<Cell>,
    selected: Option<Cell>,
    overlay: Option<Overlay>,
    notice: Option<(String, f32)>,
    working: Option<(Cell, f32)>,
    duel_animation: Option<Animation>,
}

impl Screen {
    pub fn new(gfx: Gfx, saved_game: Option<Game>) -> Self {
        let has_save = saved_game.is_some();
        let game = saved_game.unwrap_or_else(Game::new);
        let floor = game.floors.len() - 1;
        Self {
            gfx,
            game,
            title: true,
            has_save,
            #[cfg(target_arch = "wasm32")]
            refresh_requested: false,
            art: Art {
                generation: 0,
                bitmaps: Vec::new(),
                duel_poses: std::array::from_fn(|_| None),
            },
            floor,
            order: Order::Inspect,
            rarity: 0,
            hover: None,
            selected: None,
            overlay: None,
            notice: None,
            working: None,
            duel_animation: None,
        }
    }

    fn live(&self) -> bool {
        self.floor + 1 == self.game.floors.len()
    }

    fn floor(&self) -> &Floor {
        &self.game.floors[self.floor]
    }

    pub fn should_save(&self) -> bool {
        !self.title
    }

    #[cfg(target_arch = "wasm32")]
    pub fn take_refresh_request(&mut self) -> bool {
        std::mem::take(&mut self.refresh_requested)
    }

    fn enter_workshop(&mut self) {
        self.title = false;
        self.reset_view();
    }

    fn reset_view(&mut self) {
        self.floor = self.game.floors.len() - 1;
        self.order = Order::Inspect;
        self.rarity = 0;
        self.hover = None;
        self.selected = None;
        self.working = None;
        self.notice = None;
        self.overlay = None;
    }

    fn notify(&mut self, message: &str) {
        self.notice = Some((message.into(), 4.5));
    }

    fn select_order(&mut self, order: Order) {
        if order == Order::Bestiary {
            self.overlay = Some(Overlay::Bestiary(0));
            return;
        }
        self.order = if self.order == order {
            Order::Inspect
        } else {
            order
        };
        self.selected = None;
    }

    fn switch_floor(&mut self, next: usize) {
        self.floor = next.min(self.game.floors.len() - 1);
        self.order = Order::Inspect;
        self.rarity = self.rarity.min(self.floor().rarity_limit());
        self.selected = None;
        self.hover = None;
        self.working = None;
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x / self.gfx.ui - (self.gfx.width as f32 - W) / 2.0,
            y / self.gfx.ui - (self.gfx.height as f32 - H) / 2.0,
        )
    }

    fn duel_command(&mut self, command: Command) {
        if self.duel_animation.is_some() {
            return;
        }
        let Some(duel) = self.game.duel() else { return };
        let old_enemy_hp = duel.hp;
        let old_boss_hp = self.game.boss;
        if let Some(turn) = self.game.duel_command(command) {
            self.duel_animation = Some(Animation {
                turn,
                time: 0.0,
                old_boss_hp,
                old_enemy_hp,
            });
        }
    }

    fn start_raid(&mut self) {
        if !self.game.running() && !self.game.defeated {
            if self.game.floor().chest_count() == 0 {
                self.notify("運営中のフロアに宝箱を置くと、冒険者が来ます。");
                return;
            }
            self.switch_floor(self.game.floors.len() - 1);
            self.game.begin_raid();
        }
    }

    fn overlay_click(&mut self, overlay: Overlay, x: f32, y: f32) {
        if matches!(overlay, Overlay::Secretary | Overlay::Butler) {
            if COURT_CLOSE.has(x, y) {
                self.overlay = None;
            }
            return;
        }
        if let Overlay::Bestiary(page) = overlay {
            if BESTIARY_CLOSE.has(x, y) {
                self.overlay = None;
            } else if BESTIARY_BACK.has(x, y) && page > 0 {
                self.overlay = Some(Overlay::Bestiary(page - 1));
            } else if BESTIARY_NEXT.has(x, y) && (page + 1) * MONSTERS_PER_PAGE < MONSTERS.len() {
                self.overlay = Some(Overlay::Bestiary(page + 1));
            } else if let Some(index) =
                (0..MONSTERS_PER_PAGE).find(|&index| monster_card(index).has(x, y))
            {
                let id = page * MONSTERS_PER_PAGE + index;
                let monster = &MONSTERS[id];
                if self.game.monster_unlocked(monster) && self.live() && !self.game.running() {
                    self.order = Order::Recruit(id as u8);
                    self.selected = None;
                    self.overlay = None;
                }
            }
            return;
        }
        if overlay == Overlay::Help {
            #[cfg(target_arch = "wasm32")]
            if REFRESH_CACHE.has(x, y) {
                self.refresh_requested = true;
                return;
            }
            if Rect::new(786.0, 578.0, 147.0, 38.0).has(x, y) {
                self.overlay = None;
            } else if (!self.title || self.has_save)
                && Rect::new(307.0, 578.0, 150.0, 38.0).has(x, y)
            {
                self.overlay = Some(Overlay::Reset);
            }
            return;
        }
        let buttons = [
            Rect::new(307.0, 459.0, 187.0, 44.0),
            Rect::new(527.0, 459.0, 187.0, 44.0),
            Rect::new(747.0, 459.0, 187.0, 44.0),
        ];
        let Some(index) = buttons.iter().position(|button| button.has(x, y)) else {
            return;
        };
        match (overlay, index) {
            (Overlay::Descend, 1 | 2) => match self.game.descend(index == 2) {
                Ok(()) => self.reset_view(),
                Err(error) => self.notify(error),
            },
            (Overlay::Reset, 2) | (Overlay::Defeat, 2) => {
                self.game = Game::new();
                self.enter_workshop();
            }
            (Overlay::Defeat, 0) => {}
            _ => self.overlay = None,
        }
    }

    fn click(&mut self, x: f32, y: f32) {
        if let Some(overlay) = self.overlay {
            self.overlay_click(overlay, x, y);
            return;
        }
        if self.title {
            if TITLE_START.has(x, y) {
                self.enter_workshop();
            } else if TITLE_HELP.has(x, y) {
                self.overlay = Some(Overlay::Help);
            }
            return;
        }
        if let Some(duel) = self.game.duel() {
            if self.duel_animation.is_some() {
                return;
            }
            if duel.outcome.is_some() {
                if DUEL_NEXT.has(x, y) {
                    self.game.dismiss_duel();
                }
            } else if let Some(index) = (0..4).find(|&index| duel_button(index).has(x, y)) {
                self.duel_command(Command::ALL[index]);
            }
            return;
        }
        if ADVICE.has(x, y) {
            self.overlay = Some(Overlay::Secretary);
        } else if REPORT.has(x, y) {
            self.overlay = Some(Overlay::Butler);
        } else if HELP.has(x, y) {
            self.overlay = Some(Overlay::Help);
        } else if PREVIOUS.has(x, y) {
            self.switch_floor(self.floor.saturating_sub(1));
        } else if NEXT.has(x, y) {
            self.switch_floor(self.floor + 1);
        } else if PAUSE.has(x, y) {
            self.game.paused = !self.game.paused;
        } else if RAID.has(x, y) {
            if self.game.running() {
                self.game.paused = !self.game.paused;
            } else {
                self.start_raid();
            }
        } else if DEPTH.has(x, y) {
            if self.live() && self.game.can_descend() {
                self.overlay = Some(Overlay::Descend);
            } else if !self.live() {
                if let Err(error) = self.game.toggle_steward(self.floor) {
                    self.notify(error);
                }
            } else {
                if let Some(reason) = self.game.descent_blocker() {
                    self.notify(reason);
                }
            }
        } else if HEAL.has(x, y) && self.live() {
            if let Err(error) = self.game.heal() {
                self.notify(error);
            }
        } else if order_row(2).has(x, y) {
            self.select_order(Order::Bestiary);
        } else if self.live() && !self.game.running() {
            if let Some(index) = (0..Order::ALL.len()).find(|&i| order_row(i).has(x, y)) {
                self.select_order(Order::ALL[index]);
            } else if let Some(index) = (0..RARITIES.len()).find(|&i| rarity_button(i).has(x, y)) {
                if index as u8 <= self.floor().rarity_limit() {
                    self.order = Order::Chest;
                    self.rarity = index as u8;
                } else {
                    self.notify("フロアが広がると、このレア度が解放されます。");
                }
            } else if let Some(cell) = Grid::new(self.floor()).hit(x, y) {
                self.selected = Some(cell);
                if self.order != Order::Inspect {
                    match self.game.place(self.order, cell, self.rarity) {
                        Ok(()) => self.working = Some((cell, 0.9)),
                        Err(error) => self.notify(error),
                    }
                }
            }
        } else if let Some(cell) = Grid::new(self.floor()).hit(x, y) {
            self.selected = Some(cell);
        }
    }

    pub fn frame(&mut self, events: &[Event], dt: f32, focused: bool) {
        if focused {
            if let Some(animation) = self.duel_animation.as_mut() {
                animation.time += dt.clamp(0.0, 0.25);
                if animation.time >= TURN_TIME {
                    self.duel_animation = None;
                }
            }
        }
        let report_wave = self.game.last_report.as_ref().map(|report| report.wave);
        for &event in events {
            match event {
                Event::Resize(w, h) => self.gfx.resize(w, h),
                Event::Move(x, y) => {
                    let (x, y) = self.point(x, y);
                    self.hover = Grid::new(self.floor()).hit(x, y);
                }
                Event::Click(x, y) => {
                    let (x, y) = self.point(x, y);
                    self.click(x, y);
                }
                Event::Key(key)
                    if !self.title
                        && self.overlay.is_none()
                        && self.game.duel().is_some()
                        && key != 122 =>
                {
                    if self.duel_animation.is_some() {
                        continue;
                    }
                    let duel = self.game.duel().unwrap();
                    if duel.outcome.is_some() && (key == 13 || key == 32) {
                        self.game.dismiss_duel();
                    } else if (49..=52).contains(&key) {
                        self.duel_command(Command::ALL[(key - 49) as usize]);
                    }
                }
                Event::Key(112) => self.overlay = Some(Overlay::Help),
                Event::Key(122) => self.gfx.toggle_fullscreen(),
                Event::Key(27) if self.overlay != Some(Overlay::Defeat) => {
                    self.overlay = None;
                    self.order = Order::Inspect;
                }
                Event::Key(13 | 32) if self.title && self.overlay.is_none() => {
                    self.enter_workshop();
                }
                Event::Key(key) if !self.title && self.overlay.is_none() => match key {
                    13 => self.start_raid(),
                    32 => self.game.paused = !self.game.paused,
                    37 => self.switch_floor(self.floor.saturating_sub(1)),
                    39 => self.switch_floor(self.floor + 1),
                    51 => self.select_order(Order::Bestiary),
                    49..=56 if self.live() && !self.game.running() => {
                        self.select_order(Order::ALL[(key - 49) as usize])
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if !self.title && self.overlay.is_none() && focused {
            self.game.tick(dt);
        }
        if !self.title && self.overlay.is_none() {
            if self.game.defeated {
                self.overlay = Some(Overlay::Defeat);
            } else if self.game.last_report.as_ref().map(|report| report.wave) != report_wave {
                self.overlay = Some(Overlay::Butler);
            }
        }
        if let Some((_, t)) = &mut self.notice {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if let Some((_, t)) = &mut self.working {
            *t -= dt;
            if *t <= 0.0 {
                self.working = None;
            }
        }
        self.draw();
    }

    fn draw(&mut self) {
        self.gfx.begin(BG);
        if self.gfx.d2d_begin() {
            self.art.load(&self.gfx);
            if !self.title {
                if let Some(duel) = self.game.duel() {
                    self.art.load_duel(&self.gfx, duel.class);
                }
            }
            let x = (self.gfx.width as f32 - W) / 2.0;
            let y = (self.gfx.height as f32 - H) / 2.0;
            self.gfx.set_origin(x, y);
            let view = View {
                game: &self.game,
                floor: self.floor,
                order: self.order,
                rarity: self.rarity,
                hover: self.hover,
                selected: self.selected,
                overlay: self.overlay,
                notice: self.notice.as_ref().map(|(text, _)| text.as_str()),
                working: self.working,
                art: &self.art,
                can_reset: !self.title || self.has_save,
            };
            if self.title {
                view.draw_title(&mut self.gfx, self.has_save);
            } else if let Some(duel) = self.game.duel() {
                view.draw_duel(&mut self.gfx, duel, self.duel_animation.as_ref());
            } else {
                view.draw(&mut self.gfx);
            }
            self.gfx.d2d_end();
        }
        self.gfx.present();
    }
}

struct View<'a> {
    game: &'a Game,
    floor: usize,
    order: Order,
    rarity: u8,
    hover: Option<Cell>,
    selected: Option<Cell>,
    overlay: Option<Overlay>,
    notice: Option<&'a str>,
    working: Option<(Cell, f32)>,
    art: &'a Art,
    can_reset: bool,
}

impl View<'_> {
    fn live(&self) -> bool {
        self.floor + 1 == self.game.floors.len()
    }

    fn floor(&self) -> &Floor {
        &self.game.floors[self.floor]
    }

    fn draw_title(&self, g: &mut Gfx, has_save: bool) {
        let frame = Rect::new(0.0, 0.0, W, H);
        frame.fill(g, BG);
        self.art.draw(g, 11, frame, 1.0);
        for band in 0..48 {
            let t = band as f32 / 47.0;
            let width = 13.0;
            Rect::new(band as f32 * width, 0.0, width + 0.25, H)
                .fill(g, [0.035, 0.055, 0.09, 0.72 * (1.0 - t).powf(1.5)]);
        }
        frame.inset(17.0).edge(g, [0.8, 0.63, 0.39, 0.58]);
        Rect::new(81.0, 220.0, 320.0, 23.0).label(g, "YYDUNGEON", Font::Normal, GOLD);
        Rect::new(73.0, 252.0, 314.0, 78.0).label(g, "迷 宮 工 房", Font::Title, TEXT);
        Rect::new(82.0, 345.0, 301.0, 1.0).fill(g, GOLD);
        Rect::new(81.0, 364.0, 346.0, 26.0).label(
            g,
            "ゴブリンと築く、冒険者を呼ぶ迷宮。",
            Font::Big,
            TEXT,
        );
        TITLE_START.button(
            g,
            if has_save {
                "つづきから　 [Enter]"
            } else {
                "はじめる　 [Enter]"
            },
            true,
            true,
        );
        TITLE_HELP.button(g, "遊び方　 [F1]", false, true);
        if let Some(overlay) = self.overlay {
            self.overlay(g, overlay);
        }
    }

    fn draw(&self, g: &mut Gfx) {
        Rect::new(0.0, 0.0, W, H).fill(g, BG);
        Rect::new(0.0, 0.0, W, 91.0).fill(g, rgb(0x18222f));
        Rect::new(26.0, 91.0, 1188.0, 1.0).fill(g, rgb(0x8d6f47));
        self.art.draw(g, 6, Rect::new(23.0, 12.0, 62.0, 61.0), 1.0);
        Rect::new(86.0, 15.0, 206.0, 45.0).label(g, "迷 宮 工 房", Font::TitleMid, TEXT);
        Rect::new(106.0, 57.0, 195.0, 20.0).label(g, "YYDUNGEON", Font::Small, GOLD);
        for (index, (name, value, color)) in [
            ("金貨", self.game.gold, GOLD),
            ("骨材", self.game.bones, TEXT),
            ("戦利品", self.game.loot, TEAL),
            ("累計撃退", self.game.kills, RED),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 308.0 + index as f32 * 135.0;
            Rect::new(x, 24.0, 111.0, 46.0).panel(g, INSET, EDGE);
            Rect::new(x + 10.0, 29.0, 90.0, 16.0).label(g, name, Font::Small, DIM);
            Rect::new(x + 10.0, 43.0, 97.0, 23.0).label(g, &value.to_string(), Font::Big, color);
        }
        ADVICE.button(g, "秘書の助言", false, true);
        REPORT.button(g, "執事の戦果", false, true);
        HELP.button(g, "遊び方  F1", false, true);
        self.map(g);
        self.side(g);
        self.logs(g);
        if let Some(overlay) = self.overlay {
            self.overlay(g, overlay);
        }
    }

    fn map(&self, g: &mut Gfx) {
        let floor = self.floor();
        PREVIOUS.button(g, "‹", false, self.floor > 0);
        NEXT.button(g, "›", false, !self.live());
        Rect::new(125.0, 124.0, 320.0, 29.0).label(
            g,
            &format!("地下 {} 階　　迷宮規模 {}", self.floor + 1, floor.tier),
            Font::Big,
            TEXT,
        );
        Rect::new(128.0, 151.0, 300.0, 17.0).label(
            g,
            &format!(
                "{}  /  {} × {} 区画",
                if self.live() {
                    "運営中"
                } else if floor.steward {
                    "中ボスに委任中"
                } else {
                    "守備停止中"
                },
                floor.size().0,
                floor.size().1
            ),
            Font::Small,
            DIM,
        );
        let (count, level) = self.game.forecast();
        let raid_label = if self.game.running() {
            format!("第{}波　　襲撃中", self.game.wave)
        } else if count == 0 {
            "宝箱を置くと襲撃開始".into()
        } else {
            format!(
                "第{}波まで {:02}秒　　{}人 / 最大Lv.{}",
                self.game.wave + 1,
                self.game.countdown.ceil() as u32,
                count,
                level
            )
        };
        Rect::new(448.0, 133.0, 273.0, 25.0).label(
            g,
            &raid_label,
            Font::Normal,
            if self.game.running() { RED } else { GOLD },
        );
        if !self.game.running() && count > 0 {
            Rect::new(448.0, 154.0, 273.0, 16.0).label(
                g,
                &format!(
                    "{}候補　報酬 金{}%",
                    ADVENTURERS[strongest_class(level)].name,
                    REWARD_PERCENT[self.game.reward_rarity() as usize],
                ),
                Font::Small,
                DIM,
            );
        }
        PAUSE.button(
            g,
            if self.game.paused {
                "再開"
            } else {
                "一時停止"
            },
            self.game.paused,
            true,
        );
        MAP.panel(g, rgb(0x141a23), EDGE);
        MAP.inset(5.0).edge(g, rgb(0x2c353f));
        let grid = Grid::new(floor);
        grid.rect.edge(g, rgb(0x6c604d));
        let highlight = self.hover.or(self.selected);
        let route = if self.live() && !self.game.running() {
            let destination = match highlight {
                Some(cell) if matches!(floor.site(cell), Some(Site::Chest { .. })) => cell,
                _ => THRONE,
            };
            self.game.preview_route(destination)
        } else {
            Vec::new()
        };
        for y in 0..grid.rows {
            for x in 0..grid.cols {
                let cell = Cell { x, y };
                let rect = grid.cell(cell);
                let salt = (x * 19 + y * 31 + x * y * 7) % 4;
                if floor.open(cell) {
                    rect.fill(
                        g,
                        [rgb(0x35302d), rgb(0x383230), rgb(0x3c3530), rgb(0x342e2e)][salt],
                    );
                    rect.edge(g, rgb(0x282629));
                    Rect::new(
                        rect.x + rect.w * 0.11,
                        rect.y + rect.h * 0.76,
                        rect.w * 0.32,
                        1.0,
                    )
                    .fill(g, rgb(0x4b4038));
                    if route.contains(&cell) {
                        g.fill_circle(
                            rect.x + rect.w / 2.0,
                            rect.y + rect.h / 2.0,
                            2.0,
                            rgb(0xa68b5b),
                        );
                    }
                } else {
                    let rock = Rock::at(floor, cell);
                    rect.fill(g, rock.background());
                    self.art.draw(g, rock.sprite(), rect.inset(1.0), 1.0);
                }
                if cell == ENTRY || cell == THRONE {
                    rect.inset(2.0).panel(
                        g,
                        if cell == ENTRY {
                            rgb(0x254740)
                        } else {
                            rgb(0x203449)
                        },
                        if cell == ENTRY { TEAL } else { rgb(0x779ab9) },
                    );
                    if cell == ENTRY {
                        rect.label(g, "入口", Font::NormalCenter, TEXT);
                    } else {
                        self.art.draw(
                            g,
                            if self.live() { 0 } else { 1 },
                            rect.inset(2.0),
                            if self.live() || floor.steward {
                                1.0
                            } else {
                                0.45
                            },
                        );
                    }
                }
                if let Some(site) = floor.site(cell) {
                    let ink = if let Some((id, hp)) = site.monster() {
                        let alpha = if hp == 0 && self.game.running() {
                            0.32
                        } else {
                            1.0
                        };
                        let in_combat = self.live()
                            && self
                                .game
                                .invaders()
                                .iter()
                                .any(|invader| invader.cell == cell && invader.fighting());
                        let body = rect.inset(3.0);
                        let sprite = if in_combat {
                            Rect::new(body.x, body.y, body.w * 0.61, body.h - 4.0)
                        } else {
                            Rect::new(body.x, body.y, body.w, body.h - 4.0)
                        };
                        self.art.draw_monster(g, id, sprite, alpha);
                        health_bar(
                            g,
                            Rect::new(rect.x + 2.0, rect.y + rect.h - 5.0, rect.w - 4.0, 5.0),
                            hp as f32,
                            MONSTERS[id as usize].hp as f32,
                            TEAL,
                        );
                        TEAL
                    } else {
                        let (sprite, ink) = match site {
                            Site::Chest { rarity } => (8, RARE[rarity as usize]),
                            Site::Spikes => (9, RED),
                            Site::Frost => (10, RARE[2]),
                            _ => unreachable!(),
                        };
                        self.art.draw(g, sprite, rect.inset(3.0), 1.0);
                        ink
                    };
                    if site.monster().is_none() {
                        Rect::new(
                            rect.x + rect.w * 0.27,
                            rect.y + rect.h - 4.0,
                            rect.w * 0.46,
                            2.0,
                        )
                        .fill(g, ink);
                    }
                }
                if self.selected == Some(cell) {
                    rect.inset(1.0).edge(g, GOLD);
                } else if self.hover == Some(cell) {
                    rect.inset(1.0).edge(g, TEXT);
                }
            }
        }
        if self.live() {
            for cell in self.game.fallen() {
                let rect = grid.cell(*cell);
                Rect::new(rect.x + 2.0, rect.y + 2.0, 12.0, 13.0).label(g, "×", Font::Small, RED);
            }
            for (index, invader) in self.game.invaders().iter().enumerate() {
                let tile = grid.cell(invader.cell);
                let tile_mates = self.game.invaders();
                let same_cell = |other: &&crate::dungeon::Invader| other.cell == invader.cell;
                let side = tile_mates
                    .iter()
                    .any(|other| other.cell == invader.cell && other.fighting());
                let stack = tile_mates[..index].iter().filter(same_cell).count();
                let lanes = tile_mates.iter().filter(same_cell).count() as f32;
                let available = tile.w * if side { 0.43 } else { 1.0 };
                let left = tile.x + tile.w - available;
                let lane = available / lanes;
                let sprite = if side {
                    Rect::new(
                        left + stack as f32 * lane * 0.35,
                        tile.y + 5.0,
                        available * 0.82,
                        tile.h - 11.0,
                    )
                } else {
                    Rect::new(
                        tile.x + 2.0 + stack as f32 * lane * 0.45,
                        tile.y + 5.0,
                        (tile.w - 4.0) / lanes.sqrt(),
                        tile.h - 9.0,
                    )
                };
                self.art.draw_adventurer(g, invader.class, sprite);
                health_bar(
                    g,
                    Rect::new(
                        tile.x + stack as f32 * tile.w / lanes,
                        tile.y,
                        tile.w / lanes,
                        5.0,
                    ),
                    invader.hp as f32,
                    invader.max_hp as f32,
                    RED,
                );
                if lanes == 1.0 && !side {
                    Rect::new(tile.x + 2.0, tile.y + 5.0, 20.0, 13.0).label(
                        g,
                        &invader.level.to_string(),
                        Font::Small,
                        TEXT,
                    );
                }
            }
            if let Some((cell, time)) = self.working {
                let mut rect = grid.cell(cell).inset(4.0);
                rect.y -= (0.9 - time) * 10.0;
                self.art.draw(g, 7, rect, (time * 3.0).min(1.0));
            }
        }
        let detail = if let Some(cell) = highlight {
            let invader = self
                .game
                .invaders()
                .iter()
                .rev()
                .find(|invader| self.live() && invader.cell == cell);
            let label = if let Some(invader) = invader {
                format!(
                    "Lv.{} {}　HP {}/{}",
                    invader.level,
                    ADVENTURERS[invader.class as usize].name,
                    invader.hp,
                    invader.max_hp
                )
            } else {
                match floor.site(cell) {
                    Some(Site::Chest { rarity }) => format!("宝箱・{}", RARITIES[rarity as usize]),
                    Some(site) if site.monster().is_some() => {
                        let (id, hp) = site.monster().unwrap();
                        format!(
                            "{}　HP {}/{}",
                            MONSTERS[id as usize].name, hp, MONSTERS[id as usize].hp
                        )
                    }
                    Some(site) => site.order().name().into(),
                    None if cell == ENTRY => "入口".into(),
                    None if cell == THRONE => if self.live() {
                        "氷の主の玉座"
                    } else {
                        "中ボスの玉座"
                    }
                    .into(),
                    None if floor.open(cell) => "通路".into(),
                    None => match Rock::at(floor, cell) {
                        Rock::Built => "設置した岩壁　掘削で通路に戻す".into(),
                        Rock::Diggable if self.order == Order::Dig => {
                            let gain = floor.dig_development(cell).unwrap_or_default();
                            format!("掘削できる岩　迷宮開発 +{gain}")
                        }
                        rock => rock.name().into(),
                    },
                }
            };
            format!("{:02} / {:02}　　{}", cell.x + 1, cell.y + 1, label)
        } else {
            "入口から玉座まで、冒険者の通り道を守ろう。".into()
        };
        Rect::new(43.0, 615.0, 328.0, 22.0).label(g, &detail, Font::Normal, DIM);
        for (index, rock) in Rock::ALL.into_iter().enumerate() {
            let x = 383.0 + index as f32 * 105.0;
            self.art
                .draw(g, rock.sprite(), Rect::new(x, 612.0, 23.0, 23.0), 1.0);
            Rect::new(x + 26.0, 616.0, 82.0, 19.0).label(g, rock.name(), Font::Small, DIM);
        }
        Rect::new(710.0, 616.0, 112.0, 22.0).label(g, "● 予想経路", Font::Small, GOLD);
    }

    fn side(&self, g: &mut Gfx) {
        let floor = self.floor();
        let build = self.live() && !self.game.running();
        SIDE.panel(g, PANEL, EDGE);
        Rect::new(868.0, 133.0, 328.0, 91.0).panel(g, INSET, EDGE);
        self.art
            .draw(g, 7, Rect::new(873.0, 136.0, 80.0, 85.0), 1.0);
        Rect::new(958.0, 147.0, 225.0, 27.0).label(g, "ゴブリン工務隊", Font::Big, GOLD);
        Rect::new(959.0, 178.0, 221.0, 36.0).label(
            g,
            if self.live() {
                "指示を選び、迷路の区画を指定。"
            } else {
                "前の階の守備を確認できます。"
            },
            Font::Small,
            DIM,
        );
        Rect::new(868.0, 238.0, 328.0, 23.0).label(
            g,
            &format!(
                "宝箱 {}/{}　　魔物 {}/{}　　罠 {}/{}",
                floor.chest_count(),
                floor.chest_limit(),
                floor.monster_count(),
                floor.monster_limit(),
                floor.trap_count(),
                floor.trap_limit()
            ),
            Font::Normal,
            TEXT,
        );
        for (i, mut order) in Order::ALL.into_iter().enumerate() {
            if i == 2 && matches!(self.order, Order::Recruit(_)) {
                order = self.order;
            }
            let rect = order_row(i);
            let selected = build && self.order == order;
            rect.panel(
                g,
                if selected { rgb(0x433626) } else { INSET },
                if selected { GOLD } else { EDGE },
            );
            let ink = if build || i == 2 { TEXT } else { DIM };
            match i {
                2 => {
                    let id = if let Order::Recruit(id) = order {
                        id
                    } else {
                        2
                    };
                    self.art.draw_monster(
                        g,
                        id,
                        Rect::new(rect.x + 6.0, rect.y + 2.0, 29.0, 26.0),
                        1.0,
                    );
                }
                0 | 1 | 3 | 4 => self.art.draw(
                    g,
                    [8, 2, 1, 9, 10][i],
                    Rect::new(rect.x + 6.0, rect.y + 2.0, 29.0, 26.0),
                    if build { 1.0 } else { 0.55 },
                ),
                5 | 7 => self.art.draw(
                    g,
                    if i == 5 { ROCK_DIGGABLE } else { ROCK_BUILT },
                    Rect::new(rect.x + 6.0, rect.y + 2.0, 29.0, 26.0),
                    if build { 1.0 } else { 0.55 },
                ),
                _ => Rect::new(rect.x + 7.0, rect.y + 4.0, 26.0, 26.0).label(
                    g,
                    "−",
                    Font::NormalCenter,
                    GOLD,
                ),
            }
            Rect::new(rect.x + 41.0, rect.y + 5.0, 149.0, 22.0).label(
                g,
                order.name(),
                Font::Normal,
                ink,
            );
            let (price, loot) = if order == Order::Chest {
                self.game.chest_cost(self.rarity)
            } else {
                (order.price(), order.loot())
            };
            let cost = if order == Order::Bestiary {
                format!("開発 {}", self.game.development)
            } else if order == Order::Remove {
                "費用なし".into()
            } else if matches!(order, Order::Recruit(_)) {
                format!("{}金 {}骨 {}品", price, order.bones(), loot)
            } else if loot > 0 {
                format!("{} 金 / {} 品", price, loot)
            } else if order.bones() > 0 {
                format!("{} 金 / {} 骨", price, order.bones())
            } else {
                format!("{} 金貨", price)
            };
            Rect::new(rect.x + 196.0, rect.y + 3.0, 103.0, 24.0).label(
                g,
                &cost,
                Font::NormalCenter,
                GOLD,
            );
            Rect::new(rect.x + 305.0, rect.y + 7.0, 20.0, 18.0).label(
                g,
                &(i + 1).to_string(),
                Font::Small,
                DIM,
            );
        }
        Rect::new(868.0, 549.0, 328.0, 21.0).label(g, "宝箱に入れる品のレア度", Font::Normal, DIM);
        for (i, name) in RARITIES.iter().enumerate() {
            let enabled = build && i as u8 <= floor.rarity_limit();
            rarity_button(i).button(g, name, self.rarity as usize == i, enabled);
        }
        if self.live() {
            Rect::new(868.0, 611.0, 328.0, 23.0).label(
                g,
                &format!("氷の主　HP {} / {}", self.game.boss, self.game.max_boss()),
                Font::Normal,
                TEXT,
            );
            let hp = Rect::new(868.0, 638.0, 328.0, 7.0);
            hp.fill(g, INSET);
            Rect::new(
                hp.x,
                hp.y,
                hp.w * self.game.boss as f32 / self.game.max_boss() as f32,
                hp.h,
            )
            .fill(
                g,
                if self.game.boss * 3 < self.game.max_boss() {
                    RED
                } else {
                    TEAL
                },
            );
            HEAL.button(
                g,
                "主を回復 · 骨材2",
                false,
                build && self.game.bones >= 2 && self.game.boss < self.game.max_boss(),
            );
            DEPTH.button(
                g,
                "新フロアを掘る",
                self.game.can_descend(),
                self.game.can_descend(),
            );
        } else {
            Rect::new(868.0, 611.0, 328.0, 40.0).label(
                g,
                &format!("中ボス　　Lv.{}以上は素通し", floor.pass_level()),
                Font::Normal,
                GOLD,
            );
            DEPTH.button(
                g,
                if floor.steward {
                    "委任を停止"
                } else {
                    "中ボスに任せる"
                },
                floor.steward,
                !self.game.running(),
            );
        }
        RAID.button(
            g,
            if self.game.running() {
                if self.game.paused {
                    "襲撃を再開　[Space]"
                } else {
                    "襲撃を一時停止　[Space]"
                }
            } else if self.game.floor().chest_count() == 0 {
                "宝箱の設置を待機中"
            } else {
                "冒険者を呼び込む　[Enter]"
            },
            !self.game.running(),
            self.game.running() || self.game.floor().chest_count() > 0,
        );
    }

    fn logs(&self, g: &mut Gfx) {
        LOG.panel(g, PANEL, EDGE);
        Rect::new(43.0, 667.0, 103.0, 23.0).label(g, "工房日誌", Font::Normal, GOLD);
        let floor = self.floor();
        let (xp, next_xp) = self.game.boss_experience();
        Rect::new(277.0, 668.0, 536.0, 22.0).label(
            g,
            &format!(
                "規模 {}/3　撃退 {}　氷の主 Lv.{} ({xp}/{next_xp} XP)　次の階 Lv.{}",
                floor.tier,
                floor.earned(),
                self.game.boss_level(),
                self.game.descent_level(),
            ),
            Font::Small,
            TEAL,
        );
        if let Some(notice) = self.notice {
            Rect::new(43.0, 695.0, 757.0, 21.0).label(g, notice, Font::Normal, RED);
        } else if self.order != Order::Inspect && self.live() && !self.game.running() {
            Rect::new(43.0, 695.0, 757.0, 21.0).label(
                g,
                self.order.description(),
                Font::Normal,
                GOLD,
            );
        } else if let Some(line) = self.game.log.get(1) {
            Rect::new(43.0, 695.0, 757.0, 21.0).label(g, line, Font::Normal, DIM);
        }
        if let Some(line) = self.game.log.first() {
            Rect::new(43.0, 722.0, 757.0, 21.0).label(g, line, Font::Normal, TEXT);
        }
    }

    fn bestiary(&self, g: &mut Gfx, page: usize) {
        Rect::new(138.0, 78.0, 964.0, 642.0).panel(g, PANEL, GOLD);
        Rect::new(160.0, 98.0, 920.0, 45.0).label(g, "仲間モンスター図鑑", Font::TitleMid, TEXT);
        let development = self.game.development;
        let unlocked = MONSTERS
            .iter()
            .filter(|monster| self.game.monster_unlocked(monster))
            .count();
        let goal = MONSTERS
            .iter()
            .filter(|monster| {
                monster.floor <= self.game.floors.len() && monster.unlock > development
            })
            .map(|monster| monster.unlock)
            .min()
            .map(|target| format!("次の解放まで開発 +{}", target - development))
            .or_else(|| {
                MONSTERS
                    .iter()
                    .filter(|monster| monster.floor > self.game.floors.len())
                    .map(|monster| monster.floor)
                    .min()
                    .map(|floor| format!("次のランクは地下{floor}階"))
            })
            .unwrap_or_else(|| "全種解放".into());
        Rect::new(161.0, 155.0, 915.0, 27.0).label(
            g,
            &format!(
                "迷宮開発 {development}　　仲間 {unlocked}/{}種　　{goal}",
                MONSTERS.len()
            ),
            Font::Big,
            GOLD,
        );
        let detail = if self.live() && !self.game.running() {
            "迷宮開発と到達階に応じて仲間が増えます。解放済の魔物を選び、空き通路に配置してください。"
        } else {
            "配置できるのは運営中の階の準備時間です。仲間は迷宮開発と到達階に応じて増えます。"
        };
        Rect::new(161.0, 195.0, 915.0, 24.0).label(g, detail, Font::Normal, DIM);
        for (index, monster) in MONSTERS[page * MONSTERS_PER_PAGE..][..MONSTERS_PER_PAGE]
            .iter()
            .enumerate()
        {
            let id = page * MONSTERS_PER_PAGE + index;
            let rect = monster_card(index);
            let unlocked = self.game.monster_unlocked(monster);
            rect.panel(g, INSET, if unlocked { GOLD } else { EDGE });
            self.art.draw_monster(
                g,
                id as u8,
                Rect::new(rect.x + 9.0, rect.y + 5.0, 66.0, 61.0),
                if unlocked { 1.0 } else { 0.48 },
            );
            Rect::new(rect.x + 82.0, rect.y + 8.0, 350.0, 25.0).label(
                g,
                &format!("{}　{} Lv.{}", monster.name, monster.rank, monster.level),
                Font::Big,
                if unlocked { TEXT } else { DIM },
            );
            let stats = format!(
                "{}　攻撃{} / HP{}　　{}",
                monster.tactic.label(),
                monster.damage,
                monster.hp,
                if unlocked {
                    format!("{}金 {}骨 {}品", monster.gold, monster.bones, monster.loot)
                } else {
                    format!("地下{}階 / 開発{}", monster.floor, monster.unlock)
                }
            );
            Rect::new(rect.x + 82.0, rect.y + 42.0, 350.0, 22.0).label(
                g,
                &stats,
                Font::Normal,
                if unlocked { TEAL } else { DIM },
            );
        }
        BESTIARY_BACK.button(g, "前へ", false, page > 0);
        BESTIARY_NEXT.button(
            g,
            "次へ",
            false,
            (page + 1) * MONSTERS_PER_PAGE < MONSTERS.len(),
        );
        Rect::new(276.0, 652.0, 83.0, 45.0).label(
            g,
            &format!(
                "{}/{}",
                page + 1,
                MONSTERS.len().div_ceil(MONSTERS_PER_PAGE)
            ),
            Font::NormalCenter,
            DIM,
        );
        BESTIARY_CLOSE.button(g, "閉じる　Esc", true, true);
    }

    fn draw_duel(&self, g: &mut Gfx, duel: &Duel, animation: Option<&Animation>) {
        Rect::new(0.0, 0.0, W, H).fill(g, BG);
        let field = Rect::new(25.0, 82.0, 1190.0, 502.0);
        self.art.draw(g, ARENA, field, 1.0);
        field.inset(1.0).edge(g, GOLD);
        Rect::new(26.0, 24.0, 1188.0, 44.0).label(g, "玉座の戦い", Font::TitleMid, GOLD);
        let level = self.game.boss_level();
        let enemy = ADVENTURERS[duel.class as usize].name;
        let (boss_pose, enemy_pose) = animation.map_or_else(
            || resting_poses(duel.outcome),
            |animation| animation.poses(duel.outcome),
        );
        let (boss_hp, enemy_hp) = animation.map_or((self.game.boss, duel.hp), |animation| {
            animation.hp(self.game.boss, duel.hp)
        });
        self.art.draw_pose(
            g,
            None,
            Rect::new(42.0, 195.0, 526.0, 386.0),
            boss_pose,
            1.0,
        );
        self.art.draw_pose(
            g,
            Some(duel.class),
            Rect::new(717.0, 89.0, 422.0, 388.0),
            enemy_pose,
            1.0,
        );
        for (x, y, name, hp, max_hp, mp, max_mp, color) in [
            (
                829.0,
                462.0,
                format!("氷の主　Lv.{level}"),
                boss_hp,
                self.game.max_boss(),
                duel.boss_mp,
                boss_mp(level),
                TEAL,
            ),
            (
                55.0,
                102.0,
                format!("{enemy}　Lv.{}", duel.level),
                enemy_hp,
                duel.max_hp,
                duel.mp,
                duel.max_mp,
                RED,
            ),
        ] {
            Rect::new(x, y, 356.0, 104.0).panel(g, INSET, GOLD);
            Rect::new(x + 16.0, y + 11.0, 320.0, 28.0).label(g, &name, Font::Big, TEXT);
            health_bar(
                g,
                Rect::new(x + 16.0, y + 46.0, 324.0, 10.0),
                hp as f32,
                max_hp as f32,
                color,
            );
            Rect::new(x + 16.0, y + 68.0, 320.0, 22.0).label(
                g,
                &format!("HP {hp}/{max_hp}　 MP {mp}/{max_mp}"),
                Font::Normal,
                TEXT,
            );
        }
        Rect::new(25.0, 599.0, 824.0, 152.0).panel(g, INSET, GOLD);
        Rect::new(48.0, 611.0, 750.0, 24.0).label(
            g,
            &format!("第{}波　ターン {}", self.game.wave, duel.turn),
            Font::Normal,
            GOLD,
        );
        for (index, line) in duel
            .lines
            .iter()
            .take(animation.map_or(3, Animation::message_count))
            .enumerate()
        {
            Rect::new(48.0, 644.0 + index as f32 * 30.0, 767.0, 24.0).label(
                g,
                line,
                Font::Normal,
                TEXT,
            );
        }
        Rect::new(865.0, 599.0, 350.0, 152.0).panel(g, PANEL, GOLD);
        if animation.is_some() {
            Rect::new(880.0, 642.0, 320.0, 44.0).label(g, "攻防中", Font::NormalCenter, GOLD);
        } else if duel.outcome.is_some() {
            DUEL_NEXT.button(g, "次へ　[Enter]", true, true);
        } else {
            for (index, command) in Command::ALL.into_iter().enumerate() {
                let detail = if command.level() > level {
                    format!("Lv.{}で解放", command.level())
                } else if command.mp() > 0 {
                    format!("MP {}", command.mp())
                } else {
                    format!("[{}]", index + 1)
                };
                duel_button(index).button(
                    g,
                    &format!("{}  {detail}", command.name()),
                    false,
                    duel.available(command, level),
                );
            }
        }
    }

    fn court(&self, g: &mut Gfx, overlay: Overlay) {
        Rect::new(283.0, 202.0, 674.0, 379.0).panel(g, PANEL, GOLD);
        let secretary = overlay == Overlay::Secretary;
        self.art.draw(
            g,
            if secretary { SECRETARY } else { BUTLER },
            Rect::new(298.0, 244.0, 234.0, 312.0),
            1.0,
        );
        Rect::new(548.0, 235.0, 377.0, 39.0).label(
            g,
            if secretary {
                "魔族の秘書 レイラ"
            } else {
                "執事 セヴラン"
            },
            Font::Big,
            GOLD,
        );
        if secretary {
            let (count, level) = self.game.forecast();
            let (first, second) = if self.game.running() {
                (
                    "襲撃中です。冒険者が玉座に着いたら、氷の主に戦いの指示を出してください。"
                        .into(),
                    "氷の主のMPは同じ襲撃の戦いに持ち越します。防御でもMPを回復できます。".into(),
                )
            } else if count == 0 {
                ("空いている通路に宝箱を置きましょう。最初は宝箱をひとつ置くと、少人数で迎えられます。".into(), "宝箱を調べると予想経路が見えます。金貨が足りなければ、最初の並品の宝箱は工務隊が補助します。".into())
            } else {
                (format!("次は{count}人、最大Lv.{level}です。氷の主を育てるため、玉座まで来られる経路を残しましょう。"), format!("次の階は氷の主がLv.{}になると開きます。階を進み、迷宮を掘ると新しい仲間が加わります。", self.game.descent_level()))
            };
            Rect::new(548.0, 290.0, 375.0, 75.0).label(g, &first, Font::Normal, TEXT);
            Rect::new(548.0, 380.0, 375.0, 92.0).label(g, &second, Font::Normal, TEXT);
        } else if let Some(report) = &self.game.last_report {
            for (index, line) in [
                format!("第{}波の戦果をご報告します。", report.wave),
                format!(
                    "撃退 {}人　逃走 {}人　玉座到達 {}人",
                    report.stopped, report.fled, report.throne
                ),
                format!(
                    "回収　金貨 {}　骨材 {}　戦利品 {}",
                    report.gold, report.bones, report.loot
                ),
                format!("開けられた宝箱 {}個", report.chests),
            ]
            .into_iter()
            .enumerate()
            {
                Rect::new(548.0, 290.0 + index as f32 * 42.0, 377.0, 28.0).label(
                    g,
                    &line,
                    Font::Normal,
                    TEXT,
                );
            }
        } else {
            Rect::new(548.0, 290.0, 375.0, 75.0).label(
                g,
                "襲撃が終わり次第、戦果と回収した資材をご報告します。",
                Font::Normal,
                TEXT,
            );
        }
        COURT_CLOSE.button(g, "閉じる　[Esc]", true, true);
    }

    fn overlay(&self, g: &mut Gfx, overlay: Overlay) {
        Rect::new(0.0, 0.0, W, H).fill(g, [0.025, 0.04, 0.065, 0.86]);
        if matches!(overlay, Overlay::Secretary | Overlay::Butler) {
            self.court(g, overlay);
            return;
        }
        if let Overlay::Bestiary(page) = overlay {
            self.bestiary(g, page);
            return;
        }
        if overlay == Overlay::Help {
            Rect::new(283.0, 144.0, 674.0, 492.0).panel(g, PANEL, GOLD);
            Rect::new(304.0, 167.0, 632.0, 51.0).label(g, "迷宮工房の遊び方", Font::TitleMid, TEXT);
            for (i, (title, detail)) in [
                ("建設", "右の指示を選び、迷路の区画を押すとゴブリンが作業します。分岐や回り道を掘ると、図鑑から新しい魔物を配置できます。"),
                ("おびき寄せる", "宝箱が多いほど冒険者が増えます。強さは襲撃のたびに上がり、レアな品は撃退報酬を増やします。開けられた箱は消えます。"),
                ("迎撃と回収", "襲撃は時間で始まります。撃退した冒険者から金貨と資材を回収します。HPは魔物が青緑、冒険者が赤です。玉座ではターン制で戦い、氷の主が倒れると陥落します。"),
                ("拡張と委任", "撃退で迷路と配置枠が広がります。玉座で勝って氷の主を育てると次の階へ進めます。前の階は中ボスに任せられます。"),
            ].into_iter().enumerate() {
                let y = 236.0 + 78.0 * i as f32;
                Rect::new(308.0, y, 617.0, 22.0).label(g, title, Font::Normal, GOLD);
                Rect::new(308.0, y + 23.0, 617.0, 51.0).label(g, detail, Font::Normal, TEXT);
            }
            #[cfg(target_arch = "wasm32")]
            REFRESH_CACHE.button(g, "キャッシュを消して再読み込み", false, true);
            if self.can_reset {
                Rect::new(307.0, 578.0, 150.0, 38.0).button(g, "最初から", false, true);
            }
            Rect::new(786.0, 578.0, 147.0, 38.0).button(g, "閉じる　Esc", true, true);
            return;
        }
        Rect::new(283.0, 258.0, 674.0, 268.0).panel(g, PANEL, GOLD);
        let (title, detail) = match overlay {
            Overlay::Descend => ("新しいフロア", format!("氷の主が必要なレベルに達しました。地下{}階を掘ります。前の階を任せると中ボスが迎撃し、Lv.{}以上の冒険者は次の階へ通します。", self.game.floors.len() + 1, self.floor().pass_level())),
            Overlay::Reset => ("最初から始める", "現在の迷宮と進行記録を消去して、地下1階から始めます。".into()),
            Overlay::Defeat => ("迷宮陥落", format!("氷の主が倒されました。第{}波・累計撃退{}人・地下{}階", self.game.wave, self.game.kills, self.game.floors.len())),
            Overlay::Help | Overlay::Bestiary(_) | Overlay::Secretary | Overlay::Butler => unreachable!(),
        };
        Rect::new(306.0, 286.0, 628.0, 52.0).label(
            g,
            title,
            Font::TitleMid,
            if overlay == Overlay::Defeat {
                RED
            } else {
                TEXT
            },
        );
        Rect::new(315.0, 373.0, 610.0, 65.0).label(g, &detail, Font::Normal, TEXT);
        if overlay != Overlay::Defeat {
            Rect::new(307.0, 459.0, 187.0, 44.0).button(g, "戻る", false, true);
        }
        if overlay == Overlay::Descend {
            Rect::new(527.0, 459.0, 187.0, 44.0).button(g, "委任せず掘る", false, true);
        }
        Rect::new(747.0, 459.0, 187.0, 44.0).button(
            g,
            if overlay == Overlay::Descend {
                "委任して掘る"
            } else {
                "最初から始める"
            },
            true,
            true,
        );
    }
}
