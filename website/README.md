# rusttd6.com

Static home page (`public/index.html`, `public/styles.css`) and a separate mod API docs site
(`public/docs/`, its own `docs.css`), with self-hosted Luckiest Guy (headings), Inter (text) and Comic Neue (the wordmark, where Comic Sans
isn't installed).
Little JavaScript, none of it needed: `download.js` scrolls the home page to the install steps
after a Download click, points the main button at the build instructions on a Mac or Linux, and
shows the latest build under the button (from GitHub's API, the only cross-origin request; the
CSP's `connect-src` allows just that); `site.js` marks the header link for the section being read,
counts up Progress's totals as they scroll into view and copies the mod example; and `docs/nav.js` keeps the docs sidebar's scroll between pages (the CSP allows same-origin
scripts only). Menus are CSS-only `<details>`. No
analytics, and no game art apart from the hero's silhouettes (also cut up as decorations) and the cursor's shape (see Images).

The docs pages are generated from `build_docs.py` and committed as plain HTML; after editing a
page there, run `python website/build_docs.py`. `/mods/` redirects to `/docs/`; the example
downloads stay under `public/mods/`. Font licenses: `public/fonts/LuckiestGuy-LICENSE.txt` (Apache 2.0),
`public/fonts/Inter-OFL.txt` and `public/fonts/ComicNeue-OFL.txt` (SIL OFL).

## Progress section

The home page's Progress bars (and the per-scenario squares behind "Show every recorded scenario") come from `progress.json` (towers: recorded scenarios that match the
real game / recorded; other areas: rough estimates out of 10). Edit the numbers, then run
`python website/build_progress.py`; it rewrites the part of `public/index.html` between the
`progress:start` and `progress:end` markers.

## Images

`public/social.png` is the link preview, from `social-card.html`; `sh tools/site_social_card.sh` regenerates it.
The hero's background, `public/hero/` (sky, drifting clouds and the island as separate layers), is
the client's main-menu island in flat blue silhouettes; `build_hero_silhouette.py` renders it (its
header has the command). The clouds beside the sections and the footer's island,
`public/decor/`, are single shapes cut from the same captures by `build_decor.py` (the island's
hills are drawn there, with those palms on them).
The cursor, `public/cursor/hand*.png`, is BTD6's `HandCursor` and `HandCursorDown` (from
`local_extracted/ui/`) traced as a smooth white hand with a navy outline by `build_cursor.py`
(its header has the command).

## Vercel

Import `blinkywink/rrbtd6` and set Root Directory to `website`. `vercel.json` serves `public`
as-is. Add `rusttd6.com` under Settings → Domains and copy the DNS records Vercel shows.

## Preview

From the repository root:

```powershell
python -m http.server 4173 --bind 127.0.0.1 --directory website/public
```
