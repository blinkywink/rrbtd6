# rusttd6.com

Static home page (`public/index.html`, `public/styles.css`) and a separate mod API docs site
(`public/docs/`, its own `docs.css`), with self-hosted Luckiest Guy (headings), Inter (text) and Comic Neue (the wordmark, where Comic Sans
isn't installed).
Almost no JavaScript: `download.js` scrolls the home page to the install steps after a Download
click, and `docs/nav.js` keeps the docs sidebar's scroll between pages (the CSP allows same-origin
scripts only). Menus are CSS-only `<details>`. No
analytics and no game assets.

The docs pages are generated from `build_docs.py` and committed as plain HTML; after editing a
page there, run `python website/build_docs.py`. `/mods/` redirects to `/docs/`; the example
downloads stay under `public/mods/`. Font licenses: `public/fonts/LuckiestGuy-LICENSE.txt` (Apache 2.0),
`public/fonts/Inter-OFL.txt` and `public/fonts/ComicNeue-OFL.txt` (SIL OFL).

## Progress section

The home page's Progress squares come from `progress.json` (towers: recorded scenarios that match the
real game / recorded; other areas: rough estimates out of 10). Edit the numbers, then run
`python website/build_progress.py`; it rewrites the part of `public/index.html` between the
`progress:start` and `progress:end` markers.

## Images

`public/social.png` is the link preview, from `social-card.html`; `sh tools/site_social_card.sh` regenerates it.
No game assets go on the site or in the repository.

## Vercel

Import `blinkywink/rrbtd6` and set Root Directory to `website`. `vercel.json` serves `public`
as-is. Add `rusttd6.com` under Settings → Domains and copy the DNS records Vercel shows.

## Preview

From the repository root:

```powershell
python -m http.server 4173 --bind 127.0.0.1 --directory website/public
```
