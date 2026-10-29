mod = Mod("entity-lab", "Entity Lab", description="Inspect towers, heal a bloon, boost shots, absorb hits, and veto Farm purchases.")
mod.script("entities", file="entities.rhai", tick_interval=60)
