use crate::adventurers::ADVENTURERS;

pub const SHEETS: [&[u8]; ADVENTURERS.len() + 1] = [
    include_bytes!("../assets/duel-poses/demon-king.webp"),
    include_bytes!("../assets/duel-poses/00.webp"),
    include_bytes!("../assets/duel-poses/01.webp"),
    include_bytes!("../assets/duel-poses/02.webp"),
    include_bytes!("../assets/duel-poses/03.webp"),
    include_bytes!("../assets/duel-poses/04.webp"),
    include_bytes!("../assets/duel-poses/05.webp"),
    include_bytes!("../assets/duel-poses/06.webp"),
    include_bytes!("../assets/duel-poses/07.webp"),
    include_bytes!("../assets/duel-poses/08.webp"),
    include_bytes!("../assets/duel-poses/09.webp"),
    include_bytes!("../assets/duel-poses/10.webp"),
    include_bytes!("../assets/duel-poses/11.webp"),
];
