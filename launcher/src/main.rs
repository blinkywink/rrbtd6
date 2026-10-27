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

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "blinkywink/rrbtd6";
const PYTHON_VERSION: &str = "3.13.1";
const USER_AGENT: &str = "rrbtd6-launcher";

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
    let config: serde_json::Value = fs::read_to_string(dir.join("rrbtd6.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let game = config["game"].as_str().map(PathBuf::from).or_else(find_btd6)
        .ok_or("BTD6 folder unknown: install through the BTD6 mod, or write {\"game\": \"<BTD6 folder>\"} to rrbtd6.json")?;
    if !game.join("GameAssembly.dll").exists() {
        return Err(format!("{} is not a BTD6 install", game.display()).into());
    }

    if !skip_update {
        match update(&dir) {
            Ok(true) => println!("Updated to the latest build."),
            Ok(false) => println!("Up to date."),
            // Offline or GitHub unreachable: keep playing the installed build if there is one.
            Err(e) if dir.join("rrbtd6.exe").exists() && !transaction::pending(&dir) => println!("Update check skipped: {e}"),
            Err(e) => return Err(e),
        }
    }
    let python = ensure_python(&dir)?;
    extract(&dir, &python, &game)?;

    println!("Starting rrbtd6...");
    Command::new(dir.join("rrbtd6.exe")).current_dir(&dir).spawn()?;
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
fn update(dir: &Path) -> Result<bool> {
    let latest: serde_json::Value = get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))?.into_json()?;
    let assets = latest["assets"].as_array().ok_or("release has no assets")?;
    let asset = |name: &str| assets.iter().find(|a| a["name"] == name).and_then(|a| a["browser_download_url"].as_str());
    let remote: serde_json::Value = get(asset("version.json").ok_or("release has no version.json")?)?.into_json()?;
    let build = remote["build"].as_u64().ok_or("release version has no valid build number")?;
    let local: serde_json::Value = fs::read_to_string(dir.join("version.json")).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if local["build"].as_u64() == Some(build) && dir.join("rrbtd6.exe").is_file()
        && dir.join("tools/extract_all.py").is_file() && dir.join("tools/requirements.txt").is_file() {
        return Ok(false);
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
    transaction::install(dir)?;
    Ok(true)
}

/// A private embeddable Python with the extraction packages (tools/requirements.txt).
fn ensure_python(dir: &Path) -> Result<PathBuf> {
    let root = dir.join("python");
    let python = root.join("python.exe");
    let requirements = fs::read_to_string(dir.join("tools/requirements.txt"))?;
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
        .arg(dir.join("tools/requirements.txt")))?;
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
