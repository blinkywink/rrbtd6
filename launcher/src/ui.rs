//! The setup window: one small native window that shows what the launcher is doing (a stage, a
//! bar, the time left and rotating tips) and, when something goes wrong, a plain message with
//! buttons in the same window. Everything the window shows also goes to setup.log.
//!
//! The window runs on its own thread; the launcher calls `stage`, `line` and `ask` from its own.
//! It's opened only when there is something to wait for (installing, updating, extracting), so an
//! up-to-date install starts the game without any window. Without a window (it couldn't be
//! created) `ask` falls back to a message box.
use std::sync::{Condvar, Mutex, OnceLock};

const TIPS: &[&str] = &[
    "Copying Dart Monkeys...", "Inflating bloons...", "Sharpening tacks...", "Teaching the MOAB to sit still...",
    "Watering the Banana Farm...", "Giving the Glue Gunner a bath...", "Herding camo bloons...", "Polishing the Sun God...",
    "Rusting the code...", "Asking the Monkey Village for directions...", "Refilling mortar shells...", "Hiding from the BAD...",
    "Restocking Geraldo's shop...", "Untangling boomerangs...", "Counting every pop...", "Waking up the Ninja Monkey...",
    "Defrosting the Ice Monkey...", "Charging the Super Monkey's laser eyes...", "Bribing the Heli Pilot...",
];

#[derive(Default)]
#[cfg_attr(not(windows), allow(dead_code))]
struct State {
    open: bool,
    hwnd: isize,
    stage: String,
    percent: Option<f32>,
    eta: String,
    tip: usize,
    ticks: u32,
    sweep: f32,
    // Message mode: a heading, a body and buttons; `choice` is set when one is clicked.
    message: Option<(String, String, Vec<String>)>,
    choice: Option<usize>,
    // Tick boxes shown with a message (label, ticked).
    checks: Vec<(String, bool)>,
    // Button and tick box rectangles from the last paint, for clicks.
    hits: Vec<(i32, i32, i32, i32)>,
    check_hits: Vec<(i32, i32, i32, i32)>,
    hover: Option<usize>,
}

fn state() -> &'static (Mutex<State>, Condvar) {
    static S: OnceLock<(Mutex<State>, Condvar)> = OnceLock::new();
    S.get_or_init(|| (Mutex::new(State { stage: "Getting ready".into(), ..Default::default() }), Condvar::new()))
}

/// Shows the window (once; later calls do nothing).
pub fn open() {
    #[cfg_attr(not(windows), allow(unused_variables))]
    let (lock, cv) = state();
    {
        let mut s = lock.lock().unwrap();
        // Started by the old BTD6 setup mod, which shows its own progress window.
        if s.open || std::env::args().any(|a| a == "--from-installer") || crate::unattended() {
            return;
        }
        s.open = true;
        s.tip = (std::process::id() as usize) % TIPS.len();
    }
    #[cfg(windows)]
    {
        std::thread::spawn(win::run);
        // Wait (briefly) until the window exists, so the first stage shows.
        let s = lock.lock().unwrap();
        let _ = cv.wait_timeout_while(s, std::time::Duration::from_secs(3), |s| s.hwnd == 0 && s.open).unwrap();
    }
}

pub fn is_open() -> bool {
    state().0.lock().unwrap().hwnd != 0
}

/// The current stage, in plain words ("Downloading RustTD6").
pub fn stage(text: &str) {
    crate::log(text);
    let mut s = state().0.lock().unwrap();
    s.stage = text.trim().trim_end_matches('.').to_string();
    s.percent = None;
    s.eta.clear();
    s.message = None;
    repaint(&s);
}

/// One line of extraction output: its progress lines move the bar
/// ("[###---]  12.3% | Preparing maps | ETA: ~5m (estimate)").
pub fn line(text: &str) {
    let t = text.trim();
    let Some(rest) = t.strip_prefix('[').and_then(|r| r.split_once(']')).map(|(_, r)| r) else { return };
    let parts: Vec<&str> = rest.split('|').map(str::trim).collect();
    if parts.len() < 3 {
        return;
    }
    let Ok(p) = parts[0].trim_end_matches('%').trim().parse::<f32>() else { return };
    let eta = parts[2].trim_start_matches("ETA:").trim().replace(" (estimate)", "");
    let eta = if let Some(m) = eta.strip_prefix('~') {
        format!("About {m} left")
    } else if eta.starts_with("estimating") {
        "Working out the time left...".into()
    } else {
        String::new()
    };
    let mut s = state().0.lock().unwrap();
    s.stage = parts[1].to_string();
    s.percent = Some(p.clamp(0.0, 100.0));
    s.eta = eta;
    repaint(&s);
}

/// Shows a message with buttons and waits for one; returns its index. Closing the window counts
/// as the last button.
pub fn ask(heading: &str, body: &str, buttons: &[&str]) -> usize {
    ask_checks(heading, body, &[], buttons).0
}

/// `ask` with tick boxes under the message; returns the button and each box's final state.
pub fn ask_checks(heading: &str, body: &str, checks: &[(&str, bool)], buttons: &[&str]) -> (usize, Vec<bool>) {
    crate::log(&format!("{heading}: {body}"));
    let defaults: Vec<bool> = checks.iter().map(|c| c.1).collect();
    if crate::unattended() {
        return (0, defaults);
    }
    if !is_open() {
        open();
    }
    let (lock, cv) = state();
    let mut s = lock.lock().unwrap();
    if s.hwnd == 0 {
        drop(s);
        return (fallback_ask(heading, body, buttons), defaults);
    }
    s.message = Some((heading.into(), body.into(), buttons.iter().map(|b| b.to_string()).collect()));
    s.checks = checks.iter().map(|(l, v)| (l.to_string(), *v)).collect();
    s.choice = None;
    s.hover = None;
    repaint(&s);
    #[cfg(windows)]
    win::bring_to_front(s.hwnd);
    let mut s = cv.wait_while(s, |s| s.choice.is_none() && s.hwnd != 0).unwrap();
    let ticked = if s.checks.is_empty() { defaults } else { s.checks.iter().map(|c| c.1).collect() };
    s.checks.clear();
    (s.choice.unwrap_or(buttons.len().saturating_sub(1)), ticked)
}

pub fn close() {
    let mut s = state().0.lock().unwrap();
    s.open = false;
    #[cfg(windows)]
    if s.hwnd != 0 {
        win::close(s.hwnd);
    }
    s.hwnd = 0;
}

fn repaint(s: &State) {
    #[cfg(windows)]
    if s.hwnd != 0 {
        win::invalidate(s.hwnd);
    }
    #[cfg(not(windows))]
    let _ = s;
}

fn fallback_ask(heading: &str, body: &str, buttons: &[&str]) -> usize {
    // Two-choice questions as Yes / No; anything else as OK (the first button).
    let text = format!("{heading}\n\n{body}");
    if buttons.len() >= 2 {
        let r = crate::system::ask("RustTD6", &format!("{text}\n\nYes: {}\nNo: {}", buttons[0], buttons[1]), true);
        return match r { 6 => 0, 7 => 1, _ => buttons.len() - 1 };
    }
    crate::system::ask("RustTD6", &text, false);
    0
}

#[cfg(windows)]
mod win {
    use super::{state, TIPS};
    use std::ffi::c_void;

    type Hwnd = isize;
    #[repr(C)]
    struct WndClass { size: u32, style: u32, proc_: unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize, cls: i32, wnd: i32,
        instance: isize, icon: isize, cursor: isize, brush: isize, menu: *const u16, name: *const u16, small_icon: isize }
    #[repr(C)]
    struct Msg { hwnd: Hwnd, message: u32, wparam: usize, lparam: isize, time: u32, x: i32, y: i32, private: u32 }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect { l: i32, t: i32, r: i32, b: i32 }
    #[repr(C)]
    struct TrackMouse { size: u32, flags: u32, hwnd: Hwnd, hover: u32 }
    #[repr(C)]
    struct Paint { dc: isize, erase: i32, rc: Rect, restore: i32, update: i32, reserved: [u8; 32] }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassExW(c: *const WndClass) -> u16;
        fn CreateWindowExW(ex: u32, cls: *const u16, title: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Hwnd, menu: isize, inst: isize, p: *mut c_void) -> Hwnd;
        fn DefWindowProcW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
        fn GetMessageW(m: *mut Msg, h: Hwnd, a: u32, b: u32) -> i32;
        fn TranslateMessage(m: *const Msg) -> i32;
        fn DispatchMessageW(m: *const Msg) -> isize;
        fn PostQuitMessage(code: i32);
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn DestroyWindow(h: Hwnd) -> i32;
        fn PostMessageW(h: Hwnd, m: u32, w: usize, l: isize) -> i32;
        fn InvalidateRect(h: Hwnd, r: *const Rect, erase: i32) -> i32;
        fn BeginPaint(h: Hwnd, p: *mut Paint) -> isize;
        fn EndPaint(h: Hwnd, p: *const Paint) -> i32;
        fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
        fn SetTimer(h: Hwnd, id: usize, ms: u32, f: isize) -> usize;
        fn KillTimer(h: Hwnd, id: usize) -> i32;
        fn LoadCursorW(inst: isize, name: usize) -> isize;
        fn LoadIconW(inst: isize, name: usize) -> isize;
        fn GetSystemMetrics(i: i32) -> i32;
        fn GetDpiForWindow(h: Hwnd) -> u32;
        fn SetProcessDpiAwarenessContext(ctx: isize) -> i32;
        fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, w: i32, cy: i32, flags: u32) -> i32;
        fn AdjustWindowRectExForDpi(r: *mut Rect, style: u32, menu: i32, ex: u32, dpi: u32) -> i32;
        fn GetWindowRect(h: Hwnd, r: *mut Rect) -> i32;
        fn SetForegroundWindow(h: Hwnd) -> i32;
        fn FillRect(dc: isize, r: *const Rect, brush: isize) -> i32;
        fn DrawTextW(dc: isize, text: *const u16, n: i32, r: *mut Rect, format: u32) -> i32;
        fn MessageBoxW(h: *mut c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
        fn TrackMouseEvent(t: *mut TrackMouse) -> i32;
        fn SetCursor(c: isize) -> isize;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateSolidBrush(c: u32) -> isize;
        fn DeleteObject(o: isize) -> i32;
        fn SelectObject(dc: isize, o: isize) -> isize;
        fn CreateFontW(h: i32, w: i32, e: i32, o: i32, weight: i32, i: u32, u: u32, s: u32, cs: u32, op: u32, cp: u32, q: u32, pf: u32, face: *const u16) -> isize;
        fn SetTextColor(dc: isize, c: u32) -> u32;
        fn SetBkMode(dc: isize, m: i32) -> i32;
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn CreateCompatibleBitmap(dc: isize, w: i32, h: i32) -> isize;
        fn BitBlt(dst: isize, x: i32, y: i32, w: i32, h: i32, src: isize, sx: i32, sy: i32, rop: u32) -> i32;
        fn DeleteDC(dc: isize) -> i32;
        fn RoundRect(dc: isize, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> i32;
        fn GetStockObject(i: i32) -> isize;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }

    // Colours as 0x00BBGGRR: the site's blue, white and ink.
    const BLUE: u32 = 0x00F5802F;
    const BLUE_DARK: u32 = 0x00CC5F1B;
    const TRACK: u32 = 0x00F7E7DD;
    const WHITE: u32 = 0x00FFFFFF;
    const INK: u32 = 0x006B3013;
    const MUTED: u32 = 0x00866F4A;
    const SOFT: u32 = 0x00FFF2EA;
    const WIDTH: i32 = 520;
    const STYLE: u32 = 0x00C00000 | 0x00080000 | 0x00020000; // WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    pub fn invalidate(h: Hwnd) {
        unsafe { InvalidateRect(h, std::ptr::null(), 0) };
    }
    pub fn close(h: Hwnd) {
        unsafe { PostMessageW(h, 0x0010 + 0x0400, 0, 0) }; // a private "close now" message
    }
    pub fn bring_to_front(h: Hwnd) {
        unsafe {
            ShowWindow(h, 9); // SW_RESTORE
            SetForegroundWindow(h);
        }
    }

    pub fn run() {
        unsafe {
            SetProcessDpiAwarenessContext(-4); // per-monitor v2
            let inst = GetModuleHandleW(std::ptr::null());
            let name = wide("RustTD6Setup");
            let class = WndClass { size: std::mem::size_of::<WndClass>() as u32, style: 0x3, proc_: proc_, cls: 0, wnd: 0, instance: inst,
                icon: LoadIconW(inst, 1).max(LoadIconW(0, 32512)), cursor: LoadCursorW(0, 32512), brush: 0, menu: std::ptr::null(),
                name: name.as_ptr(), small_icon: 0 };
            RegisterClassExW(&class);
            let (w, h) = (WIDTH, 250);
            let hwnd = CreateWindowExW(0, name.as_ptr(), wide("RustTD6 Setup").as_ptr(), STYLE,
                (GetSystemMetrics(0) - w) / 2, (GetSystemMetrics(1) - h) / 2, w, h, 0, 0, inst, std::ptr::null_mut());
            let (lock, cv) = state();
            if hwnd == 0 {
                lock.lock().unwrap().open = false;
                cv.notify_all();
                return;
            }
            lock.lock().unwrap().hwnd = hwnd;
            resize(hwnd, 250);
            ShowWindow(hwnd, 5);
            SetTimer(hwnd, 1, 40, 0);
            cv.notify_all();
            let mut msg: Msg = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let mut s = lock.lock().unwrap();
            s.hwnd = 0;
            cv.notify_all();
        }
    }

    fn scale(h: Hwnd, v: i32) -> i32 {
        let dpi = unsafe { GetDpiForWindow(h) }.max(96) as i32;
        v * dpi / 96
    }

    /// Sets the client area to WIDTH x `height` (logical pixels) at this window's DPI.
    fn resize(h: Hwnd, height: i32) {
        unsafe {
            let dpi = GetDpiForWindow(h).max(96);
            let mut r = Rect { l: 0, t: 0, r: scale(h, WIDTH), b: scale(h, height) };
            AdjustWindowRectExForDpi(&mut r, STYLE, 0, 0, dpi);
            let mut cur = Rect::default();
            GetWindowRect(h, &mut cur);
            let (w, hh) = (r.r - r.l, r.b - r.t);
            let cx = (cur.l + cur.r) / 2;
            let cy = (cur.t + cur.b) / 2;
            SetWindowPos(h, 0, cx - w / 2, cy - hh / 2, w, hh, 0x4 | 0x10); // NOZORDER | NOACTIVATE
        }
    }

    unsafe extern "system" fn proc_(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
        unsafe {
            match m {
                0x0014 => return 1, // WM_ERASEBKGND: painted with the rest
                0x0113 => {
                    // WM_TIMER: the moving bar, and a new tip every ~4 s.
                    let mut s = state().0.lock().unwrap();
                    s.sweep = (s.sweep + 0.012) % 1.3;
                    s.ticks += 1;
                    if s.ticks % 100 == 0 {
                        s.tip = (s.tip + 7) % TIPS.len();
                    }
                    if s.message.is_none() {
                        InvalidateRect(h, std::ptr::null(), 0);
                    }
                    return 0;
                }
                0x0200 => {
                    // WM_MOUSEMOVE: highlight the button under the pointer.
                    let (x, y) = ((l & 0xFFFF) as i16 as i32, ((l >> 16) & 0xFFFF) as i16 as i32);
                    let mut s = state().0.lock().unwrap();
                    let hover = s.hits.iter().position(|&(a, b, c, d)| x >= a && x < c && y >= b && y < d);
                    if hover.is_some() {
                        SetCursor(LoadCursorW(0, 32649)); // hand
                    }
                    if hover != s.hover {
                        s.hover = hover;
                        let mut t = TrackMouse { size: std::mem::size_of::<TrackMouse>() as u32, flags: 2, hwnd: h, hover: 0 }; // TME_LEAVE
                        TrackMouseEvent(&mut t);
                        InvalidateRect(h, std::ptr::null(), 0);
                    }
                    return 0;
                }
                0x02A3 => {
                    // WM_MOUSELEAVE
                    let mut s = state().0.lock().unwrap();
                    if s.hover.take().is_some() {
                        InvalidateRect(h, std::ptr::null(), 0);
                    }
                    return 0;
                }
                0x0202 => {
                    // WM_LBUTTONUP: a button.
                    let (x, y) = ((l & 0xFFFF) as i16 as i32, ((l >> 16) & 0xFFFF) as i16 as i32);
                    let (lock, cv) = state();
                    let mut s = lock.lock().unwrap();
                    if s.message.is_some() {
                        if let Some(i) = s.check_hits.iter().position(|&(a, b, c, d)| x >= a && x < c && y >= b && y < d) {
                            if let Some(c) = s.checks.get_mut(i) {
                                c.1 = !c.1;
                            }
                            InvalidateRect(h, std::ptr::null(), 0);
                        } else if let Some(i) = s.hits.iter().position(|&(a, b, c, d)| x >= a && x < c && y >= b && y < d) {
                            s.choice = Some(i);
                            cv.notify_all();
                        }
                    }
                    return 0;
                }
                0x0010 => {
                    // WM_CLOSE by the player: during a message it's the last button; while working,
                    // confirm, because setup stops (finished stages are kept).
                    let (lock, cv) = state();
                    let mut s = lock.lock().unwrap();
                    if let Some((_, _, buttons)) = &s.message {
                        s.choice = Some(buttons.len().saturating_sub(1));
                        cv.notify_all();
                        return 0;
                    }
                    drop(s);
                    let text = wide("Stop setting up RustTD6?\n\nEverything finished so far is kept, so next time it carries on from here.");
                    if MessageBoxW(h as *mut c_void, text.as_ptr(), wide("RustTD6 Setup").as_ptr(), 0x4 | 0x30) == 6 {
                        crate::stop_children();
                        std::process::exit(0);
                    }
                    return 0;
                }
                0x0410 => {
                    // Our close: the launcher is done with the window.
                    KillTimer(h, 1);
                    DestroyWindow(h);
                    return 0;
                }
                0x0002 => {
                    PostQuitMessage(0);
                    return 0;
                }
                0x02E0 => {
                    // WM_DPICHANGED: keep the same logical size.
                    InvalidateRect(h, std::ptr::null(), 0);
                }
                0x000F => {
                    paint(h);
                    return 0;
                }
                _ => {}
            }
            DefWindowProcW(h, m, w, l)
        }
    }

    unsafe fn font(h: Hwnd, size: i32, weight: i32) -> isize {
        unsafe { CreateFontW(-scale(h, size), 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, wide("Segoe UI").as_ptr()) }
    }

    unsafe fn fill(dc: isize, r: Rect, color: u32, radius: i32) {
        unsafe {
            let brush = CreateSolidBrush(color);
            if radius <= 0 {
                FillRect(dc, &r, brush);
            } else {
                let old_b = SelectObject(dc, brush);
                let old_p = SelectObject(dc, GetStockObject(8)); // NULL_PEN
                RoundRect(dc, r.l, r.t, r.r + 1, r.b + 1, radius, radius);
                SelectObject(dc, old_b);
                SelectObject(dc, old_p);
            }
            DeleteObject(brush);
        }
    }

    unsafe fn text(dc: isize, f: isize, s: &str, r: &mut Rect, color: u32, format: u32) -> i32 {
        unsafe {
            let old = SelectObject(dc, f);
            SetTextColor(dc, color);
            // Never an empty buffer: DrawTextW reads the first character even for a count of 0.
            if s.is_empty() {
                SelectObject(dc, old);
                return 0;
            }
            let w: Vec<u16> = s.encode_utf16().collect();
            let height = DrawTextW(dc, w.as_ptr(), w.len() as i32, r, format);
            SelectObject(dc, old);
            height
        }
    }

    fn paint(h: Hwnd) {
        unsafe {
            let mut ps: Paint = std::mem::zeroed();
            let dc = BeginPaint(h, &mut ps);
            let mut client = Rect::default();
            GetClientRect(h, &mut client);
            let (cw, ch) = (client.r, client.b);
            let mem = CreateCompatibleDC(dc);
            let bmp = CreateCompatibleBitmap(dc, cw.max(1), ch.max(1));
            let old_bmp = SelectObject(mem, bmp);
            SetBkMode(mem, 1);
            let px = |v: i32| scale(h, v);
            let title_f = font(h, 22, 700);
            let head_f = font(h, 17, 600);
            let body_f = font(h, 14, 400);
            let small_f = font(h, 13, 400);
            let btn_f = font(h, 14, 600);
            let tick_f = CreateFontW(-scale(h, 14), 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, wide("Segoe UI Symbol").as_ptr());

            fill(mem, client, WHITE, 0);
            fill(mem, Rect { l: 0, t: 0, r: cw, b: px(64) }, BLUE, 0);
            let pad = px(28);
            let mut r = Rect { l: pad, t: 0, r: cw - pad, b: px(64) };
            text(mem, title_f, "RustTD6", &mut r, WHITE, 0x20 | 0x4); // SINGLELINE | VCENTER

            let mut s = state().0.lock().unwrap();
            let mut want_height = 250;
            s.hits.clear();
            s.check_hits.clear();
            if let Some((heading, body, buttons)) = s.message.clone() {
                let mut r = Rect { l: pad, t: px(86), r: cw - pad, b: px(120) };
                text(mem, head_f, &heading, &mut r, INK, 0x10 | 0x400); // WORDBREAK | CALCRECT
                let head_h = r.b - r.t;
                let mut r = Rect { l: pad, t: px(86), r: cw - pad, b: px(86) + head_h };
                text(mem, head_f, &heading, &mut r, INK, 0x10);
                let body_top = px(86) + head_h + px(10);
                let mut r = Rect { l: pad, t: body_top, r: cw - pad, b: body_top };
                text(mem, body_f, &body, &mut r, MUTED, 0x10 | 0x400);
                let body_h = r.b - r.t;
                let mut r = Rect { l: pad, t: body_top, r: cw - pad, b: body_top + body_h };
                text(mem, body_f, &body, &mut r, MUTED, 0x10);
                // Buttons, right-aligned rows (wrapping when they don't fit on one); the first is
                // the main one.
                // Tick boxes, one per row, the whole row clickable.
                let mut y = body_top + body_h + px(16);
                let checks = s.checks.clone();
                for (label, on) in &checks {
                    let size = px(20);
                    let row = Rect { l: pad, t: y, r: cw - pad, b: y + px(30) };
                    let bx = Rect { l: pad, t: y + px(5), r: pad + size, b: y + px(5) + size };
                    if *on {
                        fill(mem, bx, BLUE, px(6));
                        let mut t = bx;
                        text(mem, tick_f, "✓", &mut t, WHITE, 0x1 | 0x20 | 0x4);
                    } else {
                        fill(mem, bx, MUTED, px(6));
                        fill(mem, Rect { l: bx.l + px(2), t: bx.t + px(2), r: bx.r - px(2), b: bx.b - px(2) }, WHITE, px(4));
                    }
                    let mut t = Rect { l: bx.r + px(10), ..row };
                    text(mem, body_f, label, &mut t, INK, 0x20 | 0x4);
                    s.check_hits.push((row.l, row.t, row.r, row.b));
                    y += px(32);
                }
                let by = if checks.is_empty() { body_top + body_h + px(24) } else { y + px(12) };
                let bh = px(38);
                let (gap, avail) = (px(10), cw - 2 * pad);
                let widths: Vec<i32> = buttons.iter().map(|label| {
                    let mut m = Rect { l: 0, t: 0, r: px(400), b: bh };
                    text(mem, btn_f, label, &mut m, INK, 0x20 | 0x400);
                    (m.r - m.l) + px(32)
                }).collect();
                let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
                let mut used = 0;
                for (i, w) in widths.iter().enumerate() {
                    let need = if rows.last().unwrap().is_empty() { *w } else { used + gap + w };
                    if need > avail && !rows.last().unwrap().is_empty() {
                        rows.push(Vec::new());
                        used = *w;
                    } else {
                        used = need;
                    }
                    rows.last_mut().unwrap().push(i);
                }
                let mut rects = Vec::new();
                let mut y = by;
                for row in &rows {
                    let total: i32 = row.iter().map(|&i| widths[i]).sum::<i32>() + gap * (row.len() as i32 - 1);
                    let mut x = cw - pad - total;
                    for &i in row {
                        rects.push((i, Rect { l: x, t: y, r: x + widths[i], b: y + bh }));
                        x += widths[i] + gap;
                    }
                    y += bh + gap;
                }
                let by = y - gap - bh;
                for (i, b) in &rects {
                    let hover = s.hover == Some(*i);
                    let (bg, fg) = if *i == 0 { (if hover { BLUE_DARK } else { BLUE }, WHITE) } else { (if hover { TRACK } else { SOFT }, INK) };
                    fill(mem, *b, bg, px(12));
                    let mut t = *b;
                    text(mem, btn_f, &buttons[*i], &mut t, fg, 0x1 | 0x20 | 0x4); // CENTER
                    s.hits.push((b.l, b.t, b.r, b.b));
                }
                want_height = (by + bh + px(26)) * 96 / (GetDpiForWindow(h).max(96) as i32);
            } else {
                let mut r = Rect { l: pad, t: px(88), r: cw - pad, b: px(116) };
                text(mem, head_f, &s.stage, &mut r, INK, 0x20 | 0x4 | 0x8000); // END_ELLIPSIS
                let bar = Rect { l: pad, t: px(126), r: cw - pad, b: px(140) };
                fill(mem, bar, TRACK, px(14));
                let width = bar.r - bar.l;
                match s.percent {
                    Some(p) => {
                        let f = ((width as f32) * p / 100.0) as i32;
                        if f > 0 {
                            fill(mem, Rect { r: bar.l + f.max(px(14)), ..bar }, BLUE, px(14));
                        }
                    }
                    None => {
                        let seg = width / 4;
                        let start = bar.l + ((s.sweep - 0.3) * width as f32) as i32;
                        let (a, b) = (start.max(bar.l), (start + seg).min(bar.r));
                        if b - a > px(7) {
                            fill(mem, Rect { l: a, r: b, ..bar }, BLUE, px(14));
                        }
                    }
                }
                let mut r = Rect { l: pad, t: px(148), r: cw - pad, b: px(170) };
                if let Some(p) = s.percent {
                    text(mem, body_f, &format!("{}%", p as i32), &mut r, MUTED, 0x20);
                }
                let mut r = Rect { l: pad, t: px(148), r: cw - pad, b: px(170) };
                text(mem, body_f, &s.eta, &mut r, MUTED, 0x2 | 0x20); // RIGHT
                let mut r = Rect { l: pad, t: px(184), r: cw - pad, b: px(206) };
                text(mem, small_f, TIPS[s.tip % TIPS.len()], &mut r, MUTED, 0x1 | 0x20);
                let mut r = Rect { l: pad, t: px(214), r: cw - pad, b: px(236) };
                text(mem, small_f, "You can minimise this window. Closing it pauses setup.", &mut r, 0x00B49A86, 0x1 | 0x20);
            }
            drop(s);
            BitBlt(dc, 0, 0, cw, ch, mem, 0, 0, 0x00CC0020);
            SelectObject(mem, old_bmp);
            DeleteObject(bmp);
            DeleteDC(mem);
            for f in [title_f, head_f, body_f, small_f, btn_f, tick_f] {
                DeleteObject(f);
            }
            EndPaint(h, &ps);
            // Grow or shrink to fit a message.
            let logical = ch * 96 / (GetDpiForWindow(h).max(96) as i32);
            if (logical - want_height).abs() > 2 {
                resize(h, want_height);
            }
        }
    }
}
