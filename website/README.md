# rusttd6.com

Static home and mod documentation pages: `public/index.html` and `public/styles.css`, with self-hosted
Luckiest Guy (headings) and Inter (text). Mod docs live at `public/mods/index.html`, with a downloadable SDK example.
No build step, no JavaScript, no analytics and no
game assets. Font licenses: `public/fonts/LuckiestGuy-LICENSE.txt` (Apache 2.0) and
`public/fonts/Inter-OFL.txt` (SIL OFL).

## Vercel

Import `blinkywink/rrbtd6` and set Root Directory to `website`. `vercel.json` serves `public`
as-is. Add `rusttd6.com` under Settings → Domains and copy the DNS records Vercel shows.

## Preview

From the repository root:

```powershell
python -m http.server 4173 --bind 127.0.0.1 --directory website/public
```
