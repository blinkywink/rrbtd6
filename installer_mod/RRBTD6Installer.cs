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

    static string StatePath => Path.Combine(MelonEnvironment.UserDataDirectory, "rrbtd6-installer.json");

    public override void OnSceneWasInitialized(int buildIndex, string sceneName)
    {
        if (asked || !sceneName.StartsWith("MainMenu", StringComparison.OrdinalIgnoreCase)) return;
        asked = true;
        try { Prompt(); }
        catch (Exception e) { LoggerInstance.Error(e); Native.Message("rrbtd6 installer failed:\n" + e.Message, Native.MB_ICONERROR); }
    }

    void Prompt()
    {
        var state = LoadState();
        string installed = state.TryGetProperty("installDir", out var dir) ? dir.GetString() : null;
        if (installed != null && File.Exists(Path.Combine(installed, LauncherName)))
        {
            if (state.TryGetProperty("askOnStart", out var ask) && !ask.GetBoolean()) return;
            int choice = Native.Message($"rrbtd6 is installed in\n{installed}\n\nStart it now? (It updates itself first.)\n\n" +
                                        "Cancel: don't ask again at startup.", Native.MB_YESNOCANCEL | Native.MB_ICONQUESTION);
            if (choice == Native.IDYES) Launch(installed);
            if (choice == Native.IDCANCEL) SaveState(installed, false);
            return;
        }
        if (Native.Message("Install rrbtd6?\n\nrrbtd6 is a separate, fan-made Rust remake of BTD6. It is built from GitHub and " +
                           "uses art and data extracted from THIS BTD6 install on your PC (nothing is downloaded from or " +
                           "uploaded to Ninja Kiwi). Storage depends on the extracted content; the first setup takes a while.\n\n" +
                           "Choose an install folder next.", Native.MB_YESNO | Native.MB_ICONQUESTION) != Native.IDYES)
            return;
        string folder = Native.PickFolder("Choose a folder to install rrbtd6 into");
        if (folder == null) return;
        folder = Path.Combine(folder, "rrbtd6");
        Directory.CreateDirectory(folder);
        File.WriteAllText(Path.Combine(folder, "rrbtd6.json"),
            JsonSerializer.Serialize(new { game = MelonEnvironment.GameRootDirectory }));
        Task.Run(async () =>
        {
            try
            {
                await DownloadLauncher(folder);
                SaveState(folder, true);
                Shortcut(folder);
                Launch(folder);
            }
            catch (Exception e)
            {
                LoggerInstance.Error(e);
                Native.Message("Could not download rrbtd6 from GitHub:\n" + e.Message, Native.MB_ICONERROR);
            }
        });
    }

    static async Task DownloadLauncher(string folder)
    {
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

    static void Launch(string folder) =>
        Process.Start(new ProcessStartInfo(Path.Combine(folder, LauncherName), "--refresh-assets") { WorkingDirectory = folder, UseShellExecute = true });

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

    public static int Message(string text, uint type) => MessageBoxW(GetActiveWindow(), text, "rrbtd6", type | MB_TOPMOST);

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
