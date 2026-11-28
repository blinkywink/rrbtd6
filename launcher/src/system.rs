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
#[cfg_attr(not(windows), allow(dead_code))]
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
    // Debug builds only: see the "not found" screen on a PC that has BTD6.
    #[cfg(debug_assertions)]
    if std::env::var_os("RRBTD6_PRETEND_NO_BTD6").is_some() {
        return None;
    }
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
    #[cfg(windows)]
    {
        candidates.extend(epic_installs());
        candidates.push(PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6"));
        candidates.push(PathBuf::from("C:/Program Files/Epic Games/BloonsTD6"));
    }
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.extend(unix_candidates(&home));
    }
    candidates.into_iter().find(|p| is_btd6(p))
}

/// BTD6's Steam app and its Windows depot (the files RustTD6 reads, on every system).
pub const BTD6_APP: u32 = 960090;
pub const BTD6_WINDOWS_DEPOT: u32 = 960091;

/// Where Linux and macOS Steam keep the Windows files: Linux installs BTD6 through Proton into
/// steamapps/common like Windows does; on a Mac, `download_depot 960090 960091` in Steam's console
/// saves them under steamapps/content. Steam itself may be the regular, Flatpak or Snap one.
#[cfg_attr(windows, allow(dead_code))]
fn unix_candidates(home: &Path) -> Vec<PathBuf> {
    let roots = [
        ".steam/steam", ".steam/root", ".local/share/Steam",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam", ".var/app/com.valvesoftware.Steam/data/Steam",
        "snap/steam/common/.local/share/Steam", "Library/Application Support/Steam",
    ];
    let mut out = Vec::new();
    for root in roots.iter().map(|r| home.join(r)).filter(|r| r.join("steamapps").is_dir()) {
        for lib in steam_libraries(&root) {
            for p in [lib.join("steamapps/common/BloonsTD6"),
                      lib.join(format!("steamapps/content/app_{BTD6_APP}/depot_{BTD6_WINDOWS_DEPOT}"))] {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
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

/// Windows' folder picker (None: cancelled).
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    unsafe {
        #[repr(C)]
        struct BrowseInfo { owner: isize, root: isize, display: *mut u16, title: *const u16, flags: u32, callback: isize, param: isize, image: i32 }
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn SHBrowseForFolderW(info: *const BrowseInfo) -> isize;
            fn SHGetPathFromIDListW(list: isize, path: *mut u16) -> i32;
        }
        #[link(name = "ole32")]
        unsafe extern "system" {
            fn CoInitializeEx(reserved: isize, flags: u32) -> i32;
            fn CoTaskMemFree(p: isize);
        }
        CoInitializeEx(0, 0x2); // apartment-threaded, as the dialog needs
        let mut display = [0u16; 260];
        let t = wide(title);
        // BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_NONEWFOLDERBUTTON
        let info = BrowseInfo { owner: 0, root: 0, display: display.as_mut_ptr(), title: t.as_ptr(), flags: 0x1 | 0x40 | 0x200, callback: 0, param: 0, image: 0 };
        let list = SHBrowseForFolderW(&info);
        if list == 0 {
            return None;
        }
        let mut path = [0u16; 1024];
        let ok = SHGetPathFromIDListW(list, path.as_mut_ptr()) != 0;
        CoTaskMemFree(list);
        let n = path.iter().position(|&c| c == 0).unwrap_or(0);
        (ok && n > 0).then(|| PathBuf::from(String::from_utf16_lossy(&path[..n])))
    }
    #[cfg(not(windows))]
    {
        let _ = title;
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

    #[test]
    fn unix_steam_folders_include_flatpak_libraries_and_the_mac_depot() {
        let home = std::env::temp_dir().join(format!("rrbtd6-home-{}", std::process::id()));
        let flatpak = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
        std::fs::create_dir_all(flatpak.join("steamapps")).unwrap();
        std::fs::write(flatpak.join("steamapps/libraryfolders.vdf"), "\"libraryfolders\"\n{\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games/Steam\"\n\t}\n}\n").unwrap();
        let mac = home.join("Library/Application Support/Steam");
        std::fs::create_dir_all(mac.join("steamapps")).unwrap();
        let found = unix_candidates(&home);
        assert!(found.contains(&flatpak.join("steamapps/common/BloonsTD6")));
        assert!(found.contains(&PathBuf::from("/mnt/games/Steam").join("steamapps/common/BloonsTD6")));
        assert!(found.contains(&mac.join("steamapps/content/app_960090/depot_960091")));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn sha256_matches_a_known_digest() {
        assert_eq!(sha256_hex(b"abc").unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
