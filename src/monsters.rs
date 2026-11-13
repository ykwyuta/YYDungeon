#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tactic {
    Strike,
    Guard,
    Snare,
    Venom,
    Ambush,
}

impl Tactic {
    pub fn label(self) -> &'static str {
        match self {
            Self::Strike => "強撃",
            Self::Guard => "守備",
            Self::Snare => "足止め",
            Self::Venom => "毒",
            Self::Ambush => "奇襲",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Monster {
    pub name: &'static str,
    pub unlock: u32,
    pub gold: u32,
    pub bones: u32,
    pub hp: u16,
    pub damage: i32,
    pub tactic: Tactic,
}

impl Monster {
    const fn new(
        name: &'static str,
        unlock: u32,
        gold: u32,
        bones: u32,
        hp: u16,
        damage: i32,
        tactic: Tactic,
    ) -> Self {
        Self {
            name,
            unlock,
            gold,
            bones,
            hp,
            damage,
            tactic,
        }
    }
}

// IDs are stored in saves. Add entries at the end and keep existing IDs stable.
use Tactic::*;
pub const MONSTERS: [Monster; 40] = [
    Monster::new("ゴブリン", 0, 28, 0, 38, 21, Strike),
    Monster::new("砂の魔神", 18, 66, 3, 72, 30, Strike),
    Monster::new("スライム", 0, 16, 0, 34, 12, Snare),
    Monster::new("洞窟コウモリ", 0, 18, 0, 26, 16, Ambush),
    Monster::new("マンドラゴラ", 3, 22, 0, 32, 12, Snare),
    Monster::new("毒グモ", 6, 27, 0, 30, 13, Venom),
    Monster::new("きのこ小鬼", 9, 28, 0, 42, 15, Venom),
    Monster::new("コボルト", 12, 32, 0, 45, 23, Strike),
    Monster::new("スケルトン", 15, 34, 1, 52, 18, Guard),
    Monster::new("火の玉", 19, 39, 1, 29, 24, Ambush),
    Monster::new("ミミック", 23, 46, 1, 47, 27, Ambush),
    Monster::new("おばけ", 27, 41, 1, 47, 18, Snare),
    Monster::new("ダイアウルフ", 31, 47, 1, 49, 26, Ambush),
    Monster::new("ストーンゴーレム", 35, 56, 2, 87, 17, Guard),
    Monster::new("ピクシー", 39, 44, 1, 38, 18, Snare),
    Monster::new("リザードマン", 43, 55, 2, 61, 27, Strike),
    Monster::new("ハーピー", 47, 57, 2, 45, 26, Ambush),
    Monster::new("オーク", 51, 60, 2, 74, 28, Strike),
    Monster::new("バジリスク", 55, 63, 2, 57, 22, Snare),
    Monster::new("氷の精霊", 59, 62, 2, 60, 23, Snare),
    Monster::new("サイクロプス", 63, 70, 3, 83, 33, Strike),
    Monster::new("ミノタウロス", 67, 73, 3, 92, 27, Guard),
    Monster::new("ガーゴイル", 71, 76, 3, 96, 28, Guard),
    Monster::new("マミー", 75, 69, 3, 81, 23, Venom),
    Monster::new("サキュバス", 79, 73, 3, 65, 27, Snare),
    Monster::new("リビングアーマー", 83, 82, 4, 105, 30, Guard),
    Monster::new("ケルベロス", 87, 83, 4, 79, 36, Ambush),
    Monster::new("ヴァンパイア", 91, 84, 4, 81, 32, Venom),
    Monster::new("デュラハン", 95, 91, 4, 95, 38, Strike),
    Monster::new("サラマンダー", 99, 89, 4, 81, 34, Venom),
    Monster::new("氷牙狼", 103, 86, 4, 85, 29, Snare),
    Monster::new("ワイバーン", 107, 100, 5, 85, 40, Ambush),
    Monster::new("キマイラ", 111, 102, 5, 105, 41, Strike),
    Monster::new("ヒュドラ", 115, 106, 5, 108, 33, Venom),
    Monster::new("フェニックス", 119, 110, 5, 96, 45, Ambush),
    Monster::new("リッチ", 123, 112, 5, 88, 36, Snare),
    Monster::new("紅蓮竜", 128, 119, 6, 114, 48, Strike),
    Monster::new("氷晶竜", 133, 122, 6, 111, 43, Snare),
    Monster::new("雷光竜", 138, 126, 6, 106, 50, Ambush),
    Monster::new("深淵竜", 144, 132, 7, 132, 44, Guard),
];

pub const GOBLIN: u8 = 0;
pub const SENTINEL: u8 = 1;

#[cfg(any(windows, target_arch = "wasm32"))]
pub const MONSTER_ART: [&[u8]; 38] = [
    include_bytes!("../assets/monsters/02.png"),
    include_bytes!("../assets/monsters/03.png"),
    include_bytes!("../assets/monsters/04.png"),
    include_bytes!("../assets/monsters/05.png"),
    include_bytes!("../assets/monsters/06.png"),
    include_bytes!("../assets/monsters/07.png"),
    include_bytes!("../assets/monsters/08.png"),
    include_bytes!("../assets/monsters/09.png"),
    include_bytes!("../assets/monsters/10.png"),
    include_bytes!("../assets/monsters/11.png"),
    include_bytes!("../assets/monsters/12.png"),
    include_bytes!("../assets/monsters/13.png"),
    include_bytes!("../assets/monsters/14.png"),
    include_bytes!("../assets/monsters/15.png"),
    include_bytes!("../assets/monsters/16.png"),
    include_bytes!("../assets/monsters/17.png"),
    include_bytes!("../assets/monsters/18.png"),
    include_bytes!("../assets/monsters/19.png"),
    include_bytes!("../assets/monsters/20.png"),
    include_bytes!("../assets/monsters/21.png"),
    include_bytes!("../assets/monsters/22.png"),
    include_bytes!("../assets/monsters/23.png"),
    include_bytes!("../assets/monsters/24.png"),
    include_bytes!("../assets/monsters/25.png"),
    include_bytes!("../assets/monsters/26.png"),
    include_bytes!("../assets/monsters/27.png"),
    include_bytes!("../assets/monsters/28.png"),
    include_bytes!("../assets/monsters/29.png"),
    include_bytes!("../assets/monsters/30.png"),
    include_bytes!("../assets/monsters/31.png"),
    include_bytes!("../assets/monsters/32.png"),
    include_bytes!("../assets/monsters/33.png"),
    include_bytes!("../assets/monsters/34.png"),
    include_bytes!("../assets/monsters/35.png"),
    include_bytes!("../assets/monsters/36.png"),
    include_bytes!("../assets/monsters/37.png"),
    include_bytes!("../assets/monsters/38.png"),
    include_bytes!("../assets/monsters/39.png"),
];
