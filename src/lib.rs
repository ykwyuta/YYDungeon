mod adventurers;
mod duel;
mod dungeon;
mod monsters;

#[cfg(any(windows, target_arch = "wasm32"))]
mod screen;

#[cfg(windows)]
#[path = "win/app.rs"]
mod app;
#[cfg(windows)]
#[path = "win/crashlog.rs"]
mod crashlog;
#[cfg(windows)]
#[path = "win/gfx.rs"]
mod gfx;
#[cfg(windows)]
#[path = "win/platform.rs"]
mod platform;

#[cfg(target_arch = "wasm32")]
#[path = "web_gfx.rs"]
mod gfx;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(windows)]
mod lang {
    pub fn window_title() -> &'static str {
        "迷宮工房 · YYDungeon"
    }
}

#[cfg(windows)]
pub fn desktop_main() {
    app::main();
}
