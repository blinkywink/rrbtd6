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
using System.Linq;
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
    /// Target point for TargetSelectedPoint attacks (Mortar, Dartling, Skywarden...): [x, y];
    /// empty keeps the default. Set again after each upgrade.
    public List<float> point { get; set; } = new();
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
    /// A tower the placement UI refuses (CHIMPS: Banana Farm) is made by TowerManager.CreateTower.
    public bool forcePlace { get; set; } = true;
    /// After this many recorded frames every ability is activated (Ability.Activate(true): cooldown
    /// ignored); -1 never.
    public int ability { get; set; } = -1;
    /// Recorded frames [first, last] to log the bloon grid's buckets for (GB lines).
    public int[] gridlog { get; set; }
    /// Effect capture (<name>.fx): every rendered frame while fx_from <= recorded frames <= fx_to,
    /// the camera and each active particle system (with its particles), sprite and TextMeshPro
    /// text whose hierarchy path contains one of fx_filter (all when empty). Use with fast false.
    public bool fx { get; set; } = false;
    public int fx_from { get; set; } = 0;
    public int fx_to { get; set; } = 600;
    public List<string> fx_filter { get; set; } = new();
    /// Screenshots (<name>_<frame>.png) at these recorded frames.
    public List<int> shots { get; set; } = new();
    /// Screenshot resolution multiple (ScreenCapture superSize): close detail without a camera zoom.
    public int shot_scale { get; set; } = 1;
    /// At recorded frame paragon_investment_frame, every Paragon's investmentInfo.totalInvestment
    /// becomes this and ParagonTower.UpdateDegree runs (its degree and display level); negative never.
    public double paragon_investment { get; set; } = -1;
    public int paragon_investment_frame { get; set; } = -1;
    /// Also each particle (ParticleSystem.GetParticles; off by default: suspected of crashing the
    /// game when the interop's Particle layout differs from the engine's).
    public bool fx_particles { get; set; } = false;
    /// "victory" / "defeat": at recorded frame end_frame open that end screen as InGame does
    /// (OnVictory: ShowVictoryScreen(GetVictorySceneName, GetVictoryScreenData); Lose:
    /// MenuManager.QueueMenu(GetDefeatSceneName, GetDefeatSceneData)) without their rewards,
    /// stats or saves, then screenshot it (<name>_end_<ms>.png) at end_shots seconds after.
    public string end { get; set; } = "";
    public int end_frame { get; set; } = 60;
    public List<double> end_shots { get; set; } = new() { 0.25, 0.5, 1, 2, 4 };
    /// Seconds after opening the end screen to press its Next button (SummaryScreen.OnNextClicked);
    /// negative never. Shots are timed from the open either way.
    public double end_next { get; set; } = -1;
    /// Defeat: the cause-of-defeat entries to show ([bloon id, damage], ...), in display order
    /// (BloonCauseOfDefeat.FinalizeDeath's: ascending damage, at most three).
    public List<List<string>> end_leaks { get; set; } = new();
    /// A menu to open from the main menu instead of playing (MenuManager.OpenMenu(menu, null)),
    /// screenshot (<name>_menu_<ms>.png) at menu_shots seconds after; no game is started.
    public string menu { get; set; } = "";
    public List<double> menu_shots { get; set; } = new() { 1, 2, 4 };
    /// Objects to press one second after the menu opens, by GameObject name (its Button's
    /// onClick, else its Toggle switched on), in order, half a second apart.
    public List<string> menu_clicks { get; set; } = new();
}

public class Recorder : MelonMod
{
    /// TargetSelectedPoint(OrDefault).SetTargetPosition on every point supplier of the tower.
    public static void SetPoint(Tower tw, TowerSpec spec, int k)
    {
        if (spec.point.Count != 2) return;
        var pos = new Il2CppAssets.Scripts.Simulation.SMath.Vector3(spec.point[0], spec.point[1], 0);
        var abs = tw.attackBehaviorsInDependants.list;
        int n = 0;
        for (int j = 0; j < abs.Count; j++)
        {
            var a = abs[j]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.Attack.Behaviors.TargetSelectedPoint>();
            if (a != null) { a.SetTargetPosition(pos, true); n++; continue; }
            var b = abs[j]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.Attack.Behaviors.TargetSelectedPointOrDefault>();
            if (b != null) { b.SetTargetPosition(pos, true); n++; }
        }
        MelonLogger.Msg($"tower {k} point {spec.point[0]},{spec.point[1]} on {n} suppliers");
    }

    internal static Scenario Sc;
    internal static StreamWriter Out;
    internal static StreamWriter FxOut;
    static readonly HashSet<int> shotTaken = new();
    internal static readonly HashSet<string> weaponNamed = new();
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
        if (sceneName == "DefeatUI" && phase == 2 && string.IsNullOrEmpty(Sc.end)) Done("defeat");
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
                if (!string.IsNullOrEmpty(Sc.menu))
                {
                    Out = new StreamWriter(Path.Combine(dir, Sc.name + ".rec"), false, new UTF8Encoding(false));
                    Out.WriteLine("# SimRecorder " + Sc.name + " menu=" + Sc.menu);
                    LoggerInstance.Msg("opening " + Sc.menu);
                    mm.OpenMenu(Sc.menu, null, false);
                    phase = 30;
                    MelonCoroutines.Start(MenuShots());
                    return;
                }
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
                    // CHIMPS refuses some towers in the UI's placement (Banana Farm: no income);
                    // the simulation's own TowerManager.CreateTower makes it anyway, as the clone's
                    // harness does.
                    if (!ok && Sc.forcePlace)
                    {
                        var made = sim.towerManager.CreateTower(model, new Il2CppAssets.Scripts.Simulation.SMath.Vector3(t.x, t.y, 0f), InGame.instance.bridge.GetInputId(), ObjectId.Invalid, ObjectId.Invalid, null, false, false, 0f, false, -1, -1);
                        LoggerInstance.Msg($"forced {t.id} at {t.x},{t.y}: {made != null}");
                    }
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
                for (int k = 0; k < Sc.towers.Count && k < placed.Count; k++)
                    Recorder.SetPoint(placed[k], Sc.towers[k], k);
                DumpMap(sim);
                Out = new StreamWriter(Path.Combine(dir, Sc.name + ".rec"), false, new UTF8Encoding(false));
                Out.WriteLine($"# SimRecorder {Sc.name} map={Sc.map} difficulty={Sc.difficulty} round={Sc.round} mode={Sc.mode}");
                // GameModel.randomSeed seeds Simulation.syncedRandom / unsyncedRandom
                // (Simulation.Initialise: two BufferedRandom(seed)).
                Out.WriteLine($"# seed {sim.model.randomSeed}");
                // Simulation.DistributeXp's inputs: the map difficulty's XP scale and the rating.
                var xs = Simulation.trackDifficultyXpScale;
                var xsl = new List<string>();
                for (int k = 0; xs != null && k < xs.Length; k++) xsl.Add(F(xs[k]));
                Out.WriteLine($"# xpscale {string.Join(",", xsl)} map {sim.model.map.mapDifficulty} rating {F(sim.model.selectedDifficultyRating)} endRound {sim.map.spawner.endRound}");
                foreach (var t in Sc.towers) Out.WriteLine($"# tower {t.id} {F(t.x)} {F(t.y)}");
                for (int k = 0; k < Sc.towers.Count; k++)
                    if (!string.IsNullOrEmpty(Sc.towers[k].target)) Out.WriteLine($"# target {k} {Sc.towers[k].target}");
                if (Sc.ability >= 0) Out.WriteLine($"# ability {Sc.ability}");
                for (int k = 0; k < Sc.towers.Count; k++)
                {
                    if (Sc.towers[k].camo) Out.WriteLine($"# camo {k}");
                    if (Sc.towers[k].flip) Out.WriteLine($"# flip {k}");
                    if (Sc.towers[k].point.Count == 2) Out.WriteLine($"# point {k} {F(Sc.towers[k].point[0])} {F(Sc.towers[k].point[1])}");
                    foreach (var u in Sc.towers[k].upgrades) Out.WriteLine($"# upgrade {k} {u.path} {u.frame}");
                }
                if (Sc.fx || Sc.shots.Count > 0)
                {
                    FxOut = new StreamWriter(Path.Combine(dir, Sc.name + ".fx"), false, new UTF8Encoding(false));
                    FxOut.WriteLine($"# SimRecorder effects {Sc.name}: U unityFrame recordedFrame time screenW screenH; C camera name depth cullingMask pos rot fov ortho orthoSize aspect; S path pos rot lossyScale count playing simSpace; q quad corners(4 x xyz) u0 v0 u1 v1 rgba (BakeMesh, world); m mesh-mode vertexCount triCount boundsMin boundsMax; R sprite path pos rot lossyScale sprite size color order; T text path pos rot lossyScale fontSize color text");
                }
                // Monkey Knowledge this game plays with (InGame.ActiveKnowledge's bit masks; all
                // zero when it's switched off): the clone has none, so recordings must have none.
                {
                    var ak = InGame.instance.ActiveKnowledge;
                    var bits = new List<string>();
                    if (ak != null && ak.masks != null)
                        for (int i = 0; i < ak.masks.Length; i++) bits.Add(ak.masks[i].ToString("x"));
                    LoggerInstance.Msg("knowledge masks: " + (ak == null ? "none" : string.Join(",", bits)));
                    Out.WriteLine("# knowledge " + (ak == null ? "none" : string.Join(",", bits)));
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
                if (!string.IsNullOrEmpty(Sc.end))
                {
                    if (Recorded >= Sc.end_frame && !ending)
                    {
                        ending = true;
                        MelonCoroutines.Start(EndScreen());
                    }
                    break;
                }
                if (Recorded >= Sc.frames) Done("ok");
                break;
        }
    }

    public override void OnLateUpdate()
    {
        if (phase != 2 || Sc == null || FxOut == null) return;
        try
        {
            RecordFx();
        }
        catch (Exception e)
        {
            LoggerInstance.Error("SimRecorder fx: " + e);
            FxOut.WriteLine("# fx error " + e.Message);
            FxOut.Flush();
            FxOut.Dispose();
            FxOut = null;
        }
    }

    // A transform's names from its root ("Root/Child/..."), spaces as '_' (one token per field).
    static string PathOf(Transform t)
    {
        var names = new List<string>();
        for (; t != null; t = t.parent) names.Add(t.gameObject.name.Replace(' ', '_'));
        names.Reverse();
        return string.Join("/", names);
    }

    static bool Wanted(string path)
    {
        if (Sc.fx_filter.Count == 0) return true;
        foreach (var f in Sc.fx_filter)
            if (path.Contains(f)) return true;
        return false;
    }

    static string V(UnityEngine.Vector3 v) => $"{F(v.x)} {F(v.y)} {F(v.z)}";
    static string Q(Quaternion q) => $"{F(q.x)} {F(q.y)} {F(q.z)} {F(q.w)}";

    // What the effects look like this rendered frame (Scenario.fx), and the screenshots.
    void RecordFx()
    {
        int f = Recorded;
        if (Sc.shots.Contains(f) && shotTaken.Add(f))
            MelonCoroutines.Start(Shot(Path.Combine(dir, Sc.name + "_" + f + ".png")));
        if (!Sc.fx || f < Sc.fx_from || f > Sc.fx_to) return;
        var sb = new StringBuilder();
        sb.Append("U ").Append(Time.frameCount).Append(' ').Append(f).Append(' ').Append(F(Time.time)).Append(' ')
          .Append(Screen.width).Append(' ').Append(Screen.height).Append('\n');
        Stage("camera", () => FxCamera(sb));
        foreach (var ps in UnityEngine.Object.FindObjectsOfType<ParticleSystem>())
        {
            if (ps == null || !ps.gameObject.activeInHierarchy) continue;
            var path = PathOf(ps.transform);
            if (!Wanted(path)) continue;
            Stage("system", () => FxSystem(sb, ps, path));
            if (Sc.fx_particles) Stage("particles", () => FxParticles(sb, ps));
        }
        Stage("sprites", () => FxSprites(sb));
        Stage("texts", () => FxTexts(sb));
        FxOut.Write(sb.ToString());
    }

    // Each capture step in its own method: an interop member the runtime lacks fails when its
    // method is compiled, so only that step is lost (logged once) and the rest still records.
    static readonly HashSet<string> stageFailed = new();

    static readonly HashSet<string> stageStarted = new();

    void Stage(string name, Action a)
    {
        if (stageFailed.Contains(name)) return;
        // (Logged before a step first runs: a crash inside the game leaves its name last.)
        if (stageStarted.Add(name)) LoggerInstance.Msg("SimRecorder fx stage " + name + " first run");
        try
        {
            a();
        }
        catch (Exception e)
        {
            stageFailed.Add(name);
            LoggerInstance.Error("SimRecorder fx stage " + name + ": " + e);
            FxOut?.WriteLine("# fx stage " + name + " failed: " + e.GetType().Name + " " + e.Message);
        }
    }

    // Every enabled camera ("C name depth cullingMask pos rot fov ortho orthoSize aspect"):
    // BTD6 tags none MainCamera (Camera.main is null).
    static void FxCamera(StringBuilder sb)
    {
        foreach (var cam in Camera.allCameras)
        {
            if (cam == null || !cam.enabled) continue;
            var ct = cam.transform;
            sb.Append("C ").Append(cam.gameObject.name.Replace(' ', '_')).Append(' ').Append(F(cam.depth)).Append(' ').Append(cam.cullingMask).Append(' ')
              .Append(V(ct.position)).Append(' ').Append(Q(ct.rotation)).Append(' ').Append(F(cam.fieldOfView)).Append(' ')
              .Append(cam.orthographic).Append(' ').Append(F(cam.orthographicSize)).Append(' ').Append(F(cam.aspect)).Append('\n');
        }
    }

    // The camera that draws layer `layer` (its culling mask has it), the deepest of those.
    static Camera CameraFor(int layer)
    {
        Camera best = null;
        foreach (var cam in Camera.allCameras)
            if (cam != null && cam.enabled && (cam.cullingMask & (1 << layer)) != 0 && (best == null || cam.depth > best.depth))
                best = cam;
        return best;
    }

    static void FxSystem(StringBuilder sb, ParticleSystem ps, string path)
    {
        var t = ps.transform;
        sb.Append("S ").Append(path).Append(' ').Append(V(t.position)).Append(' ').Append(Q(t.rotation)).Append(' ').Append(V(t.lossyScale))
          .Append(' ').Append(ps.particleCount).Append(' ').Append(ps.isPlaying).Append(' ').Append((int)ps.main.simulationSpace).Append('\n');
    }

    static bool probe = true;
    static Mesh baked;

    // The particles as Unity draws them: ParticleSystemRenderer.BakeMesh (world space, this camera)
    // gives the rendered geometry, so every quad's corners, UVs and vertex colour are what's on
    // screen (GetParticles froze the game through the interop). "q" lines: 4 corners (x y z),
    // u0 v0 u1 v1, r g b a; a mesh-mode system writes "m" with its vertex and triangle counts and
    // its vertices' bounds instead.
    static void FxParticles(StringBuilder sb, ParticleSystem ps)
    {
        if (ps.particleCount <= 0) return;
        var pr = ps.GetComponent<ParticleSystemRenderer>();
        if (pr == null || !pr.enabled) return;
        bool first = probe;
        probe = false;
        if (baked == null) baked = new Mesh();
        if (first) MelonLogger.Msg("SimRecorder particles: BakeMesh");
        var cam = CameraFor(ps.gameObject.layer);
        if (cam == null) return;
        pr.BakeMesh(baked, cam, true);
        if (first) MelonLogger.Msg("SimRecorder particles: baked " + baked.vertexCount + " vertices");
        int vc = baked.vertexCount;
        if (vc == 0) return;
        var v = baked.vertices;
        var uv = baked.uv;
        var col = baked.colors32;
        if (pr.renderMode == ParticleSystemRenderMode.Mesh)
        {
            var lo = v[0];
            var hi = v[0];
            for (int i = 1; i < vc; i++)
            {
                lo = UnityEngine.Vector3.Min(lo, v[i]);
                hi = UnityEngine.Vector3.Max(hi, v[i]);
            }
            sb.Append("m ").Append(vc).Append(' ').Append(baked.triangles.Length / 3).Append(' ').Append(V(lo)).Append(' ').Append(V(hi)).Append('\n');
            // Small systems also each vertex ("mv x y z u v"), in the mesh's order: a Quad particle's
            // four corners give its full orientation (3D start rotation, alignment).
            if (vc <= 64)
                for (int i = 0; i < vc; i++)
                    sb.Append("mv ").Append(V(v[i])).Append(' ').Append(uv != null && i < uv.Length ? F(uv[i].x) + " " + F(uv[i].y) : "0 0").Append('\n');
            return;
        }
        for (int q = 0; q + 3 < vc; q += 4)
        {
            float u0 = float.MaxValue, v0 = float.MaxValue, u1 = float.MinValue, v1 = float.MinValue;
            for (int k = 0; k < 4 && uv != null && q + k < uv.Length; k++)
            {
                var t = uv[q + k];
                u0 = Math.Min(u0, t.x); v0 = Math.Min(v0, t.y); u1 = Math.Max(u1, t.x); v1 = Math.Max(v1, t.y);
            }
            var c = col != null && q < col.Length ? col[q] : new Color32(255, 255, 255, 255);
            sb.Append("q ").Append(V(v[q])).Append(' ').Append(V(v[q + 1])).Append(' ').Append(V(v[q + 2])).Append(' ').Append(V(v[q + 3])).Append(' ')
              .Append(F(u0)).Append(' ').Append(F(v0)).Append(' ').Append(F(u1)).Append(' ').Append(F(v1)).Append(' ')
              .Append(c.r).Append(' ').Append(c.g).Append(' ').Append(c.b).Append(' ').Append(c.a).Append('\n');
        }
    }

    static void FxSprites(StringBuilder sb)
    {
        foreach (var sr in UnityEngine.Object.FindObjectsOfType<SpriteRenderer>())
        {
            if (sr == null || !sr.enabled || !sr.gameObject.activeInHierarchy || sr.sprite == null) continue;
            var path = PathOf(sr.transform);
            if (!Wanted(path)) continue;
            var t = sr.transform;
            var sz = sr.sprite.bounds.size;
            var c = sr.color;
            sb.Append("R ").Append(path).Append(' ').Append(V(t.position)).Append(' ').Append(Q(t.rotation)).Append(' ').Append(V(t.lossyScale)).Append(' ')
              .Append(sr.sprite.name.Replace(' ', '_')).Append(' ').Append(F(sz.x)).Append(' ').Append(F(sz.y)).Append(' ')
              .Append(F(c.r)).Append(' ').Append(F(c.g)).Append(' ').Append(F(c.b)).Append(' ').Append(F(c.a)).Append(' ').Append(sr.sortingOrder).Append('\n');
        }
    }

    static void FxTexts(StringBuilder sb)
    {
        foreach (var tm in UnityEngine.Object.FindObjectsOfType<Il2CppTMPro.TextMeshPro>())
        {
            if (tm == null || !tm.enabled || !tm.gameObject.activeInHierarchy) continue;
            var path = PathOf(tm.transform);
            if (!Wanted(path)) continue;
            var t = tm.transform;
            var c = tm.color;
            sb.Append("T ").Append(path).Append(' ').Append(V(t.position)).Append(' ').Append(Q(t.rotation)).Append(' ').Append(V(t.lossyScale)).Append(' ')
              .Append(F(tm.fontSize)).Append(' ').Append(F(c.r)).Append(' ').Append(F(c.g)).Append(' ').Append(F(c.b)).Append(' ').Append(F(c.a)).Append(' ')
              .Append((tm.text ?? "").Replace('\n', ' ')).Append('\n');
        }
    }

    // A screenshot once the frame is drawn (WaitForEndOfFrame): CaptureScreenshotAsTexture, its
    // pixels written as a PNG here (ScreenCapture.CaptureScreenshot's file name can't be passed:
    // the runtime lacks the span method that string marshalling needs).
    static System.Collections.IEnumerator Shot(string path)
    {
        yield return new WaitForEndOfFrame();
        Texture2D tex = null;
        try
        {
            tex = ScreenCapture.CaptureScreenshotAsTexture(Math.Max(1, Sc.shot_scale));
            int w = tex.width, h = tex.height;
            var px = tex.GetPixels32();
            var rgba = new byte[w * h * 4];
            for (int y = 0; y < h; y++)
            {
                // Unity's rows run bottom up.
                int src = (h - 1 - y) * w, dst = y * w * 4;
                for (int x = 0; x < w; x++)
                {
                    var c = px[src + x];
                    rgba[dst + x * 4] = c.r;
                    rgba[dst + x * 4 + 1] = c.g;
                    rgba[dst + x * 4 + 2] = c.b;
                    rgba[dst + x * 4 + 3] = 255;
                }
            }
            File.WriteAllBytes(path, Png(w, h, rgba));
            MelonLogger.Msg("SimRecorder shot " + path);
        }
        catch (Exception e)
        {
            MelonLogger.Error("SimRecorder shot: " + e);
        }
        if (tex != null) UnityEngine.Object.Destroy(tex);
    }

    // RGBA8 -> PNG (filter 0 rows, zlib).
    static byte[] Png(int w, int h, byte[] rgba)
    {
        var raw = new MemoryStream();
        using (var z = new System.IO.Compression.ZLibStream(raw, System.IO.Compression.CompressionLevel.Fastest, true))
        {
            for (int y = 0; y < h; y++)
            {
                z.WriteByte(0);
                z.Write(rgba, y * w * 4, w * 4);
            }
        }
        var o = new MemoryStream();
        o.Write(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 });
        var ihdr = new byte[13];
        BE(ihdr, 0, (uint)w);
        BE(ihdr, 4, (uint)h);
        ihdr[8] = 8;
        ihdr[9] = 6;
        Chunk(o, "IHDR", ihdr);
        Chunk(o, "IDAT", raw.ToArray());
        Chunk(o, "IEND", Array.Empty<byte>());
        return o.ToArray();
    }

    static void BE(byte[] b, int i, uint v)
    {
        b[i] = (byte)(v >> 24);
        b[i + 1] = (byte)(v >> 16);
        b[i + 2] = (byte)(v >> 8);
        b[i + 3] = (byte)v;
    }

    static uint[] crcTable;

    static void Chunk(Stream o, string type, byte[] data)
    {
        if (crcTable == null)
        {
            crcTable = new uint[256];
            for (uint n = 0; n < 256; n++)
            {
                uint c = n;
                for (int k = 0; k < 8; k++) c = (c & 1) != 0 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
                crcTable[n] = c;
            }
        }
        var len = new byte[4];
        BE(len, 0, (uint)data.Length);
        o.Write(len);
        var td = Encoding.ASCII.GetBytes(type);
        o.Write(td);
        o.Write(data);
        uint crc = 0xFFFFFFFFu;
        foreach (var x in td) crc = crcTable[(crc ^ x) & 0xFF] ^ (crc >> 8);
        foreach (var x in data) crc = crcTable[(crc ^ x) & 0xFF] ^ (crc >> 8);
        var cb = new byte[4];
        BE(cb, 0, crc ^ 0xFFFFFFFFu);
        o.Write(cb);
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
        // GetInLine's iterator MoveNext (a shared generic, no static address): its GameAssembly
        // address, from the class of the object GetInLine returns.
        try
        {
            var it = grid.GetInLine(new Il2CppAssets.Scripts.Simulation.SMath.Vector3(0, 0, 0), new Il2CppAssets.Scripts.Simulation.SMath.Vector3(1, 0, 0), 1f);
            var obj = (Il2CppSystem.Object)it.Cast<Il2CppSystem.Object>();
            IntPtr klass = Il2CppInterop.Runtime.IL2CPP.il2cpp_object_get_class(obj.Pointer);
            IntPtr mi = Il2CppInterop.Runtime.IL2CPP.il2cpp_class_get_method_from_name(klass, "MoveNext", 0);
            long fp = System.Runtime.InteropServices.Marshal.ReadIntPtr(mi).ToInt64();
            long gbase = System.Diagnostics.Process.GetCurrentProcess().Modules.Cast<System.Diagnostics.ProcessModule>().First(x => x.ModuleName == "GameAssembly.dll").BaseAddress.ToInt64();
            m.WriteLine($"getinline_movenext 0x{fp - gbase + 0x180000000:x} {System.Runtime.InteropServices.Marshal.PtrToStringAnsi(Il2CppInterop.Runtime.IL2CPP.il2cpp_class_get_name(klass))}");
        }
        catch (Exception e) { m.WriteLine("getinline_movenext error " + e.Message); }
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

    static bool ending;

    System.Collections.IEnumerator EndScreen()
    {
        var ig = InGame.instance;
        if (Sc.end == "victory")
        {
            // OnVictory's last step (its rewards / stats / map completion skipped): FadeIn with
            // the victory loot, which opens GetVictorySceneName's menu with GetVictoryScreenData.
            ig.StartCoroutine(ig.ShowVictoryScreen(ig.GetVictoryLoot()));
        }
        else
        {
            // Lose: GetDefeatSceneData(scene, current round, cause of defeat, continue cost)
            // queued on the MenuManager; the cost by round as Lose picks it (round < 21: 100,
            // < 41: 200, < 61: 300, < 81: 400, else 500).
            int round = ig.bridge.GetCurrentRound();
            double cost = round < 21 ? 100 : round < 41 ? 200 : round < 61 ? 300 : round < 81 ? 400 : 500;
            var name = ig.GetDefeatSceneName();
            var cod = new Il2CppSystem.Collections.Generic.List<Il2CppAssets.Scripts.Unity.UI_New.InGame.BloonCoDData>();
            foreach (var e in Sc.end_leaks)
            {
                var d = new Il2CppAssets.Scripts.Unity.UI_New.InGame.BloonCoDData();
                d.id = e[0];
                d.damage = float.Parse(e[1], System.Globalization.CultureInfo.InvariantCulture);
                cod.Add(d);
            }
            var data = ig.GetDefeatSceneData(name, round, cod, new Il2CppAssets.Scripts.Utils.KonFuze(cost));
            Il2CppAssets.Scripts.Unity.Menu.MenuManager.instance.QueueMenu(name, data);
        }
        LoggerInstance.Msg("SimRecorder end screen " + Sc.end);
        float t0 = Time.realtimeSinceStartup;
        double last = 0;
        foreach (var d in Sc.end_shots) last = Math.Max(last, d);
        var shotsLeft = new List<double>(Sc.end_shots);
        bool nextDone = Sc.end_next < 0;
        float nextLog = 0;
        while (Time.realtimeSinceStartup - t0 <= last + 0.05)
        {
            float t = Time.realtimeSinceStartup - t0;
            var screen = UnityEngine.Object.FindObjectOfType<Il2CppAssets.Scripts.Unity.UI_New.GameOver.SummaryScreen>();
            for (int k = shotsLeft.Count - 1; k >= 0; k--)
                if (t >= shotsLeft[k])
                {
                    MelonCoroutines.Start(Shot(Path.Combine(dir, Sc.name + "_end_" + (int)(shotsLeft[k] * 1000) + ".png")));
                    // What the screen says then: every active text under it.
                    if (screen != null)
                        foreach (var tx in screen.GetComponentsInChildren<Il2CppTMPro.TextMeshProUGUI>(false))
                            LoggerInstance.Msg($"end text t={shotsLeft[k]:F2} {PathOf(tx.transform)} = {tx.text.Replace('\n', '|')}");
                    shotsLeft.RemoveAt(k);
                }
            // The screen's Animator: AnimationIndex and the current state (hashes, as
            // Animator.StringToHash: CRC32 of the name; strings don't cross the interop).
            if (t >= nextLog)
            {
                nextLog = t + 0.25f;
                var an = screen != null ? screen.GetComponent<Animator>() : null;
                if (an != null)
                {
                    var st = an.GetCurrentAnimatorStateInfo(0);
                    LoggerInstance.Msg($"end anim t={t:F2} AnimationIndex={an.GetInteger(1140694833)} state={st.shortNameHash} norm={st.normalizedTime:F3}");
                }
                else LoggerInstance.Msg($"end anim t={t:F2} no screen");
            }
            if (!nextDone && t >= Sc.end_next && screen != null)
            {
                nextDone = true;
                screen.OnNextClicked();
                LoggerInstance.Msg($"end next pressed t={t:F2}");
            }
            yield return null;
        }
        float t1 = Time.realtimeSinceStartup;
        while (Time.realtimeSinceStartup - t1 < 1) yield return null;
        Done("ok");
    }

    // Scenario.menu: the menu's screenshots, then the end.
    System.Collections.IEnumerator MenuShots()
    {
        float t0 = Time.realtimeSinceStartup;
        var left = new List<double>(Sc.menu_shots);
        double last = 0;
        foreach (var d in left) last = Math.Max(last, d);
        int clicked = 0;
        while (Time.realtimeSinceStartup - t0 <= last + 0.05)
        {
            float t = Time.realtimeSinceStartup - t0;
            if (clicked < Sc.menu_clicks.Count && t >= 1 + 0.5 * clicked)
            {
                var name = Sc.menu_clicks[clicked++];
                var go = GameObject.Find(name);
                if (go == null)
                    foreach (var tr in UnityEngine.Object.FindObjectsOfType<Transform>())
                        if (tr.gameObject.activeInHierarchy && tr.name == name) { go = tr.gameObject; break; }
                var btn = go?.GetComponent<UnityEngine.UI.Button>();
                var tog = go?.GetComponent<UnityEngine.UI.Toggle>();
                if (btn != null) btn.onClick.Invoke(); else if (tog != null) tog.isOn = true;
                LoggerInstance.Msg($"menu click {name}: {(btn != null ? "button" : tog != null ? "toggle" : "not found")}");
            }
            for (int k = left.Count - 1; k >= 0; k--)
                if (t >= left[k])
                {
                    MelonCoroutines.Start(Shot(Path.Combine(dir, Sc.name + "_menu_" + (int)(left[k] * 1000) + ".png")));
                    left.RemoveAt(k);
                }
            yield return null;
        }
        float t1 = Time.realtimeSinceStartup;
        while (Time.realtimeSinceStartup - t1 < 1) yield return null;
        Done("ok");
    }

    internal void Done(string why)
    {
        Out?.WriteLine("# end " + why);
        Out?.Flush();
        Out?.Dispose();
        Out = null;
        FxOut?.WriteLine("# end " + why);
        FxOut?.Flush();
        FxOut?.Dispose();
        FxOut = null;
        LoggerInstance.Msg("SimRecorder done: " + why);
        phase = -1;
        if (Sc != null && Sc.quit) Application.Quit();
    }
}

// A recording never really ends the game: InGame.OnVictory (map completion, Monkey Money, XP,
// achievements, stats, analytics) and InGame.Lose (stats, analytics) are skipped while a
// scenario runs; end screens come only from Scenario.end, which opens them without either.
[HarmonyPatch(typeof(InGame), nameof(InGame.OnVictory))]
static class NoVictoryPatch
{
    static bool Prefix()
    {
        if (Recorder.Sc == null) return true;
        MelonLogger.Msg("SimRecorder: blocked InGame.OnVictory (no rewards or profile changes)");
        return false;
    }
}

[HarmonyPatch(typeof(InGame), nameof(InGame.Lose))]
static class NoLosePatch
{
    static bool Prefix()
    {
        if (Recorder.Sc == null) return true;
        MelonLogger.Msg("SimRecorder: blocked InGame.Lose");
        return false;
    }
}

// Btd6Player.DrainXP (InGame.RoundEnd): what the round earned, the player's rank XP and each
// tower's ("X frame total id:xp ...").
[HarmonyPatch(typeof(Il2CppAssets.Scripts.Unity.Player.Btd6Player), nameof(Il2CppAssets.Scripts.Unity.Player.Btd6Player.DrainXP))]
static class DrainXpPatch
{
    static void Prefix(Il2CppAssets.Scripts.Unity.Bridge.Xp xpToDrain)
    {
        var w = Recorder.Out;
        var xp = xpToDrain;
        if (Recorder.Sc == null || w == null || xp == null) return;
        var sb = new StringBuilder($"X {Recorder.Recorded} {Recorder.F(xp.account)}");
        if (xp.towers != null)
            foreach (var kv in xp.towers) sb.Append(' ').Append(kv.Key).Append(':').Append(Recorder.F(kv.Value));
        w.WriteLine(sb.ToString());
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

    // Wind-carried bloons as the frame starts: "WC id offTrackCount x y distance" (written before
    // the frame's F line, so it belongs to the frame that F line opens).
    static void Prefix(Simulation __instance)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try
        {
            var bs = __instance.factory.GetFactory<Bloon>().up.list;
            for (int k = 0; k < bs.Count; k++)
            {
                var b = bs[k];
                if (b == null || b.IsDestroyed || b.offTrackCount <= 0) continue;
                var p = b.Position;
                w.WriteLine($"WC {b.Id.Id} {b.offTrackCount} {Recorder.F(p.X)} {Recorder.F(p.Y)} {Recorder.F(b.distanceTraveled)}");
            }
        }
        catch (Exception) { }
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
        if (Recorder.Recorded == Recorder.Sc.paragon_investment_frame && Recorder.Sc.paragon_investment >= 0)
            Part("paragon_investment", () =>
            {
                var list = __instance.factory.GetFactory<Tower>().up.list;
                for (int j = 0; j < list.Count; j++)
                {
                    var tw = list[j];
                    if (tw == null || tw.IsDestroyed) continue;
                    Il2CppAssets.Scripts.Simulation.Towers.Behaviors.ParagonTower pt = null;
                    var tb = tw.towerBehaviors?.list;
                    for (int k = 0; tb != null && k < tb.Count && pt == null; k++)
                        pt = tb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.ParagonTower>();
                    if (pt == null) continue;
                    var info = pt.investmentInfo;
                    info.totalInvestment = (float)Recorder.Sc.paragon_investment;
                    pt.investmentInfo = info;
                    pt.UpdateDegree();
                    MelonLogger.Msg($"SimRecorder paragon investment {info.totalInvestment} degree {pt.GetCurrentDegree()} at frame {Recorder.Recorded}");
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
                            Recorder.SetPoint(tw, Recorder.Sc.towers[kk], kk);
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
                // Its offTrackCount when carried: "BO id count".
                if (b.offTrackCount > 0) w.WriteLine($"BO {b.Id.Id} {b.offTrackCount}");
                // Its Position.z when that isn't 0: "BZ id z".
                if (p.Z != 0f) w.WriteLine($"BZ {b.Id.Id} {Recorder.F(p.Z)}");
                // Its layerId and lineage: "BL id layer a,b,c".
                try
                {
                    var lin = new StringBuilder();
                    if (b.lineage != null) foreach (var x in b.lineage) lin.Append(x).Append(',');
                    w.WriteLine($"BL {b.Id.Id} {b.layerId} {lin}");
                }
                catch (Exception) { }
                // Its DamageOverTime behaviours in Bloon.bloonBehaviors order: "BD id index lastTick
                // model name" (index among all its behaviours).
                try
                {
                    var bbs = b.bloonBehaviors?.list;
                    for (int j = 0; bbs != null && j < bbs.Count; j++)
                    {
                        var dot = bbs[j]?.TryCast<Il2CppAssets.Scripts.Simulation.Bloons.Behaviors.DamageOverTime>();
                        if (dot == null) continue;
                        w.WriteLine($"BD {b.Id.Id} {j} {dot.lastTick} {dot.damageOverTimeModel?.name}");
                    }
                }
                catch (Exception) { }
            }
        });
        // Every stun-like mutator instance on each bloon that has one: "S bloon id:removeAt ..."
        // (TimedMutator order as in Mutable.mutators).
        Part("stuns", () =>
        {
            var bs = sim.factory.GetFactory<Bloon>().up.list;
            for (int k = 0; k < bs.Count; k++)
            {
                var b = bs[k];
                if (b == null || b.IsDestroyed || b.mutators == null) continue;
                var m = b.mutators;
                string line = $"S {b.Id.Id}";
                bool stun = false;
                for (int j = 0; j < m.Count; j++)
                {
                    var tm = m._items[j];
                    if (tm == null || tm.mutator == null) continue;
                    var id = tm.mutator.id ?? "?";
                    stun |= id.Contains("Stun") || id.Contains("SpiritOfTheForest");
                    line += $" {id}:{tm.removeAt}";
                }
                if (stun) w.WriteLine(line);
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
            // The player's InputManager cursor (FollowTouchSetting.Process reads it each frame:
            // cursorLocation while cursorActive and not ignoreCursor): "IN x y active ignore index".
            var ts = sim.factory.GetFactory<Tower>().up.list;
            // (By the first tower's owner, as FollowTouchSetting.Process asks.)
            for (int j = 0; j < ts.Count; j++)
            {
                var t0 = ts[j];
                if (t0 == null || t0.IsDestroyed) continue;
                if (sim.InputManagerExists(t0.owner))
                {
                    var im = sim.GetInputManager(t0.owner);
                    if (im != null)
                        w.WriteLine($"IN {Recorder.F(im.cursorLocation.x)} {Recorder.F(im.cursorLocation.y)} {(im.cursorActive ? 1 : 0)} {(im.ignoreCursor ? 1 : 0)} {t0.owner}");
                }
                break;
            }
            for (int j = 0; j < ts.Count; j++)
            {
                var t = ts[j];
                if (t == null || t.IsDestroyed || t.attackBehaviorsInDependants == null) continue;
                // Carrier planes' FighterMovement each frame: "FM tower x y rotX rotY targetX targetY timer flyover"
                // (transformPosition is where it was as its Process began).
                Il2CppAssets.Scripts.Simulation.Towers.Behaviors.FighterMovement fm = null;
                var ftb = t.towerBehaviors?.list;
                for (int k = 0; ftb != null && k < ftb.Count && fm == null; k++)
                {
                    var au = ftb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.AirUnit>();
                    var mb = au?.modelBehaviors;
                    for (int q = 0; mb != null && q < mb.Count && fm == null; q++)
                        fm = mb[q]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.FighterMovement>();
                }
                // Ace-style planes' PathMovement each frame (after it ran): "PM tower x y z rotation
                // scaleX scaleY scaleZ t onPath takingOff tt ttpf pitch lastBank tpf onTheMove"
                // (the AirUnit's TransformBehavior; FireFromAirUnit ejects from it).
                for (int k = 0; ftb != null && k < ftb.Count; k++)
                {
                    var au = ftb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.AirUnit>();
                    var mb = au?.modelBehaviors;
                    for (int q = 0; mb != null && q < mb.Count; q++)
                    {
                        var pm = mb[q]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.PathMovement>();
                        var tr = au.transform;
                        if (pm == null || tr == null) continue;
                        var p = tr.position; var sc = tr.scale;
                        w.WriteLine($"PM {t.Id.Id} {Recorder.F(p.X)} {Recorder.F(p.Y)} {Recorder.F(p.Z)} {Recorder.F(tr.rotation.value)} {Recorder.F(sc.X)} {Recorder.F(sc.Y)} {Recorder.F(sc.Z)} {Recorder.F(pm.t)} {(pm.onPath ? 1 : 0)} {(pm.takingOff ? 1 : 0)} {Recorder.F(pm.tt)} {Recorder.F(pm.ttpf)} {Recorder.F(pm.pitch)} {Recorder.F(pm.lastBank)} {Recorder.F(pm.tpf)} {(pm.onTheMove ? 1 : 0)}");
                    }
                }
                // Helis' HeliMovement each frame (after it ran): "HM tower x y z rotation velX velY
                // groupOffX groupOffY pathIndex" (heliTransformBehavior; its attacks shoot from it).
                for (int k = 0; ftb != null && k < ftb.Count; k++)
                {
                    var hm = ftb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.HeliMovement>();
                    var au = ftb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.AirUnit>();
                    var mb = au?.modelBehaviors;
                    for (int q = 0; hm == null && mb != null && q < mb.Count; q++)
                        hm = mb[q]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.HeliMovement>();
                    var tr = hm?.heliTransformBehavior;
                    if (hm == null || tr == null) continue;
                    var p = tr.position;
                    w.WriteLine($"HM {t.Id.Id} {Recorder.F(p.X)} {Recorder.F(p.Y)} {Recorder.F(p.Z)} {Recorder.F(tr.rotation.value)} {Recorder.F(hm.currentVelocity.x)} {Recorder.F(hm.currentVelocity.y)} {Recorder.F(hm.lastGroupOffset.x)} {Recorder.F(hm.lastGroupOffset.y)} {hm.pathIndex}");
                    break;
                }
                if (fm != null)
                    w.WriteLine($"FM {t.Id.Id} {Recorder.F(fm.transformPosition.x)} {Recorder.F(fm.transformPosition.y)} {Recorder.F(fm.rotation.x)} {Recorder.F(fm.rotation.y)} {Recorder.F(fm.targetPosition.x)} {Recorder.F(fm.targetPosition.y)} {fm.timer} {(fm.flyoverEngaged ? 1 : 0)}");
                int i = 0;
                var l = t.attackBehaviorsInDependants.list;
                for (int k = 0; k < l.Count; k++)
                {
                    var wp = l[k]?.TryCast<Weapon>();
                    if (wp == null) continue;
                    w.WriteLine($"W {t.Id.Id} {i++} {wp.lastEmission} {wp.animationStartedAt} {wp.rateFrames} {Recorder.F(wp.missedRate)} {Recorder.F(t.Rotation)} {Recorder.F(t.Position.X)} {Recorder.F(t.Position.Y)}");
                    // WN tower index weapon attack: each weapon slot's model names, once (maps the W
                    // indices to models, e.g. the Sun Temple's mutator-added attacks).
                    if (Recorder.weaponNamed.Add($"{t.Id.Id}:{i - 1}:{wp.weaponModel?.name}"))
                        w.WriteLine($"WN {t.Id.Id} {i - 1} {wp.weaponModel?.name ?? "-"} {wp.mainAttack?.attackModel?.name ?? "-"}");
                    // FilterTargetAngleFilter's inputs each frame: "FA tower weapon displayRotation towerZ
                    // timeOfLastTargetUpdate lastFireResult" (FilterEmission @0x180c76350 reads
                    // display.node.rotation and the 3D angle from Tower.Position to Bloon.Position).
                    if (wp.weaponBehaviors != null)
                    {
                        var fbl = wp.weaponBehaviors.list;
                        for (int q = 0; q < fbl.Count; q++)
                        {
                            var fa = fbl[q]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.FilterTargetAngleFilter>();
                            if (fa == null) continue;
                            var node = fa.display?.node;
                            string dr = node == null || node.rotation == null ? "none" : Recorder.F(node.rotation.value);
                            w.WriteLine($"FA {t.Id.Id} {i - 1} {dr} {Recorder.F(t.Position.Z)} {fa.timeOfLastTargetUpdate} {(fa.lastFireResult ? 1 : 0)}");
                        }
                    }
                    // Zoned<Bloon> (Vagrant's bloon zone) methods' addresses, once: "zone_methods IsInZone Process".
                    if (!ZoneProbe.Done && wp.weaponBehaviors != null)
                    {
                        var wbl = wp.weaponBehaviors.list;
                        for (int q = 0; q < wbl.Count; q++)
                        {
                            var vb = wbl[q]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.VagrantWeaponBehavior>();
                            if (vb == null || vb.bloonZone == null) continue;
                            ZoneProbe.Done = true;
                            IntPtr klass = Il2CppInterop.Runtime.IL2CPP.il2cpp_object_get_class(vb.bloonZone.Pointer);
                            long gbase = System.Diagnostics.Process.GetCurrentProcess().Modules.Cast<System.Diagnostics.ProcessModule>().First(x => x.ModuleName == "GameAssembly.dll").BaseAddress.ToInt64();
                            var sb2 = new StringBuilder("zone_methods");
                            foreach (var (mn, ac) in new[] { ("IsInZone", 2), ("IsInZone", 1), ("Process", 1), ("Filter", 1), ("GetRange", 0) })
                            {
                                IntPtr mi = Il2CppInterop.Runtime.IL2CPP.il2cpp_class_get_method_from_name(klass, mn, ac);
                                long fp = mi == IntPtr.Zero ? 0 : System.Runtime.InteropServices.Marshal.ReadIntPtr(mi).ToInt64();
                                sb2.Append($" {mn}/{ac}=0x{(fp == 0 ? 0 : fp - gbase + 0x180000000):x}");
                            }
                            w.WriteLine(sb2.ToString());
                        }
                    }
                }
                // Orbit state (Glaive Lord / Dominus): "O tower currentSpeed angle bloonInRange active".
                var tb = t.towerBehaviors?.list;
                if (tb != null)
                {
                    for (int k = 0; k < tb.Count; k++)
                    {
                        var ob = tb[k]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Behaviors.Orbit>();
                        if (ob == null) continue;
                        var bl = ob.bloonInRange;
                        // ... then the tower's Position and the bloon's (x y z each; the keep test is
                        // their 3D distance).
                        var tp = t.Position;
                        var bp = bl == null ? tp : bl.Position;
                        w.WriteLine($"O {t.Id.Id} {Recorder.F(ob.currentSpeed)} {Recorder.F(ob.angle)} {(bl == null ? -1 : bl.Id.Id)} {(ob.isActive ? 1 : 0)} {Recorder.F(tp.X)} {Recorder.F(tp.Y)} {Recorder.F(tp.Z)} {Recorder.F(bp.X)} {Recorder.F(bp.Y)} {Recorder.F(bp.Z)}");
                    }
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
                // Its Direction: "D projectile x y z".
                try { var dv = pr.Direction; if (dv != null) w.WriteLine($"D {pr.Id.Id} {Recorder.F(dv.X)} {Recorder.F(dv.Y)} {Recorder.F(dv.Z)}"); } catch (Exception) { }
                // A Lightning's state: "LG projectile hitTarget currentDelayFrames shared:a,b collided:c,d".
                try
                {
                    var pbs = pr.projectileBehaviors?.list;
                    for (int j = 0; pbs != null && j < pbs.Count; j++)
                    {
                        var lg = pbs[j]?.TryCast<Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Behaviors.Lightning>();
                        if (lg == null) continue;
                        var sh = new StringBuilder();
                        if (lg.sharedCollidedWith != null) foreach (var x in lg.sharedCollidedWith) sh.Append(x).Append(',');
                        var cw = new StringBuilder();
                        if (pr.collidedWith != null) foreach (var x in pr.collidedWith) cw.Append(x).Append(',');
                        w.WriteLine($"LG {pr.Id.Id} {(lg.hitTarget ? 1 : 0)} {lg.currentDelayFrames} shared:{sh} collided:{cw}");
                    }
                }
                catch (Exception) { }
                // Its Target: "PT projectile bloon(-1 none) valid".
                try
                {
                    var tg = pr.target;
                    var tb = tg._bloon;
                    w.WriteLine($"PT {pr.Id.Id} {(tb == null ? -1 : tb.Id.Id)} {(tg.IsValid ? 1 : 0)}");
                }
                catch (Exception) { }
            }
        });
    }
}

// Every draw from Simulation.syncedRandom: "R kind value" (kind d = NextDouble, n = Next(),
// m = Next(max), r = Next(min, max)), written before the frame's F line, in draw order.
static class SyncedRandomLog
{
    internal static bool IsSynced(Il2CppNinjaKiwi.Common.BufferedRandom r)
    {
        if (Recorder.Out == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return false;
        try
        {
            var sim = InGame.instance?.bridge?.Simulation;
            return sim != null && sim.syncedRandom != null && sim.syncedRandom.Pointer == r.Pointer;
        }
        catch (Exception) { return false; }
    }
}

[HarmonyPatch(typeof(Il2CppNinjaKiwi.Common.BufferedRandom), nameof(Il2CppNinjaKiwi.Common.BufferedRandom.NextDouble))]
static class SyncedNextDoublePatch
{
    static void Postfix(Il2CppNinjaKiwi.Common.BufferedRandom __instance, double __result)
    {
        if (SyncedRandomLog.IsSynced(__instance))
            Recorder.Out.WriteLine($"R d {__result.ToString("R", CultureInfo.InvariantCulture)}");
    }
}

[HarmonyPatch(typeof(Il2CppNinjaKiwi.Common.BufferedRandom), nameof(Il2CppNinjaKiwi.Common.BufferedRandom.Next), new Type[0])]
static class SyncedNextPatch
{
    static void Postfix(Il2CppNinjaKiwi.Common.BufferedRandom __instance, int __result)
    {
        if (SyncedRandomLog.IsSynced(__instance)) Recorder.Out.WriteLine($"R n {__result}");
    }
}

[HarmonyPatch(typeof(Il2CppNinjaKiwi.Common.BufferedRandom), nameof(Il2CppNinjaKiwi.Common.BufferedRandom.Next), new[] { typeof(int) })]
static class SyncedNextMaxPatch
{
    static void Postfix(Il2CppNinjaKiwi.Common.BufferedRandom __instance, int __0, int __result)
    {
        if (SyncedRandomLog.IsSynced(__instance)) Recorder.Out.WriteLine($"R m {__0} {__result}");
    }
}

[HarmonyPatch(typeof(Il2CppNinjaKiwi.Common.BufferedRandom), nameof(Il2CppNinjaKiwi.Common.BufferedRandom.Next), new[] { typeof(int), typeof(int) })]
static class SyncedNextRangePatch
{
    static void Postfix(Il2CppNinjaKiwi.Common.BufferedRandom __instance, int __0, int __1, int __result)
    {
        if (SyncedRandomLog.IsSynced(__instance)) Recorder.Out.WriteLine($"R r {__0} {__1} {__result}");
    }
}

// Every MoveWithWind step: "WM bloon model speedFrames x y" after it moves (twice in a frame would
// mean the step is redone).
[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Bloons.Behaviors.MoveWithWind), nameof(Il2CppAssets.Scripts.Simulation.Bloons.Behaviors.MoveWithWind.Process))]
static class MoveWithWindPatch
{
    static void Postfix(Il2CppAssets.Scripts.Simulation.Bloons.Behaviors.MoveWithWind __instance)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try
        {
            var b = __instance.bloon;
            if (b == null) return;
            var p = b.Position;
            w.WriteLine($"WM {b.Id.Id} {b.bloonModel.id} {Recorder.F(b.bloonModel.speedFrames)} {Recorder.F(p.X)} {Recorder.F(p.Y)}");
        }
        catch (Exception) { }
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
            // The bloon's state: "HB bloon model destroyed".
            if (__0 != null) w.WriteLine($"HB {__0.Id.Id} {__0.bloonModel?.id} {__0.IsDestroyed} {Recorder.F(__0.Radius)}");
            // A projectile's first collisions (age 0-1): native callers ("HS projectile rva ...").
            if (__instance.Age <= 1 && HitStack.Count++ < 400) w.WriteLine("HS " + __instance.Id.Id + HitStack.Callers());
        }
        catch (Exception) { }
    }
}

// Weapon rate behaviours' GetRate, input and output ("Q kind in out"), written in call order before
// the frame's F line: BloonDistanceRateBonus (distance d = (in / out - 1) / (max x per / max)),
// Standoff and VagrantWeaponBehavior.
static class RateLog
{
    public static void Write(string kind, float rate, float result)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try { w.WriteLine($"Q {kind} {Recorder.F(rate)} {Recorder.F(result)}"); } catch (Exception) { }
    }
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.BloonDistanceRateBonus), "GetRate")]
static class DistanceRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("dist", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.Standoff), "GetRate")]
static class StandoffRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("standoff", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.VagrantWeaponBehavior), "GetRate")]
static class VagrantRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("vagrant", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.BloonsInRangeAttackSpeed), "GetRate")]
static class BloonsInRangeRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("inrange", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.LeakDangerAttackSpeed), "GetRate")]
static class LeakDangerRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("leak", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.LifeBasedAttackSpeed), "GetRate")]
static class LifeBasedRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("life", __0, __result);
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Weapons.Behaviors.TheBlazingSun), "GetRate")]
static class BlazingSunRatePatch
{
    static void Postfix(float __0, float __result) => RateLog.Write("sun", __0, __result);
}

// Bloon mutator bookkeeping ("M ..."), written before the frame's F line: each
// AddMutatorForNumLayers with the mutator's removeAt before and after, each charge depletion and
// each child copying its parent's mutators.
static class MutatorLog
{
    public static bool On => Recorder.Out != null && Recorder.Sc != null && Recorder.Recorded < Recorder.Sc.frames;
    public static int RemoveAt(Il2CppAssets.Scripts.Simulation.Bloons.Bloon b, string id)
    {
        try { var t = b.GetMutatorById(id); return t == null ? -1 : t.removeAt; } catch (Exception) { return -2; }
    }
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Bloons.Bloon), "AddMutatorForNumLayers")]
static class AddMutatorLayersPatch
{
    static void Prefix(Il2CppAssets.Scripts.Simulation.Bloons.Bloon __instance, Il2CppAssets.Scripts.Simulation.Objects.BehaviorMutator __0, out int __state)
    {
        __state = -3;
        if (!MutatorLog.On || __0 == null) return;
        __state = MutatorLog.RemoveAt(__instance, __0.id);
    }
    static void Postfix(Il2CppAssets.Scripts.Simulation.Bloons.Bloon __instance, Il2CppAssets.Scripts.Simulation.Objects.BehaviorMutator __0, int __1, int __2, int __state)
    {
        if (!MutatorLog.On || __0 == null) return;
        try { Recorder.Out.WriteLine($"M add {__instance.Id.Id} {__0.id} time {__1} layers {__2} before {__state} after {MutatorLog.RemoveAt(__instance, __0.id)}"); } catch (Exception) { }
    }
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Bloons.Bloon), "DepleteMutatorsForNumLayers")]
static class DepleteMutatorsPatch
{
    static void Postfix(Il2CppAssets.Scripts.Simulation.Bloons.Bloon __instance, int __0)
    {
        if (!MutatorLog.On) return;
        try { Recorder.Out.WriteLine($"M deplete {__instance.Id.Id} {__0}"); } catch (Exception) { }
    }
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Bloons.Bloon), "CopyMutatorsFromBloon")]
static class CopyMutatorsPatch
{
    static void Postfix(Il2CppAssets.Scripts.Simulation.Bloons.Bloon __instance, Il2CppAssets.Scripts.Simulation.Bloons.Bloon __0)
    {
        if (!MutatorLog.On) return;
        try { Recorder.Out.WriteLine($"M copy {__instance.Id.Id} from {(__0 == null ? -1 : __0.Id.Id)}"); } catch (Exception) { }
    }
}

// Elite targeting's sort keys in the order LINQ asks for them (GetTargets' list order):
// "ES bloon key" (written before the frame's F line).
[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Behaviors.Attack.Behaviors.TargetEliteTargetting), "Sort")]
static class EliteSortPatch
{
    static void Postfix(Bloon __0, float __result)
    {
        if (Recorder.Out == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try { Recorder.Out.WriteLine($"ES {(__0 == null ? -1 : __0.Id.Id)} {Recorder.F(__result)}"); } catch (Exception) { }
    }
}

// The bloon grid as CollisionChecker.Process finds it (before it runs), for frames listed in the
// scenario's gridlog: "GB bucket bloon ..." (written before the frame's F line).
[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Physics.CollisionChecker), "Process")]
static class GridLogPatch
{
    static void Prefix(Il2CppAssets.Scripts.Simulation.Physics.CollisionChecker __instance)
    {
        var w = Recorder.Out;
        if (w == null || Recorder.Sc == null || Recorder.Sc.gridlog == null) return;
        int f = Recorder.Recorded;
        if (f < Recorder.Sc.gridlog[0] || f > Recorder.Sc.gridlog[1]) return;
        try
        {
            var bb = __instance.BloonGrid.buckets;
            for (int k = 0; k < bb.Length; k++)
            {
                var bk = bb[k];
                if (bk == null || bk.collidables == null || bk.collidables.Count == 0) continue;
                var sb = new StringBuilder($"GB {k}");
                for (int j = 0; j < bk.collidables.Count; j++) sb.Append(' ').Append(bk.collidables[j].Id.Id);
                w.WriteLine(sb.ToString());
            }
        }
        catch (Exception) { }
    }
}

static class ZoneProbe { public static bool Done; }

static class HitStack
{
    public static int Count;
    [System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
    static extern IntPtr GetModuleHandleW(string name);
    public static unsafe string Callers()
    {
        long gbase = GetModuleHandleW("GameAssembly.dll").ToInt64();
        var sb = new StringBuilder();
        long* sp = stackalloc long[1];
        int n = 0;
        for (int i = 0; i < 4096 && n < 8; i++)
        {
            long v = sp[i], a = v - gbase;
            if (a < 0x1000 || a >= 0x3800000) continue;
            byte* c = (byte*)v;
            if (c[-5] == 0xE8 || c[-2] == 0xFF || c[-3] == 0xFF || c[-6] == 0xFF) { sb.Append(" 0x").Append((a + 0x180000000).ToString("x")); n++; }
        }
        return sb.ToString();
    }
}

// How projectiles end ("X how projectile age pierce"), written before the frame's
// F line: Expire (lifespan or an explicit expire), Exhausted (out of pierce), OnDestroy.
static class EndLog
{
    public static void Write(string how, Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile p)
    {
        if (p == null || Recorder.Out == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try { Recorder.Out.WriteLine($"X {how} {p.Id.Id} {p.Age} {Recorder.F(p.pierce)}"); } catch (Exception) { }
    }
}

// A line's list collisions (LineProjectileEmission.Emit -> Projectile.CollideBloons): "CB projectile
// count pierce src <class> skip n". The Ray of Doom passes OrderBy(GetInLine(...)).Skip(n), all lazy
// (enumerating it here would run it again), so only the Skip iterator's arguments are read.
[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile), "CollideBloons", new Type[] {
    typeof(Il2CppSystem.Collections.Generic.IEnumerable<Il2CppAssets.Scripts.Simulation.Bloons.Bloon>), typeof(int),
    typeof(Il2CppSystem.Collections.Generic.List<Il2CppAssets.Scripts.Simulation.SMath.Vector3>) })]
static class CollideBloonsPatch
{
    static void Prefix(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile __instance,
        Il2CppSystem.Collections.Generic.IEnumerable<Il2CppAssets.Scripts.Simulation.Bloons.Bloon> __0, int __1)
    {
        if (__instance == null || Recorder.Out == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
        try
        {
            var l = __0?.TryCast<Il2CppSystem.Collections.Generic.List<Il2CppAssets.Scripts.Simulation.Bloons.Bloon>>();
            var sb = new StringBuilder($"CB {__instance.Id.Id} {__1} {Recorder.F(__instance.pierce)}");
            int skip = 0;
            if (l == null && __0 != null)
            {
                // Enumerable.Skip's iterator: read its source list and count without enumerating it.
                IntPtr obj = ((Il2CppInterop.Runtime.InteropTypes.Il2CppObjectBase)(object)__0).Pointer;
                // (Not started, state -2: its arguments are in <>3__source @0x30 and <>3__count @0x3c,
                // from the CBFIELDS dump of the object.)
                IntPtr src = System.Runtime.InteropServices.Marshal.ReadIntPtr(obj + 0x30);
                if (src != IntPtr.Zero) l = new Il2CppSystem.Collections.Generic.List<Il2CppAssets.Scripts.Simulation.Bloons.Bloon>(src);
                skip = System.Runtime.InteropServices.Marshal.ReadInt32(obj + 0x3c);
                if (src != IntPtr.Zero) sb.Append(" src ").Append(System.Runtime.InteropServices.Marshal.PtrToStringAnsi(Il2CppInterop.Runtime.IL2CPP.il2cpp_class_get_name(Il2CppInterop.Runtime.IL2CPP.il2cpp_object_get_class(src))));
            }
            sb.Append(" skip ").Append(skip);
            if (l == null) sb.Append(" ?");
            else for (int k = 0; k < l.Count; k++) sb.Append(' ').Append(l[k] == null ? -1 : l[k].Id.Id);
            Recorder.Out.WriteLine(sb.ToString());
        }
        catch (Exception e) { try { Recorder.Out.WriteLine("CBERR " + e.GetType().Name + " " + e.Message.Replace((char)10, (char)32)); } catch (Exception) { } }
    }
}

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile), "Expire")]
static class ExpirePatch { static void Prefix(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile __instance) => EndLog.Write("expire", __instance); }

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile), "Exhausted")]
static class ExhaustedPatch { static void Prefix(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile __instance) => EndLog.Write("exhausted", __instance); }

[HarmonyPatch(typeof(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile), "OnDestroy")]
static class ProjDestroyPatch
{
    [System.Runtime.InteropServices.DllImport("kernel32.dll")]
    static extern ushort RtlCaptureStackBackTrace(uint skip, uint count, IntPtr[] trace, IntPtr hash);
    [System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
    static extern IntPtr GetModuleHandleW(string name);
    static void Prefix(Il2CppAssets.Scripts.Simulation.Towers.Projectiles.Projectile __instance)
    {
        EndLog.Write("destroy", __instance);
        // A projectile destroyed with pierce left before its lifespan: its native callers as
        // GameAssembly RVAs ("XS projectile rva ...").
        try
        {
            if (__instance == null || Recorder.Out == null || Recorder.Sc == null || Recorder.Recorded >= Recorder.Sc.frames) return;
            if (__instance.pierce <= 0 || __instance.Age > 3) return;
            long gbase = GetModuleHandleW("GameAssembly.dll").ToInt64();
            var sb = new StringBuilder($"XS {__instance.Id.Id}");
            // Return addresses on the stack (words pointing just after a call into GameAssembly code).
            unsafe
            {
                long* sp = stackalloc long[1];
                for (int i = 0; i < 4096; i++)
                {
                    long v = sp[i], a = v - gbase;
                    if (a < 0x1000 || a >= 0x3800000) continue;
                    byte* c = (byte*)v;
                    if (c[-5] == 0xE8 || c[-2] == 0xFF || c[-3] == 0xFF || c[-6] == 0xFF)
                        sb.Append(" 0x").Append((a + 0x180000000).ToString("x"));
                }
            }
            Recorder.Out.WriteLine(sb.ToString());
        }
        catch (Exception) { }
    }
}
