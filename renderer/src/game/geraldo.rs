//! Geraldo's shop (GeraldoItemModel): items bought with cash while Geraldo is on the map, each
//! with its own stock that refills every few rounds and a hero level that unlocks it. Using an
//! item places a tower, drops a projectile on the track, gives a tower a timed boost, turns a
//! Dart Monkey into a Super Monkey (Worn Hero's Cape) or restores lives.

use super::*;
use crate::data::GeraldoItem;

/// One item's shop state.
#[derive(Clone, Default)]
pub struct ItemStock {
    pub qty: u32,
    /// Rounds since it last refilled.
    rounds: u32,
}

impl Game {
    /// Index of the placed Geraldo, if any.
    pub fn geraldo(&self) -> Option<usize> {
        (0..self.towers.len()).find(|&t| self.towers[t].sub.is_none() && self.tower_defs[self.towers[t].def].0 == "Geraldo")
    }

    /// Geraldo's hero level (0 without him).
    pub fn geraldo_level(&self) -> u32 {
        self.geraldo().map_or(0, |t| self.towers[t].tiers[0] as u32)
    }

    pub fn geraldo_items(&self) -> &[GeraldoItem] {
        &self.geraldo_items
    }

    /// Stock of item `i` (the items start stocked when Geraldo is first placed).
    pub fn item_stock(&self, i: usize) -> u32 {
        self.geraldo_stock.get(i).map_or(self.geraldo_items[i].start, |s| s.qty)
    }

    fn ensure_stock(&mut self) {
        if self.geraldo_stock.is_empty() && self.geraldo().is_some() {
            self.geraldo_stock = self.geraldo_items.iter().map(|it| ItemStock { qty: it.start, rounds: 0 }).collect();
        }
    }

    /// levelUnlockedAt reached and not banned in this mode (bannedForModes, by mode name).
    pub fn item_unlocked(&self, i: usize) -> bool {
        let it = &self.geraldo_items[i];
        self.geraldo_level() >= it.unlock && !it.banned.iter().any(|m| self.mode_name.contains(m.as_str()) || (m == "Deflation" && self.cash_mult == 0.0))
    }

    pub fn item_cost(&self, i: usize) -> f64 {
        game_cost(self.geraldo_items[i].cost, self.cost_mult, 0.0)
    }

    /// Supported here (every kind the export knows).
    pub fn item_supported(&self, i: usize) -> bool {
        self.geraldo_items[i].kind != "self"
    }

    /// Can be bought now: Geraldo placed, unlocked, in stock, affordable, and nothing it's
    /// blocked by (blockPurchaseIfTowerPlaced) on the map.
    pub fn item_ready(&self, i: usize) -> bool {
        let it = &self.geraldo_items[i];
        self.geraldo().is_some()
            && self.item_supported(i)
            && self.item_unlocked(i)
            && self.item_stock(i) > 0
            && self.cash >= self.item_cost(i)
            && (it.block_if_placed.is_empty() || !self.towers.iter().any(|t| t.sub.as_ref().is_some_and(|s| s.spec.name == it.block_if_placed)))
    }

    /// The item is used on the map (a point or a tower) rather than at once.
    pub fn item_needs_target(&self, i: usize) -> bool {
        self.geraldo_items[i].kind != "rejuv"
    }

    /// The entry of `list` for Geraldo's level: the highest level reached.
    fn for_level<T: Clone>(&self, list: &[(u32, T)]) -> Option<T> {
        let lv = self.geraldo_level();
        list.iter().filter(|(l, _)| *l <= lv).max_by_key(|(l, _)| *l).map(|(_, v)| v.clone())
    }

    fn param(&self, i: usize, k: &str) -> f32 {
        self.geraldo_items[i].params.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32
    }

    /// Buys item `i` and uses it at `p` (a spot for towers and track items, the tower there
    /// for boosts). False (nothing spent) when it can't be used there.
    pub fn use_item(&mut self, i: usize, p: [f32; 2]) -> bool {
        if !self.item_ready(i) {
            return false;
        }
        self.ensure_stock();
        let it = self.geraldo_items[i].clone();
        let Some(g) = self.geraldo() else { return false };
        let gid = self.towers[g].id;
        let ok = match it.kind.as_str() {
            "tower" => match self.for_level(&it.towers) {
                // HYPOTHESIS P-ITEMTOWER: placed like a land tower of its footprint, owned by
                // Geraldo (pops credited to him when its model says so).
                Some(spec) if self.land_clear(p, spec.radius) && !self.towers.iter().any(|t| {
                    let r = spec.radius + self.tower_defs[t.def].1.radius;
                    t.sub.is_none() && (t.pos[0] - p[0]).powi(2) + (t.pos[1] - p[1]).powi(2) < r * r
                }) => {
                    self.spawn_sub(spec, p, gid, false);
                    true
                }
                _ => false,
            },
            "track" => match self.for_level(&it.projectiles) {
                // On the track: within 10 units of it.
                Some(proj) if self.track_within(p, 10.0) => {
                    let dir = if it.params.get("random_rotation").and_then(|v| v.as_bool()).unwrap_or(false) {
                        self.rand() * std::f32::consts::TAU
                    } else {
                        0.0
                    };
                    let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
                    let tiers = self.towers[g].tiers;
                    self.emit(p, dir, &single, &proj, None, tiers, gid);
                    true
                }
                _ => false,
            },
            // BottleHotSauceBehavior: a fire-breathing creature orbits the tower for 5 rounds
            // (pops credited to it). HYPOTHESIS P-SAUCE: one sauce per tower at a time.
            "sauce" => match (self.tower_at(p), self.for_level(&it.towers)) {
                (Some(t), Some(spec))
                    if self.towers[t].sub.is_none()
                        && !self.towers.iter().any(|o| o.sub.as_ref().is_some_and(|s| s.parent == self.towers[t].id && s.spec.name == spec.name)) =>
                {
                    let (pos, id) = (self.towers[t].pos, self.towers[t].id);
                    self.spawn_sub(spec, pos, id, false);
                    true
                }
                _ => false,
            },
            // PetRabbitBehavior: a pet rabbit on land or track (its footprint blocks nothing);
            // PetRabbitMergeModel: once there are enough they become a Wererabbit.
            // HYPOTHESIS P-RABBITMERGE: the Wererabbit appears where the last rabbit went.
            "rabbit" => match self.for_level(&it.towers) {
                Some(spec) if self.land_clear(p, 0.0) || self.track_within(p, 10.0) => {
                    let name = spec.name.clone();
                    self.spawn_sub(spec, p, gid, false);
                    if let Some((n, were)) = it.merge.clone() {
                        let rabbits: Vec<usize> = (0..self.towers.len())
                            .filter(|&t| self.towers[t].sub.as_ref().is_some_and(|s| s.parent == gid && s.spec.name == name))
                            .collect();
                        if rabbits.len() as u32 >= n.max(1) {
                            for &t in rabbits.iter().rev() {
                                self.towers.remove(t);
                                self.removed.push(t);
                            }
                            self.spawn_sub(were, p, gid, false);
                        }
                    }
                    true
                }
                _ => false,
            },
            "rejuv" => {
                self.lives += self.param(i, "lives") as f64;
                true
            }
            "cape" => match self.tower_at(p) {
                // GeraldoChangeTowerBehaviorModel: a Dart Monkey no higher than the path maxima
                // becomes a Super Monkey with the same upgrades, capped at resultingPathNTierMax.
                Some(t) if self.tower_defs[self.towers[t].def].0 == it.params.get("targetTowerBaseId").and_then(|v| v.as_str()).unwrap_or("DartMonkey")
                    && (0..3).all(|k| self.towers[t].tiers[k] as f32 <= self.param(i, &format!("path{}TierMax", k + 1))) =>
                {
                    let to = it.params.get("resultingTowerName").and_then(|v| v.as_str()).unwrap_or("SuperMonkey").to_string();
                    match self.tower_defs.iter().position(|(n, _)| *n == to) {
                        Some(def) => {
                            let cap = self.param(i, "resultingPath1TierMax").max(1.0) as u8;
                            let tw = &mut self.towers[t];
                            tw.def = def;
                            tw.tier_cap = cap;
                            let pos = tw.pos;
                            self.place_fx.push((pos, "UpgradeEffect".into(), 1.0));
                            true
                        }
                        None => false,
                    }
                }
                _ => false,
            },
            "pickles" | "stone" | "potion" | "fertilizer" => match self.tower_at(p) {
                Some(t) if self.towers[t].sub.is_none() => {
                    let mut rounds = self.param(i, "rounds").max(1.0);
                    // See Invisibility Potion: lasts roundsUpgradedTo from upgradeV2At.
                    if it.kind == "potion" && self.geraldo_level() as f32 >= self.param(i, "upgradeV2At") && self.param(i, "upgradeV2At") > 0.0 {
                        rounds = self.param(i, "roundsUpgradedTo").max(rounds);
                    }
                    let tw = &mut self.towers[t];
                    tw.items.retain(|(k, _)| *k != i);
                    tw.items.push((i, rounds as u32));
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if ok {
            self.cash -= self.item_cost(i);
            if let Some(s) = self.geraldo_stock.get_mut(i) {
                s.qty = s.qty.saturating_sub(1);
            }
        }
        ok
    }

    /// The boosts of Geraldo items on tower `t`, into its buffs.
    pub(super) fn item_buffs(&self, t: usize, b: &mut Buff) {
        let lv = self.geraldo_level() as f32;
        for &(i, _) in &self.towers[t].items {
            let Some(it) = self.geraldo_items.get(i) else { continue };
            match it.kind.as_str() {
                // JarOfPicklesBehaviorModel: attack interval x attackSpeedScale, +damageIncrease.
                "pickles" => {
                    b.rate_mult *= self.param(i, "attackSpeedScale").max(0.01);
                    b.damage_add += self.param(i, "damageIncrease");
                }
                // SharpeningStoneBehaviorModel: +pierceIncrease (+bonusPierce from bonusPierceAtLevel).
                "stone" => {
                    b.pierce_add += self.param(i, "pierceIncrease");
                    if lv >= self.param(i, "bonusPierceAtLevel") && self.param(i, "bonusPierceAtLevel") > 0.0 {
                        b.pierce_add += self.param(i, "bonusPierce");
                    }
                }
                // SeeInvisibilityPotionBehaviorModel: camo detection (range x upgradedRangeScale
                // from upgradeV2At).
                "potion" => {
                    b.camo = true;
                    if lv >= self.param(i, "upgradeV2At") && self.param(i, "upgradeV2At") > 0.0 {
                        b.range_mult += self.param(i, "upgradedRangeScale") - 1.0;
                    }
                }
                // FertilizerBehaviorModel: income x cashScale.
                "fertilizer" => b.income_mult *= self.param(i, "cashScale").max(0.0),
                _ => {}
            }
        }
    }

    /// Round end: boosts count down; with Geraldo placed, stock refills `amount` every `every`
    /// rounds up to `max` (onlyReplenishIfNonePlaced: only while none of its towers is placed).
    pub(super) fn items_round_end(&mut self) {
        for tw in &mut self.towers {
            tw.items.retain_mut(|(_, r)| {
                *r = r.saturating_sub(1);
                *r > 0
            });
        }
        self.ensure_stock();
        if self.geraldo().is_none() {
            return;
        }
        for i in 0..self.geraldo_stock.len() {
            let it = &self.geraldo_items[i];
            let placed = it.only_if_none
                && self.towers.iter().any(|t| t.sub.as_ref().is_some_and(|s| it.towers.iter().any(|(_, sp)| sp.name == s.spec.name)));
            let (every, amount, max) = (it.every.max(1), it.amount, it.max);
            let s = &mut self.geraldo_stock[i];
            s.rounds += 1;
            if s.rounds >= every {
                s.rounds = 0;
                if !placed {
                    s.qty = (s.qty + amount).min(max);
                }
            }
        }
    }
}
