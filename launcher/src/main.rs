//! rrbtd6 launcher: keeps an install folder up to date and starts the game.
//!
//! Layout of the install folder (the launcher's own folder):
//!   rrbtd6-launcher.exe   this program
//!   rrbtd6.json           {"game": "<BTD6 folder>"} (written by the BTD6 installer mod)
//!   version.json          the installed release ({"build": N, ...})
//!   rrbtd6.exe            the game, from the latest GitHub release
//!   tools/                the extraction scripts, from the same release
//!   python/               a private Python with tools/requirements.txt
//!   local_extracted/      game data extracted from your own BTD6 install (never redistributed)
//!
//! Only GitHub and python.org / PyPI are contacted, never Ninja Kiwi.

mod transaction;
mod compatibility;
mod system;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "blinkywink/rrbtd6";
const PYTHON_VERSION: &str = "3.13.1";
const USER_AGENT: &str = "rrbtd6-launcher";

enum Update{Installed,Current,KeepInstalled,SetupOpened}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("\nrrbtd6 launcher failed: {e}");
        if !std::env::args().any(|a|a=="--from-installer"){pause();}
        std::process::exit(1);
    }
}

fn pause() {
    eprint!("Press Enter to close...");
    let _ = io::stdin().read_line(&mut String::new());
}

fn run() -> Result<()> {
    let dir = std::env::current_exe()?.parent().ok_or("no install folder")?.to_path_buf();
    // Started on its own from a download folder (no installer mod): offer the recommended folder.
    if standalone_install(&dir)? {
        return Ok(());
    }
    let _install_lock = transaction::lock(&dir)?;
    transaction::recover(&dir)?;
    let skip_update = std::env::args().any(|a| a == "--offline");
    let refresh_assets=std::env::args().any(|a|a=="--refresh-assets");
    let config: serde_json::Value = fs::read_to_string(dir.join("rrbtd6.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let configured = config["game"].as_str().map(PathBuf::from).filter(|g| g.join("GameAssembly.dll").exists());
    let game = match configured {
        Some(g) => g,
        // Moved or never set: Steam (any library on any drive), Epic, the default folders.
        None => {
            let found = system::find_btd6().ok_or(
                "Couldn't find Bloons TD 6. RustTD6 reads its data from your own BTD6 install (Steam or Epic). \
                 Install BTD6, start it once, then start RustTD6 again. If BTD6 is installed somewhere unusual, \
                 write {\"game\": \"<BTD6 folder>\"} to rrbtd6.json in this folder.")?;
            let mut config = if config.is_object() { config.clone() } else { serde_json::json!({}) };
            config["game"] = serde_json::Value::String(found.to_string_lossy().into_owned());
            let _ = fs::write(dir.join("rrbtd6.json"), serde_json::to_string(&config)?);
            println!("Found BTD6 at {}", found.display());
            found
        }
    };

    if !skip_update {
        match update(&dir,&game,refresh_assets) {
            Ok(Update::Installed) => println!("Updated to the latest build."),
            Ok(Update::Current) => println!("Up to date."),
            Ok(Update::KeepInstalled) => println!("Starting the installed version."),
            Ok(Update::SetupOpened) => return Ok(()),
            // Offline or GitHub unreachable: keep playing the installed build if there is one.
            Err(e) if dir.join("rrbtd6.exe").exists() && !transaction::pending(&dir) => println!("Update check skipped: {e}"),
            Err(e) => return Err(e),
        }
    }
    // A working install is playable offline and while a new asset revision is deferred.
    if !dir.join("local_extracted/game/game.json").is_file() {
        let python = ensure_python(&dir)?;
        extract(&dir, &python, &game)?;
    }

    println!("Starting rrbtd6...");
    let mut game_process=Command::new(dir.join("rrbtd6.exe"));
    game_process.arg("--from-launcher").current_dir(&dir);
    if std::env::args().any(|a|a=="--from-installer"){
        game_process.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    }
    game_process.spawn()?;
    Ok(())
}

/// The folder `standalone_install` offers: per-user, always writable, not synced.
fn recommended_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("RustTD6"))
}

/// Started on its own (downloaded from the releases page, no installer mod) in a folder that is no
/// install yet: install to the recommended folder (copy itself there, a desktop shortcut, start
/// there) or into this folder. True when this run handed over and should end.
fn standalone_install(dir: &Path) -> Result<bool> {
    let installed = dir.join("rrbtd6.json").exists() || dir.join("version.json").exists() || dir.join("rrbtd6.exe").exists();
    let Some(target) = recommended_dir() else { return Ok(false) };
    if installed || std::env::args().any(|a| a == "--from-installer" || a == "--here") || dir == target {
        return Ok(false);
    }
    let text = format!(
        "Install RustTD6 to {}? (recommended)\n\nYes: install there and put a shortcut on the desktop.\nNo: install into this folder ({}).\nCancel: quit.",
        target.display(),
        dir.display()
    );
    match system::ask("RustTD6 Setup", &text, true) {
        6 => {
            fs::create_dir_all(&target)?;
            let me = std::env::current_exe()?;
            let copy = target.join("rrbtd6-launcher.exe");
            fs::copy(&me, &copy)?;
            desktop_shortcut(&copy, &target);
            Command::new(&copy).current_dir(&target).spawn()?;
            Ok(true)
        }
        7 => Ok(false),
        _ => std::process::exit(0),
    }
}

/// A desktop shortcut "RustTD6" to the launcher (as the installer mod makes; a convenience, so
/// failures are ignored).
fn desktop_shortcut(launcher: &Path, dir: &Path) {
    let q = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$d=[Environment]::GetFolderPath('Desktop');$s=(New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $d 'RustTD6.lnk'));$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.Save()",
        q(launcher),
        q(dir)
    );
    let _ = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", &script]).status();
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

fn download(url: &str, to: &Path) -> Result<()> {
    println!("  downloading {}", url.rsplit('/').next().unwrap_or(url));
    let mut body = get(url)?.into_reader();
    let part = to.with_extension("part");
    let mut file = fs::File::create(&part)?;
    io::copy(&mut body, &mut file)?;
    file.flush()?;
    drop(file);
    fs::rename(&part, to)?;
    Ok(())
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
    if local["build"].as_u64().is_some_and(|installed|installed>=build) && dir.join("rrbtd6.exe").is_file()
        && dir.join("tools/extract_all.py").is_file() && dir.join("tools/requirements.txt").is_file() {
        return Ok(Update::Current);
    }
    let assets:serde_json::Value=fs::read(dir.join("local_extracted/assets_version.json")).ok()
        .and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
    let needs_assets=compatibility::needs_assets(&remote,&assets);
    if needs_assets && dir.join("rrbtd6.exe").is_file() && !refresh_assets {
        if compatibility::extraction_notice(game,build){return Ok(Update::SetupOpened);}
        println!("Keeping installed build until its replacement assets have been extracted.");
        return Ok(Update::KeepInstalled);
    }
    println!("Installing build {}...", remote["build"]);
    let payload = transaction::prepare(dir)?;
    let staging = transaction::staging(dir);
    download(asset("rrbtd6.exe").ok_or("release has no rrbtd6.exe")?, &payload.join("rrbtd6.exe"))?;
    verify(&remote, "rrbtd6.exe", &payload.join("rrbtd6.exe"))?;
    download(asset("rrbtd6-tools.zip").ok_or("release has no rrbtd6-tools.zip")?, &staging.join("tools.zip"))?;
    verify(&remote, "rrbtd6-tools.zip", &staging.join("tools.zip"))?;
    if let Some(url) = asset("rrbtd6-launcher.exe") {
        download(url, &payload.join("rrbtd6-launcher.exe"))?;
        verify(&remote, "rrbtd6-launcher.exe", &payload.join("rrbtd6-launcher.exe"))?;
    }
    // Tools: replace the folder wholesale so removed scripts disappear too.
    let tools_new = staging.join("tools_new");
    zip::ZipArchive::new(fs::File::open(staging.join("tools.zip"))?)?.extract(&tools_new)?;
    let extracted_tools = if tools_new.join("tools").exists() { tools_new.join("tools") } else { tools_new.clone() };
    fs::rename(extracted_tools, payload.join("tools"))?;
    fs::write(payload.join("version.json"), serde_json::to_string(&remote)?)?;
    if needs_assets {
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
    let python = root.join("python.exe");
    let requirements = fs::read_to_string(tools.join("requirements.txt"))?;
    let stamp = root.join("requirements.installed");
    if python.exists() && fs::read_to_string(&stamp).ok().as_deref() == Some(requirements.as_str()) {
        return Ok(python);
    }
    // The release's prepared runtime first (one GitHub download, no python.org / PyPI, nothing
    // built); without it, set one up from python.org and PyPI as before.
    match bundled_python(&root, &requirements) {
        Ok(true) => return Ok(python),
        Ok(false) => {}
        Err(e) => println!("Prepared Python runtime unavailable ({e}); setting one up instead."),
    }
    // A cancelled ZIP extraction can leave python.exe present without its DLLs
    // or standard library. Probe it before treating the runtime as installed.
    let runtime_ready = python.exists() && Command::new(&python)
        .args(["-X", "utf8", "-I", "-c", "import ssl, zipfile, sysconfig"])
        .output().is_ok_and(|out| out.status.success());
    if !runtime_ready {
        println!("Setting up Python {PYTHON_VERSION} (first run only)...");
        fs::create_dir_all(&root)?;
        let archive = root.join("python.zip");
        download(&format!("https://www.python.org/ftp/python/{PYTHON_VERSION}/python-{PYTHON_VERSION}-embed-amd64.zip"), &archive)?;
        zip::ZipArchive::new(fs::File::open(&archive)?)?.extract(&root)?;
        fs::remove_file(&archive)?;
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
    let pip_ready = Command::new(&python).args(["-X", "utf8", "-I", "-m", "pip", "--version"])
        .output().is_ok_and(|out| out.status.success());
    if !pip_ready {
        let get_pip = root.join("get-pip.py");
        download("https://bootstrap.pypa.io/get-pip.py", &get_pip)?;
        check(Command::new(&python).arg(&get_pip).arg("--no-warn-script-location"))?;
    }
    println!("Installing extraction packages...");
    // Embeddable Python's ._pth intentionally ignores PYTHONPATH, including
    // pip's temporary build environment. tpk_ar ships as a source distribution:
    // give its backend an explicit home instead of relying on build isolation.
    check(Command::new(&python).args(["-m", "pip", "install", "--no-warn-script-location",
        "setuptools==80.9.0", "wheel==0.45.1"]))?;
    check(Command::new(&python).args(["-m", "pip", "install", "--no-warn-script-location", "--no-build-isolation", "-r"])
        .arg(tools.join("requirements.txt")))?;
    fs::write(&stamp, requirements)?;
    Ok(python)
}

/// The release tag of the prepared runtime for these requirements (release.yml makes it when
/// the requirements or the Python version change): `python-runtime-` + the first 16 hex digits
/// of SHA-256(requirements.txt + PYTHON_VERSION).
fn runtime_tag(requirements: &str) -> Option<String> {
    let hex = system::sha256_hex(format!("{requirements}{PYTHON_VERSION}").as_bytes())?;
    Some(format!("python-runtime-{}", &hex[..16]))
}

/// Unpacks the prepared runtime (embeddable Python with tools/requirements.txt installed, its
/// stamp included) into `root`. Ok(false): this build has none published.
fn bundled_python(root: &Path, requirements: &str) -> Result<bool> {
    let Some(tag) = runtime_tag(requirements) else { return Ok(false) };
    let url = format!("https://github.com/{REPO}/releases/download/{tag}/rrbtd6-python.zip");
    let parent = root.parent().ok_or("no install folder")?;
    let archive = parent.join("python-runtime.zip");
    println!("Setting up Python (first run only)...");
    if let Err(e) = download(&url, &archive) {
        let _ = fs::remove_file(&archive);
        // 404: not published for these requirements.
        return if e.to_string().contains("404") { Ok(false) } else { Err(e) };
    }
    let unpack = parent.join("python.new");
    let _ = fs::remove_dir_all(&unpack);
    let result = (|| -> Result<()> {
        zip::ZipArchive::new(fs::File::open(&archive)?)?.extract(&unpack)?;
        let ok = Command::new(unpack.join("python.exe")).args(["-X", "utf8", "-I", "-c", "import ssl, zipfile, UnityPy, numpy, PIL"])
            .output().is_ok_and(|out| out.status.success());
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
    check_space(dir)?;
    println!("Checking game data (the first run extracts it from your BTD6 install; this takes a while)...");
    // -X utf8: text as UTF-8 whatever the Windows code page (-I ignores PYTHONUTF8).
    check(Command::new(python).args(["-X", "utf8", "-I"]).arg(dir.join("tools/extract_all.py"))
        .arg("--game").arg(game).arg("--root").arg(dir).current_dir(dir))
}

fn check(cmd: &mut Command) -> Result<()> {
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("{:?} failed ({status})", cmd.get_program()).into());
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
