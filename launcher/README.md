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
