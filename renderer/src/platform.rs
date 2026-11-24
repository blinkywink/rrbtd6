//! Operating-system glue that keeps the game working on PCs other than the one it is built on:
//! the discrete-GPU request on switchable-graphics laptops, one floating-point path on every CPU,
//! and plain-language dialogs instead of a window that just disappears.

/// NVIDIA Optimus / AMD PowerXpress: an exe exporting these runs on the discrete GPU of a
/// switchable-graphics laptop (both drivers read the exports at start; elsewhere they do nothing).
/// build.rs exports them (`/EXPORT:`).
#[cfg(windows)]
#[unsafe(no_mangle)]
#[used]
pub static NvOptimusEnablement: u32 = 1;
#[cfg(windows)]
#[unsafe(no_mangle)]
#[used]
pub static AmdPowerXpressRequestHighPerformance: i32 = 1;

/// The C runtime's math (sinf, atan2f ... the sim calls) on its plain SSE2 path on every CPU:
/// with FMA3 present it otherwise takes a different path, which may round differently, and co-op
/// needs every PC's simulation identical. The recordings match on both paths (regression run with
/// FMA3 off, 2026-11-24), so this only removes a way PCs could disagree.
pub fn lock_math() {
    #[cfg(all(windows, target_env = "msvc", target_arch = "x86_64"))]
    unsafe {
        unsafe extern "C" {
            fn _set_FMA3_enable(flag: i32) -> i32;
        }
        _set_FMA3_enable(0);
    }
}

/// A message box (blocking). `error`: the error icon, else the warning one.
pub fn message_box(title: &str, text: &str, error: bool) {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
        }
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        // MB_OK | (MB_ICONERROR or MB_ICONWARNING) | MB_SETFOREGROUND | MB_TOPMOST
        let flags = 0x0 | if error { 0x10 } else { 0x30 } | 0x10000 | 0x40000;
        unsafe {
            MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide(title).as_ptr(), flags);
        }
    }
    #[cfg(not(windows))]
    eprintln!("{title}: {text}");
}

/// What a panic says, for the player.
pub fn panic_reason(info: &std::panic::PanicHookInfo) -> String {
    let msg = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".into());
    match info.location() {
        Some(l) => format!("{msg}\n({}:{})", l.file(), l.line()),
        None => msg,
    }
}

/// After a crash on the game's main thread: what happened and where the details are (outside
/// autotests, which have no one to click OK).
pub fn crash_dialog(reason: &str, log: Option<&std::path::Path>) {
    if std::env::var_os("RRBTD6_AUTOTEST").is_some() || std::thread::current().name() != Some("main") {
        return;
    }
    let (title, text) = crash_text(reason, log);
    message_box(title, &text, true);
}

/// RRBTD6_COMPAT_TEST: every fallback a weaker PC can take, on this one (no instancing, no
/// compressed textures, no audio device), to check those paths still play.
pub fn compat_test_env() {
    if std::env::var_os("RRBTD6_COMPAT_TEST").is_none() {
        return;
    }
    for v in ["RRBTD6_NO_INSTANCING", "RRBTD6_NO_INSTANCED_BLOONS", "RRBTD6_NO_COMPRESSED_TEXTURES", "RRBTD6_NO_AUDIO_DEVICE"] {
        // SAFETY: called first thing in main, before any other thread exists.
        unsafe { std::env::set_var(v, "1") };
    }
}

/// The crash dialog's title and text. A failed OpenGL context (WGL) or a GL function the driver
/// lacks means the PC has no real graphics driver (fresh Windows, Remote Desktop, a virtual
/// machine): that gets its own explanation.
pub fn crash_text(reason: &str, log: Option<&std::path::Path>) -> (&'static str, String) {
    if reason.starts_with("WGL:") || (reason.contains("miniquad") && (reason.contains("native\\gl.rs") || reason.contains("native/gl.rs"))) {
        let text = format!(
            "RustTD6 couldn't start its graphics.\n\nIt needs a graphics driver with OpenGL 3.1 or newer, which every PC that runs BTD6 has \
             once its graphics driver is installed.\n\nWhat to do:\n\
             \u{2022} Install the latest driver for your graphics card from NVIDIA, AMD or Intel (or through Windows Update).\n\
             \u{2022} If you are connected through Remote Desktop or a virtual machine, start RustTD6 on the PC itself.\n\n\
             Technical detail: {reason}"
        );
        return ("RustTD6: graphics driver needed", text);
    }
    let mut text = format!("RustTD6 stopped because of an error.\n\n{reason}");
    if let Some(log) = log {
        text.push_str(&format!("\n\nThe details were saved to:\n{}\n\nPlease include that file if you report this.", log.display()));
    }
    ("RustTD6 crashed", text)
}

#[cfg(test)]
mod tests {
    #[test]
    fn graphics_failures_get_the_driver_message() {
        let t = |r: &str| super::crash_text(r, None).0;
        assert_eq!(t("WGL: ARB_create_context required!\n(vendor\\miniquad\\src\\native\\windows\\wgl.rs:401:13)"), "RustTD6: graphics driver needed");
        assert_eq!(t("called `Option::unwrap()` on a `None` value\n(C:\\x\\vendor\\miniquad\\src\\native\\gl.rs:324:17)"), "RustTD6: graphics driver needed");
        assert_eq!(t("index out of bounds\n(renderer\\src\\game.rs:1:1)"), "RustTD6 crashed");
    }
}
