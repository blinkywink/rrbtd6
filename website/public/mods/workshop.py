"""Build: python -I tools/modsdk.py build tools/mod_examples/workshop/mod.py --out mods/workshop/mod.json"""
mod = Mod("workshop", "Mechanics Workshop", description="Event scripts, a custom panel, wave buttons, round bonuses and shop ordering.")
mod.existing_tower("DartMonkey").shop(after="SniperMonkey", category="military")
mod.script("mechanics", file="mechanics.rhai", tick_interval=60)
