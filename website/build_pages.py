"""Assemble the site GitHub Pages serves: public/ as-is, with the security policy added to every
page as <meta> tags (Pages can't send custom headers; frame-ancestors only works as a header).

Usage: python website/build_pages.py OUT_DIR
"""
import pathlib
import shutil
import sys

POLICY = ("default-src 'self'; img-src 'self' data:; script-src 'self'; "
          "connect-src 'self' https://api.github.com; style-src 'self' 'unsafe-inline'; "
          "base-uri 'self'; form-action 'none'")
CHARSET = '<meta charset="utf-8">'
TAGS = (f'\n<meta http-equiv="Content-Security-Policy" content="{POLICY}">'
        '\n<meta name="referrer" content="strict-origin-when-cross-origin">')

public = pathlib.Path(__file__).resolve().parent / "public"
out = pathlib.Path(sys.argv[1]).resolve()
if out == public or public in out.parents:
    sys.exit("OUT_DIR must be outside website/public")
shutil.rmtree(out, ignore_errors=True)
shutil.copytree(public, out)

pages = sorted(out.rglob("*.html"))
for page in pages:
    html = page.read_text(encoding="utf-8")
    if CHARSET not in html:
        sys.exit(f"{page.relative_to(out)}: no {CHARSET} to put the policy after")
    page.write_text(html.replace(CHARSET, CHARSET + TAGS, 1), encoding="utf-8", newline="")
print(f"built {out} ({len(pages)} pages)")
