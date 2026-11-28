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

/// The pop effect every bloon shows instead of the Pop flipbook, if an enabled item sets one:
/// CosmeticHelper.AddBloonPopFX (VERIFIED-DECOMP @0x18105ac00) stores an applyGlobal change's id as
/// globalBloonPopFX, the last enabled one winning. Its popFX prefab is exported to effects/<name>
/// (a particle effect); None when none is enabled or it isn't exported yet. Cached on the enabled
/// items, so it's cheap per frame.
pub fn global_pop_fx(root: &Path) -> Option<String> {
    static CACHE: std::sync::Mutex<Option<(Vec<String>, Option<String>)>> = std::sync::Mutex::new(None);
    let enabled = crate::local_profile::enabled_trophy_items(root);
    let mut cache = CACHE.lock().unwrap();
    if let Some((ids, fx)) = cache.as_ref().filter(|(ids, _)| *ids == enabled) {
        let _ = ids;
        return fx.clone();
    }
    let (changes, names) = enabled_changes(root);
    let fx = global_pop(&changes, &names, |n| root.join("effects").join(n).join("effect.json").is_file());
    *cache = Some((enabled, fx.clone()));
    fx
}

fn global_pop(changes: &[Value], names: &HashMap<String, String>, exported: impl Fn(&str) -> bool) -> Option<String> {
    let mut fx = None;
    for c in changes.iter().filter(|c| c["type"] == "BloonPopEffect" && c["change"]["applyGlobal"].as_i64() == Some(1)) {
        if let Some(name) = c["change"]["popFX"]["guidRef"].as_str().and_then(|g| names.get(g)).filter(|n| exported(n)) {
            fx = Some(name.clone());
        }
    }
    fx
}

/// Decals: overlay class ("Red", "RedRegrow", "Moab"...) -> (BloonDecal.layer, decal prefab name).
/// CosmeticHelper.AddBloonDecal (VERIFIED-DECOMP @0x18105a9b0) sets bloonDecals[overlay layer] to
/// the decal's id for each of its bloonOverlayLayers, overwriting: per class the last enabled decal
/// wins. Its prefabs are exported to models/projectiles/<name> like the bloon overlays. Cached on the
/// enabled items.
pub fn bloon_decals(root: &Path) -> std::sync::Arc<HashMap<String, (i32, String)>> {
    type Decals = std::sync::Arc<HashMap<String, (i32, String)>>;
    static CACHE: std::sync::Mutex<Option<(Vec<String>, Decals)>> = std::sync::Mutex::new(None);
    let enabled = crate::local_profile::enabled_trophy_items(root);
    let mut cache = CACHE.lock().unwrap();
    if let Some((_, decals)) = cache.as_ref().filter(|(ids, _)| *ids == enabled) {
        return decals.clone();
    }
    let (changes, names) = enabled_changes(root);
    let decals = std::sync::Arc::new(decals_of(&changes, &names));
    *cache = Some((enabled, decals.clone()));
    decals
}

fn decals_of(changes: &[Value], names: &HashMap<String, String>) -> HashMap<String, (i32, String)> {
    let mut out = HashMap::new();
    for c in changes.iter().filter(|c| c["type"] == "BloonDecal") {
        let layer = c["change"]["layer"].as_i64().unwrap_or(0) as i32;
        for l in c["change"]["bloonOverlayLayers"].as_array().into_iter().flatten() {
            if let (Some(class), Some(name)) = (l["bloonOverlayLayer"].as_str(), l["decal"]["guidRef"].as_str().and_then(|g| names.get(g))) {
                out.insert(class.to_owned(), (layer, name.clone()));
            }
        }
    }
    out
}

/// A tower prop (the Monkey Village flags): its display (models/props/<name>) and
/// Prop.displayOffsets, "top/middle/bottom" tier patterns ("x" any) -> Unity offset.
#[derive(Clone, Debug, PartialEq)]
pub struct TowerProp {
    pub display: String,
    pub offsets: Vec<(String, [f32; 3])>,
}

/// Enabled props by tower id, in profile order (the props group is exclusive, so one per tower
/// in practice). Cached on the enabled items.
pub fn tower_props(root: &Path) -> std::sync::Arc<HashMap<String, Vec<TowerProp>>> {
    type Props = std::sync::Arc<HashMap<String, Vec<TowerProp>>>;
    static CACHE: std::sync::Mutex<Option<(Vec<String>, Props)>> = std::sync::Mutex::new(None);
    let enabled = crate::local_profile::enabled_trophy_items(root);
    let mut cache = CACHE.lock().unwrap();
    if let Some((_, props)) = cache.as_ref().filter(|(ids, _)| *ids == enabled) {
        return props.clone();
    }
    let (changes, names) = enabled_changes(root);
    let props = std::sync::Arc::new(props_of(&changes, &names, |n| root.join("models/props").join(n).join("meta.json").is_file()));
    *cache = Some((enabled, props.clone()));
    props
}

fn props_of(changes: &[Value], names: &HashMap<String, String>, exported: impl Fn(&str) -> bool) -> HashMap<String, Vec<TowerProp>> {
    let mut out: HashMap<String, Vec<TowerProp>> = HashMap::new();
    for c in changes.iter().filter(|c| c["type"] == "Prop") {
        let change = &c["change"];
        let (Some(tower), Some(display)) = (change["towerId"].as_str(), change["display"]["guidRef"].as_str().and_then(|g| names.get(g))) else { continue };
        if !exported(display) {
            continue;
        }
        let keys = change["displayOffsets"]["keys"].as_array().cloned().unwrap_or_default();
        let values = change["displayOffsets"]["values"].as_array().cloned().unwrap_or_default();
        let offsets = keys.iter().zip(&values).filter_map(|(k, v)| {
            Some((k.as_str()?.to_owned(), [v["x"].as_f64()? as f32, v["y"].as_f64()? as f32, v["z"].as_f64()? as f32]))
        }).collect();
        out.entry(tower.to_owned()).or_default().push(TowerProp { display: display.clone(), offsets });
    }
    out
}

/// PropAssetChange.FindOffset (VERIFIED-DECOMP @0x180c25550, IsTierMatch @0x180c25a20, ParseTier
/// @0x180c25fe0): each "a/b/c" pattern matches when every given tier is at most the tower's; its
/// distance adds 10 per "x" (or unparsable) and (tower tier - n) per number. The smallest distance
/// wins, a later entry on a tie. None matching: the default offset (INFERRED: zero).
pub fn prop_offset(offsets: &[(String, [f32; 3])], tiers: [u8; 3]) -> [f32; 3] {
    let mut best = (i32::MAX, [0.0; 3]);
    'entries: for (key, offset) in offsets {
        let parts: Vec<&str> = key.split('/').collect();
        let mut distance = 0;
        for i in 0..3 {
            match parts.get(i).filter(|p| **p != "x").and_then(|p| p.parse::<i32>().ok()) {
                None => distance += 10,
                Some(n) => {
                    let d = tiers[i] as i32 - n;
                    if d < 0 {
                        continue 'entries;
                    }
                    distance += d;
                }
            }
        }
        if distance <= best.0 {
            best = (distance, *offset);
        }
    }
    best.1
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
    fn the_last_enabled_global_pop_effect_wins() {
        let names: HashMap<String, String> = [("p", "BonePop"), ("q", "PinataPop"), ("r", "HeartsPop")]
            .into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
        let pop = |g: &str, global: i64| json!({"type": "BloonPopEffect", "change": {"applyGlobal": global, "popFX": {"guidRef": g}}});
        assert_eq!(global_pop(&[pop("p", 1), pop("q", 0)], &names, |_| true).as_deref(), Some("BonePop"));
        assert_eq!(global_pop(&[pop("p", 1), pop("r", 1)], &names, |_| true).as_deref(), Some("HeartsPop"));
        assert_eq!(global_pop(&[pop("p", 1), pop("r", 1)], &names, |n| n != "HeartsPop").as_deref(), Some("BonePop"));
        assert_eq!(global_pop(&[pop("q", 0)], &names, |_| true), None);
    }
    #[test]
    fn the_last_enabled_decal_per_overlay_class_wins() {
        let names: HashMap<String, String> = [("h", "HatMed"), ("H", "HatMoab"), ("b", "BeardMed")]
            .into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
        let decal = |layers: &[(&str, &str)]| json!({"type": "BloonDecal", "change": {"layer": 0,
            "bloonOverlayLayers": layers.iter().map(|(c, g)| json!({"bloonOverlayLayer": c, "decal": {"guidRef": g}})).collect::<Vec<_>>()}});
        let d = decals_of(&[decal(&[("Red", "h"), ("Moab", "H")]), decal(&[("Red", "b")])], &names);
        assert_eq!(d["Red"].1, "BeardMed");
        assert_eq!(d["Moab"].1, "HatMoab");
        assert_eq!(d.len(), 2);
    }
    #[test]
    fn prop_offsets_pick_the_closest_tier_pattern() {
        // The Village flags' table.
        let o = |k: &str, x: f32| (k.to_owned(), [x, 0.0, 0.0]);
        let table = vec![o("x/x/x", 1.0), o("x/3/x", 2.0), o("3/x/x", 3.0), o("x/x/3", 4.0), o("x/x/5", 5.0), o("x/5/x", 6.0), o("5/x/x", 7.0)];
        assert_eq!(prop_offset(&table, [0, 0, 0])[0], 1.0);
        assert_eq!(prop_offset(&table, [2, 0, 2])[0], 1.0);
        assert_eq!(prop_offset(&table, [3, 0, 2])[0], 3.0);
        assert_eq!(prop_offset(&table, [0, 4, 2])[0], 2.0);
        assert_eq!(prop_offset(&table, [0, 2, 5])[0], 5.0);
        assert_eq!(prop_offset(&table, [5, 2, 0])[0], 7.0);
        // A tie goes to the later entry: x/x/3 (20 + 1) against... nothing else at 2/0/4.
        assert_eq!(prop_offset(&table, [2, 0, 4])[0], 4.0);
        assert_eq!(prop_offset(&[], [5, 0, 0]), [0.0; 3]);
    }
    #[test]
    fn props_keep_their_tower_and_offsets() {
        let names: HashMap<String, String> = [("f".to_owned(), "FlagUK".to_owned())].into_iter().collect();
        let c = json!({"type": "Prop", "change": {"towerId": "MonkeyVillage", "display": {"guidRef": "f"},
            "displayOffsets": {"keys": ["x/x/x"], "values": [{"x": -10.0, "y": 0.0, "z": -3.0}]}}});
        let p = props_of(&[c.clone()], &names, |_| true);
        assert_eq!(p["MonkeyVillage"], vec![TowerProp { display: "FlagUK".into(), offsets: vec![("x/x/x".into(), [-10.0, 0.0, -3.0])] }]);
        assert!(props_of(&[c], &names, |_| false).is_empty());
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
