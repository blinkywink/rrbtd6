// SimRecorder: a MelonLoader mod that plays one scripted scenario in the real BTD6 and records the
// simulation frame by frame, so the clone can be compared against it (tools/mod/README.md).
//
// Reads <game>/UserData/SimRecorder/scenario.json, starts that map from the main menu, places the
// towers, starts the round and, every Simulation.Simulate, writes the state to
// <game>/UserData/SimRecorder/<name>.rec. Quits the game when done.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using HarmonyLib;
using Il2CppAssets.Scripts;
using Il2CppAssets.Scripts.Simulation;
using Il2CppAssets.Scripts.Simulation.Bloons;
using Il2CppAssets.Scripts.Simulation.Towers;
using Il2CppAssets.Scripts.Simulation.Towers.Projectiles;
using Il2CppAssets.Scripts.Simulation.Towers.Weapons;
using Il2CppAssets.Scripts.Unity;
using Il2CppAssets.Scripts.Unity.UI_New;
using Il2CppAssets.Scripts.Unity.UI_New.InGame;
using MelonLoader;
using UnityEngine;

[assembly: MelonInfo(typeof(SimRecorder.Recorder), "SimRecorder", "1.0.0", "rrbtd6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6-Epic")]

namespace SimRecorder;

public class TowerSpec
{
    public string id { get; set; } = "DartMonkey";
    public float x { get; set; }
    public float y { get; set; }
    /// Targeting mode to set after placing ("First", "Last", "Close", "Strong"); empty keeps the default.
    public string target { get; set; } = "";
    /// Camo priority on (TargetCamo.prioritiseCamo on the tower's attacks).
    public bool camo { get; set; } = false;
    /// Throws with the left hand (the Boomerang's "UseLeftHand" tower-menu action, FlipFollowPath).
    public bool flip { get; set; } = false;
    /// Upgrades bought between recorded frames: after frame `frame`, path `path` (0-2); a path
    /// already at tier 5 with the paragon's towers placed buys the paragon.
    public List<UpgradeSpec> upgrades { get; set; } = new();
}

public class UpgradeSpec
{
    public int frame { get; set; }
    public int path { get; set; }
}

public class Scenario
{
    public string name { get; set; } = "scenario";
    public string map { get; set; } = "Tutorial";
    public string difficulty { get; set; } = "Easy";
    public string mode { get; set; } = "Standard";
    public int round { get; set; } = 1;
    public int frames { get; set; } = 1800;
    public double cash { get; set; } = -1;
    public List<TowerSpec> towers { get; set; } = new();
    public bool quit { get; set; } = true;
    public bool fast { get; set; } = true;
    /// After this many recorded frames every ability is activated (Ability.Activate(true): cooldown
    /// ignored); -1 never.
    public int ability { get; set; } = -1;
}

public class Recorder : MelonMod
{
    internal static Scenario Sc;
    internal static StreamWriter Out;
    internal static int Recorded;
    static int phase, wait;
    static string dir;
    static bool menu, pressed;
    static int shots;

    public override void OnSceneWasInitialized(int buildIndex, string sceneName)
    {
        LoggerInstance.Msg($"scene {buildIndex} {sceneName}");
        if (sceneName == "MainMenuUi") menu = true;
        if (sceneName == "LoadingUI") pressed = true;
        if (sceneName == "DefeatUI" && phase == 2) Done("defeat");
    }

    public override void OnInitializeMelon()
    {
        dir = Path.Combine(MelonLoader.Utils.MelonEnvironment.UserDataDirectory, "SimRecorder");
        Directory.CreateDirectory(dir);
        var path = Path.Combine(dir, "scenario.json");
        if (!File.Exists(path))
        {
            LoggerInstance.Msg("no scenario.json; idle");
            phase = -1;
            return;
        }
        Sc = JsonSerializer.Deserialize<Scenario>(File.ReadAllText(path));
        // Keep simulating while the window is in the background.
        Application.runInBackground = true;
        LoggerInstance.Msg($"scenario {Sc.name}: {Sc.map} {Sc.difficulty} round {Sc.round}, {Sc.towers.Count} towers, {Sc.frames} frames");
    }

    public override void OnUpdate()
    {
        if (phase < 0) return;
        Application.runInBackground = true;
        try
        {
            Step();
        }
        catch (Exception e)
        {
            LoggerInstance.Error("SimRecorder: " + e);
            Done("error: " + e.Message);
        }
    }

    void Step()
    {
        // While waiting, a picture of the game's frame every 20 s (UserData/SimRecorder/wait_N.png).
        if (phase < 2 && Time.frameCount % 1200 == 0 && shots++ < 30)
        {
            // What's on screen: active objects named like popups / panels.
            var names = new List<string>();
            foreach (var t in UnityEngine.Object.FindObjectsOfType<RectTransform>())
            {
                var n = t.gameObject.name;
                if (t.gameObject.activeInHierarchy && (n.Contains("Popup") || n.Contains("Panel") || n.Contains("Screen")) && names.Count < 40)
                    names.Add(n);
            }
            LoggerInstance.Msg("on screen: " + string.Join(", ", names));
        }
        switch (phase)
        {
            case 0: // main menu up? Press Play on the title screen until it is.
                // The mod loader's startup popup (and similar notices) wait for Continue / OK.
                if (!menu && Time.frameCount % 120 == 0)
                {
                    foreach (var btn in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
                    {
                        if (!btn.gameObject.activeInHierarchy || !btn.interactable) continue;
                        var label = btn.GetComponentInChildren<Il2CppTMPro.TMP_Text>();
                        var text = label == null ? "" : label.text.Trim().ToLowerInvariant();
                        if (text == "continue" || text == "ok")
                        {
                            LoggerInstance.Msg($"pressing '{label.text}' ({btn.gameObject.name})");
                            btn.onClick.Invoke();
                        }
                    }
                }
                if (!menu && ++wait % 180 == 0 && !pressed)
                {
                    var title = UnityEngine.Object.FindObjectOfType<Il2CppAssets.Scripts.Unity.Scenes.TitleScreen>();
                    if (title != null)
                    {
                        LoggerInstance.Msg("title screen: Play");
                        title.OnPlayButtonClicked();
                    }
                }
                if (!menu) return;
                if (UI.instance == null || InGame.instance != null || Game.instance == null || Game.instance.model == null) return;
                var mm = Il2CppAssets.Scripts.Unity.Menu.MenuManager.instance;
                if (mm == null || mm.IsTransitioning) return;
                // Main menu announcements (Frontier Legends etc.): their own Close buttons.
                if (Time.frameCount % 60 == 0) ClosePopups();
                if (++wait < 900) return;
                var d = InGameData.Editable;
                d.selectedMap = Sc.map;
                d.selectedDifficulty = Sc.difficulty;
                LoggerInstance.Msg("opening ModeSelectUI for " + Sc.map + " " + Sc.difficulty);
                mm.OpenMenu("ModeSelectUI", null, false);
                phase = 5;
                wait = 0;
                break;
            case 5: // the mode screen: pick the mode as a click does
                var ms = UnityEngine.Object.FindObjectOfType<Il2CppAssets.Scripts.Unity.UI_New.Main.ModeSelect.ModeScreen>();
                if (ms == null || Il2CppAssets.Scripts.Unity.Menu.MenuManager.instance.IsTransitioning) { if (++wait > 60 * 60) Done("no mode screen"); return; }
                if (++wait < 120) return;
                LoggerInstance.Msg("mode " + Sc.mode);
                ms.OnModeSelected(Sc.mode, false);
                phase = 1;
                wait = 0;
                break;
            case 1: // in game and simulating?
                if (++wait % 600 == 0)
                    LoggerInstance.Msg($"waiting for game: InGame={InGame.instance != null} bridge={InGame.instance?.bridge != null} sim={InGame.instance?.bridge?.simulation != null}");
                if (wait > 60 * 180) { Done("timeout waiting for the map"); return; }
                if (InGame.instance == null || InGame.instance.bridge == null || InGame.instance.bridge.simulation == null) return;
                if (wait < 180) wait = 180;
                if (++wait < 360) return;
                var sim = InGame.instance.bridge.simulation;
                if (Sc.cash >= 0) sim.SetCash(Sc.cash, -1);
                // Plenty of lives so a leak doesn't end the recording; fast forward (the
                // simulation steps the same, just more per frame).
                sim.MaxHealth = 1000000f;
                sim.MaxSoftcapHealth = 1000000f;
                sim.SetHealth(1000000f, true);
                if (Sc.fast) Il2CppAssets.Scripts.Simulation.TimeManager.FastForwardActive = true;
                foreach (var t in Sc.towers)
                {
                    // This match's own (mutated) model, as the placement UI uses.
                    var model = sim.model.GetTowerFromId(t.id);
                    var ok = sim.GetInputManager(InGame.instance.bridge.GetInputId()).CreateTowerAt(new Il2CppAssets.Scripts.Simulation.SMath.Vector2(t.x, t.y), model, ObjectId.Invalid, false, true, true, false, -1, false, -1);
                    LoggerInstance.Msg($"placed {t.id} at {t.x},{t.y}: {ok}");
                }
                // Targeting modes, in placement order (the factory's list keeps creation order).
                var placed = sim.factory.GetFactory<Tower>().up.list;
                for (int k = 0; k < Sc.towers.Count && k < placed.Count; k++)
                {
                    var name = Sc.towers[k].target;
                    if (string.IsNullOrEmpty(name)) continue;
                    // The tower model's own TargetType whose id is the mode's name.
                    Il2CppAssets.Scripts.Models.Towers.TargetType tt = null;
                    var tts = placed[k].towerModel.targetTypes;
                    for (int j = 0; j < tts.Length; j++)
                        if (tts[j].id == name) tt = tts[j];
                    if (tt == null) throw new Exception("tower has no target " + name);
                    placed[k].SetTargetType(tt);
                    LoggerInstance.Msg($"tower {k} target {name}");
                }
                // Camo priority: every TargetCamo supplier on the tower's attacks.
                for (int k = 0; k < Sc.towers.Count && k < placed.Count; k++)
                {
                    if (!Sc.towers[k].camo) continue;
                    var abs = placed[k].attackBehaviorsInDependants.list;
                    int n = 0;
                    for (int j = 0; j < abs.Count; j++)
                    {
                        var tc = abs[j]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.Attack.Behaviors.TargetCamo>();
                        if (tc == null) continue;
                        tc.prioritiseCamo = true;
                        n++;
                    }
                    LoggerInstance.Msg($"tower {k} camo priority on {n} suppliers");
                }
                // Left hand: the tower menu's theme action, as the hand button sends it.
                for (int k = 0; k < Sc.towers.Count && k < placed.Count; k++)
                {
                    if (!Sc.towers[k].flip) continue;
                    placed[k].OnTsmThemeAction("UseLeftHand", null);
                    LoggerInstance.Msg($"tower {k} left hand");
                }
                DumpMap(sim);
                Out = new StreamWriter(Path.Combine(dir, Sc.name + ".rec"), false, new UTF8Encoding(false));
                Out.WriteLine($"# SimRecorder {Sc.name} map={Sc.map} difficulty={Sc.difficulty} round={Sc.round}");
                foreach (var t in Sc.towers) Out.WriteLine($"# tower {t.id} {F(t.x)} {F(t.y)}");
                for (int k = 0; k < Sc.towers.Count; k++)
                    if (!string.IsNullOrEmpty(Sc.towers[k].target)) Out.WriteLine($"# target {k} {Sc.towers[k].target}");
                if (Sc.ability >= 0) Out.WriteLine($"# ability {Sc.ability}");
                for (int k = 0; k < Sc.towers.Count; k++)
                {
                    if (Sc.towers[k].camo) Out.WriteLine($"# camo {k}");
                    if (Sc.towers[k].flip) Out.WriteLine($"# flip {k}");
                    foreach (var u in Sc.towers[k].upgrades) Out.WriteLine($"# upgrade {k} {u.path} {u.frame}");
                }
                InGame.instance.bridge.SetRound(Sc.round - 1, false);
                InGame.instance.bridge.StartRound();
                LoggerInstance.Msg($"round set {Sc.round - 1}, start requested; current {sim.GetCurrentRound()}");
                phase = 2;
                break;
            case 2:
                if (Recorded == 60 || Recorded == 600)
                {
                    var sim2 = InGame.instance.bridge.simulation;
                    int nb = 0, np = 0;
                    nb = Count(InGame.instance.bridge.GetAllBloons());
                    np = Count(InGame.instance.bridge.GetAllProjectiles());
                    LoggerInstance.Msg($"frame {Recorded}: round {sim2.GetCurrentRound()} bridge bloons {nb} projectiles {np} lives {sim2.Health}");
                }
                if (Recorded >= Sc.frames) Done("ok");
                break;
        }
    }

    // Il2Cpp IEnumerable<T>: its enumerator's MoveNext lives on the non-generic interface.
    static int Count<T>(Il2CppSystem.Collections.Generic.IEnumerable<T> xs)
    {
        if (xs == null) return -1;
        var e = xs.GetEnumerator().Cast<Il2CppSystem.Collections.IEnumerator>();
        int n = 0;
        while (e.MoveNext()) n++;
        return n;
    }

    // The map as the simulation has it: world bounds, and each path's spawn / leak distance and
    // model points (with their per-point modifiers), to UserData/SimRecorder/<map>.map.
    void DumpMap(Simulation sim)
    {
        using var m = new StreamWriter(Path.Combine(dir, Sc.map + ".map"), false, new UTF8Encoding(false));
        m.WriteLine($"world {F(Il2CppAssets.Scripts.Constants.worldXMin)} {F(Il2CppAssets.Scripts.Constants.worldXMax)} {F(Il2CppAssets.Scripts.Constants.worldZMin)} {F(Il2CppAssets.Scripts.Constants.worldZMax)}");
        m.WriteLine($"cache {F(Il2CppAssets.Scripts.Simulation.Track.Path.CacheDistance)} {F(Il2CppAssets.Scripts.Simulation.Track.Path.MaxSegmentSize)}");
        var paths = sim.map.pathManager.paths;
        for (int i = 0; i < paths.Count; i++)
        {
            var pa = paths[i];
            var pts = pa.def.points;
            m.WriteLine($"path {pa.def.pathId} active={pa.isActive} hidden={pa.isHidden} spawn={F(pa.spawnDist)} leak={F(pa.leakDist)} untilLeak={F(pa.maximumDistanceUntilLeak)} points={pts.Length}");
            for (int k = 0; k < pts.Length; k++)
            {
                var q = pts[k];
                // Last: disablePathTargeting (PathSegment.GatherPathInfoData: a segment whose
                // centre falls on such a point can't be targeted).
                m.WriteLine($"p {F(q.point.x)} {F(q.point.y)} {F(q.point.z)} {F(q.distance)} {F(q.bloonSpeedMultiplier)} {q.bloonsInvulnerable} {q.moabsInvulnerable} {F(q.bloonScale)} {q.disablePathTargeting}");
            }
            // Path.pointInfos: the point every CacheDistance along the path that a distance is
            // looked up in (Path.DistanceToPointInfo).
            var qs = pa.pointInfos;
            var qb = new StringBuilder();
            qb.Append("quick ").Append(qs.Length);
            for (int k = 0; k < qs.Length; k++)
            {
                var q = qs[k];
                qb.Append("\nq ").Append(q.pointIndex).Append(' ').Append(F(q.point.x)).Append(' ').Append(F(q.point.y)).Append(' ').Append(F(q.point.z)).Append(' ').Append(F(q.rotation));
            }
            m.WriteLine(qb.ToString());
            // PathSegment state as the simulation has it: min, max, centre and the targeting flags.
            var segs = pa.segments;
            for (int k = 0; k < segs.Length; k++)
            {
                var sg = segs[k];
                m.WriteLine($"seg {k} {F(sg.min)} {F(sg.max)} {F(sg.centre)} {sg.canTargetSegment} {sg.bloonsInvulnerable} {sg.moabsInvulnerable} {F(sg.point.x)} {F(sg.point.y)} {sim.map.IsPointOnMap(sg.point, 0f)}");
            }
        }
        // Map.areas in order: what blocker rebounds bounce off (their polygons and holes).
        var ars = sim.map.areas;
        for (int k = 0; k < ars.Count; k++)
        {
            var am = ars[k].areaModel;
            m.WriteLine($"area {k} {am.isBlocker} {F(am.height)} {(int)am.type} {am.isDisabled} {ars[k].isActive} {ars[k].name}");
            var pb = new StringBuilder("poly");
            var pts = am.polygon.points;
            for (int j = 0; j < pts.Length; j++) pb.Append(' ').Append(F(pts[j].x)).Append(',').Append(F(pts[j].y));
            m.WriteLine(pb.ToString());
            if (am.holes == null) continue;
            for (int h = 0; h < am.holes.Length; h++)
            {
                var hb = new StringBuilder("hole");
                var hp = am.holes[h].points;
                for (int j = 0; j < hp.Length; j++) hb.Append(' ').Append(F(hp[j].x)).Append(',').Append(F(hp[j].y));
                m.WriteLine(hb.ToString());
            }
        }
        // Map.blockers: what stops projectiles (CollisionChecker's MapBlockerGrid).
        var bl = sim.map.blockers;
        for (int k = 0; k < bl.Count; k++)
        {
            var b = bl[k];
            var bp = b.Position;
            m.WriteLine($"blocker {F(bp.X)} {F(bp.Y)} {F(bp.Z)} {F(b.Radius)} {F(b.VsBlockerRadius)}");
        }
        // The blocker grid's buckets: centre, largest radius and cell span, and what each holds.
        var bg = sim.collisionChecker.MapBlockerGrid;
        m.WriteLine($"blockergrid {bg.bucketsW} {bg.bucketsH}");
        var bb = bg.buckets;
        for (int k = 0; k < bb.Length; k++)
        {
            var bk = bb[k];
            if (bk == null || bk.collidables == null || bk.collidables.Count == 0) continue;
            var sbk = new StringBuilder();
            sbk.Append($"bucket {k} {F(bk.centre.x)} {F(bk.centre.y)} {F(bk.largestRadius)} {bk.largestCellSpan}");
            for (int j = 0; j < bk.collidables.Count; j++)
            {
                var o = bk.collidables[j];
                sbk.Append($" {F(o.Position.X)},{F(o.Position.Y)},{F(o.Radius)}");
            }
            m.WriteLine(sbk.ToString());
        }
        // The bloon collision grid: size, bounds and the neighbour offset order its queries use.
        var grid = sim.collisionChecker.BloonGrid;
        m.WriteLine($"grid {grid.bucketsW} {grid.bucketsH} {F(Il2CppAssets.Scripts.Simulation.Physics.CollisionGrid<Bloon>.collisionMargin)} {F(Il2CppAssets.Scripts.Simulation.Physics.CollisionGrid<Bloon>.collisionXMin)} {F(Il2CppAssets.Scripts.Simulation.Physics.CollisionGrid<Bloon>.collisionYMin)}");
        var offs = grid.orderedNeighborOffsets;
        var sb = new StringBuilder("offsets");
        for (int k = 0; k < grid.orderedNeighborOffsetsLength * 2 && k < offs.Length; k++) sb.Append(' ').Append(offs[k]);
        m.WriteLine(sb.ToString());
        LoggerInstance.Msg("map dumped: " + paths.Count + " paths");
    }

    void ClosePopups()
    {
        foreach (var btn in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
        {
            if (!btn.gameObject.activeInHierarchy || !btn.interactable) continue;
            var n = btn.gameObject.name;
            if (!n.Contains("Close") && !n.Contains("Exit")) continue;
            var inPopup = false;
            for (var t = btn.transform; t != null; t = t.parent)
                if (t.gameObject.name.Contains("Popup")) { inPopup = true; break; }
            if (!inPopup) continue;
            LoggerInstance.Msg("closing popup via " + n);
            btn.onClick.Invoke();
        }
    }

    internal static string F(float v) => v.ToString("R", CultureInfo.InvariantCulture);

    internal void Done(string why)
    {
        Out?.WriteLine("# end " + why);
        Out?.Flush();
        Out?.Dispose();
        Out = null;
        LoggerInstance.Msg("SimRecorder done: " + why);
        phase = -1;
        if (Sc != null && Sc.quit) Application.Quit();
    }
}

[HarmonyPatch(typeof(Simulation), nameof(Simulation.Simulate))]
static class SimulatePatch
{
    static readonly HashSet<string> reported = new();

    // One section of the frame; its first failure is logged with the cause, the rest skipped.
    static void Part(string name, Action a)
    {
        try { a(); }
        catch (Exception e)
        {
            if (reported.Add(name)) MelonLogger.Error($"SimRecorder {name}: {e}");
        }
    }

    static void Postfix(Simulation __instance)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        Recorder.Recorded++;
        // Abilities go off between frames, after the scenario's frame (the clone does the same).
        if (Recorder.Recorded == Recorder.Sc.ability)
            Part("ability", () =>
            {
                var bridge = InGame.instance.bridge;
                var alts = bridge.GetAllAbilities(bridge.GetInputId(), true);
                for (int k = 0; k < alts.Count; k++)
                {
                    alts[k].ability.Activate(true);
                    MelonLogger.Msg($"SimRecorder ability {alts[k].model.name} activated at frame {Recorder.Recorded}");
                }
            });
        // Upgrades go in between frames too, through the same call the upgrade buttons make.
        for (int k = 0; k < Recorder.Sc.towers.Count; k++)
            foreach (var u in Recorder.Sc.towers[k].upgrades)
                if (u.frame == Recorder.Recorded)
                {
                    int kk = k, path = u.path;
                    Part("upgrade", () =>
                    {
                        var bridge = InGame.instance.bridge;
                        var list = __instance.factory.GetFactory<Tower>().up.list;
                        int seen = 0;
                        for (int j = 0; j < list.Count; j++)
                        {
                            var tw = list[j];
                            if (tw == null || tw.IsDestroyed) continue;
                            if (seen++ != kk) continue;
                            var before = tw.towerModel.name;
                            Il2CppSystem.Action<bool> cb = (Il2CppSystem.Action<bool>)(ok => MelonLogger.Msg($"SimRecorder upgrade callback {ok}"));
                            if (path == 3)
                            {
                                // The paragon button: UnityToSimulation.UpgradeTowerParagon.
                                float cost = 0;
                                bool can = __instance.towerManager.CanUpgradeTowerParagon(tw, bridge.GetInputId(), ref cost, 0);
                                MelonLogger.Msg($"SimRecorder paragon can {can} cost {cost}");
                                bridge.UpgradeTowerParagon(bridge.GetInputId(), tw.Id, 0, cb);
                            }
                            else bridge.UpgradeTower(bridge.GetInputId(), tw.Id, path, 0, cb);
                            MelonLogger.Msg($"SimRecorder upgrade tower {kk} path {path} at frame {Recorder.Recorded}: {before} -> {tw.towerModel.name} worth {tw.worth}");
                            break;
                        }
                    });
                }
        var sim = __instance;
        long t = -1, rt = -1; int round = -1; double cash = -1;
        Part("time", () => t = sim.time.elapsed);
        Part("roundTime", () => rt = sim.roundTime.elapsed);
        Part("round", () => round = sim.GetCurrentRound());
        Part("cash", () => cash = sim.GetCash(-1));
        w.WriteLine($"F {t} {rt} {round} {cash}");
        Part("bloons", () =>
        {
            var bs = sim.factory.GetFactory<Bloon>().up.list;
            for (int k = 0; k < bs.Count; k++)
            {
                var b = bs[k];
                if (b == null || b.IsDestroyed) continue;
                var p = b.Position;
                w.WriteLine($"B {b.Id.Id} {b.bloonModel.id} {Recorder.F(b.distanceTraveled)} {Recorder.F(p.X)} {Recorder.F(p.Y)} {b.health}");
            }
        });
        // Each path's bloonsList (Path.Process adds them to its segments in this order).
        Part("paths", () =>
        {
            var paths = sim.map.pathManager.paths;
            for (int i = 0; i < paths.Count; i++)
            {
                var l = paths[i].bloonsList.list;
                var sb = new StringBuilder("L ").Append(i);
                for (int k = 0; k < l.Count; k++) sb.Append(' ').Append(l[k].Id.Id);
                w.WriteLine(sb.ToString());
            }
        });
        Part("weapons", () =>
        {
            var ts = sim.factory.GetFactory<Tower>().up.list;
            for (int j = 0; j < ts.Count; j++)
            {
                var t = ts[j];
                if (t == null || t.IsDestroyed || t.attackBehaviorsInDependants == null) continue;
                int i = 0;
                var l = t.attackBehaviorsInDependants.list;
                for (int k = 0; k < l.Count; k++)
                {
                    var wp = l[k]?.TryCast<Weapon>();
                    if (wp == null) continue;
                    w.WriteLine($"W {t.Id.Id} {i++} {wp.lastEmission} {wp.animationStartedAt} {wp.rateFrames} {Recorder.F(wp.missedRate)} {Recorder.F(t.Rotation)} {Recorder.F(t.Position.X)} {Recorder.F(t.Position.Y)}");
                }
            }
        });
        Part("projectiles", () =>
        {
            var ps = sim.factory.GetFactory<Projectile>().up.list;
            // The factory list as it is, dead entries included (its flush order).
            var pl = new StringBuilder("PL");
            for (int k = 0; k < ps.Count; k++)
            {
                var q = ps[k];
                pl.Append(' ').Append(q == null ? "null" : (q.IsDestroyed ? "x" : "") + q.Id.Id);
            }
            w.WriteLine(pl.ToString());
            for (int k = 0; k < ps.Count; k++)
            {
                var pr = ps[k];
                if (pr == null || pr.IsDestroyed) continue;
                var p = pr.Position;
                var ef = pr.emittedFrom;
                w.WriteLine($"P {pr.Id.Id} {Recorder.F(p.X)} {Recorder.F(p.Y)} {Recorder.F(pr.pierce)} {Recorder.F(pr.EjectRotation)} {Recorder.F(ef.x)} {Recorder.F(ef.y)} {Recorder.F(ef.z)} {Recorder.F(p.Z)} {Recorder.F(pr.VsBlockerRadius)}");
            }
        });
    }
}

// Every projectile-bloon collision as the CollisionChecker makes it: "H projectile bloon health-after
// pierce-after" (written before the frame's F line).
[HarmonyPatch(typeof(Projectile), nameof(Projectile.CollideBloon))]
static class CollideBloonPatch
{
    static void Postfix(Projectile __instance, Bloon __0)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try
        {
            w.WriteLine($"H {__instance.Id.Id} {(__0 == null ? -1 : __0.Id.Id)} {(__0 == null ? 0 : __0.health)} {Recorder.F(__instance.pierce)}");
        }
        catch (Exception) { }
    }
}
