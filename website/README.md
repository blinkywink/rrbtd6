# rusttd6.com

Buildless static landing page. Uses the project logo from assets/brand/title_logo.png,
a charcoal/purple palette, and self-hosted Luckiest Guy for headings and Inter
for small text. Font licenses: public/fonts/LuckiestGuy-LICENSE.txt (Apache 2.0)
and public/fonts/Inter-OFL.txt (SIL OFL). No JavaScript, analytics, or extracted game assets.

## Vercel

Import blinkywink/rrbtd6. Set Root Directory to `website`.
The checked-in vercel.json serves `public` without a build or install step.
Connect rusttd6.com using the records Vercel provides. This checkout does not change DNS.

## Preview

From the repository root:

```powershell
python -m http.server 4173 --bind 127.0.0.1 --directory website/public
```

The download uses GitHub's stable latest-release installer link. Setup instructions
are in a native disclosure, collapsed by default. No API or JavaScript is needed.
Clean-install storage and time remain unmeasured; do not advertise a fixed requirement.
