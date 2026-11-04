//! Not in BTD6 (DEVIATIONS.md 14): joining a private room that uses mods first shows a
//! CommonPopup listing them, asking whether to download them to play. Download keeps a copy of
//! each one the player doesn't have yet in their own mods folder (left disabled; the room uses
//! the host's copies either way) and joins; Leave leaves the room.

use crate::uitree::{block_input, draw_tree, Ctx, Node, R};
use macroquad::prelude::*;
use std::path::Path;

const LAYOUT: &str = "AnimatedObject/Layout";
/// Marks a mod folder made from a co-op room (rewritable by later rooms).
const ROOM_MARK: &str = "ROOM";
/// Popup.TransitionAnim.Scale: seconds the popup takes to scale in (as the welcome popup).
const OPEN_SECS: f64 = 0.25;

pub enum Choice {
    Download,
    Leave,
}

pub struct ModsPopup {
    node: Option<Node>,
    opened_at: Option<f64>,
    /// The room mod set (digests) the player already answered for.
    answered: Option<Vec<String>>,
}

impl ModsPopup {
    pub fn load(prefabs: &Path) -> Self {
        let node = Node::load(&prefabs.join("CommonPopup.json")).map(|mut n| {
            for p in ["Buttons/ConfirmButton/CashInfo", "CashInfo", "UserId", "GameVersion", "Cross", "Tick", "Spinner"] {
                n.set_active(&format!("{LAYOUT}/{p}"), false);
            }
            n.hide_named(&["Jewel"]);
            if let Some(b) = n.find_mut(&format!("{LAYOUT}/Body")).and_then(|b| b.txt.as_mut()) {
                b.size = 60.0;
            }
            n.set_text(&format!("{LAYOUT}/Banner/Title"), "This match uses mods");
            n.set_text(&format!("{LAYOUT}/Buttons/ConfirmButton/ConfirmText"), "Download");
            n.set_text(&format!("{LAYOUT}/Buttons/CancelButton/CancelText"), "Leave");
            n
        });
        Self { node, opened_at: None, answered: None }
    }

    pub fn is_open(&self) -> bool {
        self.opened_at.is_some() && self.node.is_some()
    }

    /// A guest in a room: opens the popup when the room's mod set is new to this player.
    pub fn offer(&mut self, mods: &[rrbtd6_coop::ModPackage]) {
        if mods.is_empty() || self.is_open() {
            return;
        }
        let set: Vec<String> = mods.iter().map(|m| m.digest.clone()).collect();
        if self.answered.as_ref() == Some(&set) {
            return;
        }
        let names: Vec<String> = mods.iter().map(|m| {
            let name = serde_json::from_str::<serde_json::Value>(&m.manifest).ok()
                .and_then(|v| v["name"].as_str().map(str::to_owned)).unwrap_or_else(|| m.id.clone());
            format!("{name} ({})", m.version)
        }).collect();
        let body = format!("The host is using these mods:\n{}\n\nDownload them to play?", names.join("\n"));
        if let Some(n) = self.node.as_mut() {
            n.set_text(&format!("{LAYOUT}/Body"), &body);
        }
        self.answered = Some(set);
        self.opened_at = Some(get_time());
    }

    /// Forgets the last answer (leaving a room, or a new room).
    pub fn reset(&mut self) {
        self.answered = None;
        self.opened_at = None;
    }

    /// Keeps a copy of each room mod the player doesn't have yet in their mods folder (marked
    /// with a ROOM file: a room may later update it), and lists the art files still to fetch
    /// from the host. A mod of the player's own with different art is left alone: an error.
    pub fn download(root: &Path, mods: &[rrbtd6_coop::ModPackage]) -> Result<Vec<rrbtd6_coop::AssetFile>, String> {
        let dir = crate::modding::directory(root);
        for m in mods {
            // Package IDs are validated (lowercase letters, digits, ._-), so this stays inside mods/.
            let folder = dir.join(&m.id);
            if !folder.exists() {
                std::fs::create_dir_all(&folder).map_err(|e| format!("Cannot create mods/{}: {}", m.id, e.kind()))?;
                let _ = std::fs::write(folder.join("mod.json"), &m.manifest);
                let _ = std::fs::write(folder.join(ROOM_MARK), "Downloaded from a co-op room.\n");
            }
            let own = !folder.join(ROOM_MARK).is_file();
            if !own {
                let _ = std::fs::write(folder.join("mod.json"), &m.manifest);
            }
            for f in &m.assets {
                let path = folder.join(&f.path);
                if Self::have(&path, f) {
                    continue;
                }
                if own {
                    return Err(format!("Your copy of {} has different files from the host's; update it to join", m.id));
                }
                if f.size == 0 {
                    std::fs::create_dir_all(path.parent().unwrap_or(&folder)).and_then(|_| std::fs::write(&path, b"")).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(Self::missing(root, mods))
    }

    /// The room's art files this player doesn't have (by size; downloads are hash-checked).
    pub fn missing(root: &Path, mods: &[rrbtd6_coop::ModPackage]) -> Vec<rrbtd6_coop::AssetFile> {
        let dir = crate::modding::directory(root);
        mods.iter().flat_map(|m| m.assets.iter().filter(|f| !Self::have(&dir.join(&m.id).join(&f.path), f)).cloned().collect::<Vec<_>>()).collect()
    }

    fn have(path: &Path, f: &rrbtd6_coop::AssetFile) -> bool {
        std::fs::metadata(path).is_ok_and(|m| m.len() == f.size)
    }

    /// A downloaded art file (already checked against its hash): written wherever the room lists
    /// it, into room-downloaded mod folders only.
    pub fn save(root: &Path, mods: &[rrbtd6_coop::ModPackage], digest: &str, bytes: &[u8]) -> Result<(), String> {
        let dir = crate::modding::directory(root);
        for m in mods {
            let folder = dir.join(&m.id);
            if !folder.join(ROOM_MARK).is_file() {
                continue;
            }
            for f in m.assets.iter().filter(|f| f.digest == digest && rrbtd6_coop::mods::asset_path_ok(&f.path)) {
                let path = folder.join(&f.path);
                std::fs::create_dir_all(path.parent().unwrap_or(&folder)).and_then(|_| std::fs::write(&path, bytes)).map_err(|e| format!("Cannot save {}: {}", f.path, e.kind()))?;
            }
        }
        Ok(())
    }

    /// Draws the popup over the dimmed screen; the player's answer on the frame it is given.
    pub fn draw(&mut self, full: R, ctx: &mut Ctx) -> Option<Choice> {
        let opened = self.opened_at?;
        let n = self.node.as_mut()?;
        let k = ((get_time() - opened) / OPEN_SECS).clamp(0.0, 1.0) as f32;
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.5 * k));
        let back = 1.70158;
        let e = 1.0 + (back + 1.0) * (k - 1.0).powi(3) + back * (k - 1.0).powi(2);
        if let Some(a) = n.find_mut("AnimatedObject") {
            a.rt.scale = vec2(e, e);
        }
        block_input(false);
        draw_tree(n, full, ctx);
        let choice = if k < 1.0 {
            None
        } else if n.clicked(&format!("{LAYOUT}/Buttons/ConfirmButton")) || is_key_pressed(KeyCode::Enter) {
            Some(Choice::Download)
        } else if n.clicked(&format!("{LAYOUT}/Buttons/CancelButton")) || is_key_pressed(KeyCode::Escape) {
            Some(Choice::Leave)
        } else {
            None
        };
        if choice.is_some() {
            self.opened_at = None;
        }
        block_input(choice.is_none());
        choice
    }
}

#[cfg(test)]
mod tests {
    use super::ModsPopup;

    fn fresh(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rrbtd6-room-art-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data/game")).unwrap();
        // room_packages checks the mods against the base game data.
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/game/game.json");
        std::fs::copy(base, dir.join("data/game/game.json")).unwrap();
        dir.join("data")
    }

    #[test]
    fn room_art_is_listed_fetched_and_saved_into_room_folders_only() {
        let host = fresh("host");
        let folder = crate::modding::directory(&host).join("example");
        std::fs::create_dir_all(folder.join("assets/robot")).unwrap();
        std::fs::write(folder.join("mod.json"), r#"{"api_version":1,"id":"example","name":"Example","version":"1"}"#).unwrap();
        std::fs::write(folder.join("assets/robot/meta.json"), b"{}").unwrap();
        std::fs::write(folder.join("assets/robot/mesh.bin"), vec![7u8; 100_000]).unwrap();
        let (mods, store) = crate::modding::room_packages_with_assets(&host, &["example".into()]).unwrap();
        assert_eq!(mods[0].assets.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["assets/robot/mesh.bin", "assets/robot/meta.json"]);
        // A guest without the mod: a marked copy, then each file as it arrives.
        let guest = fresh("guest");
        let wanted = ModsPopup::download(&guest, &mods).unwrap();
        assert_eq!(wanted.len(), 2);
        for f in &wanted {
            ModsPopup::save(&guest, &mods, &f.digest, &store[&f.digest]).unwrap();
        }
        assert!(ModsPopup::missing(&guest, &mods).is_empty());
        assert_eq!(std::fs::read(crate::modding::directory(&guest).join("example/assets/robot/mesh.bin")).unwrap().len(), 100_000);
        // A guest whose own copy differs is told so, and their files are left alone.
        let other = fresh("other");
        let own = crate::modding::directory(&other).join("example/assets/robot");
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(own.join("meta.json"), b"{\"mine\":1}").unwrap();
        assert!(ModsPopup::download(&other, &mods).is_err());
        ModsPopup::save(&other, &mods, &mods[0].assets[1].digest, b"{}").unwrap();
        assert_eq!(std::fs::read(own.join("meta.json")).unwrap(), b"{\"mine\":1}");
        for d in [host, guest, other] {
            let _ = std::fs::remove_dir_all(d.parent().unwrap());
        }
    }
}
