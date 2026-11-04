//! Co-op map divisions: where each player may place towers.
//!
//! The game's divisions are the same for every map (game/coop_areas.json, from
//! tools/assets/export_coop_areas.py: Game.coopLayouts). Map.SetNumberPlayers (@0x180f5f510) uses
//! the PLAYERS_2 layout for one or two players and PLAYERS_4 for three or four (a FREE_FOR_ALL
//! layout fits any). Simulation.DistributePlacementAreaInventory (@0x180b45400) gives area i
//! (CoopArea.player, P1 = 0) to input i + 1 (our slot i) when that player is in the match, else
//! to -1; FREE_FOR_ALL gives every area -1. GetValidPlacementAreasForPlayer (@0x180b485b0): a
//! player's areas are their own and the -1 ones. Map.CheckCoopPlacement (@0x180f52930): with a
//! FREE_FOR_ALL layout, or a tower model with ignoreCoopAreas, anywhere; otherwise the point must
//! be inside one of the player's areas (CoopAreaModel.IsPointInside: in the first polygon and
//! none of the others).

use super::*;

/// One area: its index (CoopArea.player), its polygons (outer first, then holes) and owner
/// (None: anyone, the game's -1).
#[derive(Clone, Debug)]
pub struct CoopArea {
    pub index: u8,
    pub polygons: Vec<Vec<[f32; 2]>>,
    pub owner: Option<u8>,
}

/// The selected layout (none placed: Free For All).
#[derive(Clone, Debug, Default)]
pub struct CoopAreas {
    pub areas: Vec<CoopArea>,
}

fn inside(poly: &[[f32; 2]], p: [f32; 2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            c = !c;
        }
    }
    c
}

impl CoopAreas {
    /// `division` (game/coop_areas.json's key, e.g. "Horizontal") for the match's player slots.
    pub fn load(json: &serde_json::Value, division: &str, slots: &[u8]) -> Result<Self, String> {
        let d = json.get(division).ok_or_else(|| format!("No co-op division {division}"))?;
        let want = if slots.len() <= 2 { 0 } else { 1 };
        let layouts = d["layouts"].as_array().ok_or("Bad co-op division")?;
        let layout = layouts.iter().find(|l| l["layout"] == 2 || l["layout"] == want).ok_or("No co-op layout for this many players")?;
        if layout["layout"] == 2 {
            return Ok(Self::default());
        }
        let mut areas = Vec::new();
        for p in layout["players"].as_array().into_iter().flatten() {
            let index = p["player"].as_u64().ok_or("Bad co-op area")? as u8;
            let polygons: Vec<Vec<[f32; 2]>> = p["polygons"].as_array().into_iter().flatten()
                .map(|r| r.as_array().into_iter().flatten().filter_map(|q| Some([q[0].as_f64()? as f32, q[1].as_f64()? as f32])).collect())
                .collect();
            if polygons.is_empty() {
                continue;
            }
            let owner = slots.contains(&index).then_some(index);
            areas.push(CoopArea { index, polygons, owner });
        }
        Ok(Self { areas })
    }

    /// Map.CheckCoopPlacement for `player` at `p` (no areas: Free For All).
    pub fn allows(&self, player: u8, p: [f32; 2]) -> bool {
        self.areas.is_empty()
            || self.areas.iter().any(|a| {
                a.owner.is_none_or(|o| o == player)
                    && a.polygons.first().is_some_and(|outer| inside(outer, p))
                    && !a.polygons[1..].iter().any(|hole| inside(hole, p))
            })
    }
}

impl Game {
    /// Co-op: whether `player` may place a tower at `p` (the division's areas).
    pub(crate) fn coop_area_allows(&self, player: u8, p: [f32; 2]) -> bool {
        self.coop_areas.as_ref().is_none_or(|a| a.allows(player, p))
    }

    /// What the placement preview shows for the local player: Game::can_place and, in co-op,
    /// their areas.
    pub fn can_place_here(&self, def: usize, p: [f32; 2]) -> bool {
        self.can_place(def, p) && self.coop_view.is_none_or(|me| self.coop_wallets.is_none() || self.coop_area_allows(me, p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json() -> serde_json::Value {
        serde_json::json!({
            "Horizontal": {"layouts": [
                {"layout": 0, "players": [
                    {"player": 0, "polygons": [[[-10, 0], [10, 0], [10, 10], [-10, 10]]]},
                    {"player": 1, "polygons": [[[-10, -10], [10, -10], [10, 0], [-10, 0]]]}]},
                {"layout": 1, "players": [
                    {"player": 0, "polygons": [[[-10, 0], [0, 0], [0, 10], [-10, 10]]]},
                    {"player": 1, "polygons": [[[-10, -10], [0, -10], [0, 0], [-10, 0]]]},
                    {"player": 2, "polygons": [[[0, 0], [10, 0], [10, 10], [0, 10]]]},
                    {"player": 3, "polygons": [[[0, -10], [10, -10], [10, 0], [0, 0]]]}]}]},
            "RadioActive": {"layouts": [
                {"layout": 0, "players": [
                    {"player": 0, "polygons": [[[-2, -2], [2, -2], [2, 2], [-2, 2]]]},
                    {"player": 1, "polygons": [[[-10, -10], [10, -10], [10, 10], [-10, 10]], [[-2, -2], [2, -2], [2, 2], [-2, 2]]]}]}]},
            "FreeForAll": {"layouts": [{"layout": 2, "players": [{"player": 0, "polygons": [[[-10, -10], [10, -10], [10, 10], [-10, 10]]]}]}]}
        })
    }

    #[test]
    fn players_place_in_their_own_areas_and_missing_players_areas_are_open() {
        let j = json();
        let two = CoopAreas::load(&j, "Horizontal", &[0, 1]).unwrap();
        assert!(two.allows(0, [0.0, 5.0]) && !two.allows(1, [0.0, 5.0]) && two.allows(1, [0.0, -5.0]));
        assert!(!two.allows(0, [50.0, 50.0]), "outside every area");
        // Three players: the 4-player layout, P4's area open to everyone.
        let three = CoopAreas::load(&j, "Horizontal", &[0, 1, 2]).unwrap();
        assert!(three.allows(2, [5.0, 5.0]) && !three.allows(0, [5.0, 5.0]));
        assert!(three.allows(0, [5.0, -5.0]) && three.allows(1, [5.0, -5.0]));
        // A hole: the centre belongs to P1 only.
        let radio = CoopAreas::load(&j, "RadioActive", &[0, 1]).unwrap();
        assert!(radio.allows(0, [0.0, 0.0]) && !radio.allows(1, [0.0, 0.0]) && radio.allows(1, [5.0, 5.0]));
        assert!(CoopAreas::load(&j, "FreeForAll", &[0, 1, 2, 3]).unwrap().allows(3, [500.0, 0.0]));
    }
}
