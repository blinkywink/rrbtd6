# rusttd6.com

Static landing page for the fan-made RustTD6 project. No framework, dependencies,
build step, analytics, proprietary game assets, or account integration.
The illustration is original SVG, not extracted BTD6 artwork.

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
pauses decorative motion. Download links and the native FAQ work without JS.
If GitHub's API is unavailable or rate limited, the stable link still works.

Installation follows the root README and the current launcher/installer.
The hour-or-more time and 35 GB free-space figures are conservative planning
guidance, not measured setup guarantees. They exclude the user's BTD6 install.
Update those figures after a representative clean-install benchmark. Never
present live FPS numbers, percentage speedups, or completed mod compatibility
without supporting measurements/implementation.
