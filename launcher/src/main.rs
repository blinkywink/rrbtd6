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
        pause();
        std::process::exit(1);
    }
}

fn pause() {
    eprint!("Press Enter to close...");
    let _ = io::stdin().read_line(&mut String::new());
}

fn run() -> Result<()> {
    let dir = std::env::current_exe()?.parent().ok_or("no install folder")?.to_path_buf();
    let _install_lock = transaction::lock(&dir)?;
    transaction::recover(&dir)?;
    let skip_update = std::env::args().any(|a| a == "--offline");
    let refresh_assets=std::env::args().any(|a|a=="--refresh-assets");
    let config: serde_json::Value = fs::read_to_string(dir.join("rrbtd6.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let game = config["game"].as_str().map(PathBuf::from).or_else(find_btd6)
        .ok_or("BTD6 folder unknown: install through the BTD6 mod, or write {\"game\": \"<BTD6 folder>\"} to rrbtd6.json")?;
    if !game.join("GameAssembly.dll").exists() {
        return Err(format!("{} is not a BTD6 install", game.display()).into());
    }

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
    Command::new(dir.join("rrbtd6.exe")).arg("--from-launcher").current_dir(&dir).spawn()?;
    Ok(())
}

/// Default Steam / Epic install locations.
fn find_btd6() -> Option<PathBuf> {
    ["C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6", "C:/Program Files/Epic Games/BloonsTD6"]
        .iter().map(PathBuf::from).find(|p| p.join("GameAssembly.dll").exists())
}

fn get(url: &str) -> Result<ureq::Response> {
    static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    let agent = AGENT.get_or_init(|| ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60))
        .timeout_write(std::time::Duration::from_secs(60)).build());
    Ok(agent.get(url).set("User-Agent", USER_AGENT).call()?)
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
    download(asset("rrbtd6-tools.zip").ok_or("release has no rrbtd6-tools.zip")?, &staging.join("tools.zip"))?;
    if let Some(url) = asset("rrbtd6-launcher.exe") {
        download(url, &payload.join("rrbtd6-launcher.exe"))?;
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
        for name in ["settings.json","local-profile.json","local-profile.backup.json"] {
            let old=dir.join("local_extracted").join(name);
            if old.is_file(){fs::copy(old,payload.join("local_extracted").join(name))?;}
        }
        for name in ["account-cache","online"]{preserve_user_tree(&dir.join("local_extracted").join(name),&payload.join("local_extracted").join(name))?;}
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
    // A cancelled ZIP extraction can leave python.exe present without its DLLs
    // or standard library. Probe it before treating the runtime as installed.
    let runtime_ready = python.exists() && Command::new(&python)
        .args(["-I", "-c", "import ssl, zipfile, sysconfig"])
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
    let pip_ready = Command::new(&python).args(["-I", "-m", "pip", "--version"])
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

/// tools/extract_all.py skips finished steps and invalidates them by game fingerprint/output version.
fn extract(dir: &Path, python: &Path, game: &Path) -> Result<()> {
    println!("Checking game data (the first run extracts it from your BTD6 install; this takes a while)...");
    check(Command::new(python).arg("-I").arg(dir.join("tools/extract_all.py"))
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
