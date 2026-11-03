//! Monkey XP per tower (Btd6Player's tower XP): what each game earns (`Game::xp_towers`), moved
//! here at every round end as InGame.RoundEnd -> DrainXpAndCheckLevelUp -> Btd6Player.DrainXP
//! does, kept beside the local profile in tower-xp.json.
use std::{collections::BTreeMap, path::Path, sync::{Mutex, OnceLock}};

static XP: OnceLock<Mutex<BTreeMap<String, f64>>> = OnceLock::new();

fn state(root: &Path) -> &'static Mutex<BTreeMap<String, f64>> {
    XP.get_or_init(|| {
        let saved = std::fs::read(root.join("tower-xp.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        Mutex::new(saved.unwrap_or_default())
    })
}

/// Btd6Player.GetTowerXP: tower `id`'s Monkey XP.
pub fn get(root: &Path, id: &str) -> f64 {
    state(root).lock().unwrap().get(id).copied().unwrap_or(0.0)
}

/// Btd6Player.DrainXP's tower part: each tower's earned XP added to its total (the game keeps
/// them as doubles), then the game's tally emptied. The account part (the player's rank XP) is
/// the profile's.
pub fn drain(root: &Path, game: &mut crate::game::Game) {
    game.xp_account = 0.0;
    if game.xp_towers.is_empty() {
        return;
    }
    let snapshot = {
        let mut xp = state(root).lock().unwrap();
        for (id, x) in game.xp_towers.drain(..) {
            *xp.entry(id).or_insert(0.0) += x as f64;
        }
        xp.clone()
    };
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let (file, tmp) = (root.join("tower-xp.json"), root.join("tower-xp.tmp"));
        if serde_json::to_vec_pretty(&snapshot).ok().and_then(|b| std::fs::write(&tmp, b).ok()).is_some() {
            let _ = std::fs::rename(tmp, file);
        }
    });
}

/// LocalizationManager.FormatFloat(x, "N0") in English: whole, rounded half away from zero,
/// with thousands separators.
pub fn format_n0(x: f64) -> String {
    let v = x.abs().round() as u64;
    let digits = v.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if x < 0.0 && v > 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    #[test]
    fn n0_groups_and_rounds() {
        assert_eq!(super::format_n0(0.0), "0");
        assert_eq!(super::format_n0(526.3158), "526");
        assert_eq!(super::format_n0(1973.6842), "1,974");
        assert_eq!(super::format_n0(2.5), "3");
        assert_eq!(super::format_n0(1234567.0), "1,234,567");
    }
}
