# Intentional deviations from the original game

Everything not listed here should match the original BTD6 exactly. These are the only
places the clone is meant to differ, by the owner's request.

## 1. No purchases: support-Ninja-Kiwi popup

- Opening the in-game store (any store/shop purchase screen) shows the store as it looks in
  the original, but blurred and with nothing clickable behind it.
- Over it, a small popup reads exactly:

  > Please support ninja kiwi through their official client. They also have an official
  > webstore where you can purchase items.

- Buttons:
  - **take me to the webstore** — opens https://btd6store.ninjakiwi.com/ in the browser.
  - **return home** — closes the store and returns to the main menu.
- Anywhere else that lets you buy something with real money (e.g. buying Powers, Monkey
  Money packs, Trophy Store real-money items, Insta Monkey packs) shows the same popup when
  the purchase button is clicked, instead of buying.

## 2. FPS counter in the window title bar

- The "show FPS" setting shows the frame rate in the window's title bar (e.g.
  `RustTD6 - 144 FPS`), not drawn inside the game.

## 3. Advanced settings

- Advanced settings has three collapsible sections, closed whenever the screen opens:
  **Default** (original settings), **Graphics** (client rendering options), and **UI**
  (FPS display, menu visibility and startup options).
- The menu UI should be a bit nicer than a plain list (section headers, consistent spacing).
- Main menu toggles (all off by default), to clear up space:
  - **Hide Legends Button** — hides Legends on the main menu.
  - **Hide Shop Button** — hides the Shop button; the Trophy Store moves up into its place.
  - **Hide Race / Boss / Boss Rush / Odyssey / Contested Territory / Social Season Button** —
    one toggle per event button on the right of the main menu; the remaining buttons move up to
    close the gap.
- **Hide Double Cash / Fast Track Button** switches the respective client preference
  off before hiding its control; unhiding does not switch it back on.
- **Hide Perks Button** only hides the navigation button; it never disables perks.
- **Hide Friends' Map Records** hides map badges and the selected map's friends panel.
- The Bloons TV and its hit volume are removed from the home island at the user's request.
- Daily chest opening is a session-only preview: the native calendar uses cached server rewards
  and the profile's shifted daily clock, without crediting loot or writing a claim to an account.
- Imported official saves show their native markers and a confirmation before a new local game.
  They cannot be resumed or overwritten yet; the imported snapshot is retained.
- **Skip Startup Screen** (off by default) — go straight to the main menu instead of the Ninja
  Kiwi splash, loading screen and title screen.
- Planned settings (added once the screens they skip exist):
  - **Always Use Cloud Save** — skip the "use your local save or your cloud save?" question
    after pressing Play on the startup screen, and always load the newest cloud save.
  - **Mods Button** — implemented as Show Mods Button in UI settings (see 7).

## 4. No Legends DLC modes

- The two Legends DLC modes (Rogue Legends, Frontier Legends) are not included, and none of
  their assets are extracted.
- Clicking Legends on the main menu shows the main menu blurred and inert with a popup like the
  support one (1):

  > RustTD6 does not support the Legends DLCs.

  with a single **return home** button.

## 5. Account login (planned)

- Logging in to and out of a Ninja Kiwi account from the game's own UI, and the startup
  screen, as in the original. Not built yet.

## 6. Co-op codes and servers (planned)

- Co-op games hosted by this client use peer-to-peer (iroh) connections, with join codes that
  look like the official BTD6 codes.
- Entering a code first looks for an iroh game hosted by this client, then the official BTD6
  co-op servers, so it can play with both official and RustTD6 clients.
- Not built yet: connecting to the official co-op servers needs Ninja Kiwi's network protocol.

## 7. Mods

- Show Mods Button in Advanced > UI is off by default. The manager installs/selects/toggles
  file-authored mods and optional settings; it is not an in-game authoring editor.
- API 1 supports cloned towers/bloons/modes, weapon/projectile/stat/round data edits,
  changes to all existing modes, and load-time text/visibility edits on frontend nodes.
- Authors can use tools/modsdk.py (Python SDK) or emit the manifest from another language.
  No author scripts or injected native DLLs run in the game. See mods/README.md and /mods/
  on the website. Existing Practice Start is only an optional Sandbox example.
- Reload applies to new/restarted games without restarting the client. Already running
  entities retain their definitions. Unsupported arbitrary behavior hooks, new scripted
  widgets/profile counters, custom art imports and large co-op asset transfers are future work.
- Private iroh rooms have a room-specific host-selected mod list. Data manifests transfer
  P2P, are hash/size checked, stay in the room snapshot rather than installed folders, and
  are compiled on every client. Guests inspect but cannot change the list. Changes reset
  readiness. Room-provided custom modes appear in the host's mode dropdown.
- Client executable/base game identity must match. The load barrier also verifies shared
  initial state. Official-server interoperability remains unimplemented.
- Local content mods bypass official events; co-op uses room mods rather than individual
  local selections. The original Sandbox cash/lives hook remains Sandbox-only.

## 8. RustTD6 branding

- The window title is **RustTD6**. The window and taskbar use the game's own icon, read from the
  player's BloonsTD6.exe during setup; the executable itself carries no icon (nothing of the
  game's art is shipped).
- The title screen shows the original BTD6 logo, from the player's own game files.

## 9. User requirements: mod platform, co-op and updates (2026-10-28)

These are requested behavior, not a claim that everything below is implemented.

- Sandbox: click cash or hearts/lives to enter testing values. Zero lives must never open
  defeat; towers stop attacking and editing lives back above zero allows testing to resume.
- Mods are a whole-client feature, not Sandbox-only: new modes, changes to one/all existing
  modes, tower and projectile changes, custom towers/projectiles/bloons, UI/menu changes and
  statistics extensions. Authoring should be simpler than BTD6 DLL injection.
- Apply/reload supported mods without restarting the client. Compile definitions into existing
  runtime structures, with no injected DLLs or mandatory per-frame scripting overhead.
  Do not pretend data definitions implement arbitrary new simulation behavior.
- Add a website mod documentation page explaining installation, manifest/API, examples,
  reload lifecycle, game integration, supported hooks and limitations.
- Private iroh co-op: the host chooses a room-specific mod list in a dedicated menu.
  Show/toggle room mods; clients missing a selected version receive its data over P2P.
  Validate bounded content and hashes, isolate it from local installed mods, never auto-run
  native code, and require identical client/assets/mod/settings identities before starting.
  Changing room mods invalidates readiness. Official-server rooms must not use this path.
- Continue fleshing out playable co-op and the in-BTD6 setup/extractor DLL workflow.
- On launch, check GitHub for a newer client build and install compatible updates.
  If an update requires newly extracted assets, offer to keep playing the installed version
  or rerun the setup DLL inside BTD6 to extract the required data. Preserve the older working
  build/assets until the new extraction and update are complete; support offline fallback.
- Co-op is only between identical client versions. Make incompatible builds/assets clear
  in the room UI rather than allowing a desynchronized game.

- Authoring clarification: mods are authored as code/data files against a documented API.
  Do not build a cluttered in-game mod/tower editor. The in-game UI only manages installed
  mods, optional author-defined settings, and host-selected co-op mods.
