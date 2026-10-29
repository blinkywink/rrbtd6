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

- **Show Mods Button** in Advanced > UI is off by default and adds a Mods button on home.
- The native menu-style manager has an installed-mod list, selected-mod details and effect
  preview, explicit enable/disable, Open Mods Folder, reload, and validation/conflict messages.
- Data-only mod API 1 is implemented: versioned JSON manifests in `mods/<folder>/mod.json`,
  with a new-local-Sandbox hook for starting cash/lives. The included Practice Start example
  is disabled by default. See `mods/README.md` and `mods/mod.schema.json`.
- Manifests are read only on manager open/reload or new modded Sandbox creation; there are
  no per-frame callbacks. Hiding the button does not disable previously enabled mods.
- Normal games, events, co-op, account data and uploads are unchanged. Tower/projectile hooks,
  scripting and co-op mod negotiation remain future work.

## 8. RustTD6 branding

- The window title is **RustTD6**. The window and taskbar use the game's own icon, read from the
  player's BloonsTD6.exe during setup; the executable itself carries no icon (nothing of the
  game's art is shipped).
- The title screen shows the original BTD6 logo, from the player's own game files.
