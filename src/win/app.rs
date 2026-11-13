use crate::{
    crashlog,
    gfx::Gfx,
    lang, platform,
    screen::{Event, Screen},
};
use std::cell::RefCell;
use std::time::{Duration, Instant};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

thread_local! {
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let point = || {
        (
            (lp.0 & 0xffff) as i16 as f32,
            ((lp.0 >> 16) & 0xffff) as i16 as f32,
        )
    };
    let event = match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
        WM_SIZE => Some(Event::Resize(
            (lp.0 & 0xffff) as u32,
            ((lp.0 >> 16) & 0xffff) as u32,
        )),
        WM_MOUSEMOVE => {
            let (x, y) = point();
            Some(Event::Move(x, y))
        }
        WM_LBUTTONDOWN => {
            let (x, y) = point();
            Some(Event::Click(x, y))
        }
        WM_KEYDOWN if lp.0 & (1 << 30) == 0 => Some(Event::Key(wp.0 as u32)),
        WM_ERASEBKGND => return LRESULT(1),
        _ => return unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    };
    if let Some(event) = event {
        EVENTS.with_borrow_mut(|events| events.push(event));
    }
    LRESULT(0)
}

pub fn main() {
    crashlog::init();
    match std::panic::catch_unwind(run) {
        Ok(Ok(())) => crashlog::clean_exit(),
        Ok(Err(error)) => {
            crashlog::report_failure(&format!("{error:?}"));
            std::process::exit(1);
        }
        Err(_) => {
            crashlog::report_failure("パニック");
            std::process::exit(101);
        }
    }
}

fn run() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(None)?;
        let class = w!("YYDungeonWnd");
        let icon = |x, y| {
            LoadImageW(
                Some(instance.into()),
                PCWSTR(1 as *const u16),
                IMAGE_ICON,
                x,
                y,
                LR_DEFAULTCOLOR,
            )
            .map(|handle| HICON(handle.0))
            .unwrap_or_default()
        };
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hIcon: icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)),
            hIconSm: icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let mut area = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut area as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let width = if area.right > area.left {
            (area.right - area.left - 44).clamp(640, 1440)
        } else {
            1240
        };
        let height = if area.bottom > area.top {
            (area.bottom - area.top - 60).clamp(440, 900)
        } else {
            780
        };
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            &HSTRING::from(lang::window_title()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        let gfx = Gfx::new(
            hwnd,
            client.right.max(1) as u32,
            client.bottom.max(1) as u32,
        )?;
        let mut screen = Screen::new(gfx, platform::load());
        let mut last = Instant::now();
        let mut saved = last;
        let mut state_at = last;
        let mut msg = MSG::default();
        'outer: loop {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'outer;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let now = Instant::now();
            let dt = (now - last).as_secs_f32().min(0.25);
            last = now;
            crashlog::tick();
            if !IsIconic(hwnd).as_bool() {
                EVENTS.with_borrow_mut(|events| {
                    for event in events.iter() {
                        match event {
                            Event::Click(x, y) => {
                                crashlog::crumb(format!("click ({x:.0}, {y:.0})"))
                            }
                            Event::Key(key) => crashlog::crumb(format!("key {key}")),
                            _ => {}
                        }
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        screen.frame(events, dt, GetForegroundWindow() == hwnd);
                    }));
                    if let Err(payload) = result {
                        crashlog::write_crash_save(&screen.game.save());
                        std::panic::resume_unwind(payload);
                    }
                    events.clear();
                });
            } else {
                std::thread::sleep(Duration::from_millis(35));
            }
            if now - state_at >= Duration::from_secs(1) {
                state_at = now;
                let game = &screen.game;
                crashlog::set_state(format!(
                    "floor={} wave={} kills={} boss={}",
                    game.floors.len(),
                    game.wave,
                    game.kills,
                    game.boss
                ));
            }
            if screen.should_save() && now - saved >= Duration::from_secs(2) {
                saved = now;
                if let Err(error) = platform::save(&screen.game) {
                    crashlog::warn_limited("save", &error.to_string());
                }
            }
        }
        if screen.should_save() {
            if let Err(error) = platform::save(&screen.game) {
                crashlog::warn_limited("save", &error.to_string());
            }
        }
    }
    Ok(())
}
