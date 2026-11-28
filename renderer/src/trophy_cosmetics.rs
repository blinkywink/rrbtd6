//! Trophy Store items in game: what the enabled ones change (game/trophy_changes.json, from
//! tools/assets/export_trophy_changes.py). The game gathers them in CosmeticHelper; so far bloon
//! skins (BloonAssetChange).
use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// The enabled items' changes, in profile order (the game gathers the profile's enabled item ids;
/// later ones overwrite earlier ones where they change the same thing).
fn enabled_changes(root: &Path) -> (Vec<Value>, HashMap<String, String>) {
    let Some(data) = std::fs::read_to_string(root.join("game/trophy_changes.json")).ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok()) else { return (Vec::new(), HashMap::new()) };
    let names = data["names"].as_object().map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned()))).collect())
        .unwrap_or_default();
    let profile = crate::local_profile::get(root);
    let changes = profile.trophy_items.iter().filter(|(_, flags)| flags.0)
        .filter_map(|(id, _)| data["items"][id.as_str()].as_array().cloned())
        .flatten().collect();
    (changes, names)
}

/// Bloon skins: CosmeticHelper.AddBloonAssetChange (VERIFIED-DECOMP @0x18105a850) keeps one change
/// per BloonAssetChange.bloonId (dictionary insert, overwriting: the last enabled one wins), and that
/// bloon's displays are swapped by its prefabSwaps (original -> swapToo). Here: the bloon's model and
/// damage-state models by prefab name. A skin whose models weren't exported (an install older than
/// the step that exports them) is left out rather than drawing nothing.
pub fn apply_bloon_skins(root: &Path, data: &mut crate::data::GameData) {
    let (changes, names) = enabled_changes(root);
    let exported = |name: &str| root.join("models/bloons").join(name).join("meta.json").is_file();
    for (id, swaps) in bloon_skins(&changes, &names, exported) {
        let Some(bloon) = data.bloons.get_mut(&id) else { continue };
        let swap = |m: &mut String| if let Some(to) = swaps.get(m.as_str()) { *m = to.clone(); };
        if let Some(m) = bloon.model.as_mut() { swap(m); }
        for d in &mut bloon.damage_models { swap(&mut d.model); }
    }
}

/// bloon id -> {original prefab name: skin prefab name}, the last change per bloon winning.
fn bloon_skins(changes: &[Value], names: &HashMap<String, String>, exported: impl Fn(&str) -> bool) -> HashMap<String, HashMap<String, String>> {
    let mut per_bloon = HashMap::new();
    for c in changes.iter().filter(|c| c["type"] == "BloonAssetChange") {
        let change = &c["change"];
        let Some(bloon) = change["bloonId"].as_str() else { continue };
        let swaps: HashMap<String, String> = change["prefabSwaps"].as_array().into_iter().flatten().filter_map(|s| {
            let from = names.get(s["original"]["guidRef"].as_str()?)?;
            let to = names.get(s["swapToo"]["guidRef"].as_str()?)?;
            exported(to).then(|| (from.clone(), to.clone()))
        }).collect();
        per_bloon.insert(bloon.to_owned(), swaps);
    }
    per_bloon
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn last_enabled_skin_per_bloon_wins_and_unexported_models_are_skipped() {
        let names: HashMap<String, String> = [("a", "Bad3D"), ("b", "BadBones"), ("c", "BadPinata"), ("d", "BadDamageState3D1"), ("e", "BadPinataDS1")]
            .into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
        let swap = |o: &str, t: &str| json!({"original": {"guidRef": o}, "swapToo": {"guidRef": t}});
        let changes = vec![
            json!({"type": "BloonAssetChange", "change": {"bloonId": "Bad", "prefabSwaps": [swap("a", "b")]}}),
            json!({"type": "BloonDecal", "change": {}}),
            json!({"type": "BloonAssetChange", "change": {"bloonId": "Bad", "prefabSwaps": [swap("a", "c"), swap("d", "e")]}}),
        ];
        let skins = bloon_skins(&changes, &names, |n| n != "BadPinataDS1");
        assert_eq!(skins.len(), 1);
        assert_eq!(skins["Bad"].get("Bad3D").map(String::as_str), Some("BadPinata"));
        assert!(!skins["Bad"].contains_key("BadDamageState3D1"));
    }
    #[test]
    fn the_exported_sepia_skin_maps_every_moab_class_model() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_extracted/game/trophy_changes.json");
        let Some(data) = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) else { return };
        let names: HashMap<String, String> = serde_json::from_value(data["names"].clone()).unwrap();
        let changes: Vec<Value> = data["items"]["BloonsAllMOABSepia"].as_array().unwrap().clone();
        let skins = bloon_skins(&changes, &names, |_| true);
        assert_eq!(skins["Bad"]["Bad3D"], "BadSepia");
        assert_eq!(skins["Bad"]["BadDamageState3D1"], "BadSepiaDS1");
        assert!(skins.len() >= 12, "{:?}", skins.keys().collect::<Vec<_>>());
    }
}
