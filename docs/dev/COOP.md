# Co-op: status and verified rules

Private co-op is deterministic lockstep: the host orders every player's inputs into frames
(`coop/src/match_flow.rs` `Action`), every client applies them with `renderer/src/coop_game.rs`
`Replica::apply_action` (tower actions only on the player's own towers) and acknowledges a state
digest. Money goes through per-player wallets (`game/coop_cash.rs`): during an action
`coop_actor(slot)` makes `game.cash` that player's balance. Protocol version: `coop/src/lib.rs`
`VERSION` (12).

## Synchronised

Place, upgrade, sell, target, point, abilities, start round, speed, auto start, freeplay, cash
pickup, donations / cash requests, the tower selection menu's settings (camo priority, submerge,
reverse, hand, second target, default point, stance, bank collect, Collect All), map obstacles,
heroes, Paragons, Sun Temple upgrades, Geraldo's items, Powers and mod widget buttons.

## Verified rules

- **Heroes.** `Simulation.StockStandardTowerInventory(inventory, playerHero)` (@0x180b500d0):
  each player's own tower inventory holds only their lobby hero, one of it. Two players may pick
  the same hero. Clone: `Replica::heroes` (from the lobby profiles), `Action::Place` checks it;
  `Game::place_at`'s one-hero-per-game rule is off in co-op. Each player's shop shows their own hero.
- **Paragons.**
  - `TowerManager.CanUpgradeTowerParagon(tower, inputId, ...)` (@0x180c26fb0): only the tower's
    owner can buy it, with their own cash (`Simulation.GetCash(inputId)`).
  - `Tower.CanUpgradeToParagon` (@0x180c0b7c0): the "a tier 5 on every path" requirement takes
    the highest tier per path over every tower with the same `towerModel.baseId`, whoever owns
    it, so teammates' tier 5s count; an existing Paragon of that base makes it false.
  - `ParagonTower.StartSacrifice` (@0x180d54a50): every tower of the base ID is sacrificed,
    whoever owns it.
  - `TowerManager.IsParagonLocked` (@0x180c2c8e0) is a challenge restriction on the owner's
    inventory: nothing to do (no challenges).
  - Clone: the confirmation popup sends `Action::Paragon{tower, extra}`.
- **Sun Temple.** `MonkeyTemple.StartSacrifice` (@0x180d43510): the towers in range come from
  `TowerManager.GetTowersInRange` with no owner check; each is destroyed for its own owner
  (`Simulation.DestroyTower(tower, tower.owner)`). The temple popup then sends `Action::Upgrade`.
- **Geraldo.** `GeraldoPurchaseManager.GetGeraldoTower(inputId)` and
  `Simulation.GetGeraldoShopInventory(index)`: each player has their own Geraldo and stock.
  Clone: `game/geraldo.rs` (`geraldo_of`, `geraldo_stock` per slot; the HUD shows the local
  player's via `coop_view`). `GeraldoItem.CheckLocation` (@0x180fa4030): "blocked while placed"
  counts only the buyer's own towers. Tower items may go on anyone's tower (no owner check in
  `JarOfPicklesBehavior.IsTowerValid` @0x180fa4da0, `GeraldoChangeTowerBehavior.IsPositionValid`
  @0x180fa30c0, `BottleHotSauceBehavior.CheckLocation` @0x180f92d00); their level is the buyer's
  Geraldo's (`Game::item_buyers`).
- **Powers.** `Action::Power{id, at}`; the power's behaviours come from game/powers.json on every
  client; the user's own profile pays once the match used it (`Replica::powers_used`). A Cash Drop
  crate pays the player who used it (`Game::power_owner`).
- **Modes and divisions.** Every Standard-match mode works. The divisions' areas are the same
  for every map (`tools/assets/export_coop_areas.py` -> game/coop_areas.json, from
  `Game.coopLayouts`; `game/coop_areas.rs`):
  - `Map.SetNumberPlayers` (@0x180f5f510): two-player layout for 1-2 players, four-player for 3-4.
  - `Simulation.DistributePlacementAreaInventory` (@0x180b45400): area i is player i+1's when
    they are in the match, else open to anyone; Free For All opens every area.
  - `Map.CheckCoopPlacement` (@0x180f52930): a tower must stand in one of the player's areas
    (towers with `ignoreCoopAreas`: only AmbushBot, SheRa, Skeletor and some sub towers).
- **A player leaving.** The match goes on: the host sends `Action::Leave` in their sequence and
  stops waiting for their acknowledgements (`Flow::drop_player`). `Simulation.RemovePlayer`
  (@0x180b4c1e0): their cash is split evenly over the players left, their debt goes.
  `RedistributeTowers` (@0x180b4a5a0): their towers, most `worth` first (OrderByDescending),
  are dealt snake-fashion to the remaining players in input order; sub towers and projectiles
  follow; `destroyTowerOnRedistribution` towers (only RareQuincyActionFigure) are destroyed.
  `RedistributePlayerInventory`: whoever gets their Geraldo combines his stock
  (`GeraldoShopInventory.CombineStock`, capped at each item's max). `RedistributePlacementAreaInventory`:
  their areas open to everyone. The host leaving ends the room; nobody can rejoin a running
  match (no state transfer).

## Mods in co-op

Create Match (Mods) picks the room's mods before the room opens (`mod_browser` pick mode,
`coop_session.queued_mods`); joiners get `coop_mods_popup` (Download keeps a disabled copy in a
folder marked `ROOM`, Leave leaves; a joiner can't ready up until every file is there).

- Data and event scripts travel in the manifest. Scripts run in the lockstep simulation (rhai
  built with `no_time`); widget buttons are `Action::ModEvent`.
- Model art: the host lists each mod's `assets/<asset>/<file>` by hash (`ModPackage.assets`,
  128 files / 32 MiB per room); joiners fetch it in 40 KiB chunks (`Wire::AssetRequest` /
  `AssetChunk`, paced under the host's 64-messages-a-second limit) and check the hash. A mod
  with art must be in a folder named after its ID.

## Not yet

- Rejoining a running match (needs a full state transfer).
- Co-op areas for Geraldo's item towers (placed with `spawn_sub`, not `place_at`).
- Custom maps in rooms.
- The player's area overlay while placing (`ShowOverlays(inputId, highlightPlayerArea)`); the
  placement ring already turns red outside the player's areas.
