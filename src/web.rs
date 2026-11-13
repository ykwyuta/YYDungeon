use crate::{
    dungeon::{Game, SAVE_LIMIT},
    gfx::Gfx,
    screen::{Event, Screen},
};
use std::cell::RefCell;

thread_local! {
    static GAME: RefCell<Option<Screen>> = const { RefCell::new(None) };
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    static LOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SAVE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[link(wasm_import_module = "env")]
extern "C" {
    fn browser_error(ptr: *const u8, len: usize);
}

#[no_mangle]
pub extern "C" fn yy_load_buffer(len: usize) -> *mut u8 {
    if len > SAVE_LIMIT {
        return std::ptr::null_mut();
    }
    LOAD.with_borrow_mut(|data| {
        data.resize(len, 0);
        data.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn yy_init(w: u32, h: u32) {
    std::panic::set_hook(Box::new(|panic| {
        let message = panic.to_string();
        unsafe { browser_error(message.as_ptr(), message.len()) };
    }));
    let bytes = LOAD.take();
    let game = Game::load(&bytes);
    GAME.with_borrow_mut(|slot| *slot = Some(Screen::new(Gfx::new(w, h), game)));
}

#[no_mangle]
pub extern "C" fn yy_event(kind: u32, x: f32, y: f32) {
    let event = match kind {
        0 => Event::Move(x, y),
        1 => Event::Click(x, y),
        2 => Event::Key(x as u32),
        3 => Event::Resize(x as u32, y as u32),
        _ => return,
    };
    EVENTS.with_borrow_mut(|events| events.push(event));
}

#[no_mangle]
pub extern "C" fn yy_frame(dt: f32, focused: u32) {
    GAME.with_borrow_mut(|slot| {
        if let Some(screen) = slot {
            EVENTS.with_borrow_mut(|events| {
                screen.frame(events, dt, focused != 0);
                events.clear();
            });
        }
    });
}

#[no_mangle]
pub extern "C" fn yy_can_autosave() -> u32 {
    GAME.with_borrow(|slot| u32::from(slot.as_ref().is_some_and(Screen::should_save)))
}

#[no_mangle]
pub extern "C" fn yy_save_size() -> usize {
    GAME.with_borrow(|slot| {
        let Some(screen) = slot else { return 0 };
        SAVE.with_borrow_mut(|data| {
            *data = screen.game.save();
            data.len()
        })
    })
}

#[no_mangle]
pub extern "C" fn yy_save_ptr() -> *const u8 {
    SAVE.with_borrow(|data| data.as_ptr())
}
