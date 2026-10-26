//! Hotkeys (BTD6's HotkeysData; rebindable on the Hotkeys screen, saved in settings.json
//! "hotkeys" as action -> key name, "" for unbound).
//!
//! The screen's sections follow HotkeysData's lists: monkeys / gameplay / sandbox and their
//! "Advanced Hotkeys" counterparts, actions named after HotkeysData's fields. Defaults are the
//! well-known BTD6 ones; actions whose default is a key chord (Shift+Space ...) or that we don't
//! know start unbound. STAND-IN: HotkeysData.Default's exact table isn't read out of the game.

use macroquad::prelude::KeyCode;
use std::collections::HashMap;

/// Tower hotkeys in shop order, with the game's default keys (Q W E R T Y primaries, Z X C V B
/// N M military, A S D F G magic, H J K L support, then the newer towers).
pub const TOWER_DEFAULTS: [(&str, Option<KeyCode>); 26] = [
    ("DartMonkey", Some(KeyCode::Q)),
    ("BoomerangMonkey", Some(KeyCode::W)),
    ("BombShooter", Some(KeyCode::E)),
    ("TackShooter", Some(KeyCode::R)),
    ("IceMonkey", Some(KeyCode::T)),
    ("GlueGunner", Some(KeyCode::Y)),
    ("Desperado", Some(KeyCode::P)),
    ("SniperMonkey", Some(KeyCode::Z)),
    ("MonkeySub", Some(KeyCode::X)),
    ("MonkeyBuccaneer", Some(KeyCode::C)),
    ("MonkeyAce", Some(KeyCode::V)),
    ("HeliPilot", Some(KeyCode::B)),
    ("MortarMonkey", Some(KeyCode::N)),
    ("DartlingGunner", Some(KeyCode::M)),
    ("WizardMonkey", Some(KeyCode::A)),
    ("SuperMonkey", Some(KeyCode::S)),
    ("NinjaMonkey", Some(KeyCode::D)),
    ("Alchemist", Some(KeyCode::F)),
    ("Druid", Some(KeyCode::G)),
    ("Mermonkey", Some(KeyCode::O)),
    ("BananaFarm", Some(KeyCode::H)),
    ("SpikeFactory", Some(KeyCode::J)),
    ("MonkeyVillage", Some(KeyCode::K)),
    ("EngineerMonkey", Some(KeyCode::L)),
    ("BeastHandler", Some(KeyCode::I)),
    ("Skywarden", Some(KeyCode::Apostrophe)),
];

/// An action row: (action id, label / localization key, default key).
pub type Row = (&'static str, &'static str, Option<KeyCode>);

pub const MONKEYS_EXTRA: [Row; 1] = [("hero", "Heroes", Some(KeyCode::U))];

pub const GAMEPLAY: [Row; 11] = [
    ("upgrade1", "Upgrade Path 1", Some(KeyCode::Comma)),
    ("upgrade2", "Upgrade Path 2", Some(KeyCode::Period)),
    ("upgrade3", "Upgrade Path 3", Some(KeyCode::Slash)),
    ("targeting", "ChangeTargeting", Some(KeyCode::Tab)),
    ("reverse_targeting", "ReverseChangeTargeting", None),
    ("special", "Tower Special", Some(KeyCode::PageDown)),
    ("special2", "Tower Special 2", Some(KeyCode::PageUp)),
    ("sell", "Sell", Some(KeyCode::Backspace)),
    ("play", "Play / Fast Forward", Some(KeyCode::Space)),
    ("send_next_round", "SendNextRound", None),
    ("pause", "Pause", Some(KeyCode::Escape)),
];

pub const SANDBOX: [Row; 17] = [
    ("send:Red", "Send Red Bloon", None),
    ("send:Blue", "Send Blue Bloon", None),
    ("send:Green", "Send Green Bloon", None),
    ("send:Yellow", "Send Yellow Bloon", None),
    ("send:Pink", "Send Pink Bloon", None),
    ("send:Black", "Send Black Bloon", None),
    ("send:Purple", "Send Purple Bloon", None),
    ("send:White", "Send White Bloon", None),
    ("send:Lead", "Send Lead Bloon", None),
    ("send:Zebra", "Send Zebra Bloon", None),
    ("send:Rainbow", "Send Rainbow Bloon", None),
    ("send:Ceramic", "Send Ceramic Bloon", None),
    ("send:Moab", "Send MOAB", None),
    ("send:Bfb", "Send BFB", None),
    ("send:Zomg", "Send ZOMG", None),
    ("send:Ddt", "Send DDT", None),
    ("send:Bad", "Send BAD", None),
];

pub const GAMEPLAY_ADVANCED: [Row; 19] = [
    ("ability1", "Activated Ability 1", Some(KeyCode::Key1)),
    ("ability2", "Activated Ability 2", Some(KeyCode::Key2)),
    ("ability3", "Activated Ability 3", Some(KeyCode::Key3)),
    ("ability4", "Activated Ability 4", Some(KeyCode::Key4)),
    ("ability5", "Activated Ability 5", Some(KeyCode::Key5)),
    ("ability6", "Activated Ability 6", Some(KeyCode::Key6)),
    ("ability7", "Activated Ability 7", Some(KeyCode::Key7)),
    ("ability8", "Activated Ability 8", Some(KeyCode::Key8)),
    ("ability9", "Activated Ability 9", Some(KeyCode::Key9)),
    ("ability10", "Activated Ability 10", Some(KeyCode::Key0)),
    ("ability11", "Activated Ability 11", Some(KeyCode::Minus)),
    ("ability12", "Activated Ability 12", Some(KeyCode::Equal)),
    ("selected_ability1", "Selected Tower Ability 1", None),
    ("selected_ability2", "Selected Tower Ability 2", None),
    ("selected_ability3", "Selected Tower Ability 3", None),
    ("copy", "Copy", None),
    ("merge_beast", "Merge Beast", None),
    ("quick_restart", "Quick Restart", None),
    ("centered_path", "Centered Path Position", None),
];

pub const SANDBOX_ADVANCED: [Row; 6] = [
    ("clear_bloons", "Clear Bloons", None),
    ("clear_towers", "Clear Towers", None),
    ("reset_damage", "Reset Damage", None),
    ("reset_cooldowns", "Reset Cooldowns", None),
    ("set_cash", "Set Cash", None),
    ("set_health", "Set Health", None),
];

/// Keys a binding may use, with their display / save names.
pub const KEYS: [(KeyCode, &str); 67] = [
    (KeyCode::A, "A"), (KeyCode::B, "B"), (KeyCode::C, "C"), (KeyCode::D, "D"), (KeyCode::E, "E"), (KeyCode::F, "F"),
    (KeyCode::G, "G"), (KeyCode::H, "H"), (KeyCode::I, "I"), (KeyCode::J, "J"), (KeyCode::K, "K"), (KeyCode::L, "L"),
    (KeyCode::M, "M"), (KeyCode::N, "N"), (KeyCode::O, "O"), (KeyCode::P, "P"), (KeyCode::Q, "Q"), (KeyCode::R, "R"),
    (KeyCode::S, "S"), (KeyCode::T, "T"), (KeyCode::U, "U"), (KeyCode::V, "V"), (KeyCode::W, "W"), (KeyCode::X, "X"),
    (KeyCode::Y, "Y"), (KeyCode::Z, "Z"), (KeyCode::Key0, "0"), (KeyCode::Key1, "1"), (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"), (KeyCode::Key4, "4"), (KeyCode::Key5, "5"), (KeyCode::Key6, "6"), (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"), (KeyCode::Key9, "9"), (KeyCode::Minus, "-"), (KeyCode::Equal, "="), (KeyCode::Comma, ","),
    (KeyCode::Period, "."), (KeyCode::Slash, "/"), (KeyCode::Semicolon, ";"), (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["), (KeyCode::RightBracket, "]"), (KeyCode::Backslash, "\\"), (KeyCode::GraveAccent, "`"),
    (KeyCode::Tab, "Tab"), (KeyCode::Backspace, "Backspace"), (KeyCode::Space, "Space"), (KeyCode::Enter, "Enter"),
    (KeyCode::Escape, "Escape"), (KeyCode::Delete, "Delete"), (KeyCode::Insert, "Insert"), (KeyCode::Home, "Home"),
    (KeyCode::End, "End"), (KeyCode::PageUp, "Page Up"), (KeyCode::PageDown, "Page Down"), (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"), (KeyCode::Left, "Left"), (KeyCode::Right, "Right"), (KeyCode::Kp0, "Num 0"),
    (KeyCode::Kp1, "Num 1"), (KeyCode::Kp2, "Num 2"), (KeyCode::Kp3, "Num 3"), (KeyCode::Kp4, "Num 4"),
];

pub fn key_name(k: KeyCode) -> &'static str {
    KEYS.iter().find(|(c, _)| *c == k).map_or("?", |(_, n)| n)
}

fn key_from(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(_, n)| *n == name).map(|(c, _)| *c)
}

/// Every non-tower row.
pub fn all_rows() -> impl Iterator<Item = &'static Row> {
    MONKEYS_EXTRA.iter().chain(GAMEPLAY.iter()).chain(SANDBOX.iter()).chain(GAMEPLAY_ADVANCED.iter()).chain(SANDBOX_ADVANCED.iter())
}

#[derive(Clone)]
pub struct Keybinds {
    /// action -> key ("tower:<id>" for towers); unbound actions are absent.
    pub keys: HashMap<String, KeyCode>,
    /// A text field has the keyboard: no hotkey fires.
    pub muted: bool,
}

impl Keybinds {
    pub fn defaults() -> Self {
        let mut keys: HashMap<String, KeyCode> =
            TOWER_DEFAULTS.iter().filter_map(|(t, k)| Some((format!("tower:{t}"), (*k)?))).collect();
        keys.extend(all_rows().filter_map(|(a, _, k)| Some((a.to_string(), (*k)?))));
        Self { keys, muted: false }
    }

    pub fn load(settings: &serde_json::Value) -> Self {
        let mut b = Self::defaults();
        if let Some(m) = settings["hotkeys"].as_object() {
            for (a, v) in m {
                match v.as_str().and_then(key_from) {
                    Some(k) => {
                        b.keys.insert(a.clone(), k);
                    }
                    None if v.as_str() == Some("") => {
                        b.keys.remove(a);
                    }
                    None => {}
                }
            }
        }
        b
    }

    pub fn to_json(&self) -> serde_json::Value {
        let every = TOWER_DEFAULTS.iter().map(|(t, _)| format!("tower:{t}")).chain(all_rows().map(|(a, _, _)| a.to_string()));
        serde_json::Value::Object(every.map(|a| {
            let v = self.keys.get(&a).map_or("", |k| key_name(*k));
            (a, serde_json::json!(v))
        }).collect())
    }

    pub fn pressed(&self, action: &str) -> bool {
        !self.muted && self.keys.get(action).is_some_and(|k| macroquad::prelude::is_key_pressed(*k))
    }

    /// Binds `action` to `key`; another action that had the key gets this action's old key
    /// (a swap, or unbound if this one had none).
    pub fn set(&mut self, action: &str, key: KeyCode) {
        let old = self.keys.get(action).copied();
        if let Some(other) = self.keys.iter().find(|(a, k)| **k == key && a.as_str() != action).map(|(a, _)| a.clone()) {
            match old {
                Some(o) => {
                    self.keys.insert(other, o);
                }
                None => {
                    self.keys.remove(&other);
                }
            }
        }
        self.keys.insert(action.to_string(), key);
    }
}

/// The key pressed this frame, if it's one a binding may use.
pub fn any_pressed() -> Option<KeyCode> {
    KEYS.iter().map(|(k, _)| *k).find(|k| macroquad::prelude::is_key_pressed(*k))
}
