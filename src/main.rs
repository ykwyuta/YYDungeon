#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    yydungeon::desktop_main();
    #[cfg(not(windows))]
    eprintln!("Windows: cargo run --bin yydungeon-win. Web: node tools/build.mjs");
}
