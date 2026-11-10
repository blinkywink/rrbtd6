"""Local copy of the website where any text can be clicked and rewritten.

    python tools/site_editor.py            # then open http://127.0.0.1:4174/

Serves website/public as the site does, with an "Edit text" button on every page. In edit mode,
click any text and type; "Save" stores each changed piece (page, old text, new text, and the
element's old and new HTML) in website/text_edits.json. Nothing on the site changes until those
edits are applied to the real sources (website/public/*.html, or website/build_docs.py for the
docs pages), then the file is cleared. Local only: binds 127.0.0.1.
"""
import http.server
import json
import os
import sys
from datetime import datetime

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "website")
PUBLIC = os.path.abspath(os.path.join(ROOT, "public"))
EDITS = os.path.abspath(os.path.join(ROOT, "text_edits.json"))
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 4174

EDITOR = r"""
<style>
#__ed{position:fixed;right:16px;bottom:16px;z-index:2147483647;display:flex;gap:8px;font:600 14px system-ui,sans-serif}
#__ed button{padding:10px 14px;border:0;border-radius:10px;background:#1d4ed8;color:#fff;cursor:pointer;box-shadow:0 4px 14px #0005}
#__ed button.off{background:#444}
#__ed span{align-self:center;padding:8px 10px;border-radius:8px;background:#000c;color:#fff}
body.__editing [data-ed]{outline:1px dashed #60a5fa88;cursor:text}
body.__editing [data-ed]:focus{outline:2px solid #2563eb;background:#2563eb14}
body.__editing [data-ed].__changed{outline:2px solid #16a34a}
</style>
<div id="__ed"><span id="__edmsg" hidden></span><button id="__edsave" hidden>Save</button><button id="__edtoggle" class="off">Edit text</button></div>
<script>
(() => {
  const SKIP = 'script,style,svg,#__ed,noscript';
  const msg = document.getElementById('__edmsg');
  const saveBtn = document.getElementById('__edsave');
  const toggle = document.getElementById('__edtoggle');
  let on = false, marked = [];
  const say = (t) => { msg.textContent = t; msg.hidden = !t; };
  // Elements holding text directly (a text node child with letters), innermost first.
  function textElements() {
    const out = new Set();
    const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n; n = w.nextNode()) {
      if (!/\S/.test(n.nodeValue)) continue;
      const el = n.parentElement;
      if (!el || el.closest(SKIP)) continue;
      // Edit the whole block (p, li, h1...) so inline bits (links, <b>) stay inside it.
      const block = el.closest('p,li,h1,h2,h3,h4,h5,h6,summary,figcaption,td,th,dt,dd,blockquote,label,button,a') || el;
      out.add(block);
    }
    // Drop ones inside another chosen element.
    return [...out].filter((e) => ![...out].some((o) => o !== e && o.contains(e)));
  }
  function start() {
    marked = textElements();
    for (const el of marked) {
      el.dataset.ed = '1';
      el.__before = { html: el.innerHTML, text: el.innerText };
      el.contentEditable = 'true';
      el.addEventListener('input', () => el.classList.toggle('__changed', el.innerHTML !== el.__before.html));
    }
    document.body.classList.add('__editing');
  }
  function stop() {
    for (const el of marked) { el.removeAttribute('contenteditable'); delete el.dataset.ed; }
    document.body.classList.remove('__editing');
  }
  // Links and buttons don't act while editing.
  document.addEventListener('click', (e) => {
    if (on && e.target.closest('[data-ed]') && !e.target.closest('#__ed')) e.preventDefault();
  }, true);
  toggle.onclick = () => {
    on = !on;
    toggle.textContent = on ? 'Stop editing' : 'Edit text';
    toggle.classList.toggle('off', !on);
    saveBtn.hidden = !on;
    on ? start() : stop();
    say(on ? 'Click any text to change it' : '');
  };
  saveBtn.onclick = async () => {
    const changes = marked.filter((el) => el.innerHTML !== el.__before.html).map((el) => ({
      tag: el.tagName.toLowerCase(),
      old_text: el.__before.text, new_text: el.innerText,
      old_html: el.__before.html, new_html: el.innerHTML,
    }));
    if (!changes.length) { say('Nothing changed yet'); return; }
    const r = await fetch('/__save', { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ page: location.pathname, changes }) });
    if (!r.ok) { say('Save failed'); return; }
    for (const el of marked) { el.__before = { html: el.innerHTML, text: el.innerText }; el.classList.remove('__changed'); }
    say(`Saved ${changes.length} change${changes.length > 1 ? 's' : ''}`);
  };
})();
</script>
"""


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=PUBLIC, **k)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def html_path(self):
        path = self.path.split("?", 1)[0].split("#", 1)[0]
        full = os.path.abspath(os.path.join(PUBLIC, path.lstrip("/")))
        if not full.startswith(PUBLIC):
            return None
        if os.path.isdir(full):
            full = os.path.join(full, "index.html")
        elif not os.path.exists(full) and os.path.exists(full + ".html"):
            full += ".html"
        return full if full.endswith(".html") and os.path.isfile(full) else None

    def do_GET(self):
        full = self.html_path()
        if full is None:
            return super().do_GET()
        page = open(full, encoding="utf-8").read()
        i = page.rfind("</body>")
        page = page[:i] + EDITOR + page[i:] if i >= 0 else page + EDITOR
        body = page.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if self.path != "/__save":
            self.send_error(404)
            return
        data = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
        edits = json.load(open(EDITS, encoding="utf-8")) if os.path.exists(EDITS) else []
        stamp = datetime.now().isoformat(timespec="seconds")
        for c in data.get("changes", []):
            c = {"page": data.get("page", ""), "saved": stamp, **c}
            # A later edit of the same text replaces the earlier one.
            edits = [e for e in edits if not (e["page"] == c["page"] and e["new_html"] == c["old_html"])] + [
                {**c, "old_html": next((e["old_html"] for e in edits if e["page"] == c["page"] and e["new_html"] == c["old_html"]), c["old_html"]),
                 "old_text": next((e["old_text"] for e in edits if e["page"] == c["page"] and e["new_html"] == c["old_html"]), c["old_text"])}]
        with open(EDITS, "w", encoding="utf-8") as f:
            json.dump(edits, f, indent=1, ensure_ascii=False)
        self.send_response(204)
        self.end_headers()


if __name__ == "__main__":
    print(f"Site editor on http://127.0.0.1:{PORT}/  (edits go to {EDITS})", flush=True)
    http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
