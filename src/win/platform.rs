use crate::dungeon::Game;
use std::{fs, io, path::PathBuf};

pub fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("yydungeon")
}

pub fn load() -> Option<Game> {
    Game::load(&fs::read(app_dir().join("dungeon.json")).ok()?)
}

pub fn save(game: &Game) -> io::Result<()> {
    let dir = app_dir();
    fs::create_dir_all(&dir)?;
    let temp = dir.join(format!("dungeon-{}.tmp", std::process::id()));
    fs::write(&temp, game.save())?;
    fs::rename(&temp, dir.join("dungeon.json"))
}
