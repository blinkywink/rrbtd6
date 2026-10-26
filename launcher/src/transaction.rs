//! Recoverable replacement of one release. Downloads/extraction finish before
//! any installed component moves; the journal stays until commit or rollback.
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Result;

const COMPONENTS: [&str; 4] = ["tools", "rrbtd6.exe", "rrbtd6-launcher.exe", "version.json"];

pub fn staging(dir: &Path) -> PathBuf {
    dir.join("update")
}
pub fn payload(dir: &Path) -> PathBuf {
    staging(dir).join("payload")
}

/// The open file lock is released by the OS even if the process crashes.
pub fn lock(dir: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(".launcher.lock"))?;
    file.try_lock()
        .map_err(|e| format!("Another launcher is installing or extracting in this folder: {e}"))?;
    Ok(file)
}

pub fn pending(dir: &Path) -> bool {
    staging(dir).join("install.json").exists() && !committed(&staging(dir)).unwrap_or(false)
}

fn committed(stage: &Path) -> io::Result<bool> {
    match fs::read(stage.join("committed")) {
        Ok(bytes) => Ok(bytes == b"complete"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn remove(path: &Path) -> io::Result<()> {
    if !path.try_exists()? {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn durable_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("pending");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)
}

/// Restore an interrupted installation before considering network access or
/// --offline play. Repeated recovery is safe, including after partial rollback.
pub fn recover(dir: &Path) -> Result<()> {
    let stage = staging(dir);
    if !stage.try_exists()? {
        return Ok(());
    }
    let journal = stage.join("install.json");
    if journal.try_exists()? && !committed(&stage)? {
        let plan: serde_json::Value = serde_json::from_slice(&fs::read(&journal)?)?;
        let entries = plan.as_array().ok_or("Invalid update recovery journal")?;
        let mut seen = std::collections::HashSet::new();
        // Validate the entire journal before touching any installed path.
        for entry in entries {
            let name = entry["name"].as_str().ok_or("Invalid update component")?;
            if !COMPONENTS.contains(&name) || !seen.insert(name) || !entry["old"].is_boolean() {
                return Err(
                    "Invalid update recovery journal; installed files were left in place".into(),
                );
            }
        }
        for entry in entries.iter().rev() {
            let name = entry["name"].as_str().unwrap();
            let target = dir.join(name);
            let backup = stage.join("rollback").join(name);
            if entry["old"].as_bool().unwrap() {
                if backup.try_exists()? {
                    remove(&target)?;
                    fs::rename(backup, target)?;
                }
            } else if !payload(dir).join(name).try_exists()? {
                // No previous component; a consumed payload means we installed it.
                remove(&target)?;
            }
        }
    }
    remove(&stage)?;
    Ok(())
}

pub fn prepare(dir: &Path) -> Result<PathBuf> {
    recover(dir)?;
    let payload = payload(dir);
    fs::create_dir_all(&payload)?;
    Ok(payload)
}

fn begin(dir: &Path) -> Result<Vec<&'static str>> {
    let payload = payload(dir);
    for name in [
        "rrbtd6.exe",
        "version.json",
        "tools/extract_all.py",
        "tools/requirements.txt",
    ] {
        if !payload.join(name).is_file() {
            return Err(format!(
                "Incomplete release: missing {name}; installed build was left in place"
            )
            .into());
        }
    }
    let names: Vec<_> = COMPONENTS
        .into_iter()
        .filter(|name| payload.join(name).exists())
        .collect();
    let mut plan = Vec::new();
    for name in &names {
        plan.push(serde_json::json!({"name": name, "old": dir.join(name).try_exists()?}));
    }
    fs::create_dir_all(staging(dir).join("rollback"))?;
    durable_write(
        &staging(dir).join("install.json"),
        &serde_json::to_vec(&plan)?,
    )?;
    Ok(names)
}

fn move_component(dir: &Path, name: &str) -> Result<()> {
    let target = dir.join(name);
    if target.try_exists()? {
        fs::rename(&target, staging(dir).join("rollback").join(name))?;
    }
    fs::rename(payload(dir).join(name), target)?;
    Ok(())
}

pub fn install(dir: &Path) -> Result<()> {
    let names = begin(dir)?;
    let result = (|| -> Result<()> {
        for name in names {
            move_component(dir, name)?;
        }
        // version.json moves last. Only a fully installed release is committed.
        durable_write(&staging(dir).join("committed"), b"complete")?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Err(rollback) = recover(dir) {
            return Err(format!("Update failed: {error}. Recovery is pending: {rollback}. Close programs using this install and start the launcher again.").into());
        }
        return Err(error);
    }
    // The previous launcher image may still be running from rollback/. Windows
    // then defers deletion until recovery on the next launch, after it exits.
    let _ = recover(dir);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "rrbtd6-update-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn release(dir: &Path, value: &str, launcher: bool) {
        fs::create_dir_all(dir.join("tools")).unwrap();
        for name in [
            "tools/extract_all.py",
            "tools/requirements.txt",
            "rrbtd6.exe",
            "version.json",
        ] {
            fs::write(dir.join(name), value).unwrap();
        }
        if launcher {
            fs::write(dir.join("rrbtd6-launcher.exe"), value).unwrap();
        }
    }

    fn check(dir: &Path, value: &str, launcher: bool) {
        for name in COMPONENTS {
            if name == "tools" {
                assert_eq!(
                    fs::read_to_string(dir.join("tools/extract_all.py")).unwrap(),
                    value
                );
            } else if launcher || name != "rrbtd6-launcher.exe" {
                assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), value);
            }
        }
    }

    #[test]
    fn installs_complete_release_and_preserves_user_files() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        fs::create_dir(dir.join("local_extracted")).unwrap();
        fs::write(dir.join("local_extracted/settings.json"), "preferences").unwrap();
        fs::write(dir.join("rrbtd6.json"), "game path").unwrap();
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        install(dir).unwrap();
        check(dir, "new", true);
        assert_eq!(
            fs::read_to_string(dir.join("local_extracted/settings.json")).unwrap(),
            "preferences"
        );
        assert_eq!(
            fs::read_to_string(dir.join("rrbtd6.json")).unwrap(),
            "game path"
        );
        assert!(!staging(dir).exists());
    }

    #[test]
    fn interrupted_swaps_restore_every_previous_component() {
        // Every cut between moving an old component aside and installing its
        // replacement, including after all swaps but before the commit marker.
        for cut in 0..=COMPONENTS.len() * 2 {
            let temp = Temp::new();
            let dir = &temp.0;
            release(dir, "old", true);
            let payload = prepare(dir).unwrap();
            release(&payload, "new", true);
            let names = begin(dir).unwrap();
            for (index, name) in names.iter().enumerate() {
                if index * 2 < cut {
                    fs::rename(dir.join(name), staging(dir).join("rollback").join(name)).unwrap();
                }
                if index * 2 + 1 < cut {
                    fs::rename(payload.join(name), dir.join(name)).unwrap();
                }
            }
            recover(dir).unwrap();
            recover(dir).unwrap();
            check(dir, "old", true);
        }
    }

    #[test]
    fn interrupted_first_install_removes_only_new_components() {
        let temp = Temp::new();
        let dir = &temp.0;
        let payload = prepare(dir).unwrap();
        release(&payload, "new", false);
        let names = begin(dir).unwrap();
        for name in names {
            move_component(dir, name).unwrap();
        }
        recover(dir).unwrap();
        for name in COMPONENTS {
            assert!(!dir.join(name).exists());
        }
    }

    #[test]
    fn committed_install_survives_deferred_cleanup() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        for name in begin(dir).unwrap() {
            move_component(dir, name).unwrap();
        }
        durable_write(&staging(dir).join("committed"), b"complete").unwrap();
        recover(dir).unwrap();
        check(dir, "new", true);
    }

    #[test]
    fn incomplete_commit_marker_rolls_back() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        for name in begin(dir).unwrap() {
            move_component(dir, name).unwrap();
        }
        fs::write(staging(dir).join("committed"), b"comp").unwrap();
        assert!(pending(dir));
        recover(dir).unwrap();
        check(dir, "old", true);
    }

    #[test]
    fn incomplete_payload_never_changes_installed_build() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        fs::remove_file(payload.join("tools/requirements.txt")).unwrap();
        assert!(install(dir).is_err());
        check(dir, "old", true);
        recover(dir).unwrap();
        check(dir, "old", true);
    }

    #[cfg(windows)]
    #[test]
    fn locked_executable_rolls_back_tools_already_replaced() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(dir.join("rrbtd6.exe"))
            .unwrap();
        assert!(install(dir).is_err());
        drop(held);
        check(dir, "old", true);
        assert!(!pending(dir));
    }

    #[cfg(windows)]
    #[test]
    fn failed_rollback_can_be_retried_after_unlocking() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        let payload = prepare(dir).unwrap();
        release(&payload, "new", true);
        begin(dir).unwrap();
        move_component(dir, "tools").unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(dir.join("tools/extract_all.py"))
            .unwrap();
        assert!(recover(dir).is_err());
        assert!(pending(dir));
        drop(held);
        recover(dir).unwrap();
        check(dir, "old", true);
        assert!(!pending(dir));
    }

    #[cfg(windows)]
    #[test]
    fn replaces_its_own_running_image_and_recovers_on_next_start() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", false);
        let executable = std::env::current_exe().unwrap();
        fs::copy(&executable, dir.join("rrbtd6-launcher.exe")).unwrap();
        let payload = prepare(dir).unwrap();
        release(&payload, "new", false);
        fs::copy(&executable, payload.join("rrbtd6-launcher.exe")).unwrap();
        let status = std::process::Command::new(dir.join("rrbtd6-launcher.exe"))
            .args([
                "--ignored",
                "--exact",
                "transaction::tests::running_image_helper",
            ])
            .env("RRBTD6_UPDATE_TEST_DIR", dir)
            .status()
            .unwrap();
        assert!(status.success());
        recover(dir).unwrap();
        check(dir, "new", false);
        assert_eq!(
            fs::read(dir.join("rrbtd6-launcher.exe")).unwrap(),
            fs::read(executable).unwrap()
        );
        assert!(!staging(dir).exists());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "subprocess fixture invoked by the running-image test"]
    fn running_image_helper() {
        let dir =
            PathBuf::from(std::env::var_os("RRBTD6_UPDATE_TEST_DIR").expect("fixture directory"));
        assert_eq!(dir.parent(), Some(std::env::temp_dir().as_path()));
        assert!(
            dir.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("rrbtd6-update-test-")
        );
        assert_eq!(
            std::env::current_exe().unwrap().canonicalize().unwrap(),
            dir.join("rrbtd6-launcher.exe").canonicalize().unwrap()
        );
        install(&dir).unwrap();
        check(&dir, "new", false);
        assert!(dir.join("rrbtd6-launcher.exe").is_file());
    }

    #[test]
    fn install_lock_is_exclusive_and_released_on_drop() {
        let temp = Temp::new();
        let dir = &temp.0;
        let held = lock(dir).unwrap();
        assert!(lock(dir).is_err());
        drop(held);
        assert!(lock(dir).is_ok());
    }

    #[test]
    fn invalid_recovery_paths_leave_install_untouched() {
        let temp = Temp::new();
        let dir = &temp.0;
        release(dir, "old", true);
        prepare(dir).unwrap();
        fs::write(
            staging(dir).join("install.json"),
            br#"[{"name":"../rrbtd6.json","old":false},{"name":"tools","old":false}]"#,
        )
        .unwrap();
        assert!(recover(dir).is_err());
        check(dir, "old", true);
    }
}
