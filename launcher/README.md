# Setup

The launcher is also the installer. Releases ship it twice: `rrbtd6-launcher.exe` (what installs
update) and `RustTD6Setup.exe` (what players download). Run from anywhere that isn't an install, it
copies itself to `%LOCALAPPDATA%\RustTD6` (or a folder the player picks if that one can't be used),
adds desktop and Start menu shortcuts and carries on from there. It finds BTD6 itself (Steam on any
drive, Epic, default folders; "Find BTD6 myself" if not), so neither MelonLoader nor a running BTD6
is needed: extraction reads the game's own files (`tools/il2cpp/type_dump.py`).

It's a window program (`ui.rs`): a small setup window shows the stage, a bar, the time left and
tips while it installs, updates or extracts, and an up-to-date install starts the game with no
window. Everything is also written to `setup.log` in the install folder (and the console, when
started from one).

When something fails it fixes what it can first: a retry (finished extraction stages are kept), then
a retry after a repair (fresh tools and Python packages). Downloads retry three times. Extraction
counts as stuck after 3 minutes without output or 30 minutes without its progress line changing, and
is stopped and retried. If it still can't finish, the window says what happened in plain words
(no BTD6, disk space, memory, network, antivirus, an unsupported BTD6 build) with Try again, Open setup
log and, when BTD6 wasn't found, Find BTD6 myself. Closing the window mid-setup asks first; finished
stages are kept for next time. An update that needs new game data asks "Update now" or "Play now,
update later". `--unattended` (tests) turns off the window, retries and questions.

# Launcher updates

The launcher downloads a complete release into `update/payload`, validates the
required game and extraction entry points, and then replaces the installed tools,
game, optional launcher and version file. It records the previous files before
moving them; `version.json` moves last. Download or unpack failures leave the
installed build in place.

A journal restores interrupted swaps at the next start, including before
`--offline` startup. A complete commit marker keeps the new release and allows
leftover backups to be removed. Windows may retain the previous running launcher
image until it exits; the next start finishes that cleanup. Failed recovery keeps
the journal and prevents starting a mixed installation until recovery succeeds.
This covers process interruption and filesystem errors, not guaranteed recovery
from disk failure or power loss.

One OS file lock covers update and extraction. Crashes release the lock, so a
stale lock filename does not block future launches. Settings, extracted assets,
Python and `rrbtd6.json` are outside the release replacement set.

HTTP connections are reused, with a 10-second connection timeout and 60-second
read/write timeouts. Existing installs can still start when an update check fails,
provided no installation recovery is pending. `--offline` skips the GitHub update
check; missing Python dependencies or missing/outdated data still require setup.

Run `cargo test --profile game -p rrbtd6-launcher`. Tests cover every interrupted
swap boundary, a first install, incomplete downloads, commit-marker interruption,
Windows file locks, retried rollback, exclusive launcher ownership and replacement
of an actual running executable image. They use temporary directories and make no
network calls. The ignored subprocess fixture is invoked by the running-image test.

First-time Python setup retries a partially unpacked runtime and bootstraps pip
even if Python was unpacked before an interruption. Source-only dependencies
build with explicitly installed setuptools/wheel: pip's temporary build isolation
cannot work with the embedded runtime's `._pth` import rules. The requirements
stamp is written only after all dependencies install successfully.

Local verification (2026-10-26): started with embedded Python 3.13.1 unpacked but
no pip, ran the launcher with `--offline`, installed the full shipped requirements
and imported UnityPy/Pillow/NumPy. The extraction test stub deliberately exited
73 to prevent game launch; this was a bootstrap test, not a full extraction test.
A second start reused the dependencies without downloads. The embedded runtime
also exported a real map and terrain from the local game successfully.

`python -I tools/tests/test_launcher_bootstrap.py <built-launcher.exe>` runs the
embedded-runtime recovery/reuse check with a synthetic game fixture. It needs
Windows and downloads Python/PyPI packages. The release workflow runs it after
building; no proprietary game files, user account or game service are used.
