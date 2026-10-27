# rusttd6.com

Static landing page for the fan-made RustTD6 project. No framework, dependencies,
build step, analytics, proprietary game assets, or account integration.
The tropical background is original SVG, not extracted BTD6 artwork.
Lilita One is bundled under the SIL Open Font License (see public/fonts/OFL.txt).

## Vercel

Import `blinkywink/rrbtd6` into Vercel and set **Root Directory: `website`**.
The checked-in `vercel.json` selects Other / no framework, no install/build
commands, and serves **`public`**. Add `rusttd6.com` in the project's Domains
settings, then use the DNS records Vercel provides for that particular domain.
This checkout does not configure your Vercel account or change your DNS.

Preview locally from the repository root:

```powershell
python -m http.server 4173 --bind 127.0.0.1 --directory website/public
```

## Content

The primary installer URL uses GitHub's `/releases/latest/download/` route;
it is not pinned to a release. JavaScript only enhances the release label and
displays the client download size. Download links and the native FAQ work without JS.
If GitHub's API is unavailable or rate limited, the stable link still works.

Installation follows the root README and the current launcher/installer.
Clean-install time and disk requirements have not been measured reliably;
the page says they are being measured rather than presenting an estimate as a requirement. Never
present live FPS numbers, percentage speedups, or completed mod compatibility
without supporting measurements/implementation.
