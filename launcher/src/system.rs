//! Windows facts the launcher needs on any PC: where BTD6 is (Steam on any drive, Epic), the
//! system proxy, free disk space, file hashes and plain dialogs. No extra crates: the Win32 calls
//! are declared here.
use std::path::{Path, PathBuf};

#[cfg(windows)]
mod ffi {
    #[link(name = "advapi32")]
    unsafe extern "system" {
        pub fn RegGetValueW(hkey: isize, sub: *const u16, value: *const u16, flags: u32, ty: *mut u32, data: *mut u8, len: *mut u32) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    #[link(name = "bcrypt")]
    unsafe extern "system" {
        pub fn BCryptOpenAlgorithmProvider(alg: *mut *mut std::ffi::c_void, id: *const u16, imp: *const u16, flags: u32) -> i32;
        pub fn BCryptHash(alg: *mut std::ffi::c_void, secret: *const u8, secret_len: u32, input: *const u8, input_len: u32, out: *mut u8, out_len: u32) -> i32;
        pub fn BCryptCloseAlgorithmProvider(alg: *mut std::ffi::c_void, flags: u32) -> i32;
    }
    pub const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    pub const HKEY_LOCAL_MACHINE: isize = 0x8000_0002u32 as i32 as isize;
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn wide_path(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// A REG_SZ value (None: missing).
#[cfg(windows)]
fn reg_string(machine: bool, key: &str, value: &str) -> Option<String> {
    let root = if machine { ffi::HKEY_LOCAL_MACHINE } else { ffi::HKEY_CURRENT_USER };
    let (k, v) = (wide(key), wide(value));
    let mut len = 0u32;
    // RRF_RT_REG_SZ
    unsafe {
        if ffi::RegGetValueW(root, k.as_ptr(), v.as_ptr(), 0x2, std::ptr::null_mut(), std::ptr::null_mut(), &mut len) != 0 || len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize / 2 + 1];
        if ffi::RegGetValueW(root, k.as_ptr(), v.as_ptr(), 0x2, std::ptr::null_mut(), buf.as_mut_ptr() as *mut u8, &mut len) != 0 {
            return None;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..n]))
    }
}

/// A REG_DWORD value.
#[cfg(windows)]
fn reg_dword(key: &str, value: &str) -> Option<u32> {
    let (k, v) = (wide(key), wide(value));
    let mut data = 0u32;
    let mut len = 4u32;
    // RRF_RT_REG_DWORD
    let ok = unsafe { ffi::RegGetValueW(ffi::HKEY_CURRENT_USER, k.as_ptr(), v.as_ptr(), 0x10, std::ptr::null_mut(), &mut data as *mut u32 as *mut u8, &mut len) } == 0;
    ok.then_some(data)
}

fn is_btd6(p: &Path) -> bool {
    p.join("GameAssembly.dll").is_file()
}

/// Steam library folders (`steamapps/libraryfolders.vdf`'s "path" entries), the Steam folder first.
fn steam_libraries(steam: &Path) -> Vec<PathBuf> {
    let mut out = vec![steam.to_path_buf()];
    if let Ok(text) = std::fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")) {
        for line in text.lines() {
            let parts: Vec<&str> = line.split('"').collect();
            // \t"path"\t\t"D:\\SteamLibrary"
            if parts.len() >= 4 && parts[1].eq_ignore_ascii_case("path") {
                let p = PathBuf::from(parts[3].replace("\\\\", "\\"));
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// Epic install locations (its launcher's `.item` manifests) named Bloons TD 6.
fn epic_installs() -> Vec<PathBuf> {
    let dir = std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/ProgramData"));
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir.join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests")) else { return out };
    for e in entries.flatten() {
        if e.path().extension().is_some_and(|x| x == "item") {
            if let Ok(v) = std::fs::read_to_string(e.path()).map(|s| serde_json::from_str::<serde_json::Value>(&s).unwrap_or_default()) {
                let name = v["DisplayName"].as_str().unwrap_or("");
                if name.to_ascii_lowercase().contains("bloons td 6") {
                    if let Some(at) = v["InstallLocation"].as_str() {
                        out.push(PathBuf::from(at));
                    }
                }
            }
        }
    }
    out
}

/// The BTD6 install: Steam (any library on any drive), Epic, then the default folders.
pub fn find_btd6() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        let steam = reg_string(false, "Software\\Valve\\Steam", "SteamPath")
            .or_else(|| reg_string(true, "SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"))
            .or_else(|| reg_string(true, "SOFTWARE\\Valve\\Steam", "InstallPath"));
        if let Some(steam) = steam {
            for lib in steam_libraries(Path::new(&steam)) {
                candidates.push(lib.join("steamapps").join("common").join("BloonsTD6"));
            }
        }
    }
    candidates.extend(epic_installs());
    candidates.push(PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6"));
    candidates.push(PathBuf::from("C:/Program Files/Epic Games/BloonsTD6"));
    candidates.into_iter().find(|p| is_btd6(p))
}

/// The proxy to reach GitHub / python.org through: HTTPS_PROXY / ALL_PROXY, else the Windows
/// system proxy (Internet Settings, "Use a proxy server").
pub fn proxy() -> Option<String> {
    for name in ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
        if let Some(v) = std::env::var(name).ok().filter(|v| !v.trim().is_empty()) {
            return Some(v);
        }
    }
    #[cfg(windows)]
    {
        const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
        if reg_dword(KEY, "ProxyEnable") == Some(1) {
            let server = reg_string(false, KEY, "ProxyServer")?;
            // "host:port", or "http=host:port;https=host:port;...".
            let pick = if server.contains('=') {
                let entries: Vec<(&str, &str)> = server.split(';').filter_map(|e| e.split_once('=')).collect();
                entries.iter().find(|e| e.0.eq_ignore_ascii_case("https")).or_else(|| entries.iter().find(|e| e.0.eq_ignore_ascii_case("http"))).map(|e| e.1.to_string())?
            } else {
                server
            };
            let pick = pick.trim().to_string();
            if !pick.is_empty() {
                return Some(if pick.contains("://") { pick } else { format!("http://{pick}") });
            }
        }
    }
    None
}

/// Bytes free to this user on the drive holding `dir` (None: it can't say).
pub fn free_bytes(dir: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        let mut avail = 0u64;
        let ok = unsafe { ffi::GetDiskFreeSpaceExW(wide_path(dir).as_ptr(), &mut avail, std::ptr::null_mut(), std::ptr::null_mut()) } != 0;
        ok.then_some(avail)
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
        None
    }
}

/// SHA-256 of `bytes`, lowercase hex (Windows' own BCrypt).
pub fn sha256_hex(bytes: &[u8]) -> Option<String> {
    #[cfg(windows)]
    unsafe {
        let mut alg = std::ptr::null_mut();
        if ffi::BCryptOpenAlgorithmProvider(&mut alg, wide("SHA256").as_ptr(), std::ptr::null(), 0) != 0 {
            return None;
        }
        let mut out = [0u8; 32];
        let mut done = true;
        // BCryptHash takes a u32 length: hash in one call when it fits (every release file does).
        if bytes.len() > u32::MAX as usize || ffi::BCryptHash(alg, std::ptr::null(), 0, bytes.as_ptr(), bytes.len() as u32, out.as_mut_ptr(), 32) != 0 {
            done = false;
        }
        ffi::BCryptCloseAlgorithmProvider(alg, 0);
        done.then(|| out.iter().map(|b| format!("{b:02x}")).collect())
    }
    #[cfg(not(windows))]
    {
        let _ = bytes;
        None
    }
}

/// A message box with Yes / No / Cancel (6 / 7 / 2), or OK.
pub fn ask(title: &str, text: &str, yes_no_cancel: bool) -> i32 {
    #[cfg(windows)]
    {
        // MB_YESNOCANCEL | MB_ICONQUESTION, or MB_OK | MB_ICONWARNING; MB_SETFOREGROUND
        let flags = if yes_no_cancel { 0x3 | 0x20 } else { 0x30 } | 0x10000;
        unsafe { ffi::MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide(title).as_ptr(), flags) }
    }
    #[cfg(not(windows))]
    {
        println!("{title}: {text}");
        if yes_no_cancel { 7 } else { 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_library_paths_parse_with_escaped_backslashes() {
        let dir = std::env::temp_dir().join(format!("rrbtd6-vdf-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("steamapps")).unwrap();
        std::fs::write(
            dir.join("steamapps").join("libraryfolders.vdf"),
            "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"E:\\\\Games\\\\Steam\"\n\t}\n}\n",
        )
        .unwrap();
        let libs = steam_libraries(&dir);
        assert_eq!(libs[1], PathBuf::from("D:\\SteamLibrary"));
        assert_eq!(libs[2], PathBuf::from("E:\\Games\\Steam"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn sha256_matches_a_known_digest() {
        assert_eq!(sha256_hex(b"abc").unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
