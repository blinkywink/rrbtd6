//! rrbtd6 launcher: installs RustTD6, keeps it up to date and starts the game. It's also the
//! setup program: run it from anywhere and it installs itself, finds BTD6 and extracts the data.
//!
//! Layout of the install folder (the launcher's own folder):
//!   rrbtd6-launcher.exe   this program
//!   rrbtd6.json           {"game": "<BTD6 folder>"} (found automatically, or picked by the player)
//!   version.json          the installed release ({"build": N, ...})
//!   rrbtd6.exe            the game, from the latest GitHub release
//!   tools/                the extraction scripts, from the same release
//!   python/               a private Python with tools/requirements.txt
//!   local_extracted/      game data extracted from your own BTD6 install (never redistributed)
//!   setup.log             what the last setup or update did, for troubleshooting
//!
//! Only GitHub and python.org / PyPI are contacted, never Ninja Kiwi.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod transaction;
mod compatibility;
mod system;
mod ui;

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const REPO: &str = "blinkywink/rrbtd6";
const PYTHON_VERSION: &str = "3.13.1";
const USER_AGENT: &str = "rrbtd6-launcher";
const DISCORD: &str = "https://discord.gg/GBah4crmpC";

/// This system's file names: the game and the launcher in the install folder, and their names
/// among a release's files (version.json lists each file's SHA-256 under that name).
#[cfg(windows)]
pub const GAME: &str = "rrbtd6.exe";
#[cfg(not(windows))]
pub const GAME: &str = "rrbtd6";
#[cfg(windows)]
pub const LAUNCHER: &str = "rrbtd6-launcher.exe";
#[cfg(not(windows))]
pub const LAUNCHER: &str = "rrbtd6-launcher";
#[cfg(windows)]
const ASSETS: (&str, &str) = ("rrbtd6.exe", "rrbtd6-launcher.exe");
#[cfg(target_os = "macos")]
const ASSETS: (&str, &str) = ("rrbtd6-macos", "rrbtd6-launcher-macos");
#[cfg(all(unix, not(target_os = "macos")))]
const ASSETS: (&str, &str) = ("rrbtd6-linux", "rrbtd6-launcher-linux");

enum Update{Installed,Current,KeepInstalled}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static LOG: OnceLock<Mutex<Option<fs::File>>> = OnceLock::new();
static CHILDREN: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// A line for setup.log (and the console, when started from one).
pub fn log(text: &str) {
    println!("{text}");
    if let Some(lock) = LOG.get() {
        if let Some(file) = lock.lock().unwrap().as_mut() {
            let _ = writeln!(file, "{text}");
        }
    }
}

macro_rules! say { ($($t:tt)*) => { log(&format!($($t)*)) } }

fn open_log(dir: &Path) {
    let path = dir.join("setup.log");
    // Keep the log small: start over when it passes 4 MB.
    let big = fs::metadata(&path).map(|m| m.len() > 4 << 20).unwrap_or(false);
    let file = if big { fs::File::create(&path) } else { fs::OpenOptions::new().create(true).append(true).open(&path) };
    let _ = LOG.set(Mutex::new(file.ok()));
    let version = env!("CARGO_PKG_VERSION");
    say!("\n==== RustTD6 launcher {version} ({}) ====", std::env::args().skip(1).collect::<Vec<_>>().join(" "));
}

/// Ends every process the launcher started (and their children): setup was stopped.
pub fn stop_children() {
    for pid in CHILDREN.lock().unwrap().drain(..) {
        let mut kill = Command::new("taskkill");
        kill.args(["/T", "/F", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null());
        hide_console(&mut kill);
        let _ = kill.status();
    }
}

/// Console programs started by the launcher (Python, pip, taskkill) get no window of their own.
fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Started from a terminal: print there too (the launcher is a window program).
fn attach_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" { fn AttachConsole(pid: u32) -> i32; }
        unsafe { AttachConsole(u32::MAX) };
    }
}

fn main() {
    // macOS / Linux: windows belong to the main thread, so the work runs on another one.
    #[cfg(not(windows))]
    {
        std::thread::spawn(|| {
            app();
            ui::finish();
        });
        ui::run_main_thread();
    }
    #[cfg(windows)]
    app();
}

fn app() {
    attach_console();
    let dir = match std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        Some(d) => d,
        None => { system::ask("RustTD6", "RustTD6 couldn't tell which folder it's in. Download it again and run it from your Downloads folder.", false); return }
    };
    // Run from a download: install into the recommended folder and continue from there.
    match standalone_install(&dir) {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            system::ask("RustTD6", &format!("RustTD6 couldn't install itself.\n\n{e}\n\nTry running it again. If it keeps happening, ask for help on Discord: {DISCORD}"), false);
            std::process::exit(1);
        }
    }
    open_log(&dir);
    let mut attempt = 0u32;
    loop {
        let outcome = run(&dir);
        stop_children();
        let error = match outcome {
            Ok(()) => { ui::close(); return; }
            Err(e) => e,
        };
        say!("Setup stopped: {error}");
        // Started again while another launcher is working in this folder (a second click on the
        // shortcut): that one carries on, so this one just ends.
        if error.to_string().contains("Another launcher is installing") {
            ui::close();
            return;
        }
        // --unattended (tests, scripts): no window, no retries, just the exit code.
        if unattended() {
            eprintln!("
rrbtd6 launcher failed: {error}");
            std::process::exit(1);
        }
        let problem = Problem::of(&error.to_string());
        attempt += 1;
        // Fix it ourselves first: once as it was, then after a repair (fresh tools and Python
        // packages). Setup keeps finished stages, so a retry carries on where it stopped.
        if problem.retry && attempt <= 2 {
            if attempt == 2 {
                repair(&dir);
            }
            if ui::is_open() {
                ui::stage("Something went wrong, trying again");
            }
            std::thread::sleep(Duration::from_secs(3));
            continue;
        }
        let mut buttons = vec!["Try again"];
        if problem.pick_folder {
            buttons.insert(0, "Find BTD6 myself");
            #[cfg(target_os = "macos")]
            buttons.insert(0, "Open Steam console");
        }
        buttons.extend(["Open setup log", "Close"]);
        loop {
            // The error itself only when it says more than the heading.
            let detail = error.to_string();
            let body = if detail.to_ascii_lowercase().starts_with(&problem.heading.to_ascii_lowercase()) {
                problem.body.to_string()
            } else {
                format!("{}\n\nWhat went wrong: {}", problem.body, short(&detail))
            };
            let choice = buttons[ui::ask(problem.heading, &body, &buttons)];
            match choice {
                "Find BTD6 myself" => match system::pick_folder("Choose your Bloons TD 6 folder (the one with BloonsTD6.exe)") {
                    Some(p) if p.join("GameAssembly.dll").is_file() => {
                        let _ = fs::write(dir.join("rrbtd6.json"), serde_json::json!({"game": p}).to_string());
                        break;
                    }
                    Some(_) => { ui::ask("That isn't the BTD6 folder", "Choose the folder that has BloonsTD6.exe and GameAssembly.dll in it. In Steam: right-click Bloons TD 6 > Manage > Browse local files.", &["OK"]); }
                    None => {}
                },
                "Open Steam console" => {
                    let _ = Command::new("open").arg("steam://open/console").spawn();
                }
                "Open setup log" => {
                    let _ = Command::new("notepad.exe").arg(dir.join("setup.log")).spawn();
                }
                "Try again" => break,
                _ => { ui::close(); std::process::exit(1); }
            }
        }
        attempt = 0;
        ui::stage("Trying again");
    }
}

pub fn unattended() -> bool {
    std::env::args().any(|a| a == "--unattended")
}

fn short(e: &str) -> String {
    let e = e.trim();
    if e.chars().count() > 300 { format!("{}...", e.chars().take(300).collect::<String>()) } else { e.to_string() }
}

/// What to tell the player about an error, and whether to retry it automatically.
struct Problem { heading: &'static str, body: &'static str, retry: bool, pick_folder: bool }

impl Problem {
    fn of(error: &str) -> Problem {
        let e = error.to_ascii_lowercase();
        let has = |words: &[&str]| words.iter().any(|w| e.contains(w));
        // A Mac runs RustTD6 from BTD6's Windows files, which Steam downloads on request.
        #[cfg(target_os = "macos")]
        if has(&["couldn't find bloons td 6"]) {
            return Problem { heading: "Get BTD6's Windows files", retry: false, pick_folder: true,
                body: "RustTD6 reads the game's art and sounds from the Windows version of BTD6, which Steam can download on a Mac too (about 3 GB; RustTD6 deletes it again when it's done).

1. Press Open Steam console (Steam must be running).
2. Type:  download_depot 960090 960091  and press Enter.
3. Wait until Steam says the depot download is complete, then press Try again." };
        }
        if has(&["couldn't find bloons td 6"]) {
            return Problem { heading: "Couldn't find Bloons TD 6", retry: false, pick_folder: true,
                body: "RustTD6 needs BTD6 installed on this PC (Steam or Epic), because it reads the game's art and sounds from your own copy. Install BTD6 and press Try again, or show RustTD6 where it is." };
        }
        if has(&["btd6 is out of date"]) {
            return Problem { heading: "Update Bloons TD 6 first", retry: false, pick_folder: false,
                body: "Your BTD6 is older than the version RustTD6 matches, and RustTD6 reads its stats and art from your copy. Update BTD6, then press Try again. Steam: restart Steam, or right-click Bloons TD 6 > Properties > Updates. Epic: open the Epic Games Launcher; it updates BTD6 from your Library." };
        }
        if has(&["not enough free disk space"]) {
            return Problem { heading: "Not enough disk space", retry: false, pick_folder: false,
                body: "Setup needs about 15 GB free the first time (the game data is about 7 GB). Free up some space, then press Try again." };
        }
        if has(&["memoryerror", "out of memory", "0xc0000017", "unable to allocate", "not enough memory"]) {
            return Problem { heading: "Your PC ran out of memory", retry: false, pick_folder: false,
                body: "Setup uses a lot of memory. Close other programs (browsers and games especially), then press Try again. It carries on from where it stopped." };
        }
        if has(&["stuck"]) {
            return Problem { heading: "Setup got stuck", retry: true, pick_folder: false,
                body: "Setup stopped making progress, so RustTD6 stopped it. Press Try again, or close RustTD6 and open it again: it carries on from where it stopped." };
        }
        if has(&["setup understands", "only v39", "newer unity", "type table"]) {
            return Problem { heading: "This BTD6 version isn't supported yet", retry: false, pick_folder: false,
                body: "BTD6 has updated in a way RustTD6's setup doesn't understand yet. RustTD6 updates itself when support is added, so try again in a day or two." };
        }
        if has(&["dns", "connect", "timed out", "timeout", "network", "resolve", "github", "403", "429", "502", "503", "certificate", "tls"]) {
            return Problem { heading: "Couldn't download RustTD6", retry: true, pick_folder: false,
                body: "RustTD6 couldn't reach GitHub, where it downloads from. Check your internet connection, then press Try again. School and work networks sometimes block GitHub." };
        }
        if has(&["os error 5", "access is denied", "permission", "os error 32", "being used by another process", "os error 225", "virus"]) {
            return Problem { heading: "Windows blocked a file", retry: true, pick_folder: false,
                body: "Windows or an antivirus program stopped RustTD6 from using one of its files. Press Try again. If it keeps happening, allow RustTD6's folder in your antivirus." };
        }
        if has(&["another launcher is installing"]) {
            return Problem { heading: "RustTD6 is already setting up", retry: false, pick_folder: false,
                body: "Another RustTD6 window is already setting up. Let that one finish, or close it and press Try again." };
        }
        Problem { heading: "Setup couldn't finish", retry: true, pick_folder: false,
            body: "Something went wrong while setting up RustTD6. Press Try again: it carries on from where it stopped. If it keeps happening, ask for help on Discord (discord.gg/GBah4crmpC) and attach setup.log." }
    }
}

/// Before the second automatic retry: get fresh tools and Python packages next time. The
/// extracted data and finished stages are kept.
fn repair(dir: &Path) {
    say!("Repairing: fresh tools and Python packages on the next try.");
    let _ = fs::remove_file(dir.join("python/requirements.installed"));
    if dir.join(GAME).is_file() {
        // An unreadable version.json makes the update step download the release again.
        let _ = fs::remove_file(dir.join("version.json"));
    }
}

fn run(dir: &Path) -> Result<()> {
    let _install_lock = transaction::lock(dir)?;
    transaction::recover(dir)?;
    let skip_update = std::env::args().any(|a| a == "--offline");
    let refresh_assets=std::env::args().any(|a|a=="--refresh-assets");
    let first_run = !setup_complete(dir);
    if first_run {
        ui::open();
        ui::stage("Looking for Bloons TD 6");
    }
    let config: serde_json::Value = fs::read_to_string(dir.join("rrbtd6.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let configured = config["game"].as_str().map(PathBuf::from).filter(|g| g.join("GameAssembly.dll").exists());
    let game = match configured {
        Some(g) => g,
        // Moved or never set: Steam (any library on any drive), Epic, the default folders.
        None => {
            let found = system::find_btd6().ok_or("Couldn't find Bloons TD 6")?;
            let mut config = if config.is_object() { config.clone() } else { serde_json::json!({}) };
            config["game"] = serde_json::Value::String(found.to_string_lossy().into_owned());
            let _ = fs::write(dir.join("rrbtd6.json"), serde_json::to_string(&config)?);
            say!("Found BTD6 at {}", found.display());
            found
        }
    };

    if !skip_update {
        match update(dir,&game,refresh_assets) {
            Ok(Update::Installed) => say!("Updated to the latest build."),
            Ok(Update::Current) => say!("Up to date."),
            Ok(Update::KeepInstalled) => say!("Starting the installed version."),
            // Offline or GitHub unreachable: keep playing the installed build if there is one.
            Err(e) if dir.join(GAME).exists() && !transaction::pending(dir) && !first_run => say!("Update check skipped: {e}"),
            Err(e) => return Err(e),
        }
    }
    // A working install is playable offline and while a new asset revision is deferred.
    if !setup_complete(dir) {
        ui::open();
        let python = ensure_python(dir)?;
        extract(dir, &python, &game)?;
        drop_depot_download(&game);
    }

    // The first setup ends on a "ready" screen: where to put shortcuts, then Play.
    let fresh: serde_json::Value = fs::read_to_string(dir.join("rrbtd6.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if first_run && ui::is_open() && !fresh["shortcuts_asked"].as_bool().unwrap_or(false) {
        let launcher = dir.join(LAUNCHER);
        // A Mac starts it from the app in Applications, so there's nothing to choose there.
        let (choice, ticked) = if cfg!(target_os = "macos") {
            (ui::ask("RustTD6 is ready", "Setup is done. Start RustTD6 from its app any time; it keeps itself up to date.", &["Play RustTD6", "Close"]), vec![false, false])
        } else {
            ui::ask_checks("RustTD6 is ready",
                &format!("Setup is done. Choose where you'd like a shortcut to start it from next time. You can also start it from {}.", launcher.display()),
                &[("Desktop shortcut", true), (if cfg!(windows) { "Start menu" } else { "App menu" }, true)],
                &["Play RustTD6", "Close"])
        };
        shortcuts(&launcher, dir, ticked[0], ticked[1]);
        let mut config = if fresh.is_object() { fresh } else { serde_json::json!({}) };
        config["shortcuts_asked"] = serde_json::Value::Bool(true);
        let _ = fs::write(dir.join("rrbtd6.json"), serde_json::to_string(&config)?);
        if choice != 0 {
            return Ok(());
        }
    }
    say!("Starting rrbtd6...");
    if ui::is_open() {
        ui::stage("Starting RustTD6");
    }
    let mut game_process=Command::new(dir.join(GAME));
    game_process.arg("--from-launcher").current_dir(dir)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    game_process.spawn().map_err(|e| format!("Couldn't start {GAME} ({e})"))?;
    Ok(())
}

/// Whether extraction finished: assets_version.json is the last thing it writes. game.json alone
/// isn't enough, because it's written partway through, so a setup that stopped later would look
/// complete and start a game with missing data.
fn setup_complete(dir: &Path) -> bool {
    dir.join("local_extracted/game/game.json").is_file() && dir.join("local_extracted/assets_version.json").is_file()
}

/// BTD6's Windows files downloaded with Steam's download_depot (a Mac) are only needed while setup
/// reads them: delete them afterwards (about 3 GB). Steam's own installs are never touched.
fn drop_depot_download(game: &Path) {
    let depot = format!("steamapps/content/app_{}/depot_{}", system::BTD6_APP, system::BTD6_WINDOWS_DEPOT);
    if game.to_string_lossy().replace('\\', "/").ends_with(&depot) && game.join("GameAssembly.dll").is_file() {
        match fs::remove_dir_all(game) {
            Ok(()) => say!("Removed the downloaded Windows files ({}); setup has what it needs.", game.display()),
            Err(e) => say!("Couldn't remove {} ({e}); you can delete it yourself.", game.display()),
        }
    }
}

/// The folder RustTD6 installs into: per-user, always writable, not synced.
fn recommended_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("RustTD6"));
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    Some(if cfg!(target_os = "macos") { home.join("Library/Application Support/RustTD6") } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("RustTD6")
    })
}

/// Why RustTD6 can't go in `folder` (None: it can).
fn folder_problem(folder: &Path) -> Option<String> {
    let text = folder.display().to_string();
    // Windows limits paths to 260 characters and the game data goes about 160 deep.
    if text.chars().count() > 95 {
        return Some(format!("The folder path is {} characters long; Windows limits file paths to 260 and the game data goes about 160 deep.", text.chars().count()));
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)", "SystemRoot"] {
        if let Some(root) = std::env::var_os(var) {
            if folder.starts_with(PathBuf::from(root)) {
                return Some("Windows only lets administrators change files there, so RustTD6 couldn't save its data.".into());
            }
        }
    }
    if fs::create_dir_all(folder).and_then(|_| fs::write(folder.join(".write-test"), b"ok")).and_then(|_| fs::remove_file(folder.join(".write-test"))).is_err() {
        return Some("RustTD6 can't write to that folder.".into());
    }
    None
}

/// Run from anywhere that isn't an install yet (a download): install into the recommended folder
/// without asking and carry on from there (shortcuts are offered when setup finishes). If that folder
/// can't be used, the player picks one. True when this run handed over and should end.
fn standalone_install(dir: &Path) -> Result<bool> {
    let installed = dir.join("rrbtd6.json").exists() || dir.join("version.json").exists() || dir.join(GAME).exists();
    let Some(mut target) = recommended_dir() else { return Ok(false) };
    if installed || std::env::args().any(|a| a == "--from-installer" || a == "--here") || dir == target {
        return Ok(false);
    }
    while let Some(problem) = folder_problem(&target) {
        let text = format!("RustTD6 can't install to {}.\n\n{problem}\n\nChoose another folder? A short path like C:\\Games works well.", target.display());
        if system::ask("RustTD6 Setup", &text, true) != 6 {
            std::process::exit(0);
        }
        match system::pick_folder("Choose where to install RustTD6") {
            Some(p) => target = p.join("RustTD6"),
            None => std::process::exit(0),
        }
    }
    fs::create_dir_all(&target)?;
    let me = std::env::current_exe()?;
    let copy = target.join(LAUNCHER);
    // Already installed there: keep that launcher (it updates itself and may be newer than this
    // download) and just start it.
    if !copy.is_file() {
        fs::copy(&me, &copy).map_err(|e| format!("Couldn't copy RustTD6 into {} ({e})", target.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&copy, fs::Permissions::from_mode(0o755));
        }
        // macOS: the copy keeps the download's quarantine flag, which would stop it starting; the
        // player already chose to open the app it came from.
        #[cfg(target_os = "macos")]
        let _ = Command::new("xattr").args(["-d", "com.apple.quarantine"]).arg(&copy).output();
    }
    Command::new(&copy).current_dir(&target).spawn()?;
    Ok(true)
}

/// "RustTD6" shortcuts to the launcher on the desktop and in the Start menu (a convenience, so
/// failures are ignored).
#[cfg(not(windows))]
fn shortcuts(launcher: &Path, dir: &Path, desktop: bool, menu: bool) {
    // Linux: a .desktop entry in the app menu and/or on the desktop.
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return };
    let entry = format!("[Desktop Entry]
Type=Application
Name=RustTD6
Comment=Bloons TD 6, built to go further
Exec=\"{}\"
Path={}
Terminal=false
Categories=Game;
",
        launcher.display(), dir.display());
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
    let mut places = Vec::new();
    if menu {
        places.push(data.join("applications"));
    }
    if desktop {
        places.push(home.join("Desktop"));
    }
    for place in places {
        let file = place.join("rusttd6.desktop");
        if fs::create_dir_all(&place).and_then(|_| fs::write(&file, &entry)).is_ok() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&file, fs::Permissions::from_mode(0o755));
        }
    }
}

#[cfg(windows)]
fn shortcuts(launcher: &Path, dir: &Path, desktop: bool, start_menu: bool) {
    let places: Vec<&str> = [("'Desktop'", desktop), ("'Programs'", start_menu)].iter().filter(|p| p.1).map(|p| p.0).collect();
    if places.is_empty() {
        return;
    }
    let q = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "foreach($f in {}){{$d=[Environment]::GetFolderPath($f);if($d){{$s=(New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $d 'RustTD6.lnk'));$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.IconLocation='{}';$s.Save()}}}}",
        places.join(","),
        q(launcher),
        q(dir),
        q(&dir.join(GAME))
    );
    let mut ps = Command::new("powershell.exe");
    ps.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    hide_console(&mut ps);
    let _ = ps.status();
}

fn get(url: &str) -> Result<ureq::Response> {
    static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    let agent = AGENT.get_or_init(|| {
        let mut b = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(60))
            .timeout_write(std::time::Duration::from_secs(60));
        // School / office networks: HTTPS_PROXY, or the Windows proxy setting.
        if let Some(p) = system::proxy().and_then(|p| ureq::Proxy::new(&p).ok()) {
            b = b.proxy(p);
        }
        b.build()
    });
    Ok(agent.get(url).set("User-Agent", USER_AGENT).call()?)
}

/// A downloaded release file against the SHA-256 the release lists (version.json "sha256"; older
/// releases list none).
fn verify(remote: &serde_json::Value, name: &str, path: &Path) -> Result<()> {
    let Some(want) = remote["sha256"][name].as_str() else { return Ok(()) };
    let got = system::sha256_hex(&fs::read(path)?).ok_or("could not hash the download")?;
    if !got.eq_ignore_ascii_case(want) {
        let _ = fs::remove_file(path);
        return Err(format!("{name} was damaged while downloading (checksum mismatch); try again").into());
    }
    Ok(())
}

/// Downloads `url` to `to`, trying up to three times (flaky Wi-Fi, a GitHub hiccup). A 404 isn't
/// retried: the file isn't there.
fn download(url: &str, to: &Path) -> Result<()> {
    say!("  downloading {}", url.rsplit('/').next().unwrap_or(url));
    let mut last = None;
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(2 << attempt));
            say!("  retrying the download");
        }
        let result = (|| -> Result<()> {
            let mut body = get(url)?.into_reader();
            let part = to.with_extension("part");
            let mut file = fs::File::create(&part)?;
            io::copy(&mut body, &mut file)?;
            file.flush()?;
            drop(file);
            fs::rename(&part, to)?;
            Ok(())
        })();
        match result {
            Ok(()) => return Ok(()),
            Err(e) if e.to_string().contains("404") => return Err(e),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| "download failed".into()))
}

/// Installs the latest release when it is newer than version.json. Returns whether it changed.
fn update(dir: &Path,game:&Path,refresh_assets:bool) -> Result<Update> {
    let latest: serde_json::Value = get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))?.into_json()?;
    let assets = latest["assets"].as_array().ok_or("release has no assets")?;
    let asset = |name: &str| assets.iter().find(|a| a["name"] == name).and_then(|a| a["browser_download_url"].as_str());
    let remote: serde_json::Value = get(asset("version.json").ok_or("release has no version.json")?)?.into_json()?;
    let build = remote["build"].as_u64().ok_or("release version has no valid build number")?;
    let local: serde_json::Value = fs::read_to_string(dir.join("version.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if local["build"].as_u64().is_some_and(|installed|installed>=build) && dir.join(GAME).is_file()
        && dir.join("tools/extract_all.py").is_file() && dir.join("tools/requirements.txt").is_file() {
        return Ok(Update::Current);
    }
    let assets:serde_json::Value=fs::read(dir.join("local_extracted/assets_version.json")).ok()
        .and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
    let needs_assets=compatibility::needs_assets(&remote,&assets);
    if needs_assets && dir.join(GAME).is_file() && !refresh_assets && !compatibility::update_now(build) {
        say!("Keeping installed build until its replacement assets have been extracted.");
        return Ok(Update::KeepInstalled);
    }
    ui::open();
    ui::stage(if dir.join(GAME).is_file() { "Downloading the RustTD6 update" } else { "Downloading RustTD6" });
    say!("Installing build {}...", remote["build"]);
    let payload = transaction::prepare(dir)?;
    let staging = transaction::staging(dir);
    download(asset(ASSETS.0).ok_or("this release has no game for this system")?, &payload.join(GAME))?;
    verify(&remote, ASSETS.0, &payload.join(GAME))?;
    download(asset("rrbtd6-tools.zip").ok_or("release has no rrbtd6-tools.zip")?, &staging.join("tools.zip"))?;
    verify(&remote, "rrbtd6-tools.zip", &staging.join("tools.zip"))?;
    if let Some(url) = asset(ASSETS.1) {
        download(url, &payload.join(LAUNCHER))?;
        verify(&remote, ASSETS.1, &payload.join(LAUNCHER))?;
    }
    // Tools: replace the folder wholesale so removed scripts disappear too.
    let tools_new = staging.join("tools_new");
    zip::ZipArchive::new(fs::File::open(staging.join("tools.zip"))?)?.extract(&tools_new)?;
    let extracted_tools = if tools_new.join("tools").exists() { tools_new.join("tools") } else { tools_new.clone() };
    fs::rename(extracted_tools, payload.join("tools"))?;
    fs::write(payload.join("version.json"), serde_json::to_string(&remote)?)?;
    if needs_assets {
        ui::stage("Updating game data");
        // Extract beside the staged release: never mutate the playable asset tree on failure.
        seed_assets(dir,&payload)?;
        let python=ensure_python_from(dir,&payload.join("tools"))?;
        extract(&payload,&python,game)?;
        let staged:serde_json::Value=serde_json::from_slice(&fs::read(payload.join("local_extracted/assets_version.json"))?)?;
        if compatibility::needs_assets(&remote,&staged){return Err("New extraction did not satisfy this build's asset requirements".into());}
        // Preserve only user-owned state, never copy stale generated assets into the new tree.
        for name in ["settings.json","local-profile.json","local-profile.backup.json","player-name.json"] {
            let old=dir.join("local_extracted").join(name);
            if old.is_file(){fs::copy(old,payload.join("local_extracted").join(name))?;}
        }
        for name in ["online"]{preserve_user_tree(&dir.join("local_extracted").join(name),&payload.join("local_extracted").join(name))?;}
    }
    transaction::install(dir)?;
    Ok(Update::Installed)
}

/// Copy the generated tree/checkpoints so unchanged exporter steps can be skipped.
/// Copies, not hardlinks: exporters may overwrite files in place while the old build runs.
fn seed_assets(installed:&Path,payload:&Path)->Result<()>{
    let old=installed.join("local_extracted");if !old.is_dir(){return Ok(());}
    let new=payload.join("local_extracted");fs::create_dir_all(&new)?;
    for name in ["audio","bloon_displays","brand","data","decoded","display_plans","effects","engine_data","fonts","game","game_model","maps","models","shaders","sprites","text","ui"]{
        preserve_user_tree(&old.join(name),&new.join(name))?;
    }
    for name in ["all_container_paths.json","cab_index.json","asset_references_paths.json","game_version.json",".extract_state.json"]{
        if old.join(name).is_file(){fs::copy(old.join(name),new.join(name))?;}
    }
    preserve_user_tree(&installed.join("research/dumps"),&payload.join("research/dumps"))?;
    let checkpoint=new.join(".extract_state.json");
    if let Ok(bytes)=fs::read(&checkpoint){if let Ok(mut value)=serde_json::from_slice::<serde_json::Value>(&bytes){
        // Ordinary copies lose within-tree deduplication; always compact the staged tree again.
        if let Some(done)=value["done"].as_object_mut(){done.remove("dedupe");}
        fs::write(checkpoint,serde_json::to_vec(&value)?)?;
    }}
    Ok(())
}

fn preserve_user_tree(source:&Path,destination:&Path)->Result<()>{
    if !source.try_exists()?{return Ok(());}
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)?{
        let entry=entry?;let kind=entry.file_type()?;
        if kind.is_symlink(){return Err("User data contains a symbolic link; update left installed files intact".into());}
        let to=destination.join(entry.file_name());
        if kind.is_dir(){preserve_user_tree(&entry.path(),&to)?;}else if kind.is_file(){fs::copy(entry.path(),to)?;}
    }
    Ok(())
}

/// A private embeddable Python with the extraction packages (tools/requirements.txt).
fn ensure_python(dir:&Path)->Result<PathBuf>{ensure_python_from(dir,&dir.join("tools"))}
fn ensure_python_from(dir: &Path,tools:&Path) -> Result<PathBuf> {
    let root = dir.join("python");
    let python = python_bin(&root);
    let requirements = fs::read_to_string(tools.join("requirements.txt"))?;
    let stamp = root.join("requirements.installed");
    if python.exists() && fs::read_to_string(&stamp).ok().as_deref() == Some(requirements.as_str()) {
        return Ok(python);
    }
    // The release's prepared runtime first (one GitHub download, no python.org / PyPI, nothing
    // built); without it, set one up from python.org and PyPI as before.
    #[cfg(windows)]
    match bundled_python(&root, &requirements) {
        Ok(true) => return Ok(python),
        Ok(false) => {}
        Err(e) => say!("Prepared Python runtime unavailable ({e}); setting one up instead."),
    }
    // A cancelled ZIP extraction can leave python.exe present without its DLLs
    // or standard library. Probe it before treating the runtime as installed.
    let runtime_ready = python.exists() && {
        let mut probe = Command::new(&python);
        probe.args(["-X", "utf8", "-I", "-c", "import ssl, zipfile, sysconfig"]);
        hide_console(&mut probe);
        probe.output().is_ok_and(|out| out.status.success())
    };
    if !runtime_ready {
        ui::stage("Setting up Python (first time only)");
        fs::create_dir_all(&root)?;
        #[cfg(not(windows))]
        {
            // A standalone build (python-build-standalone): unpacks to python/ beside the launcher.
            let archive = dir.join("python.tgz");
            download(&unix_python_url(), &archive)?;
            check(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(dir))?;
            fs::remove_file(&archive)?;
        }
        #[cfg(windows)]
        {
            let archive = root.join("python.zip");
            download(&format!("https://www.python.org/ftp/python/{PYTHON_VERSION}/python-{PYTHON_VERSION}-embed-amd64.zip"), &archive)?;
            zip::ZipArchive::new(fs::File::open(&archive)?)?.extract(&root)?;
            fs::remove_file(&archive)?;
        }
        // The embeddable build ignores site-packages until `import site` is enabled in its ._pth.
        for entry in fs::read_dir(&root)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "_pth") {
                let text = fs::read_to_string(&path)?.replace("#import site", "import site");
                fs::write(&path, text)?;
            }
        }
    }
    // get-pip may have been interrupted after the runtime was unpacked. Retry
    // bootstrap independently instead of getting stuck on 'No module named pip'.
    let pip_ready = {
        let mut probe = Command::new(&python);
        probe.args(["-X", "utf8", "-I", "-m", "pip", "--version"]);
        hide_console(&mut probe);
        probe.output().is_ok_and(|out| out.status.success())
    };
    if !pip_ready {
        let get_pip = root.join("get-pip.py");
        download("https://bootstrap.pypa.io/get-pip.py", &get_pip)?;
        check(Command::new(&python).current_dir(&root).arg(&get_pip).arg("--no-warn-script-location"))?;
    }
    ui::stage("Installing setup packages (first time only)");
    // Embeddable Python's ._pth intentionally ignores PYTHONPATH, including
    // pip's temporary build environment. tpk_ar ships as a source distribution:
    // give its backend an explicit home instead of relying on build isolation.
    check(Command::new(&python).current_dir(&root).args(["-m", "pip", "install", "--no-warn-script-location",
        "setuptools==80.9.0", "wheel==0.45.1"]))?;
    check(Command::new(&python).current_dir(&root).args(["-m", "pip", "install", "--no-warn-script-location", "--no-build-isolation", "-r"])
        .arg(tools.join("requirements.txt")))?;
    fs::write(&stamp, requirements)?;
    Ok(python)
}

/// The Python executable inside a runtime folder.
fn python_bin(root: &Path) -> PathBuf {
    if cfg!(windows) { root.join("python.exe") } else { root.join("bin").join("python3") }
}

/// Python for macOS and Linux: python-build-standalone's install-only build of PYTHON_VERSION
/// (the same release on every system, so extraction runs the same Python everywhere).
#[cfg(not(windows))]
fn unix_python_url() -> String {
    const TAG: &str = "20241219";
    let triple = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" }
    } else if cfg!(target_arch = "aarch64") { "aarch64-unknown-linux-gnu" } else { "x86_64-unknown-linux-gnu" };
    format!("https://github.com/astral-sh/python-build-standalone/releases/download/{TAG}/cpython-{PYTHON_VERSION}+{TAG}-{triple}-install_only.tar.gz")
}

/// The release tag of the prepared runtime for these requirements (release.yml makes it when
/// the requirements or the Python version change): `python-runtime-` + the first 16 hex digits
/// of SHA-256(requirements.txt + PYTHON_VERSION).
#[cfg(windows)]
fn runtime_tag(requirements: &str) -> Option<String> {
    let hex = system::sha256_hex(format!("{requirements}{PYTHON_VERSION}").as_bytes())?;
    Some(format!("python-runtime-{}", &hex[..16]))
}

/// Unpacks the prepared runtime (embeddable Python with tools/requirements.txt installed, its
/// stamp included) into `root`. Ok(false): this build has none published.
#[cfg(windows)]
fn bundled_python(root: &Path, requirements: &str) -> Result<bool> {
    let Some(tag) = runtime_tag(requirements) else { return Ok(false) };
    let url = format!("https://github.com/{REPO}/releases/download/{tag}/rrbtd6-python.zip");
    let parent = root.parent().ok_or("no install folder")?;
    let archive = parent.join("python-runtime.zip");
    ui::stage("Setting up Python (first time only)");
    if let Err(e) = download(&url, &archive) {
        let _ = fs::remove_file(&archive);
        // 404: not published for these requirements.
        return if e.to_string().contains("404") { Ok(false) } else { Err(e) };
    }
    let unpack = parent.join("python.new");
    let _ = fs::remove_dir_all(&unpack);
    let result = (|| -> Result<()> {
        zip::ZipArchive::new(fs::File::open(&archive)?)?.extract(&unpack)?;
        let mut probe = Command::new(unpack.join("python.exe"));
        probe.args(["-X", "utf8", "-I", "-c", "import ssl, zipfile, UnityPy, numpy, PIL"]);
        hide_console(&mut probe);
        let ok = probe.output().is_ok_and(|out| out.status.success());
        if !ok || fs::read_to_string(unpack.join("requirements.installed")).ok().as_deref() != Some(requirements) {
            return Err("the prepared runtime doesn't start or doesn't match these requirements".into());
        }
        if root.exists() {
            fs::remove_dir_all(root)?;
        }
        fs::rename(&unpack, root)?;
        Ok(())
    })();
    let _ = fs::remove_file(&archive);
    if result.is_err() {
        let _ = fs::remove_dir_all(&unpack);
    }
    result.map(|_| true)
}

/// Space the extraction needs on the install drive: the data (~7 GB) and, the first time, its
/// working copy of the game's bundles (~5 GB) and Python.
fn check_space(dir: &Path) -> Result<()> {
    let first = !dir.join("local_extracted/game/game.json").is_file();
    let need: u64 = if first { 15 } else { 7 } << 30;
    if let Some(free) = system::free_bytes(dir) {
        if free < need {
            return Err(format!(
                "Not enough free disk space: setup needs about {} GB free on the drive with {} and {:.1} GB are free. Free up some space and start RustTD6 again.",
                need >> 30,
                dir.display(),
                free as f64 / (1u64 << 30) as f64
            )
            .into());
        }
    }
    Ok(())
}

/// tools/extract_all.py skips finished steps and invalidates them by game fingerprint/output version.
fn extract(dir: &Path, python: &Path, game: &Path) -> Result<()> {
    btd6_version_gate(game)?;
    check_space(dir)?;
    ui::stage("Reading your BTD6 files");
    say!("Checking game data (the first run extracts it from your BTD6 install; this takes a while)...");
    // -X utf8: text as UTF-8 whatever the Windows code page (-I ignores PYTHONUTF8).
    check(Command::new(python).args(["-X", "utf8", "-I"]).arg(dir.join("tools/extract_all.py"))
        .arg("--game").arg(game).arg("--root").arg(dir).current_dir(dir))
}

/// Game data is read from the player's own BTD6, so it must be the version RustTD6 was matched
/// against, or newer: an older one would bring that version's stats and be missing what was added
/// since (and co-op would disagree with everyone else's). A newer one works; RustTD6 just doesn't
/// have its changes yet. An unreadable version doesn't stop setup.
fn btd6_version_gate(game: &Path) -> Result<()> {
    let supported = compatibility::SUPPORTED_BTD6.trim();
    let Some(installed) = compatibility::installed_btd6(game) else {
        say!("BTD6 version: couldn't read it; continuing (RustTD6 matches {supported})");
        return Ok(());
    };
    say!("BTD6 version {installed}; RustTD6 matches {supported}");
    match compatibility::compare_btd6(&installed, supported) {
        std::cmp::Ordering::Less => Err(format!("BTD6 is out of date: this PC has version {installed}, RustTD6 needs {supported}").into()),
        std::cmp::Ordering::Greater if !unattended() => {
            ui::ask("Your BTD6 is newer than RustTD6",
                &format!("You have BTD6 {installed}; RustTD6 currently matches {supported}. Everything still works, but what that update added or changed isn't in RustTD6 yet. RustTD6 updates itself when it catches up, usually within a few days.\n\nCo-op works with players on the same RustTD6 version."),
                &["Continue"]);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// How long setup may go without any output, or without its progress line changing, before it
/// counts as stuck. The progress line repeats every 5 s while a stage works, so silence means the
/// process itself hung; an unchanged percentage means a stage stopped moving.
const SILENT: Duration = Duration::from_secs(3 * 60);
const UNCHANGED: Duration = Duration::from_secs(30 * 60);

/// Runs a setup program with no console window: its output goes to setup.log and moves the setup
/// window's bar. Fails if it fails, or if it gets stuck (then it's stopped).
fn check(cmd: &mut Command) -> Result<()> {
    hide_console(cmd);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Couldn't start {:?} ({e})", cmd.get_program()))?;
    CHILDREN.lock().unwrap().push(child.id());
    let (send, lines) = std::sync::mpsc::channel::<String>();
    for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn io::Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn io::Read + Send>)].into_iter().flatten() {
        let send = send.clone();
        std::thread::spawn(move || {
            for line in io::BufReader::new(pipe).lines().map_while(std::result::Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(send);
    let mut tail: Vec<String> = Vec::new();
    let (mut heard, mut moved, mut last_progress) = (Instant::now(), Instant::now(), String::new());
    loop {
        match lines.recv_timeout(Duration::from_secs(1)) {
            Ok(line) => {
                heard = Instant::now();
                log(&line);
                ui::line(&line);
                // "[###---]  12.3% | Preparing maps | ETA..." : the part that changes when work moves.
                if line.starts_with('[') {
                    let key: String = line.split('|').take(2).collect();
                    if key != last_progress {
                        last_progress = key;
                        moved = Instant::now();
                    }
                } else {
                    moved = Instant::now();
                }
                if !line.trim().is_empty() {
                    tail.push(line);
                    if tail.len() > 12 {
                        tail.remove(0);
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if heard.elapsed() > SILENT || moved.elapsed() > UNCHANGED {
            say!("No progress for a long time: stopping it.");
            stop_children();
            let _ = child.wait();
            return Err("Setup got stuck".into());
        }
    }
    let status = child.wait()?;
    CHILDREN.lock().unwrap().retain(|&p| p != child.id());
    if !status.success() {
        // The last error lines say what went wrong (a Python traceback ends with it).
        let reason = tail.iter().rev().find(|l| l.contains("Error") || l.contains("error") || l.contains("Exception"))
            .cloned().unwrap_or_else(|| tail.last().cloned().unwrap_or_default());
        return Err(format!("{:?} stopped ({status}): {reason}", cmd.get_program().to_string_lossy().rsplit(['\\', '/']).next().unwrap_or("")).into());
    }
    Ok(())
}

#[cfg(test)]
mod update_tests{
    use super::*;
    #[test]
    fn staged_incremental_assets_do_not_link_to_or_modify_the_playable_install(){
        let dir=std::env::temp_dir().join(format!("rrbtd6-seed-{}",std::process::id()));
        let old=dir.join("installed");let stage=dir.join("staged");
        fs::create_dir_all(old.join("local_extracted/game")).unwrap();
        fs::write(old.join("local_extracted/game/game.json"),"original").unwrap();
        fs::write(old.join("local_extracted/.extract_state.json"),r#"{"fingerprint":"source","done":{"game_data":1,"dedupe":1}}"#).unwrap();
        seed_assets(&old,&stage).unwrap();
        fs::write(stage.join("local_extracted/game/game.json"),"updated").unwrap();
        assert_eq!(fs::read_to_string(old.join("local_extracted/game/game.json")).unwrap(),"original");
        let state:serde_json::Value=serde_json::from_slice(&fs::read(stage.join("local_extracted/.extract_state.json")).unwrap()).unwrap();
        assert_eq!(state["done"]["game_data"],1);assert!(state["done"]["dedupe"].is_null());
        fs::remove_dir_all(dir).unwrap();
    }
}
