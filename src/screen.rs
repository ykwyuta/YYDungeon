use crate::dungeon::{Cell, Floor, Game, Order, Site, ENTRY, RARITIES, THRONE};
use crate::gfx::{Bitmap, Font, Gfx};

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

const SPRITES: [&[u8]; 12] = [
    include_bytes!("../assets/warden.png"),
    include_bytes!("../assets/sentinel.png"),
    include_bytes!("../assets/goblin.png"),
    include_bytes!("../assets/adventurer.png"),
    include_bytes!("../assets/archer.png"),
    include_bytes!("../assets/knight.png"),
    include_bytes!("../assets/fort.png"),
    include_bytes!("../assets/foreman.png"),
    include_bytes!("../assets/chest.png"),
    include_bytes!("../assets/spikes.png"),
    include_bytes!("../assets/frost.png"),
    include_bytes!("../assets/title.jpg"),
];

struct Art {
    generation: u32,
    bitmaps: Vec<Option<(Bitmap, f32)>>,
}

impl Art {
    fn load(&mut self, gfx: &Gfx) {
        if self.generation == gfx.d2d_gen {
            return;
        }
        self.generation = gfx.d2d_gen;
        self.bitmaps = SPRITES
            .iter()
            .map(|bytes| {
                let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
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
            })
            .collect();
    }

    fn draw(&self, gfx: &Gfx, id: usize, rect: Rect, alpha: f32) {
        if let Some(Some((bitmap, ratio))) = self.bitmaps.get(id) {
            let w = rect.w.min(rect.h * ratio);
            let h = w / ratio;
            let (x, y) = (rect.x + (rect.w - w) / 2.0, rect.y + (rect.h - h) / 2.0);
            gfx.draw_bitmap_alpha(bitmap, x, y, x + w, y + h, alpha);
        }
    }
}

const fn order_row(index: usize) -> Rect {
    Rect::new(868.0, 270.0 + index as f32 * 39.0, 328.0, 34.0)
}
const fn rarity_button(index: usize) -> Rect {
    Rect::new(868.0 + index as f32 * 82.0, 574.0, 76.0, 27.0)
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
            },
            floor,
            order: Order::Inspect,
            rarity: 0,
            hover: None,
            selected: None,
            overlay: None,
            notice: None,
            working: None,
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

    fn start_raid(&mut self) {
        if !self.game.running() && !self.game.defeated {
            self.switch_floor(self.game.floors.len() - 1);
            self.game.begin_raid();
        }
    }

    fn overlay_click(&mut self, overlay: Overlay, x: f32, y: f32) {
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
        if HELP.has(x, y) {
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
                self.notify("新階層には撃退20・規模3・金貨140・骨材8が必要です。");
            }
        } else if HEAL.has(x, y) && self.live() {
            if let Err(error) = self.game.heal() {
                self.notify(error);
            }
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
                    49..=55 if self.live() && !self.game.running() => {
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
        if !self.title && self.game.defeated && self.overlay.is_none() {
            self.overlay = Some(Overlay::Defeat);
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
            let x = 338.0 + index as f32 * 129.0;
            Rect::new(x, 24.0, 111.0, 46.0).panel(g, INSET, EDGE);
            Rect::new(x + 10.0, 29.0, 90.0, 16.0).label(g, name, Font::Small, DIM);
            Rect::new(x + 10.0, 43.0, 97.0, 23.0).label(g, &value.to_string(), Font::Big, color);
        }
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
                    rect.fill(g, rgb(0x151b24));
                    let brick = Rect::new(rect.x + 1.0, rect.y + 1.0, rect.w - 2.0, rect.h - 6.0);
                    brick.fill(
                        g,
                        [rgb(0x4d5864), rgb(0x454f5a), rgb(0x505b64), rgb(0x475361)][salt],
                    );
                    Rect::new(brick.x, brick.y, brick.w, 2.0).fill(g, rgb(0x65707b));
                    Rect::new(brick.x, brick.y + brick.h * 0.48, brick.w, 2.0)
                        .fill(g, rgb(0x303945));
                    Rect::new(brick.x + brick.w * 0.62, brick.y, 1.0, brick.h * 0.5)
                        .fill(g, rgb(0x303945));
                    Rect::new(
                        brick.x + brick.w * 0.31,
                        brick.y + brick.h * 0.5,
                        1.0,
                        brick.h * 0.5,
                    )
                    .fill(g, rgb(0x303945));
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
                    let (sprite, alpha, ink) = match site {
                        Site::Chest { rarity } => (8, 1.0, RARE[rarity as usize]),
                        Site::Goblin { hp } => (
                            2,
                            if hp == 0 && self.game.running() {
                                0.32
                            } else {
                                1.0
                            },
                            TEAL,
                        ),
                        Site::Sentinel { hp } => (
                            1,
                            if hp == 0 && self.game.running() {
                                0.32
                            } else {
                                1.0
                            },
                            GOLD,
                        ),
                        Site::Spikes => (9, 1.0, RED),
                        Site::Frost => (10, 1.0, RARE[2]),
                    };
                    self.art.draw(g, sprite, rect.inset(3.0), alpha);
                    Rect::new(
                        rect.x + rect.w * 0.27,
                        rect.y + rect.h - 4.0,
                        rect.w * 0.46,
                        2.0,
                    )
                    .fill(g, ink);
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
                let mut rect = grid.cell(invader.cell).inset(2.0);
                rect.x += (index % 3) as f32 * 2.0 - 2.0;
                self.art.draw(g, 3 + invader.class as usize, rect, 1.0);
                Rect::new(rect.x, rect.y, rect.w, 3.0).fill(g, INSET);
                Rect::new(
                    rect.x,
                    rect.y,
                    rect.w * invader.hp as f32 / invader.max_hp as f32,
                    3.0,
                )
                .fill(g, RED);
                Rect::new(rect.x, rect.y + 3.0, 22.0, 14.0).label(
                    g,
                    &invader.level.to_string(),
                    Font::Small,
                    TEXT,
                );
            }
            if let Some((cell, time)) = self.working {
                let mut rect = grid.cell(cell).inset(4.0);
                rect.y -= (0.9 - time) * 10.0;
                self.art.draw(g, 7, rect, (time * 3.0).min(1.0));
            }
        }
        let detail = if let Some(cell) = highlight {
            let label = match floor.site(cell) {
                Some(Site::Chest { rarity }) => format!("宝箱・{}", RARITIES[rarity as usize]),
                Some(Site::Goblin { hp } | Site::Sentinel { hp }) => {
                    format!("{}　体力 {}", floor.site(cell).unwrap().order().name(), hp)
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
                None => "石壁".into(),
            };
            format!("{:02} / {:02}　　{}", cell.x + 1, cell.y + 1, label)
        } else {
            "入口から玉座まで、冒険者の通り道を守ろう。".into()
        };
        Rect::new(43.0, 615.0, 510.0, 22.0).label(g, &detail, Font::Normal, DIM);
        Rect::new(598.0, 616.0, 200.0, 22.0).label(g, "●　玉座・宝箱への道筋", Font::Small, GOLD);
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
        for (i, order) in Order::ALL.into_iter().enumerate() {
            let rect = order_row(i);
            let selected = build && self.order == order;
            rect.panel(
                g,
                if selected { rgb(0x433626) } else { INSET },
                if selected { GOLD } else { EDGE },
            );
            let ink = if build { TEXT } else { DIM };
            match i {
                0..=4 => self.art.draw(
                    g,
                    [8, 2, 1, 9, 10][i],
                    Rect::new(rect.x + 6.0, rect.y + 3.0, 29.0, 28.0),
                    if build { 1.0 } else { 0.55 },
                ),
                _ => Rect::new(rect.x + 7.0, rect.y + 4.0, 26.0, 26.0).label(
                    g,
                    if i == 5 { "＋" } else { "−" },
                    Font::NormalCenter,
                    GOLD,
                ),
            }
            Rect::new(rect.x + 41.0, rect.y + 7.0, 149.0, 22.0).label(
                g,
                order.name(),
                Font::Normal,
                ink,
            );
            let (price, loot) = if order == Order::Chest {
                Game::chest_cost(self.rarity)
            } else {
                (order.price(), 0)
            };
            let cost = if order == Order::Remove {
                "+5 金貨".into()
            } else if loot > 0 {
                format!("{} 金 / {} 品", price, loot)
            } else if order.bones() > 0 {
                format!("{} 金 / {} 骨", price, order.bones())
            } else {
                format!("{} 金貨", price)
            };
            Rect::new(rect.x + 196.0, rect.y + 5.0, 103.0, 24.0).label(
                g,
                &cost,
                Font::NormalCenter,
                GOLD,
            );
            Rect::new(rect.x + 305.0, rect.y + 9.0, 20.0, 18.0).label(
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
            } else {
                "冒険者を呼び込む　[Enter]"
            },
            !self.game.running(),
            true,
        );
    }

    fn logs(&self, g: &mut Gfx) {
        LOG.panel(g, PANEL, EDGE);
        Rect::new(43.0, 667.0, 103.0, 23.0).label(g, "工房日誌", Font::Normal, GOLD);
        let floor = self.floor();
        let expansion = if floor.tier < 3 {
            "規模拡張"
        } else {
            "新フロア"
        };
        Rect::new(597.0, 668.0, 217.0, 22.0).label(
            g,
            &format!(
                "{}まで　撃退 {}/{}",
                expansion,
                floor.earned(),
                floor.next_goal()
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

    fn overlay(&self, g: &mut Gfx, overlay: Overlay) {
        Rect::new(0.0, 0.0, W, H).fill(g, [0.025, 0.04, 0.065, 0.86]);
        if overlay == Overlay::Help {
            Rect::new(283.0, 144.0, 674.0, 492.0).panel(g, PANEL, GOLD);
            Rect::new(304.0, 167.0, 632.0, 51.0).label(g, "迷宮工房の遊び方", Font::TitleMid, TEXT);
            for (i, (title, detail)) in [
                ("建設", "右の指示を選び、迷路の区画を押すとゴブリンが作業します。針の罠と魔物は、冒険者が通る床へ。"),
                ("おびき寄せる", "宝箱が多いほど冒険者が増えます。レアな品を入れると、強敵と引き換えに撃退報酬が増えます。"),
                ("迎撃と回収", "襲撃は時間で始まります。撃退した冒険者から金貨、死体の骨材、持ち物を回収します。玉座の主が倒されると陥落します。"),
                ("拡張と委任", "撃退で迷路・建設予算・配置枠が増えます。新しい階へ移ると、前の階を中ボスに任せられます。一定レベル以上の敵は新しい階まで来ます。"),
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
            Overlay::Descend => ("新しいフロア", format!("金貨140と骨材8を使い、地下{}階を掘ります。前の階を任せると中ボスが迎撃し、Lv.{}以上の冒険者は次の階へ通します。", self.game.floors.len() + 1, self.floor().pass_level())),
            Overlay::Reset => ("最初から始める", "現在の迷宮と進行記録を消去して、地下1階から始めます。".into()),
            Overlay::Defeat => ("迷宮陥落", format!("氷の主が倒されました。第{}波・累計撃退{}人・地下{}階", self.game.wave, self.game.kills, self.game.floors.len())),
            Overlay::Help => unreachable!(),
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
