# Build with: python tools/modsdk.py build mods/quickstart/mod.py
# Mod is supplied by the SDK runner. This file is never executed by the game.
mod = Mod("quickstart", "Quick Start", description="A custom dart, bloon and mode.")
dart = mod.tower("dart", base="DartMonkey", cost=100)
dart.projectile(damage=3, pierce=4)
dart.weapon(rate=0.5)
mod.bloon("durable-red", base="Red", maxHealth=5)
mod.mode("training", base="Medium", cash=2000, lives=300)
