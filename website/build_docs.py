"""Writes the mod API docs site (public/docs/) from the page list below.

The output is plain static HTML, committed as-is: the site has no build step or JavaScript.
Edit a page here, then run:

    python website/build_docs.py
"""
import html
import os
import re

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "public", "docs")
GH = "https://github.com/blinkywink/rrbtd6"


def code(src, lang=""):
    """A code block (escaped)."""
    return f'<pre class="code" data-lang="{lang}"><code>{html.escape(src.strip(chr(10)))}</code></pre>'


def note(text, kind="note"):
    label = {"note": "Note", "warn": "Limit", "tip": "Tip"}[kind]
    return f'<div class="callout {kind}"><strong>{label}</strong><p>{text}</p></div>'


def table(head, rows):
    th = "".join(f"<th>{h}</th>" for h in head)
    tr = "".join("<tr>" + "".join(f"<td>{c}</td>" for c in r) + "</tr>" for r in rows)
    return f'<div class="table"><table><thead><tr>{th}</tr></thead><tbody>{tr}</tbody></table></div>'


PAGES = []


def page(slug, group, title, summary, body):
    PAGES.append({"slug": slug, "group": group, "title": title, "summary": summary, "body": body})


# ---------------------------------------------------------------- Getting started

page("", "Getting started", "Overview",
     "RustTD6's own mod format: plain files, built on your computer, loaded by the native client.",
     f"""
<p class="lead">Mods are authored in files, not an in-game editor. You describe what you want in a Python
script (or any language that writes JSON), build it once into a <code>mod.json</code> manifest, and RustTD6
compiles that into its native simulation. Optional event scripts written in Rhai add real running code and
custom in-match UI.</p>

<div class="cards">
  <a class="card" href="/docs/quick-start/"><strong>Quick start</strong><span>Your first tower, bloon and mode in five minutes.</span></a>
  <a class="card" href="/docs/content/"><strong>Content operations</strong><span>Clone, patch, copy and rearrange the game's data.</span></a>
  <a class="card" href="/docs/scripts/"><strong>Event scripts</strong><span>Rhai callbacks that react to rounds, pops and towers.</span></a>
  <a class="card" href="/docs/ui/"><strong>Custom UI</strong><span>Native panels, buttons and popups owned by your script.</span></a>
  <a class="card" href="/docs/entities/"><strong>Entities &amp; hooks</strong><span>Inspect and edit live towers, bloons and projectiles.</span></a>
  <a class="card" href="/docs/models/"><strong>Custom models</strong><span>Import OBJ meshes or bake rigged Blender animation.</span></a>
</div>

<h2 id="how">How a mod runs</h2>
<ol class="flow">
  <li><strong>Author.</strong> Write <code>mods/&lt;id&gt;/mod.py</code> using the SDK (<code>tools/modsdk.py</code>).</li>
  <li><strong>Build.</strong> <code>python tools/modsdk.py build mods/&lt;id&gt;/mod.py</code> writes <code>mod.json</code>. Your script runs only when you build it.</li>
  <li><strong>Load.</strong> In the client, open Mods, press Reload Mods and enable it. The manifest is validated and compiled into native data.</li>
  <li><strong>Play.</strong> Start a new local game. Edit, rebuild and reload as often as you like, with no client restart.</li>
</ol>

<h2 id="what">What you can change</h2>
<ul class="checks">
  <li>New towers cloned from existing ones, with their own stats, projectiles, upgrades, shop position and models</li>
  <li>New bloons, edits to rounds and round groups, and new or changed modes</li>
  <li>Any existing native field through JSON Pointer paths (<code>set</code>, <code>copy</code>, <code>insert</code>, <code>remove</code>)</li>
  <li>Text and visibility of existing menu nodes</li>
  <li>Event scripts: cash, lives, spawning, popups, custom UI, entity queries and behaviour interceptors</li>
</ul>

{note("This is not a MelonLoader compatibility layer: BTD6 MelonLoader mods do not run in RustTD6, and no DLLs are injected. Never bundle or redistribute extracted BTD6 content in a mod.")}

<h2 id="scope">Where mods apply</h2>
<p>Content mods apply to every local game. Official events bypass local content mods.
In private co-op rooms the host's room mod set replaces each player's local selection.</p>
""")

page("quick-start", "Getting started", "Quick start",
     "Build and load a mod that adds a tower, a bloon and a mode.",
     f"""
<h2 id="needs">You need</h2>
<ul>
  <li>An installed copy of RustTD6. The SDK ships with it at <code>tools/modsdk.py</code>.</li>
  <li>Python 3.10 or newer (only for building; nothing runs in Python during play).</li>
</ul>

<h2 id="write">1. Write the mod</h2>
<p>Save this as <code>mods/quickstart/mod.py</code>, next to your game. <a href="/mods/quickstart.py" download>Download it</a>.</p>
{code('''# The SDK supplies Mod when it builds this script.
mod = Mod("quickstart", "Quick Start", description="A custom dart, bloon and mode.")

dart = mod.tower("dart", base="DartMonkey", cost=100)
dart.projectile(damage=3, pierce=4)
dart.weapon(rate=0.5)

mod.bloon("durable-red", base="Red", maxHealth=5)
mod.mode("training", base="Medium", cash=2000, lives=300)''', "python")}

<h2 id="build">2. Build its manifest</h2>
<p>From the installation folder:</p>
{code("python tools/modsdk.py build mods/quickstart/mod.py", "shell")}
<p>This writes <code>mods/quickstart/mod.json</code>. You can also generate the same JSON from another language.</p>

<h2 id="enable">3. Enable it</h2>
<ol>
  <li>Turn on <strong>Settings → Advanced → UI → Show Mods Button</strong>.</li>
  <li>Open <strong>Mods</strong> and press <strong>Reload Mods</strong>.</li>
  <li>Flip the switch beside <em>Quick Start</em>. Selecting its row only previews its properties and settings.</li>
</ol>

<h2 id="play">4. Play</h2>
<p>Start a new local game. Your tower is in the shop, your bloon is in Sandbox, and the mode screen has a
custom-mode selector with <em>training</em>.</p>

<h2 id="iterate">Edit, rebuild, reload</h2>
<p>Change the script, build again, then press Reload Mods. Changes apply to the next new or restarted game.
A match already in progress keeps the definitions it started with.</p>
{note("Hiding the Mods button doesn't disable enabled mods. Enabling and selecting are separate: the switch controls whether a mod is active.", "tip")}
""")

page("recipes", "Getting started", "Recipes",
     "Short, complete answers to common modding tasks.",
     f"""
<p>Each recipe uses only the documented API. Put the Python lines in your <code>mod.py</code> after
<code>mod = Mod(...)</code>, and the Rhai in a script embedded with <code>mod.script(...)</code>.</p>

<h2 id="price">Change a tower's price</h2>
{code('mod.set("/towers/DartMonkey/cost", 100)', "python")}

<h2 id="cash">Start a mode with more cash and lives</h2>
{code("""mod.set("/modes/Medium/cash", 1500)
mod.all_modes(lives=500)            # every mode defined so far""", "python")}

<h2 id="faster-dart">A faster, harder-hitting copy of a tower</h2>
{code("""dart = mod.tower("sharp-dart", base="DartMonkey", cost=250)
dart.weapon(rate=0.4)               # seconds between shots
dart.projectile(damage=2, pierce=5)
dart.shop(after="DartMonkey")""", "python")}

<h2 id="tough-bloon">A tougher bloon</h2>
{code('mod.bloon("armoured-red", base="Red", maxHealth=20, isCamo=True)', "python")}

<h2 id="shop">Hide or move shop entries</h2>
{code("""mod.existing_tower("BananaFarm").shop(visible=False)
mod.existing_tower("SniperMonkey").shop(index=0, category="primary")""", "python")}

<h2 id="round-bonus">Pay a growing bonus every round</h2>
{code("""fn on_new_game(ctx) { this.bonus = 50; [] }
fn on_round_end(ctx) {
    if ctx.sandbox { return []; }
    this.bonus += 25;
    [#{op: "cash", amount: this.bonus}]
}""", "rhai")}

<h2 id="shield">A shield that absorbs the first 10 health hits</h2>
{code("""fn on_new_game(ctx) { this.charges = 10; [] }
fn before_damage(ctx) {
    this.charges -= 1;
    #{cancel: true, unsubscribe: this.charges <= 0}
}""", "rhai")}
<p>The last charge applies its decision and then mutes the hook, so later hits cost nothing.</p>

<h2 id="no-sell">Stop a tower type being sold</h2>
{code('fn before_sell(ctx) { #{cancel: ctx.event.tower == "DartMonkey"} }', "rhai")}

<h2 id="double">Double every projectile's damage</h2>
{code("fn before_fire(ctx) { #{damage: ctx.event.damage * 2} }", "rhai")}

<h2 id="wave">Send an extra wave every fifth round</h2>
{code("""fn on_round_start(ctx) {
    if ctx.round % 5 != 4 { return []; }
    [#{op: "spawn", name: "Pink", count: 20, spacing: 0.15},
     #{op: "popup", title: "Bonus wave", body: "20 Pinks incoming!"}]
}""", "rhai")}

<h2 id="heal">Top up lives when they get low</h2>
{code("""fn on_round_end(ctx) {
    if ctx.lives < 20 { return [#{op: "lives", value: 20}]; }
    []
}""", "rhai")}
{note("Commands from one callback are validated together; if any is invalid, none apply and the script stops for that match.", "tip")}
""")

page("mods-menu", "Getting started", "The Mods menu",
     "Finding, enabling and previewing installed mods, and mod artwork.",
     f"""
<h2 id="list">Installed mods</h2>
<p>The Mods screen lists every folder in <code>mods/</code> that has a valid <code>mod.json</code>, up to 256 folders.
<strong>Open Mods Folder</strong> opens the installation directory. Enabled IDs are saved in local settings
under <code>mods</code>; missing or invalid enabled IDs are reported rather than silently removed.</p>

<h2 id="search">Search and filters</h2>
<p>The search box matches names, IDs and descriptions as you type. <em>All</em>, <em>Enabled</em> and
<em>Disabled</em> filters combine with the search. The gold outline marks the mod whose details you're
viewing; its switch controls whether it's enabled. Filtering never changes activation.</p>

<h2 id="icon">Mod artwork</h2>
<p>Put an optional <code>icon.png</code> beside your <code>mod.json</code>:</p>
<ul>
  <li>Transparent or opaque PNG, at most 512 × 512 pixels and 1 MiB.</li>
  <li>The list and details view use the same icon and keep its aspect ratio.</li>
  <li>Missing or invalid images fall back to the standard mod icon.</li>
</ul>
<p>Press Reload Mods after changing artwork. Icons are local presentation assets and aren't sent to co-op rooms.</p>
""")

# ---------------------------------------------------------------- Content API

page("manifest", "Content API", "The manifest",
     "The fields of mod.json and the rules every manifest must follow.",
     f"""
<p><code>mods/&lt;folder&gt;/mod.json</code> is strict JSON. The SDK writes it for you, but any tool can.</p>
{code('''{
  "api_version": 1,
  "id": "quickstart",
  "name": "Quick Start",
  "version": "1.0.0",
  "description": "A custom dart, bloon and mode.",
  "content": [ ... ]
}''', "json")}

<h2 id="fields">Fields</h2>
{table(["Field", "Required", "Rules"], [
    ["<code>api_version</code>", "yes", "<code>1</code>"],
    ["<code>id</code>", "yes", "Unique, lowercase, 1–64 ASCII letters, digits or <code>._-</code>, starting alphanumeric. Keep it stable between releases."],
    ["<code>name</code>", "yes", "Nonempty, at most 80 UTF-8 bytes"],
    ["<code>version</code>", "yes", "At most 32 bytes"],
    ["<code>description</code>", "no", "At most 1024 bytes"],
    ["<code>content</code>", "no", "List of content operations (see <a href='/docs/content/'>Content operations</a>)"],
    ["<code>on_new_sandbox</code>, <code>settings</code>", "no", "Optional Sandbox starting values (<a href='#sandbox'>below</a>)"],
])}
<p>Text metadata can't contain control characters. Duplicate IDs are invalid.</p>

<h2 id="order">Order and conflicts</h2>
<ul>
  <li>Operations inside a mod run in written order.</li>
  <li>Enabled mods compile in sorted ID order.</li>
  <li>The SDK prefixes your content IDs with your mod ID, so names don't collide.</li>
  <li>Two writes to the same exact <code>set</code> path conflict.</li>
</ul>

<h2 id="validation">Validation</h2>
<p>Unknown operations, duplicate IDs, missing or misspelled fields, bloon child cycles and conflicting writes are
rejected. Definitions are deserialized before use; an invalid content transaction leaves the base data unchanged
and reports its error in the client log.</p>
{note("A manifest is limited to 64 KiB and 256 operations.", "warn")}
<h2 id="sandbox">Optional: Sandbox starting values</h2>
<p>A mod can also set the cash and lives a new local Sandbox starts with, and let players adjust them from the mod's settings:</p>
{code('''"on_new_sandbox": { "starting_cash": 100000 },
"settings": { "starting_cash": { "min": 0, "max": 1000000, "step": 10000 } }''', "json")}
<p>Values are whole numbers (cash 0–1e12, lives 1–1e9); two enabled mods setting the same value conflict.</p>

<p><a href="{GH}/blob/main/mods/mod.schema.json">JSON schema</a> · <a href="{GH}/blob/main/mods/README.md">Format reference on GitHub</a></p>
""")

page("content", "Content API", "Content operations",
     "Every operation a manifest's content list can contain.",
     f"""
<p>The <code>content</code> list is the heart of a mod. Each entry is one operation, applied in order.</p>

<h2 id="tower"><code>tower</code></h2>
{code('{"op": "tower", "id": "quickstart.dart", "base": "DartMonkey", "patch": {"cost": 100}}', "json")}
<p>Clones a non-hero tower: its upgrade tiers, shop entry and shared visual references. <code>patch</code> changes existing
base fields such as cost, range or icon. No game assets are copied or distributed. Heroes can't be clone bases.</p>

<h2 id="bloon"><code>bloon</code></h2>
{code('{"op": "bloon", "id": "quickstart.durable-red", "base": "Red", "patch": {"maxHealth": 5}}', "json")}
<p>Clones a bloon and adds it to the Sandbox list. Native camel-case fields include <code>maxHealth</code>,
<code>isCamo</code>, <code>speed</code>, <code>radius</code> and <code>children</code>. Children must exist and can't form cycles.</p>

<h2 id="mode"><code>mode</code> and <code>all_modes</code></h2>
{code('''{"op": "mode", "id": "quickstart.training", "base": "Medium", "patch": {"cash": 2000}}
{"op": "all_modes", "patch": {"lives": 500}}''', "json")}
<p><code>mode</code> clones a mode; new IDs appear in the mode screen's custom selector (IDs are stable names, not indices).
<code>all_modes</code> patches every mode present at that point in the list.</p>

<h2 id="set"><code>set</code></h2>
{code('''{"op": "set", "path": "/modes/Medium/cash", "value": 1500}
{"op": "set", "path": "/towers/DartMonkey/cost", "value": 100}''', "json")}
<p>Replaces an existing field addressed by a JSON Pointer. Roots:</p>
{table(["Root", "Contains"], [
    ["<code>towers</code>", "Tower definitions (shop cost, range, footprint…)"],
    ["<code>tower_tiers</code>", "Per-tier attacks, weapons, projectiles and upgrades"],
    ["<code>bloons</code>", "Bloon definitions"],
    ["<code>modes</code>", "Difficulties and modes"],
    ["<code>start</code>", "Starting values"],
    ["<code>rounds</code>, <code>round_sets</code>", "Round groups and round sets"],
    ["<code>freeplay</code>", "Freeplay round groups"],
])}
<p>Inspect your own <code>local_extracted/game/game.json</code> for native field names and supported behaviours.</p>

<h2 id="copy"><code>copy</code></h2>
<p><code>{{"op": "copy", "from": &lt;pointer&gt;, "path": &lt;pointer&gt;}}</code> copies an existing native definition to another
existing path. The SDK's <a href="/docs/python-sdk/#templates">projectile templates</a> use it, so you rarely write it by hand.</p>

<h2 id="arrays"><code>insert</code> and <code>remove</code></h2>
<p>Change arrays, including attacks, weapons, abilities and round groups. Values must match native data types; adding an
array entry doesn't create a new native behaviour type.</p>

<h2 id="menu"><code>menu</code></h2>
{code('{"op": "menu", "page": "MainMenuUI", "path": "<node path from ui/prefabs/MainMenuUI.json>", "text": "Hello"}', "json")}
<p>Changes an existing frontend node's <code>text</code> or <code>visible</code> state when that scene loads. <code>page</code>
is the prefab filename without <code>.json</code>; paths are relative to the node hierarchy in your local
<code>ui/prefabs</code> export. Values the game updates dynamically may overwrite a load-time edit.</p>
""")

page("python-sdk", "Content API", "Python SDK",
     "The authoring helpers modsdk.py gives your build script.",
     f"""
<p>The SDK runner supplies <code>Mod</code> to your script; keep your instance named <code>mod</code>. Every helper just
emits content operations, so anything here can also be written as raw JSON.</p>

{table(["Helper", "Does"], [
    ["<code>Mod(id, name, description=…)</code>", "Starts a mod"],
    ["<code>mod.tower(name, base, **patch)</code>", "Clones a tower; returns a handle"],
    ["<code>mod.existing_tower(id)</code>", "Handle for a native tower (for <code>.shop</code>, <code>.stats</code>, <code>.upgrade</code>)"],
    ["<code>tower.projectile(**fields)</code>", "Edits a weapon's projectile (or takes a template)"],
    ["<code>tower.weapon(rate=…)</code>", "Edits a weapon; <code>rate</code> is seconds between shots"],
    ["<code>tower.stats(**fields)</code>", "Edits the tower definition"],
    ["<code>tower.upgrade(path, tier, …)</code>", "Edits one upgrade and its later states"],
    ["<code>tower.shop(before|after|index, category, visible)</code>", "Places the tower in the shop"],
    ["<code>tower.model(name)</code>", "Uses a custom baked model"],
    ["<code>mod.projectile(name, base_tower, **fields)</code>", "Reusable projectile template"],
    ["<code>mod.bloon(name, base, **patch)</code>", "Clones a bloon"],
    ["<code>mod.mode(name, base, **patch)</code>", "Clones a mode"],
    ["<code>mod.all_modes(**patch)</code>", "Patches every mode so far"],
    ["<code>mod.set(path, value)</code>", "Raw JSON Pointer write"],
    ["<code>mod.insert(path, index, value)</code> / <code>mod.remove(path, index)</code>", "Array edits"],
    ["<code>mod.menu(page, path, text=…, visible=…)</code>", "Menu node edit"],
    ["<code>mod.model(name)</code>", "Reference to a baked model in your assets"],
    ["<code>mod.script(name, file=…, tick_interval=…)</code>", "Embeds a Rhai event script"],
])}

<h2 id="defaults">Tiers, attacks and weapons</h2>
<p>Projectile and weapon helpers default to tier <code>000</code>, attack 0 and weapon 0. Pass <code>tier="100"</code>,
<code>attack=</code> or <code>weapon=</code> to edit others. Upgraded tiers keep their inherited values unless you edit them too.</p>
{code('''dart = mod.tower("dart", base="DartMonkey", cost=100)
dart.projectile(damage=3, pierce=4)               # base tier
dart.projectile(tier="100", damage=4)             # after the first top-path upgrade
dart.weapon(rate=0.5)''', "python")}

<h2 id="templates">Projectile templates</h2>
{code('''bomb = mod.projectile("big-bomb", base_tower="BombShooter", damage=10)
dart.projectile(bomb, pierce=30)''', "python")}
<p>A template copies the complete native projectile, including its behaviour models, then applies your overrides.
Projectiles are compiled native data attached to weapons; they aren't Python objects or callbacks during play.</p>
""")

page("upgrades", "Content API", "Upgrades",
     "Editing individual upgrades, their prices, stats, projectiles and models.",
     f"""
{code('''tower.upgrade(0, 1, cost=150,
    stats={"range": 40},
    weapon={"rate": 0.5},
    projectile={"damage": 5, "pierce": 5, "display": mod.model("bolt")},
    model="robot-upgraded")''', "python")}

<h2 id="rules">How upgrade edits apply</h2>
<ul>
  <li>Paths are 0–2 and tiers 1–5.</li>
  <li>The edit sets the purchase price and the resulting tier states, including later tiers and crosspaths.</li>
  <li>Values are absolute replacements: write lower tiers first, then higher tiers to override them.</li>
  <li><code>stats</code> edits native tower-tier fields, <code>weapon</code> the primary weapon, and <code>projectile</code> that weapon's projectile.</li>
  <li>Unknown fields fail validation rather than being silently ignored.</li>
  <li>A model swap uses the exported muzzle and release time.</li>
</ul>

<h2 id="replace">Replacing a projectile completely</h2>
{code('''bomb = mod.projectile("grenade", base_tower="BombShooter", damage=10)
tower.upgrade(0, 2, projectile=bomb)''', "python")}
<p>A template copies the whole native projectile configuration, behaviours included, before your overrides.
A plain dict keeps any behaviours you don't mention.</p>
<p>Secondary weapons and other attacks can be edited through explicit <code>set</code>/<code>copy</code> paths or per-tier
<code>tower.weapon</code>/<code>tower.projectile</code> calls.</p>
{note("Fourth upgrade paths aren't supported: the engine and upgrade screen store exactly three paths.", "warn")}
""")

page("bloons-rounds-modes", "Content API", "Bloons, rounds & modes",
     "New bloons, round edits and custom modes.",
     f"""
<h2 id="bloons">Bloons</h2>
{code('mod.bloon("durable-red", base="Red", maxHealth=5)', "python")}
<p>Creates a new bloon using an existing visual. Editable: <code>maxHealth</code>, <code>speed</code>, <code>radius</code>,
<code>children</code>, <code>isCamo</code> and supported native behaviours. Child references must exist and can't form cycles.
New bloons appear in the Sandbox bloon list.</p>

<h2 id="rounds">Rounds</h2>
<p>Use <code>mod.set</code> to replace a round group with one that references your bloon, or
<code>mod.insert</code>/<code>mod.remove</code> to change round group arrays. Event scripts can also queue spawns at runtime
(<a href="/docs/scripts/#commands"><code>spawn</code></a>).</p>

<h2 id="modes">Modes</h2>
{code('''mod.mode("training", base="Medium", cash=2000, lives=300)
mod.all_modes(lives=500)
mod.set("/modes/Medium/cash", 1500)''', "python")}
<p>New modes appear in the custom-mode selector below the ordinary modes. These changes work in ordinary local
games, not just Sandbox; official events keep their own rules.</p>
""")

page("shop", "Content API", "Shop layout",
     "Rearranging, categorising and hiding towers in the shop.",
     f"""
{code('''mod.existing_tower("DartMonkey").shop(after="SniperMonkey", category="military")
robot = mod.tower("robot", base="DartMonkey", cost=300)
robot.shop(before="DartMonkey")
mod.existing_tower("BananaFarm").shop(visible=False)''', "python")}
<ul>
  <li>Give exactly <strong>one</strong> of <code>before</code>, <code>after</code> or zero-based <code>index</code>.</li>
  <li>Categories are <code>primary</code>, <code>military</code>, <code>magic</code> and <code>support</code>.</li>
  <li><code>visible=False</code> hides an entry.</li>
  <li>Both native shop layouts change. Existing mode restrictions still apply.</li>
</ul>
""")

page("menus", "Content API", "Menus",
     "Changing text and visibility of existing frontend nodes.",
     f"""
{code('mod.menu("MainMenuUI", "<node path>", visible=False)  # path from your ui/prefabs/MainMenuUI.json', "python")}
<ul>
  <li><code>page</code> matches an exported prefab filename, such as <code>MainMenuUI</code>.</li>
  <li><code>path</code> matches the node hierarchy in your local <code>ui/prefabs</code> export.</li>
  <li>Set <code>text</code> or <code>visible</code>; use them separately.</li>
  <li>The edit applies when that scene loads. Values the game updates dynamically may overwrite it.</li>
</ul>
<p>For interactive in-match panels, use <a href="/docs/ui/">custom UI</a> from an event script instead.
Arbitrary new frontend screens aren't implemented yet.</p>
""")

# ---------------------------------------------------------------- Assets

page("models", "Assets", "Custom models",
     "Original meshes and animations for towers and projectiles.",
     f"""
<h2 id="template">Start from the template</h2>
{code('''python tools/mod_model.py template mods/throwbot
python tools/modsdk.py build mods/throwbot/mod.py''', "shell")}
<p>Enable Throwbot and start a new game: a blue animated robot with a gold custom projectile and a release socket.
Everything in it is original art; no game art is included.</p>

<h2 id="use">Use a model</h2>
{code('''mod = Mod("my-mod", "My mod")
tower = mod.tower("robot", base="DartMonkey")
tower.model("robot")
tower.projectile(display=mod.model("bolt"), damage=3, pierce=4)''', "python")}
<p>Put baked models in <code>mods/my-mod/assets/robot/</code> and <code>assets/bolt/</code>; the folder name must match the mod ID.
<code>tower.model</code> reads the baked socket and applies the primary muzzle and release lead to every inherited upgrade tier.
Secondary weapons keep their original sockets.</p>

<h2 id="obj">Import an OBJ mesh</h2>
{code("python tools/mod_model.py import robot.obj --texture atlas.png --out mods/my-mod/assets/robot --muzzle 4 8 6 --release 0.15", "shell")}
<ul>
  <li>Y up, +Z forward, feet at the origin, in game units (the starter is about 14 units tall).</li>
  <li><code>--muzzle X HEIGHT FORWARD</code> is the hand/barrel release point; <code>--release</code> is seconds into Attack.</li>
  <li>Add <code>--attack frame01.obj frame02.obj …</code> (identical topology and face order) to bake a throw, and <code>--fps</code> for the sample rate.</li>
  <li>Without attack frames, the model aims and fires but has no throwing motion.</li>
</ul>

<h2 id="blender">Bake a rigged Blender animation</h2>
{code("blender --background robot.blend --python tools/blender_mod_model.py -- --out mods/my-mod/assets/robot --idle 1 60 --attack 61 90 --release 70 --texture atlas.png", "shell")}
<ul>
  <li>Blender is Z up, −Y forward.</li>
  <li>Add an Empty named <code>Muzzle</code>, parented to the throwing hand or barrel.</li>
  <li>Put the meshes to export in a collection named <code>RustTD6</code>.</li>
  <li>Rigs and modifiers are baked into vertex poses. Use one PNG atlas; Blender material shaders aren't imported. Keep topology stable.</li>
</ul>

{note("At most 60,000 split vertices, 600 frames per clip and 128 MiB of uncompressed poses per model. Lower-poly models and fewer samples load faster.", "warn")}
<p>Blender and Python never run during gameplay; meshes and poses go through the native renderer. Models don't add
simulation behaviour: the cloned tower and weapon supply targeting and projectile rules. Custom models travel with
co-op rooms: players who choose Download get the host's <code>assets/</code> files, checked against their hashes. To
share a mod's art, name its folder after its ID.</p>
""")

# ---------------------------------------------------------------- Scripting

page("scripts", "Scripting", "Event scripts",
     "Real running code: Rhai callbacks that react to the match and return commands.",
     f"""
<p>Rhai is a small embedded language with functions, loops, conditions, arrays and maps. Embed a script from your
build script, and it compiles once when a match loads.</p>
{code('mod.script("mechanics", file="mechanics.rhai", tick_interval=60)', "python")}

<h2 id="first">Your first script</h2>
{code('''fn on_new_game(ctx) { this.reward = 100; [] }
fn on_round_end(ctx) {
    if ctx.sandbox { return []; }
    this.reward += 10;
    [#{op: "cash", amount: this.reward}]
}''', "rhai")}
<p>Define only the callbacks you need. Each receives <code>ctx</code> and returns an array of commands.
<code>this</code> is a per-script, per-match map for your state (not a save file). Top-level statements don't run;
put helpers in functions, and call ones that use state as <code>this.helper(ctx)</code> so <code>this</code> stays bound.</p>

<h2 id="hooks">Hooks</h2>
{table(["Callback", "Runs"], [
    ["<code>on_new_game</code>", "When the match starts"],
    ["<code>on_round_start</code> / <code>on_round_end</code>", "At each round boundary"],
    ["<code>on_tower_placed</code>", "After a placement; <code>ctx.event</code> is the tower snapshot"],
    ["<code>on_tower_sold</code>", "After a sale; <code>ctx.event</code> is the pre-removal snapshot"],
    ["<code>on_towers_changed</code>", "When the tower list changes"],
    ["<code>on_pop</code>", "Once per simulation step, aggregating that step's pops"],
    ["<code>on_tick</code>", "Every <code>tick_interval</code> simulation steps (60 per simulated second)"],
    ["<code>on_query</code>", "When an <a href='/docs/entities/'>inspect</a> request is answered"],
])}
<p>No callbacks run while paused, except explicit UI actions.</p>

<h2 id="ctx">The <code>ctx</code> snapshot</h2>
{table(["Field", "Meaning"], [
    ["<code>cash</code>", "Current cash"],
    ["<code>lives</code>", "Current lives"],
    ["<code>round</code>", "Completed rounds"],
    ["<code>tick</code>", "Simulation step"],
    ["<code>popped</code>", "Pops (in <code>on_pop</code>, this step's)"],
    ["<code>tower_count</code>, <code>bloon_count</code>", "Live counts"],
    ["<code>sandbox</code>", "Whether this is a Sandbox match"],
    ["<code>event</code>", "Hook-specific data, when present"],
])}

<h2 id="commands">Commands</h2>
{code('''#{op: "cash", amount: 100}                       // add (negative subtracts)
#{op: "lives", value: 200}                       // set; doesn't revive an ended game
#{op: "spawn", name: "Blue", count: 30, spacing: 0.2}
#{op: "clear_bloons"}
#{op: "reset_cooldowns"}
#{op: "popup", title: "Hello", body: "Your message"}
#{op: "ui", widget: #{id: "panel", tree: node, actions: #{"Button": "on_click"}, modal: true}}
#{op: "remove_ui", id: "panel"}
#{op: "inspect", query: #{kind: "towers", offset: 0, limit: 8, tag: "list"}}
#{op: "entity_patch", patch: #{kind: "bloons", id: b.id, layer: b.layer, fields: #{hp: 50}}}
#{op: "damage_bloon", id: b.id, layer: b.layer, amount: 5}
#{op: "listen", event: "before_fire", enabled: true}''', "rhai")}
<p>Commands in one callback are validated together before any apply. An invalid batch or an over-budget script disables
that script for the match.</p>

<h2 id="limits">Limits</h2>
{table(["Limit", "Value"], [
    ["Operations per callback", "30,000"],
    ["Operations per simulation tick (all scripts)", "100,000"],
    ["Call depth", "16"],
    ["Commands per callback", "32"],
    ["Persistent <code>this</code> state", "64 KiB"],
    ["Script source", "32 KiB (inside the 64 KiB manifest)"],
    ["Callback output", "256 KiB"],
])}
{note("Scripts have no filesystem, network, process or clock APIs. In co-op they run inside the shared simulation on every player, and a widget button runs its event for everyone on behalf of the player who pressed it.", "warn")}
<p>A complete example is <a href="/docs/examples/#workshop">Mechanics Workshop</a>.</p>
""")

page("ui", "Scripting", "Custom UI",
     "Native panels, buttons and popups created and owned by your script.",
     f"""
{code('''fn on_new_game(ctx) {
    // this.panel() is your own helper that returns a node tree.
    [#{op: "ui", widget: #{
        id: "panel",
        tree: this.panel(),
        actions: #{"Body/Button": "on_click"},
        modal: true
    }}]
}
fn on_click(ctx) {
    [#{op: "cash", amount: 500}, #{op: "remove_ui", id: "panel"}]
}''', "rhai")}

<h2 id="tree">The tree format</h2>
<p>Widgets use the same <code>name</code> / <code>active</code> / <code>rect</code> / <code>comps</code> / <code>children</code>
shape as the game's exported UI. Text, images, buttons, masks, anchoring and layout groups render through the native
UI renderer.</p>
<ul>
  <li>Rect coordinates are in a 3840 × 2160 reference canvas with Y up; anchors adapt to the viewport.</li>
  <li>Bind paths <strong>relative to the widget root</strong> to your own one-argument functions. An empty path binds the root itself.</li>
  <li>Your action callback receives <code>ctx</code>, can change <code>this</code>, update its UI and return game commands.</li>
  <li>Send the same widget <code>id</code> again to replace it; <code>remove_ui</code> removes it.</li>
  <li>The topmost modal widget owns input. Native OK popups block the UI and map underneath.</li>
  <li>Trees rebuild only when they change.</li>
</ul>

{table(["Limit", "Value"], [
    ["Active widgets", "64"],
    ["Nodes per widget", "128"],
    ["Tree depth", "12 levels"],
    ["Action bindings", "32"],
])}
{note("Runtime UI targets in-match overlays. To change existing frontend pages, use <a href='/docs/menus/'>menu edits</a>; arbitrary new frontend screens are still pending.")}
""")

page("entities", "Scripting", "Entities & hooks",
     "Inspect live towers, bloons and projectiles, edit them by ID, and intercept native behaviour.",
     f"""
<p>These interfaces are local-only, like the rest of the script runtime. When nothing listens, the native path is
unchanged: no entity snapshots or script calls are made.</p>

<h2 id="inspect">Inspect live entities</h2>
{code('''fn on_new_game(ctx) {
    [#{op: "inspect", query: #{kind: "towers", offset: 0, limit: 8, tag: "my-list"}}]
}
fn on_query(ctx) {
    for tower in ctx.event.items {
        // tower.id is a stable entity ID, not an array index.
    }
    []
}''', "rhai")}
<ul>
  <li>Kinds: <code>towers</code>, <code>bloons</code>, <code>projectiles</code>.</li>
  <li>Responses arrive through the requesting script's <code>on_query</code> at a simulation boundary, with <code>kind</code>, <code>tag</code>, <code>offset</code>, <code>total</code> and <code>items</code>.</li>
  <li>Pages hold 1–128 items. At most eight queries are serviced per step, with up to 64 pending.</li>
  <li>Paging uses current storage order, so entities can appear, disappear or reorder between pages. Deduplicate by ID.</li>
</ul>

<h2 id="snapshots">Snapshot fields</h2>
{table(["Kind", "Fields"], [
    ["Tower", "<code>id</code>, <code>tower</code> (definition ID), <code>position</code>, <code>tiers</code>, <code>pops</code>, <code>damage</code>, <code>target</code>, <code>point</code>"],
    ["Bloon", "<code>id</code>, <code>layer</code>, <code>bloon</code> (definition ID), <code>hp</code>, <code>position</code>, <code>distance</code>, <code>path</code>"],
    ["Projectile", "<code>id</code>, <code>owner</code> (tower ID or 0), <code>position</code>, <code>velocity</code>, <code>damage</code>, <code>pierce</code>, <code>remaining</code>"],
])}
<p>Positions are native world coordinates. Projectile velocity is world units per step; <code>remaining</code> is seconds.
Position and remaining can be null for native infinities.</p>

<h2 id="patch">Edit by ID</h2>
{code('''#{op: "entity_patch", patch: #{kind: "bloons", id: b.id, layer: b.layer, fields: #{hp: 50}}}
#{op: "entity_patch", patch: #{kind: "projectiles", id: p.id, fields: #{damage: 10, pierce: 20, remaining: 2.0, velocity: [1.0, 0.0]}}}
#{op: "entity_patch", patch: #{kind: "towers", id: t.id, fields: #{target: "Strong"}}}
#{op: "damage_bloon", id: b.id, layer: b.layer, amount: 5}''', "rhai")}
<ul>
  <li>HP edits set positive current HP directly. Use <code>damage_bloon</code> for real damage: child spawning, cash, pops and damage hooks. It runs at the next simulation boundary and doesn't credit a tower.</li>
  <li><code>layer</code> is optional but recommended: a first child can reuse its parent's object ID with a new layer ID.</li>
  <li>Tower targets: First, Last, Close, Strong or Point, when that tower allows them. <code>point: [x, y]</code> moves a point target.</li>
  <li>Missing entities and mismatched layers are ignored. IDs are per match.</li>
  <li>Projectile movement controllers can overwrite velocity changes.</li>
</ul>

<h2 id="intercept">Change or cancel native behaviour</h2>
<p>Interceptors return a <strong>map</strong> (event callbacks return arrays):</p>
{code('''fn before_damage(ctx) {
    if ctx.event.bloon.bloon == "Red" { return #{cancel: true}; }
    #{damage: ctx.event.damage * 2}
}
fn before_fire(ctx) { #{damage: ctx.event.damage * 2, count: 3} }
fn before_place(ctx) { #{cancel: ctx.event.tower == "BananaFarm"} }
fn before_sell(ctx) { #{cancel: ctx.event.tower == "DartMonkey"} }
fn before_upgrade(ctx) { #{cancel: ctx.event.paragon} }''', "rhai")}
{table(["Interceptor", "Event", "Can return"], [
    ["<code>before_damage</code>", "<code>bloon</code> snapshot, <code>damage</code>", "<code>damage</code> (rounded to native integer), <code>cancel</code>"],
    ["<code>before_fire</code>", "<code>owner</code>, <code>position</code>, <code>direction</code> (radians), <code>count</code>, <code>damage</code>, <code>speed</code>, <code>pierce</code>, <code>tiers</code>", "any of those, or <code>cancel</code>"],
    ["<code>before_place</code>", "<code>tower</code>, <code>position</code>, <code>cost</code>", "<code>cancel</code>"],
    ["<code>before_sell</code>", "tower snapshot", "<code>cancel</code>"],
    ["<code>before_upgrade</code>", "<code>tower</code>, <code>paragon</code>, <code>path</code>/<code>cost</code> or <code>extra</code>", "<code>cancel</code>"],
])}
<ul>
  <li>An empty map <code>#{{}}</code> keeps native behaviour. <code>cancel: true</code> stops later scripts for that action.</li>
  <li>Scripts compose in stable order; later scripts see earlier overrides.</li>
  <li>Interceptors can read <code>ctx</code> and update <code>this</code>, but can't return game commands.</li>
  <li><code>before_damage</code> runs once per top-level health hit, not for child overflow. Cancelling prevents health loss only, not other hit effects or pierce use. Whole-bloon destruction that bypasses health damage doesn't enter it.</li>
  <li><code>before_fire</code> also sees emitted sub-projectiles. Cancelling doesn't refund the cooldown.</li>
  <li>Forced dependent cleanup when a parent tower is sold bypasses child sell vetoes.</li>
</ul>

<h2 id="listen">Turn hooks on and off</h2>
{code('''fn on_toggle(ctx) {
    this.enabled = !this.enabled;
    [#{op: "listen", event: "before_fire", enabled: this.enabled}]
}
// Apply the last shield charge, then stop receiving damage callbacks:
fn before_damage(ctx) { #{cancel: true, unsubscribe: true} }''', "rhai")}
<p>All declared callbacks start subscribed. <code>listen</code> pauses or resumes one of your own callbacks after the
current one, keeping state and widgets. An interceptor can return <code>unsubscribe: true</code> to mute only itself
after its decision. When nobody listens, the native hook skips script execution and snapshots entirely.</p>
<p>See <a href="/docs/examples/#entity-lab">Entity Lab</a> for a complete example.</p>
""")

# ---------------------------------------------------------------- Playing

page("coop", "Playing", "Co-op",
     "Sharing a mod set in a private room.",
     f"""
<p>In Co-op, choose <strong>Create Match (Mods)</strong>, tick the mods for the match and press <strong>Continue</strong>, then pick the map. Players who join see the list and choose <strong>Download</strong> to play with them (a copy is kept in their mods folder, disabled) or <strong>Leave</strong>.</p>
<ul>
  <li>The host sends hash-checked manifests over the encrypted peer-to-peer connection, even to players who don't have those mods installed.</li>
  <li>Received manifests stay isolated in the room; they don't replace installed mods, and author scripts or native code are never sent or run.</li>
  <li>The set is fixed when the room opens; there's no mods menu in the lobby.</li>
  <li>The normal load barrier checks the compiled starting state.</li>
  <li>All players need the same client build and base game data.</li>
</ul>
{table(["Limit", "Value"], [
    ["Data mods per room", "16"],
    ["Manifest text", "24 KiB (40 KiB encoded)"],
    ["Runtime script mods", "Not yet supported"],
    ["Custom art", "Not yet supported (rejected with a message)"],
])}
<p>Official-server interoperability isn't implemented. Use a private RustTD6 room.</p>
""")

# ---------------------------------------------------------------- Reference

page("examples", "Reference", "Examples & downloads",
     "Complete example mods to copy and learn from.",
     f"""
<h2 id="quickstart">Quick Start</h2>
<p>A tower, a bloon and a mode. <a href="/mods/quickstart.py" download>mod.py</a></p>

<h2 id="workshop">Mechanics Workshop</h2>
<p>Moves Dart Monkey after Sniper and adds a workshop button with a custom panel, wave spawner, popup and adjustable
round reward. Download <a href="/mods/workshop.py" download>mod.py</a> and <a href="/mods/mechanics.rhai" download>mechanics.rhai</a>
into <code>mods/workshop/</code>.</p>
{code("python -I tools/modsdk.py build mods/workshop/mod.py", "shell")}

<h2 id="entity-lab">Entity Lab</h2>
<p>Inspects live towers, adds 50 HP to a specific bloon, doubles projectile damage, absorbs five health hits and blocks
Farm purchases, turning each hook on only while its toggle is on. Download <a href="/mods/entity-lab.py" download>mod.py</a>
and <a href="/mods/entities.rhai" download>entities.rhai</a> into <code>mods/entity-lab/</code>.</p>
{code("python -I tools/modsdk.py build mods/entity-lab/mod.py", "shell")}

<h2 id="throwbot">Throwbot</h2>
<p>An original animated robot tower with a custom projectile.</p>
{code("python tools/mod_model.py template mods/throwbot", "shell")}

<h2 id="sdk">SDK</h2>
<p>The SDK ships with the client at <code>tools/modsdk.py</code>. If your copy predates event scripts, use the
<a href="/mods/modsdk.py" download>updated SDK</a>.</p>
<p>After building any example: Mods → Reload Mods → enable it → start a new local match.</p>
""")

page("limits", "Reference", "Limits & roadmap",
     "Every limit in one place, and what isn't implemented yet.",
     f"""
<h2 id="limits">Limits</h2>
{table(["What", "Limit"], [
    ["Manifest size", "64 KiB"],
    ["Operations per mod", "256"],
    ["Installed mod folders", "256"],
    ["Mod ID", "1–64 characters"],
    ["Script source", "32 KiB"],
    ["Operations per callback / per tick", "30,000 / 100,000"],
    ["Commands per callback", "32"],
    ["Persistent script state", "64 KiB"],
    ["UI widgets / nodes / depth / bindings", "64 / 128 / 12 / 32"],
    ["Entity query page", "128 items; 8 per step; 64 pending"],
    ["Model", "60,000 vertices; 600 frames per clip; 128 MiB poses"],
    ["Mod icon", "512 × 512 px, 1 MiB"],
    ["Co-op room", "16 mods, 24 KiB of manifests; 128 art files, 32 MiB"],
])}

<h2 id="not-yet">Not implemented yet</h2>
<ul>
  <li>Fourth (and further) upgrade paths</li>
  <li>Arbitrary new frontend screens (in-match overlays and menu edits work)</li>
  <li>Custom map registration and import</li>
  <li>Arbitrary new native behaviour types and unrestricted engine writes</li>
  <li>New profile counters</li>
</ul>

<h2 id="direction">Direction</h2>
<p>The goal is a general mod platform: arbitrary upgrade graphs, complete per-upgrade behaviour and art, custom maps
with scripted interactions, custom UI with actions, and co-op sharing of everything a mod needs, all through
documented code APIs that reload safely, with no in-game editor and no per-frame Python or DLL injection.</p>
""")


page("units", "Reference", "Units & coordinates",
     "Time, distance, angle and UI units used across the API.",
     f"""
{table(["Quantity", "Unit"], [
    ["Simulation time", "60 steps per simulated second; <code>tick_interval</code> and <code>ctx.tick</code> count steps"],
    ["Weapon <code>rate</code>", "Seconds between shots"],
    ["Projectile <code>velocity</code>", "World units per step"],
    ["Projectile <code>remaining</code>", "Seconds"],
    ["<code>direction</code> (<code>before_fire</code>)", "Radians"],
    ["Entity <code>position</code>", "Native world coordinates"],
    ["<code>ctx.round</code>", "Completed rounds"],
    ["Upgrade paths / tiers", "Paths 0–2, tiers 1–5 (<code>tower.upgrade</code>); tier strings like <code>&quot;100&quot;</code> in helpers"],
    ["Spawn <code>spacing</code>", "Seconds between bloons"],
    ["UI rects", "3840 × 2160 reference canvas, Y up"],
    ["Models", "Game units; the starter robot is about 14 units tall"],
])}

<h2 id="axes">Model axes</h2>
{table(["Source", "Up", "Forward", "Origin"], [
    ["OBJ import", "+Y", "+Z", "Feet at the origin"],
    ["Blender export", "+Z", "−Y", "Feet at the origin"],
])}
<p><code>--muzzle X HEIGHT FORWARD</code> is in the model's own units; <code>--release</code> is seconds into the Attack clip.</p>

<h2 id="ids">IDs</h2>
<ul>
  <li>Content IDs are namespaced by your mod ID (the SDK adds the prefix).</li>
  <li>Entity IDs are stable within a match and are not array positions. Bloons also have a <code>layer</code> ID that changes as layers pop.</li>
  <li>Mode IDs are stable names, not indices.</li>
</ul>
""")

page("troubleshooting", "Reference", "Troubleshooting",
     "What to check when a mod doesn't show up, load or behave.",
     f"""
<h2 id="missing">My mod isn't in the list</h2>
<ul>
  <li>Its folder must be inside <code>mods/</code> next to the game, with a <code>mod.json</code> (build it with the SDK first).</li>
  <li>Press <strong>Reload Mods</strong> after adding or rebuilding.</li>
  <li>Check the manifest rules: lowercase ID, required fields, 64 KiB and 256 operations at most.</li>
</ul>

<h2 id="no-button">There's no Mods button</h2>
<p>Turn on <strong>Settings → Advanced → UI → Show Mods Button</strong>. Hiding the button doesn't disable enabled mods.</p>

<h2 id="no-change">My changes don't appear</h2>
<ul>
  <li>Changes apply to the next new or restarted game. A match in progress keeps its definitions.</li>
  <li>Make sure the mod's switch is on; selecting a row only previews it.</li>
  <li>Official events ignore local content mods.</li>
</ul>

<h2 id="errors">It fails to load</h2>
<p>An invalid content transaction leaves the base data unchanged and writes its error to the client log. Common causes:</p>
<ul>
  <li>A misspelled or missing field in a patch (patches never add new fields).</li>
  <li>Two operations writing the same exact <code>set</code> path, or two mods setting the same Sandbox value.</li>
  <li>A bloon's children referring to something missing, or forming a cycle.</li>
  <li>A content root other than <code>towers</code>, <code>tower_tiers</code>, <code>bloons</code>, <code>modes</code>, <code>start</code>, <code>rounds</code>, <code>round_sets</code> or <code>freeplay</code>.</li>
</ul>

<h2 id="script-stopped">My script stopped working mid-match</h2>
<p>A script disables itself for the rest of the match when it errors, goes over its operation budget, returns an invalid
command batch, returns a bad interceptor decision, or listens for a callback that doesn't exist. Its widgets are removed
too. Fix it, rebuild, reload and start a new match.</p>

<h2 id="coop">A co-op room rejects my mod</h2>
<p>A room takes up to 16 mods with 24 KiB of manifests, and up to 128 art files totalling 32 MiB. A mod with custom
art must sit in a folder named after its ID, and every <code>mod:</code> model it uses must be in that folder's
<code>assets/</code>.</p>

<h2 id="settings">A setting won't save</h2>
<p>Values outside the setting's bounds, or not on its step, are rejected without changing the saved value.</p>
""")

page("faq", "Reference", "Modder FAQ",
     "Quick answers about what the mod system is and isn't.",
     f"""
<h2 id="melon">Do MelonLoader mods work?</h2>
<p>No. RustTD6 is a separate engine with its own mod format; BTD6 MelonLoader mods don't run in it.</p>

<h2 id="python">Does Python run while I play?</h2>
<p>No. Python only builds your manifest. During play the client runs native data and, optionally, sandboxed Rhai scripts.</p>

<h2 id="language">Do I have to use Python?</h2>
<p>No. Any tool that writes a valid <code>mod.json</code> works.</p>

<h2 id="restart">Do I need to restart the game after changing a mod?</h2>
<p>No. Rebuild, press Reload Mods, and start a new match.</p>

<h2 id="assets">Can I include art or sounds from BTD6?</h2>
<p>No. Never bundle or redistribute extracted game content. Reuse the game's art by cloning (each player's own install
supplies it), or ship your own original models.</p>

<h2 id="events">Do mods affect official events?</h2>
<p>No. Official events bypass local content mods.</p>

<h2 id="coop">Can friends use my mod?</h2>
<p>Yes. The host's room sends its mods to every player: data, event scripts and custom models. Joiners are asked
whether to download them; a copy goes in their mods folder, left disabled.</p>

<h2 id="safe">Can a script read my files or go online?</h2>
<p>No. Scripts have no filesystem, network or process APIs, and run under strict operation and memory limits.</p>

<h2 id="paths">Can I add a fourth upgrade path?</h2>
<p>Not yet. The engine and upgrade screen store exactly three paths. See <a href="/docs/limits/#not-yet">what isn't implemented yet</a>.</p>
""")


# ---------------------------------------------------------------- Rendering

GROUPS = []
for p in PAGES:
    if p["group"] not in GROUPS:
        GROUPS.append(p["group"])


def href(p):
    return "/docs/" + (p["slug"] + "/" if p["slug"] else "")


def sidebar(cur):
    out = []
    for g in GROUPS:
        out.append(f'<p class="side-group">{g}</p><ul>')
        for p in PAGES:
            if p["group"] == g:
                active = ' aria-current="page"' if p is cur else ""
                out.append(f'<li><a href="{href(p)}"{active}>{p["title"]}</a></li>')
        out.append("</ul>")
    return "".join(out)


def toc(body):
    heads = re.findall(r'<h2 id="([^"]+)">(.*?)</h2>', body)
    if len(heads) < 2:
        return ""
    items = "".join(f'<li><a href="#{i}">{re.sub("<[^>]+>", "", t)}</a></li>' for i, t in heads)
    return f'<aside class="toc"><p>On this page</p><ul>{items}</ul></aside>'


def render(i, p):
    prev = PAGES[i - 1] if i > 0 else None
    nxt = PAGES[i + 1] if i + 1 < len(PAGES) else None
    pager = '<nav class="pager" aria-label="Previous and next">'
    pager += f'<a class="prev" href="{href(prev)}"><span>Previous</span>{prev["title"]}</a>' if prev else "<span></span>"
    pager += f'<a class="next" href="{href(nxt)}"><span>Next</span>{nxt["title"]}</a>' if nxt else "<span></span>"
    pager += "</nav>"
    title = "Mod Docs | RustTD6" if not p["slug"] else f'{p["title"]} | RustTD6 Mod Docs'
    nav = sidebar(p)
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="theme-color" content="#2f80f5">
<title>{html.escape(title)}</title>
<meta name="description" content="{html.escape(p["summary"])}">
<link rel="canonical" href="https://rusttd6.com{href(p)}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="RustTD6">
<meta property="og:title" content="{html.escape(title)}">
<meta property="og:description" content="{html.escape(p["summary"])}">
<meta property="og:url" content="https://rusttd6.com{href(p)}">
<meta property="og:image" content="https://rusttd6.com/social.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="RustTD6: Bloons TD 6, on a new engine.">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="/logo.png" type="image/png">
<link rel="apple-touch-icon" href="/logo.png">
<link rel="preload" href="/fonts/Inter.ttf" as="font" type="font/ttf" crossorigin>
<link rel="stylesheet" href="/docs/docs.css">
</head>
<body>
<header class="bar"><div class="bar-inner">
  <a class="logo" href="/docs/">RustTD6 <span>Mod API</span></a>
  <nav class="bar-links" aria-label="Site">
    <a href="/">Home</a>
    <a href="{GH}">GitHub</a>
    <a class="pill" href="/docs/quick-start/">Get started</a>
  </nav>
  <details class="menu">
    <summary aria-label="Open menu"><span class="bars" aria-hidden="true"><i></i><i></i><i></i></span></summary>
    <nav class="menu-panel" aria-label="Documentation">{nav}<p class="side-group">Site</p><ul><li><a href="/">Home</a></li><li><a href="{GH}">GitHub</a></li></ul></nav>
  </details>
</div></header>
<div class="layout">
  <nav class="side" aria-label="Documentation">{nav}</nav>
  <main class="content">
    <p class="crumb">{p["group"]}</p>
    <h1>{p["title"]}</h1>
    <p class="summary">{p["summary"]}</p>
    {p["body"]}
    {pager}
    <footer class="foot">Unofficial fan project. Not affiliated with Ninja Kiwi. <a href="/">rusttd6.com</a></footer>
  </main>
  {toc(p["body"])}
</div>
<script src="/docs/nav.js" defer></script>
</body>
</html>
"""


def main():
    for i, p in enumerate(PAGES):
        d = os.path.join(OUT, p["slug"]) if p["slug"] else OUT
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "index.html"), "w", encoding="utf-8", newline="\n") as f:
            f.write(render(i, p))
    print(f"wrote {len(PAGES)} pages to {OUT}")


if __name__ == "__main__":
    main()
