# Installer mod

`RRBTD6Installer.dll`, the one-time setup players download. It's a MelonLoader mod for BTD6 that
installs RustTD6 into a folder they choose, then hands over to the
[launcher](../launcher/README.md), which downloads the game and extracts the data from their BTD6
install.

| File | What it does |
|---|---|
| `RRBTD6Installer.cs` | The mod: offers the install and starts the launcher |
| `SetupDialog.cs` | The welcome dialog and desktop shortcut option |
| `SetupProgress.cs` | The progress window, with stages, time left and tips |

It only talks to GitHub, and it doesn't change BTD6, the player's save or their account. CI builds it
in [`release.yml`](../.github/workflows/release.yml).
