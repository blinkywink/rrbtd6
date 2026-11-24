// rrbtd6 installer: a MelonLoader mod for BTD6 that installs rrbtd6 (a separate game) into a folder
// you choose, then hands over to rrbtd6-launcher.exe, which downloads the game from GitHub
// releases, extracts the game data from this BTD6 install and keeps everything up to date.
//
// It only talks to GitHub. It does not change BTD6, your save or your account.
using System;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Threading.Tasks;
using MelonLoader;
using MelonLoader.Utils;

[assembly: MelonInfo(typeof(RRBTD6Installer.Installer), "rrbtd6 Installer", "1.0.0", "blinkywink", "https://github.com/blinkywink/rrbtd6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6-Epic")]

namespace RRBTD6Installer;

public class Installer : MelonMod
{
    const string Repo = "blinkywink/rrbtd6";
    const string LauncherName = "rrbtd6-launcher.exe";
    bool asked;

    static string PreviewDirectory => Path.Combine(MelonEnvironment.GameRootDirectory, "Mods", "RustTD6SetupPreview");
    static bool HasPreview => File.Exists(Path.Combine(PreviewDirectory, "preview.json"));

    static string StatePath => Path.Combine(MelonEnvironment.UserDataDirectory, "rrbtd6-installer.json");

    public override void OnSceneWasInitialized(int buildIndex, string sceneName)
    {
        if (asked || !sceneName.StartsWith("MainMenu", StringComparison.OrdinalIgnoreCase)) return;
        asked = true;
        try { Prompt(); }
        catch (Exception e) { LoggerInstance.Error(e); Native.Error("The installer ran into a problem", e.Message); }
    }

    void Prompt()
    {
        var state = LoadState();
        string installed = state.TryGetProperty("installDir", out var dir) ? dir.GetString() : null;
        if (!HasPreview && installed != null && File.Exists(Path.Combine(installed, LauncherName)))
        {
            if (state.TryGetProperty("askOnStart", out var ask) && !ask.GetBoolean()) return;
            int choice = Native.Ask(DialogKind.Info, "RustTD6", "Start RustTD6?",
                $"RustTD6 is installed in {installed}. It checks for updates before it starts.", null,
                new[] { "Start", "Not now", "Don't ask again" }, 1);
            if (choice == 0) { SeedPlayerName(installed); Launch(installed); }
            if (choice == 2) SaveState(installed, false);
            return;
        }
        if (!Native.InstallPrompt(out bool shortcut)) return;
        // The recommended folder first: always writable for this user and not synced to OneDrive.
        int where = Native.Ask(DialogKind.Info, "RustTD6 Setup", "Where should RustTD6 go?",
            "Recommended: " + Recommended + "\n\nThis folder belongs to your Windows account, is always writable and isn't synced to OneDrive.", null,
            new[] { "Install there", "Choose another folder", "Cancel" }, 2);
        if (where != 0 && where != 1) return;
        string folder = Recommended;
        if (where == 1)
        {
            string picked = Native.PickFolder("Choose a folder to install rrbtd6 into");
            if (picked == null) return;
            folder = Path.Combine(picked, "rrbtd6");
        }
        string problem = FolderProblem(folder, out string warning);
        if (problem != null) { Native.Error("RustTD6 can't be installed there", problem); return; }
        if (warning != null && Native.Ask(DialogKind.Warning, "RustTD6 Setup", "Install here anyway?", warning, null,
                new[] { "Install here", "Cancel" }, 1) != 0) return;
        if (HasPreview && (File.Exists(Path.Combine(folder, "local_extracted", "game", "game.json")) || File.Exists(Path.Combine(folder, "rrbtd6.exe")))) {
            Native.Ask(DialogKind.Warning, "RustTD6 Setup", "Please choose a new, empty folder",
                "This is a setup test. An existing installation would reuse prepared assets and would not measure first-time extraction.", null,
                new[] { "OK" }, 0);
            return;
        }
        Directory.CreateDirectory(folder);
        File.WriteAllText(Path.Combine(folder, "rrbtd6.json"),
            JsonSerializer.Serialize(new { game = MelonEnvironment.GameRootDirectory }));
        SeedPlayerName(folder);
        Task.Run(async () =>
        {
            try
            {
                await DownloadLauncher(folder);
                SaveState(folder, true);
                if (shortcut) Shortcut(folder);
                Launch(folder);
            }
            catch (Exception e)
            {
                LoggerInstance.Error(e);
                Native.Error("Could not download RustTD6 from GitHub", e.Message);
            }
        });
    }

    static string Recommended => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "RustTD6");

    /// Why RustTD6 can't be installed in this folder (null: it can), and what is worth a warning.
    static string FolderProblem(string folder, out string warning)
    {
        warning = null;
        string full;
        try { full = Path.GetFullPath(folder); }
        catch (Exception e) { return "That folder path isn't valid: " + e.Message; }
        // Windows limits file paths to 260 characters, and the game data goes about 160 deep.
        if (full.Length > 95)
            return "The folder path is " + full.Length + " characters long. Windows limits file paths to 260 characters and the game's data goes about 160 characters deep, so choose a folder with a shorter path, for example " + Recommended + ".";
        foreach (var special in new[] { Environment.SpecialFolder.ProgramFiles, Environment.SpecialFolder.ProgramFilesX86, Environment.SpecialFolder.Windows })
        {
            string root = Environment.GetFolderPath(special);
            if (!string.IsNullOrEmpty(root) && (full.Equals(root, StringComparison.OrdinalIgnoreCase) || full.StartsWith(root.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase)))
                return "Windows only lets administrators change files inside " + root + ", so RustTD6 couldn't save its data or settings there. Choose another folder, for example " + Recommended + ".";
        }
        try
        {
            Directory.CreateDirectory(full);
            string probe = Path.Combine(full, ".rrbtd6-write-test");
            File.WriteAllText(probe, "ok");
            File.Delete(probe);
        }
        catch (Exception e) { return "RustTD6 can't write to that folder (" + e.Message + "). Choose another folder, for example " + Recommended + "."; }
        try
        {
            long free = new DriveInfo(Path.GetPathRoot(full)).AvailableFreeSpace;
            const long need = 15L * 1024 * 1024 * 1024;
            if (free < need)
                return "Setup needs about 15 GB free on " + Path.GetPathRoot(full) + " (the game data is about 7 GB, plus working space while it is prepared), and " + (free / 1073741824.0).ToString("0.0") + " GB are free. Free up some space, or choose a folder on another drive.";
        }
        catch (Exception) { /* a network drive may not say; the launcher checks again */ }
        foreach (var name in new[] { "OneDrive", "OneDriveConsumer", "OneDriveCommercial" })
        {
            string synced = Environment.GetEnvironmentVariable(name);
            if (!string.IsNullOrEmpty(synced) && full.StartsWith(synced.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase))
            {
                warning = "This folder is synced by OneDrive. RustTD6's game data is several GB, which OneDrive would upload, and files it keeps only online make the game slow to load. " + Recommended + " isn't synced.";
                break;
            }
        }
        return null;
    }

    // Reads only the currently loaded display name. No account IDs, sessions or save data.
    static void SeedPlayerName(string folder)
    {
        try
        {
            const System.Reflection.BindingFlags flags = System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static | System.Reflection.BindingFlags.Instance;
            Type gameType = null;
            foreach (var assembly in AppDomain.CurrentDomain.GetAssemblies())
            {
                gameType = assembly.GetType("Il2CppAssets.Scripts.Unity.Game", false);
                if (gameType != null) break;
            }
            if (gameType == null) return;
            object game = gameType.GetProperty("instance", flags)?.GetValue(null) ?? gameType.GetField("instance", flags)?.GetValue(null);
            object player = game == null ? null : gameType.GetProperty("Player", flags)?.GetValue(game);
            string name = player?.GetType().GetProperty("SafeDisplayName", flags)?.GetValue(player) as string;
            if (string.IsNullOrWhiteSpace(name)) return;
            name = name.Trim();
            if (name.Length > 24 || System.Linq.Enumerable.Any(name, char.IsControl)) return;
            string data = Path.Combine(folder, "local_extracted");
            Directory.CreateDirectory(data);
            string seed = Path.Combine(data, "player-name.json");
            // Never overwrite a name the player has already chosen in RustTD6.
            if (!File.Exists(Path.Combine(data, "local-profile.json")) && !File.Exists(seed))
                File.WriteAllText(seed, JsonSerializer.Serialize(new { name }));
        }
        catch { /* A missing name never prevents installation. */ }
    }

    static async Task DownloadLauncher(string folder)
    {
        if (HasPreview) {
            using var manifest = JsonDocument.Parse(await File.ReadAllTextAsync(Path.Combine(PreviewDirectory, "preview.json")));
            foreach (string name in new[] { LauncherName, "rrbtd6.exe", "tools.zip" }) {
                string source = Path.Combine(PreviewDirectory, name);
                byte[] bytes = await File.ReadAllBytesAsync(source);
                string actual = Convert.ToHexString(SHA256.HashData(bytes));
                string expected = manifest.RootElement.GetProperty("sha256").GetProperty(name).GetString();
                if (!actual.Equals(expected, StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("Setup test file is incomplete: " + name);
            }
            File.Copy(Path.Combine(PreviewDirectory, LauncherName), Path.Combine(folder, LauncherName), true);
            File.Copy(Path.Combine(PreviewDirectory, "rrbtd6.exe"), Path.Combine(folder, "rrbtd6.exe"), true);
            ZipFile.ExtractToDirectory(Path.Combine(PreviewDirectory, "tools.zip"), folder);
            await File.WriteAllTextAsync(Path.Combine(folder, "version.json"), JsonSerializer.Serialize(new {
                build = 0, preview = true, assets_revision = manifest.RootElement.GetProperty("assets_revision").GetString()
            }));
            return;
        }
        using var http = new HttpClient();
        http.DefaultRequestHeaders.UserAgent.ParseAdd("rrbtd6-installer");
        using var latest = JsonDocument.Parse(await http.GetStringAsync($"https://api.github.com/repos/{Repo}/releases/latest"));
        foreach (var asset in latest.RootElement.GetProperty("assets").EnumerateArray())
        {
            if (asset.GetProperty("name").GetString() != LauncherName) continue;
            var bytes = await http.GetByteArrayAsync(asset.GetProperty("browser_download_url").GetString());
            await File.WriteAllBytesAsync(Path.Combine(folder, LauncherName), bytes);
            return;
        }
        throw new InvalidOperationException("the latest release has no " + LauncherName);
    }

    void Launch(string folder)
    {
        LoggerInstance.Msg("RustTD6 setup is starting. Keep BTD6 open and follow progress in this MelonLoader terminal.");
        var process = new Process { StartInfo = new ProcessStartInfo(Path.Combine(folder, LauncherName), "--refresh-assets --from-installer" + (HasPreview ? " --offline" : "")) {
            WorkingDirectory = folder, UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardOutput = true, RedirectStandardError = true, RedirectStandardInput = true,
            StandardOutputEncoding = System.Text.Encoding.UTF8, StandardErrorEncoding = System.Text.Encoding.UTF8
        }};
        // A progress window, so players don't need the terminal; the terminal still gets every line.
        SetupDialog.Progress window = null;
        // The launcher's last lines, for the dialog if setup stops.
        var lastLines = new System.Collections.Concurrent.ConcurrentQueue<string>();
        void Remember(string line) { lastLines.Enqueue(line); while (lastLines.Count > 6) lastLines.TryDequeue(out _); }
        try { window = new SetupDialog.Progress(); } catch (Exception e) { LoggerInstance.Warning("Progress window unavailable: " + e.Message); }
        process.OutputDataReceived += (_, e) => {
            if (string.IsNullOrEmpty(e.Data)) return;
            LoggerInstance.Msg(e.Data);
            Remember(e.Data);
            window?.Report(e.Data);
            if (e.Data.StartsWith("Starting rrbtd6")) window?.Close();
        };
        process.ErrorDataReceived += (_, e) => { if (!string.IsNullOrEmpty(e.Data)) { LoggerInstance.Warning(e.Data); Remember(e.Data); } };
        if (!process.Start()) { process.Dispose(); throw new InvalidOperationException("Could not start the RustTD6 launcher."); }
        process.StandardInput.Close();
        process.BeginOutputReadLine();process.BeginErrorReadLine();
        Task.Run(async () => {
            try {
                await process.WaitForExitAsync();
                window?.Close();
                if (process.ExitCode != 0) {
                    LoggerInstance.Error("RustTD6 setup stopped. Completed extraction stages are saved; retry to continue.");
                    string detail = string.Join("\n", lastLines);
                    Native.Error("Setup could not finish", "Completed stages are saved, so you can retry without starting over.",
                                 detail.Length > 0 ? "Last messages:\n" + detail : "See the last error in the MelonLoader terminal.");
                }
            } catch (Exception e) { LoggerInstance.Error(e); }
            finally { process.Dispose(); }
        });
    }

    /// Desktop shortcut "rrbtd6" to the launcher (WScript.Shell through PowerShell; no extra references).
    static void Shortcut(string folder)
    {
        string desktop = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
        string link = Path.Combine(desktop, "rrbtd6.lnk").Replace("'", "''");
        string target = Path.Combine(folder, LauncherName).Replace("'", "''");
        string dir = folder.Replace("'", "''");
        string script = $"$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{link}');$s.TargetPath='{target}';$s.WorkingDirectory='{dir}';$s.Save()";
        try
        {
            Process.Start(new ProcessStartInfo("powershell.exe", $"-NoProfile -NonInteractive -Command \"{script}\"")
                { CreateNoWindow = true, UseShellExecute = false })?.WaitForExit(15000);
        }
        catch { /* the shortcut is a convenience */ }
    }

    static JsonElement LoadState()
    {
        try { return JsonDocument.Parse(File.ReadAllText(StatePath)).RootElement; }
        catch { return JsonDocument.Parse("{}").RootElement; }
    }

    static void SaveState(string folder, bool askOnStart) =>
        File.WriteAllText(StatePath, JsonSerializer.Serialize(new { installDir = folder, askOnStart }));
}

static class Native
{
    public const uint MB_YESNO = 0x4, MB_YESNOCANCEL = 0x3, MB_ICONQUESTION = 0x20, MB_ICONERROR = 0x10, MB_TOPMOST = 0x40000;
    public const int IDYES = 6, IDCANCEL = 2;

    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int MessageBoxW(IntPtr hWnd, string text, string caption, uint type);
    [DllImport("user32.dll")] static extern IntPtr GetActiveWindow();

    public static int Message(string text, uint type) => MessageBoxW(GetActiveWindow(), text, "RustTD6", type | MB_TOPMOST);

    /// The installer's dialog (SetupDialog); a plain MessageBox if it cannot be shown.
    /// Returns the clicked button's index, `cancel` for Esc / close.
    public static int Ask(DialogKind kind, string title, string heading, string body, string note, string[] buttons, int cancel)
    {
        try { return SetupDialog.Show(title, heading, body, note, kind, buttons, cancel); }
        catch (Exception)
        {
            string text = heading + "\n\n" + body + (note != null ? "\n\n" + note : "");
            if (buttons.Length == 1) { Message(text, kind == DialogKind.Error ? MB_ICONERROR : MB_ICONQUESTION); return 0; }
            if (buttons.Length == 2) return Message(text + "\n\n" + buttons[0] + "?", MB_YESNO | MB_ICONQUESTION) == IDYES ? 0 : cancel;
            int choice = Message(text + "\n\nYes: " + buttons[0] + "\nNo: " + buttons[1] + "\nCancel: " + buttons[2], MB_YESNOCANCEL | MB_ICONQUESTION);
            return choice == IDYES ? 0 : choice == IDCANCEL ? 2 : 1;
        }
    }

    public static void Error(string heading, string body, string note = null) =>
        Ask(DialogKind.Error, "RustTD6", heading, body, note, new[] { "OK" }, 0);

    public static bool InstallPrompt(out bool shortcut)
    {
        const string heading = "Welcome to RustTD6";
        const string body = "Choose a folder to install RustTD6 into. Setup then prepares the game's assets from your BTD6 install, which can take a while. Keep BTD6 open during setup.";
        const string note = "Setup uses a lot of CPU, memory and disk while it runs. It goes faster with more free memory, so close programs you aren't using, then let it finish.";
        shortcut = true;
        try { return SetupDialog.Show("RustTD6 Setup", heading, body, note, DialogKind.Warning, new[] { "Choose folder", "Not now" }, 1, "Create a desktop shortcut", ref shortcut) == 0; }
        catch (Exception) { return Message(heading + "\n\n" + body + "\n\n" + note + "\n\nChoose a folder?", MB_YESNO | MB_ICONQUESTION) == IDYES; }
    }

    /// The Windows folder picker (IFileOpenDialog with FOS_PICKFOLDERS). Null when cancelled.
    public static string PickFolder(string title)
    {
        var dialog = (IFileOpenDialog)new FileOpenDialogRCW();
        try
        {
            dialog.GetOptions(out uint options);
            dialog.SetOptions(options | 0x20 /*FOS_PICKFOLDERS*/ | 0x40 /*FOS_FORCEFILESYSTEM*/);
            dialog.SetTitle(title);
            if (dialog.Show(GetActiveWindow()) != 0) return null;
            dialog.GetResult(out IShellItem item);
            item.GetDisplayName(0x80058000 /*SIGDN_FILESYSPATH*/, out IntPtr path);
            try { return Marshal.PtrToStringUni(path); }
            finally { Marshal.FreeCoTaskMem(path); }
        }
        finally { Marshal.ReleaseComObject(dialog); }
    }

    [ComImport, Guid("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7")] class FileOpenDialogRCW { }

    [ComImport, Guid("42f85136-db7e-439c-85f1-e4075d135fc8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IFileOpenDialog
    {
        [PreserveSig] int Show(IntPtr parent);
        void SetFileTypes(uint cFileTypes, IntPtr rgFilterSpec);
        void SetFileTypeIndex(uint iFileType);
        void GetFileTypeIndex(out uint piFileType);
        void Advise(IntPtr pfde, out uint pdwCookie);
        void Unadvise(uint dwCookie);
        void SetOptions(uint fos);
        void GetOptions(out uint pfos);
        void SetDefaultFolder(IShellItem psi);
        void SetFolder(IShellItem psi);
        void GetFolder(out IShellItem ppsi);
        void GetCurrentSelection(out IShellItem ppsi);
        void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string pszName);
        void GetFileName([MarshalAs(UnmanagedType.LPWStr)] out string pszName);
        void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string pszTitle);
        void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string pszText);
        void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string pszLabel);
        void GetResult(out IShellItem ppsi);
    }

    [ComImport, Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IShellItem
    {
        void BindToHandler(IntPtr pbc, ref Guid bhid, ref Guid riid, out IntPtr ppv);
        void GetParent(out IShellItem ppsi);
        void GetDisplayName(uint sigdnName, out IntPtr ppszName);
        void GetAttributes(uint sfgaoMask, out uint psfgaoAttribs);
        void Compare(IShellItem psi, uint hint, out int piOrder);
    }
}
