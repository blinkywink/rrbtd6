# Things to test by hand

Added without a hands-on check. Tick them off (or note what's wrong) when you play. Newest at the top.

## Setup and install (Claude, 2026-11-01)

Checked from files and tests, not yet in a real install from a release:

- [ ] Installer in BTD6: welcome dialog looks sharp, Comic Sans titles, "Create a desktop shortcut" works
      ticked and unticked.
- [ ] Progress window during setup: bar, stage, time left and tips change; it closes when the game starts.
- [ ] Full setup in one run on the final code finishes. Note the time, and how much memory was free.
- [ ] New player: Geraldo's shop items have models; the profile page opens from the main-menu avatar; map
      cards and rewards show; hero icons and event medals show.
- [ ] Placement Cancel and Lock & Drop buttons animate on a fresh install (RightPanel).
- [ ] After a BTD6 update, setup re-exports models, maps and audio.

## In-game fixes (Claude, 2026-11-01)

- [ ] Paragons animate instead of T-posing, at every degree: Glaive Dominus, Ascended Shadow, Navarch, Sniper
      Paragon, Magus Perfectus. Also Dan D Monke and Druid's tree. Their poses look natural.
- [ ] Bloons are the right size: plain ones about 5% smaller than before, Regrow ones no longer wide.
- [ ] Sandbox test bloons: Test Bloon is a normal-sized dashed outline; Test MOAB is the right size and faces
      along the track (check it isn't backwards).
- [ ] Sandbox: with the bloon menu open, clicking the map closes it and doesn't select a tower.
- [ ] Tower "i" button shows a description for all 3 paths at once, on any crosspath (try a 4-2-0).
- [ ] Holding the mouse on an owned upgrade icon shows its description; releasing hides it; no press animation.
- [ ] Sounds and music play as normal (now Ogg).
- [ ] Memory failsafe: send a huge number of ZOMGs in the sandbox: the bloons clear with a red message at the top, and
      Windows stays fine, with the display's refresh rate and scaling unchanged. Normal games are unaffected.
- [ ] Trophies replace Monkey Money: the menus' balance shows trophies with the trophy icon; the difficulty screen's rewards
      are trophies; winning a map shows +N on the victory screen and the balance goes up by N.
- [ ] Powers and Knowledge: prices show the trophy icon; buying takes the trophies and adds the Power or learns the Knowledge;
      without enough trophies nothing happens; purchases survive a restart.
- [ ] Scrollbars, input fields and dropdowns draw their real sprites instead of plain boxes, and masked lists
      still clip properly (Unity's built-in UISprite, Background, InputFieldBackground, UIMask).
- [ ] Victory screen: award callouts show their own icon, not the fallback.
- [ ] Geraldo's Genie Bottle: the genie floats above the bottle and its shots show their effect, both versions.
- [ ] Adora's three BFB warning lines play (base, Joan, Void skins).
- [ ] Map spawning: Chutes or Ravine change path by round; on Quiet Street or Sunset Gulch MOABs take their own
      path while bloons don't; Adora's Temple changes group each round. Nothing spawns off a path or stalls.
- [ ] Bloody Puddles, Chutes, Kart's n Darts, Rake, Streambed: no helicopter rotors floating at the map centre.
- [ ] Removeables: click Bazaar's palm trees: a red "Remove for $500" button shows; red text and no effect without the
      cash; with it, $500 goes, the trees vanish and towers can be placed there. A new game brings them back.
      Clicking elsewhere closes the button. Try Chutes' statues and Quiet Street's cars too (these just vanish for
      now: their animations don't play yet).
- [ ] Cargo: after round 40, or at the first MOAB before that, the second path turns on and MOABs use it; its two
      $500 removeables can then be bought.
- [ ] Placement on maps with areas that start off (One Two Tree, Spa Pits, Dark Path, Encrypted): spots that were wrongly
      blocked are free now, until an event creates the area.

Known, low priority: the scarf on Ace 0-3-1 / 1-3-0, Ezili's totem and the Inferno Ring base still don't animate.

## Tower placement: Drag & Drop / Lock & Drop (Claude)

- [ ] Drag & Drop (pause menu default): drag a tower out of the shop and let go on a free spot: it's placed. Pick one up with its hotkey and click the map: it's placed when the button is released, not pressed.
- [ ] Drag & Drop: let go on the track (red): the tower stays where it was dropped. Press on it (within ~10 units) and it follows the cursor again; press anywhere else and dragging nudges it at a third of the mouse's speed. Releasing tries again.
- [ ] While a tower is in hand, a red X slides in left of the shop (left of the bottom bar on 4:3); clicking it cancels. Letting go over the HUD cancels too.
- [ ] Lock & Drop: letting go never places; the tower stays put and a green tick slides in below the X. The tick places it (on a valid spot). Note what the real game does when the tick is clicked on a red (invalid) spot: the clone follows the code and drops the tower (the release lands off the world).
- [ ] Not done yet: Accessibility "Tower Placement Snapping" (snap back to the last valid spot, its ghost and sound) is not implemented, so placement behaves as with it off.

## Difficulties (Claude, session 5)

- [ ] Press D at the start (Shift+D back) to pick Easy / Medium / Hard / Impoppable / CHIMPS. Check the starting cash and lives (Easy 200 lives, Medium 150, Hard 100, Impoppable / CHIMPS 1), the first round (Hard starts at 3, Impoppable / CHIMPS at 6) and the last (40 / 60 / 80 / 100).
- [ ] Prices: Dart Monkey $170 Easy, $200 Medium, $215 Hard, $240 Impoppable. Bloons are faster on Medium than on Easy, and faster again on Hard.
- [ ] CHIMPS: no round-end cash, you can't sell, and Banana Farms can't be bought.

## Maps and Paragons (Claude, session 5)

- [ ] Press N at the start to step through every BTD6 map (about 80). Each shows its own scene and track. Bloons should follow the roads. On maps with several entrances they come from all of them in turn. Note maps where bloons go somewhere odd, or where placing is wrong. Monkey Meadow and Logs are the reference, and the rest are experimental.
- [ ] Paragon: get one tower of a kind to 5-x-x, another to x-5-x and another to x-x-5. Then the tier-5 upgrade button on one of them offers the Paragon (e.g. Apex Plasma Master for the Dart Monkey, $150,000 on Medium). Buying it removes the others and turns that tower into the Paragon with its own model.
- [ ] The Paragon's degree: with just the three tier 5s it's degree 1. More pops on the sacrificed towers, more money spent on lower towers and extra tier 5s raise it. A higher degree attacks faster and hits harder.

## Hero abilities (Claude, session 5)

- [ ] Churchill (level 10) MOAB Barrage: the strongest MOAB-class bloons (up to 10) take repeated hits for ~9 s.
- [ ] Churchill (3) Armor Piercing Shells: for ~9 s his shells pop Black bloons and do more to Ceramics and MOABs.
- [ ] Benjamin (3) Biohack: the 4 towers nearest him do +1 damage for 6 s. (10) Syphon Funding: for 10 s, new bloons come in one rank lower.
- [ ] Adora (3) Long Arm of Light: her range doubles and her bolts get bigger for 10 s.
- [ ] Etienne (3) Drone Swarm: 4 extra drones for 22 s. (10) UCAV: a big drone for 18 s.
- [ ] Brickell (10) Mega Mine: press it, click a spot near the track; a mine appears and blows up big when a bloon reaches it. (7) Blast Chain: she attacks much faster for 15 s.
- [ ] DanDMonke (12) Quick Thinking: leaks cost no lives for 6 s.
- [ ] Sauda (10) Sword Charge: something rushes back along the track from the exit and pops everything it meets. (3) Leaping Sword: a hit lands on the leading bloon after half a second.
- [ ] Easy, round 40: the MOAB goes down faster (it has 133 health instead of 200).
- [ ] A Paragon's look changes as its degree goes up (every 20 degrees, 5 looks).

## Logs map and water towers (Claude, session 5)

- [ ] Press N at the very start to switch to Logs (Shift+N back). The map, its track and its pond show; the choice is remembered.
- [ ] On Logs: Monkey Sub and Buccaneer only go in the water, land towers not; the Mermonkey goes on either and has a bigger range circle in the water.
- [ ] Buccaneer: the ship turns side-on to its target and fires out of that side; when bloons are also on the other side, that side fires at the same time. It should not machine-gun (it used to fire every 0.05 s).
- [ ] Merchantman / Favored Trades pay cash at round end. With Favored Trades near another tower, that tower's sell price goes up (Backspace / Sell shows it).
- [ ] Trade Empire: an extra "+$" over it at round end ($10 per other merchant ship, $20 per Favored Trades). Carrier Flagship makes towers in water attack faster.
- [ ] Sub with Advanced Intel (2-x-x) shoots bloons anywhere another tower can see.
- [ ] Sub with Submerge and Support (3-x-x): the targeting arrows include "Submerge". The sub sinks, stops shooting darts and pulses camo off bloons around it. Bloontonium Reactor (4-x-x) pulses also pop bloons; Energizer speeds up ability cooldowns.
- [ ] Mermonkey: tridents that fly far do more damage. Alluring Melody (x-x-3): every ~12 s, bloons near it (not MOABs) leave the track and circle the Mermonkey for 6 s, then go back where they were.
- [ ] Aircraft Carrier (4-x-x): up to 3 small planes launch and fly over the bloons shooting, turning round for another pass. They vanish when the ship is sold. Flagship (5-x-x) launches its own planes.
- [ ] Late-game crowds: lots of bloons popping at once (Ring of Fire, big blasts) should no longer hitch. The sim now runs about 2x faster and its worst frames are about 8x shorter.
- [ ] Monkey Pirates (x-4-x) ability: the strongest MOAB / BFB / DDT on screen vanishes at once and you get double its pop cash. Pirate Lord (x-5-x): up to 6 hooks, a ZOMG takes 3.
- [ ] Carrier Flagship (5-x-x): drop a Dart Monkey (or most towers) onto the ship's deck; it stands on the deck and shoots. Banana Farms, Aces and Helis can't go there. Selling the ship sells them too. Is the deck where the model's deck is? (If the spots are mirrored or behind the ship, tell me.)
- [ ] Sub First Strike (x-4-x) ability: a missile hits the strongest bloon ~0.6 s later and takes out up to a ZOMG. Pre-emptive Strike (x-5-x): every MOAB-class bloon gets hit by a strike as soon as it appears.
- [ ] Sub Commander (x-x-5): Subs near it (and itself) do double damage with more pierce.
- [ ] Not done yet (don't report): plane rolls and loops, the hook reel-in animation, First Strike / Pre-emptive Strike, Sub Commander, Mermonkey heat / totems / abilities, sink animation.

## All heroes (Claude, forked session)

- [ ] Press H at the very start (before placing anything) to switch hero; Shift+H goes back. The shop's hero slot, its price and portrait change, and your choice is remembered next time. All 18 heroes are there.
- [ ] U picks up the chosen hero. Each hero has its own model per level, portraits that change at the levels the game has art for, and its level descriptions.
- [ ] Every hero attacks except Benjamin (he makes cash instead).
- [ ] Etienne: his drone flies out and chases bloons (it chases the first bloon on the map; in the game it follows his targeting). More drones at higher levels.
- [ ] Psi: grabs a bloon, holds it still and crushes it over time.
- [ ] Silas: ice shots coat bloons in Rime (slower; his next hits on them do +1 damage). The Rime freeze/shatter isn't done.
- [ ] Adora: homing bolts.
- [ ] Not done yet (don't report): a proper hero select screen, hero abilities beyond what the generic ability system covers, hero-specific passive buffs to other towers, Admiral Brickell (water), Benjamin's cash/hacks, Geraldo's shop, Corvus's spellbook, Rosalia's workshop, hero voice lines.

## More abilities (Claude, forked session)

- [ ] Wizard Summon Phoenix (0-4-0): a phoenix appears and circles the middle of the map for 20 s, shooting fireballs everywhere. The circle's start point and direction are my guess.
- [ ] Wizard Lord Phoenix (0-5-0): the wizard turns into the phoenix form and stops its own attacks for 20 s, and a big lava phoenix circles the map.
- [ ] Alchemist Transforming Tonic / Total Transformation: the alchemist turns into the blue monster with more range for 20 s.
- [ ] Engineer Overclock (0-4-0) / Ultraboost (0-5-0): works between rounds too. Press the ability: "Select a Monkey" slides in at the top, green markers sit on every tower it can boost, the cancel button shows. Click a marked tower: a pulse, then the Overclock buff effect on it while it attacks faster (60 s; shorter on tier 4 and 5 towers). Clicking a tower it can't boost (a Sentry, a paragon...) says "Invalid Target" and keeps waiting; clicking empty ground does nothing. Right-click, Esc or the cancel button cancels. Ultraboost also leaves a small permanent speed-up that stacks up to 10 times.
- [ ] Desperado Take Aim: press it, then click a tower to give it more range for 15 s.
- [ ] Wizard portraits and the Guided Magic icon show (they were blank).
- [ ] Super Monkey Darkshift (Dark Knight / Dark Champion): press it, then click a free spot within a short distance; the monkey teleports there. Too far or blocked spots do nothing.
- [ ] Heli Pilot Redeploy: press it, then click any free spot; the helipad moves there.
- [ ] Legend of the Night: for 8 s leaking bloons cost no lives.
- [ ] Skywarden Thunder Charge: towers in its range shoot much faster projectiles for 9 s and can pop frozen bloons. The bouncing part isn't done.
- [ ] Desperado Marked To Pop: nothing for 2 s (marking), then a burst of strong shots for 1 s. The mark glow and the double damage/cash on marked bloons aren't done.
- [ ] Sniper Supply Drop: a plane drops a $1,100 crate that you collect by moving the cursor over it.

## Skywarden (Claude, forked session)

- [ ] Skywarden is in the shop between Druid and Banana Farm. It has its model, upgrade models, icons, portraits and proper upgrade names.
- [ ] It shoots one arrow volley at a time (its Spread stance), not two.
- [ ] Press G with a Skywarden selected to switch to Focus stance: bigger range, slightly slower, long arrows. G again switches back. (The game has an on-screen stance button; that isn't drawn yet.)
- [ ] It speeds up the longer it keeps hitting bloons (+5% per 10 pops, up to 20 times) and loses that after 2 s without hits.
- [ ] Middle path (Storm's Pulse and up): arcs land on the target and burst; the burst gets bigger the further the arc flew.
- [ ] Not done yet (don't report): Chilled, Galvanized, Windlash and Heat effects, lightning rods, Winter's Mercy, the range-sharing aura.

## Beast Handler and Monkey Fan Club (Claude, forked session)

- [ ] Beast Handler is the last tower in the shop. Its pet appears next to it inside its range, and the pet model changes with each upgrade (Microraptor, Velociraptor, T-Rex, Giganotosaurus; Gyrfalcon, Horned Owl, Golden Eagle, Giant Condor, Pouakai).
- [ ] Dinosaur pets pop bloons near them.
- [ ] Bird pets fly out to a bloon in range, grab it (the higher tiers grab several, Pouakai even MOABs), hold it still and keep hitting it, then grab more nearby or fly back. The dive point is picked automatically (in the game you choose it), and carried bloons stay where they were grabbed (both P-BIRD).
- [ ] The fish path (Piranha, Megalodon...) can't be bought on this map: it needs water.
- [ ] Selling the handler removes its pet. Upgrading swaps the pet for the new one.
- [ ] Pouakai's upgrade icon shows (it was blank).
- [ ] Not done yet (don't report): pet power levels from merging handlers, picking the bird's dive point, the dino stomp abilities.
- [ ] Super Monkey Fan Club (Dart 0-4-0): the ability turns the Dart Monkey and up to 9 nearby Dart Monkeys with no upgrade above tier 2 into caped Super Monkey fans for 15 s. They throw much faster, then turn back. Plasma Monkey Fan Club (0-5-0) makes up to 20 plasma fans. Which towers count as "nearby" is my guess (P-FANCLUB).

## Quincy, volume sliders, jukebox, portable copy (Claude, forked session)

- [ ] Quincy (U key, or the first shop slot, $540): only one hero per game. He starts at level 1, and after each round he gets XP (40 after round 1, 60 after round 2, ...). He levels up on his own at 180 / 460 / 1000 ... XP, plays the upgrade chime, and changes model at levels 3, 7, 10 and 20.
- [ ] Click Quincy: his menu shows the hero panel instead of three upgrade paths. It has a level badge, a blue XP bar, "Level N" with that level's description, the next level's description, and an "Unlock Next Level" button. That button costs $1 per missing XP, at least $11.
- [ ] Number keys still start at the Dart Monkey (1 = Dart, 2 = Boomerang, ...). Heroes are skipped; U picks the hero.
- [ ] Pause screen (Esc / gear): drag the Music, Sound Effects and Hero Voice sliders. The percent updates and the volume changes live, and the values are remembered next time you start the game.
- [ ] Jukebox on the pause screen shows the playing track's name. Next / previous skip tracks, loop repeats the current track, and shuffle picks a random next track.
- [ ] Portable copy: unzip `BTD6-rrbtd6.zip` (Desktop) on another Windows PC and run `btd6-game.exe`. It needs no install. It should look and play the same.
- [ ] Not done yet (don't report): Quincy's Rapid Shot / Storm of Arrows ability buttons beyond what the generic ability system gives, his portrait in the menu, hero voice lines, the other 16 heroes.

## Abilities

- [ ] Upgrade a tower to an ability tier (e.g. Spike Factory 0-4-0 Spike Storm, Boomerang 0-4-0 Turbo Charge, Ice 0-4-0 Snowstorm, Glue 0-4-0 Glue Strike, Super 0-4-0 Tech Terror, Bomb 0-4-0 Assassinate MOAB). A round button with the upgrade's icon appears at the bottom left of the map, dark with a countdown at first.
- [ ] The countdown only runs during rounds. When it's done the button lights up (yellow ring); clicking it during a round uses the ability and plays its sound. Clicking it shouldn't place or select anything underneath.
- [ ] Spike Storm covers the track with spikes; Turbo Charge makes the boomerangs fly much faster for 10 s; Snowstorm freezes everything once; Glue Strike glues everything.
- [ ] Two towers with the same ability share one button with a number for how many are ready.
- [ ] Banana Farm 0-4-0 IMF Loan: +$9,000 at once, a red "-$9,000" under the cash, and half of every later income (pops, round bonus, bananas) pays it off until it's gone.
- [ ] Village 0-4-0 Call to Arms: towers in the village's range attack ~1.5x faster for 15 s. Ice 0-5-0 Absolute Zero also speeds up every Ice Monkey for 10 s.
- [ ] Not done yet (don't report): Supply Drop, Darkshift and other special abilities (their buttons appear but do nothing beyond any attack they include), number-key hotkeys.

## Monkey Ace (key 9)

- [ ] The number keys moved again: 9 is the Monkey Ace, 0 the Heli Pilot (Mortar and Dartling are shop-only now).
- [ ] Placing an Ace puts down its runway; the plane takes off, joins a circle round the runway and keeps flying, shooting 8 darts in all directions every so often (whether or not bloons are near).
- [ ] The targeting arrows switch Circle / Figure Eight / Infinite. Figure Eight is upright and Infinite lies sideways; both pass over the runway. Changing the mode makes the plane fly over to the new path. Are the shapes and sizes close to the game's?
- [ ] Neva-Miss Targeting (x-x-3) and Flying Fortress add "Centered Path": a big circle round the map centre. The square Set Target button moves its centre.
- [ ] Upgrades show each tier's plane model (Fighter Plane, Bomber Ace, Sky Shredder, Tsar Bomba, Flying Fortress...) and runway. Fighter Plane / Sky Shredder fire missiles at the strongest bloon; Flying Fortress's guns shoot First / Close / Last.
- [ ] Do the projectiles look right? Some may show the wrong picture (the asset catalog suggested plane prefabs for a couple of them).
- [ ] Not done yet (don't report): bombs and pineapples only over the track, Ground Zero / Tsar Bomba abilities, take-off animation.

## Heli Pilot (key 0)

- [ ] Placing a Heli puts down a heli pad; the helicopter lifts off and flies after your cursor ("Follow Touch"). It speeds up, slows down near the cursor and turns to face where it's going, or towards bloons while shooting.
- [ ] The range circle follows the helicopter, not the pad. The darts come from the helicopter.
- [ ] Targeting arrows switch between "Follow Touch" and "Lock In Place" (plus "Pursuit" from tier 3 on either side path). In Lock In Place the square "Set Target" button appears; set a spot and the heli flies there and stays. Before you set one it holds where it was.
- [ ] Pursuit: the heli chases the bloon furthest along the track.
- [ ] Several Helis following the cursor spread out instead of stacking on one spot.
- [ ] Upgrades: each tier and crosspath shows its own helicopter model (e.g. 1-0-1, Downdraft, Apache Prime, Comanche Commander) and the pad changes at tier 4 and 5. Razor Rotors (3-x-x) pops bloons right under the heli.
- [ ] Is the helicopter's height and size about right next to the game? (Guessed: it hovers 40 units up.)
- [ ] Not done yet (don't report): Patrol Points mode, Downdraft blowing bloons back, MOAB Shove, the Comanche Defense mini-helis, Support Chinook and Special Poperations abilities, IFR's camo.

## Engineer Monkey (shop only, after the Village)

- [ ] Engineer places and shoots nails; upgrades show the right models, icons and portraits.
- [ ] Sentry Gun (1-x-x): during rounds the Engineer throws a small sentry onto the grass nearby (never onto the track) every 10 s (5 s from Faster Engineering). Each sentry shoots on its own, turns to aim, and disappears after 25 s. Its pops count towards the Engineer's pop count.
- [ ] Sentry Expert (4-x-x): the thrown sentries take turns being green (spiky balls), red (bombs), blue (cold) and purple (energy). Sentry Champion (5-x-x) throws the champion sentry.
- [ ] Sentries can't be clicked or sold, and you can place towers on top of them. Clicking near one shouldn't select it or the wrong tower; selecting a tower while sentries expire shouldn't jump to another tower.
- [ ] Cleansing Foam (x-3-x): foam patches land on the track spot nearest the Engineer. The square "Set Target" button and the red marker appear; set the spot and foam should land there. The targeting arrows still switch First / Last / Close / Strong.
- [ ] Bloon Trap (x-x-4): a trap lands on the track spot nearest the Engineer and swallows bloons (they vanish, MOABs pass). Only one trap at a time until it's full (500 RBE; XXXL 10,000). A full trap stays put; hover over it to collect the cash (2x the RBE caught, 3x for XXXL). Then a new one is thrown. Is the payout in the right ballpark?
- [ ] Not done yet (don't report): Sentry Champion explosion on expiry, the full-trap picture.

## Window size and performance (Claude, forked session)

- [ ] Resize the window to any shape. The whole map should always be visible. Narrow or tall windows get bars above and below, very wide ones get bars at the sides with a thin dark border at the map edge. Nothing on the map should be cut off.
- [ ] Late rounds or big crowds should stay smooth. A flood test with ~6k Ceramics ran at ~120 fps, and ~16k at ~40 fps. Note any stutter, especially when lots of bloons pop at once.
- [ ] Bloons should still look the same (draw order, tints for frozen and glued). Their drawing was batched for speed.

## Mortar Monkey (key 9) and Dartling Gunner (key 0)

- [ ] The number keys moved again: 9 is the Mortar, 0 the Dartling (Wizard and Super are shop-only now).
- [ ] Placing a Mortar asks for a target straight away: the red target marker follows the cursor, click the map to set it (right click / Esc keeps the default: the track spot nearest the Mortar).
- [ ] The Mortar's upgrade menu has a long green "Set Target" button instead of targeting arrows; the marker shows where it's aimed while it's selected. Is the marker the right size?
- [ ] Shells land around the target (a little random spread; tighter with Increased Accuracy) half a second after firing, and explode. Burny Stuff sets bloons on fire, Signal Flare removes camo, Shattering Shells strips Fortified. The Big One / The Biggest One make huge blasts.
- [ ] The Mortar fires whenever bloons are on the map, even if none is at the target.
- [ ] Dartling: the gun turns to follow your cursor (not instantly) and sprays darts that way while bloons are on the map. With the cursor over the HUD it keeps aiming at the last map spot.
- [ ] Dartling targeting: "Normal" (cursor) and "Locked". In Locked a small square button with a target icon appears; click it, then the map, to lock the aim point.
- [ ] Dartling upgrades: Laser Shock/Cannon, Plasma Accelerator and Ray of Doom fire a beam that crosses the map (no beam art yet; bloons on the line pop). Hydra Rocket Pods / Rocket Storm missiles explode on each hit. Buckshot sprays short-range pellets.
- [ ] Not done yet (don't report): Pop and Awe, Rocket Storm / M.A.D abilities, Artillery Command, beam / shell / explosion art beyond a glow.

## Sound, music, MOAB models, crosspath label

- [ ] Music plays in game (the game's own tracks, one after another). M mutes / unmutes it.
- [ ] Pops sound like the game: one pop sound at a time even when lots pop (the game's limit), louder crunch when a Ceramic breaks, metal "thunk" on MOAB hits, a big pop when a MOAB is destroyed.
- [ ] Placing a monkey / structure (Tack, Bomb, Spike Factory) / Ice / Banana Farm each has its own place sound; upgrades play the upgrade chime, selling the sell sound.
- [ ] Bomb explosions, Ice freezes, Glue splats, Druid lightning / Ball Lightning, Alchemist potions have their sounds. Many towers (Dart, Sniper base, Desperado pistol) have no attack sound in the game's data, so silence there is expected.
- [ ] HUD buttons click. Winning round 60 plays the victory jingle.
- [ ] Too loud / too quiet / sounds piling up? Note which.
- [ ] MOABs, BFBs, ZOMGs, DDTs and BADs are the game's 3D models (3D view, the default), facing along the track, and switch to the cracked damage models at 80/60/40/20% health. Fortified ones have the armour plating. Check the size and height look right next to the towers.
- [ ] Upgrade panel: at 2-3-0 the top path shows "Max Upgrades" (not "Path Closed"); only the unused third path says "Path Closed".

## Desperado (key 7)

- [ ] The number keys moved: 7 is now the Desperado, 8 Sniper, 9 Wizard, 0 Super. Ninja is shop-only.
- [ ] Pistol: two quick shots (~0.1 s apart) per attack, drawn as tracer lines. More shots per burst further up the top path (6 at 3-x-x).
- [ ] Rifle (x-3-x+): long-range shots only at bloons farther than 40 units, piercing everything on a line (a long tracer). The pistol still shoots close bloons at the same time.
- [ ] Shotgun (x-x-3+): a spread of pellets.
- [ ] Upgrade icons show in the upgrade panel and the upgrades screen (they're named by tier in the game files).
- [ ] Not done yet (don't report): Standoff / distance rate bonuses, crits, marks, Vigilante, Bounty Hunter, The Blazing Sun, Take Aim.

## Alchemist (shop only, no number key)

- [ ] Potions arc to bloons and leave a red acid splash; acid keeps popping layers for a few seconds.
- [ ] Acid Pool (x-x-2+): with no bloons in range it throws red puddles onto the track that pop bloons running through them (about 7 s each).
- [ ] Acidic Mixture Dip (2-x-x): during rounds it "dips" nearby attacking towers (a flash on the tower). Their next 30 shots do +1 to Ceramics, MOABs and Fortified Lead. Farms, Villages and other Alchemists are skipped.
- [ ] Berserker Brew (3-x-x): towers it brews get +1 damage, +2 pierce, +10% attack speed and range (the range circle grows) for 25 shots or 5 s. Stronger Stimulant (4-x-x): 40 shots or 12 s, +3 pierce, +15%. Permanent Brew (5-x-x): forever.
- [ ] Lead to Gold (x-x-3): popping a Lead bloon with the acid pays $50 ("+$50" floats up). Lead and DDTs take +9 from the splash.
- [ ] The upgrade panel's "Faster Throwing" icon shows now (it was blank).
- [ ] Not done yet (don't report): Rubber to Gold, Bloon Master Alchemist's shrinking, Unstable Concoction explosions, Transforming Tonic / Total Transformation, brew potions flying to towers, buff icons.

## Druid (shop only, no number key)

- [ ] Thorns: a spray of thorns in a small cone towards the target. Purple from Heart of Thunder (2-x-x), green from Heart of Oak (x-2-x), Poplust / Avatar of Wrath thorns at x-x-4 / x-x-5.
- [ ] Heart of Thunder (2-x-x): lightning jumps from bloon to bloon (up to 5 hits). There's no lightning art yet, so you'll only see bloons pop.
- [ ] Druid of the Storm (3-x-x): a tornado (a swirly stand-in sprite) that blows bloons back and removes glue / freeze from them.
- [ ] Ball Lightning (4-x-x): a slow ball that zaps a random nearby bloon every ~0.35 s.
- [ ] Superstorm (5-x-x): bigger tornado.
- [ ] Druid of the Jungle (x-3-x): every ~2.6 s a vine grabs any non-MOAB bloon anywhere on screen and destroys it, then leaves a root that pops bloons passing over it for 4.5 s. In the real game the bloon is held for 1.4 s first.
- [ ] Not done yet (don't report): Heart of Oak regrow removal, Heart of Vengeance / Avatar of Wrath speed-up from lost lives, Poplust buff, Jungle's Bounty cash, Spirit of the Forest track vines, all abilities, lightning art.

## Spike Factory, Banana Farm, Monkey Village (shop only, no number key)

- [ ] Spike Factory: during a round it throws spike piles onto the track in its range; a pile shrinks one spike per pop and vanishes at round end.
- [ ] Faster Production (x-1-x+) throws 4x as fast for the first few seconds of each round (x-0-3: 3 s, x-0-5: 10 s).
- [ ] Long Life Spikes (0-0-3) survive 1 round end, Perma-Spike (0-0-5) 3. Their lifespans only tick during rounds.
- [ ] White Hot Spikes (2-x-x) show the glowing pile and pop Lead; Spiked Balls (3-x-x) do +3 to Ceramics; Spiked Mines (4-x-x) explode and burn when used up or expired.
- [ ] Banana Farm: 4 bananas per round land 20-40 units from the farm. Move the mouse over one to collect $20 (a yellow "+$20" floats up). They vanish after 15 s.
- [ ] How close the cursor has to be is a guess (12.5 units for normal, ~19 for EZ Collect). Say if it's too fussy or too easy.
- [ ] Bananas come out very fast (the data says one every 0.05 s until the round's count is reached). In the real game they may trickle out over the round. Tell me which it is.
- [ ] Marketplace (0-0-3+) pays straight to your cash with no bananas. Banana Crates (4-x-x) are worth $300.
- [ ] Monkey Bank (x-3-x): the upgrade panel's title shows "Bank $x / 7000"; press C with the bank selected to collect. 15% interest each round. Selling also pays the bank out.
- [ ] Monkey Village: towers in its radius get +10% range (the range circle grows), Jungle Drums (2-x-x) +15% attack speed, Primary Training (3-x-x) +1 pierce and faster darts for Primary towers, Radar Scanner (x-2-x) camo detection, MIB (x-3-x) lets everything pop Lead, Black, White, Frozen and Purple.
- [ ] Monkey Business / Commerce (x-x-1/2): towers bought or upgraded (up to tier 3) in its radius are 10% / up to 25% cheaper; Monkey Town (x-x-3) +50% pop cash for towers in range.
- [ ] Shinobi Tactics (Ninja x-3-x) now speeds up Ninjas near it (8% each, stacking up to 20).
- [ ] Wizard bolts, fireballs and Dragon's Breath now show a sprite (the game draws them as particles; this is the first frame of their texture).
- [ ] Not done yet (don't report): Spike Factory's Close / Smart / Far / pick-a-spot target modes, Spike Storm / Carpet of Spikes, IMF Loan, Banana Central / Central Market buffs, Monkey-nomics, Grow Blocker, Call to Arms, Homeland Defense, Monkey City income, Monkeyopolis, Primary Mentoring's free upgrade.

## Wizard Monkey (key 9) and Super Monkey (key 0)

- [ ] Wizard: purple bolts; Guided Magic (1-x-x) bolts curve to bloons; Arcane Blast (2-x-x) has the bigger blast art.
- [ ] Fireball (x-1-x): every ~2 s a fireball that explodes in a small area.
- [ ] Wall of Fire (x-2-x): an orange disc appears on the track closest to the Wizard every ~6.5 s and burns bloons for 4.5 s.
- [ ] Dragon's Breath (x-3-x): a fast stream of short-range flames.
- [ ] Shimmer (x-x-3): camo bloons near the Wizard (big radius) lose camo every 2 s.
- [ ] Super Monkey: very fast darts; Laser / Plasma / Sun Avatar art per tier; Robo Monkey (x-3-x) shoots from two arms (two streams).
- [ ] Dark Knight (x-x-3): spinning blades.
- [ ] Not done yet (don't report): Phoenix, Necromancer, Sun Temple sacrifices, all abilities.

## Ninja Monkey (shop only now)

- [ ] Shurikens spin and hit camo bloons; the Ninja picks camo bloons first.
- [ ] Seeking Shuriken (x-x-1+): shurikens curve onto their target and pick a new one if it pops (the turn is instant, as the data says, so it may look snappy).
- [ ] Sharp Shurikens / Double Shot (2-x-x+): the fan of shurikens wobbles a little (random spread up to 5°).
- [ ] Distraction (x-2-x+): about 1 in 7 hits blows a bloon back along the track (less for Ceramics, never MOABs).
- [ ] Counter-Espionage (x-2-x+): camo bloons it damages lose camo for good, children too. They'll still look camo (only the logic changes).
- [ ] Caltrops (x-x-2+): every ~4 s a caltrop flies onto the track near the Ninja and pops bloons that run over it.
- [ ] Flash Bomb (x-x-3+): every 4th throw is a bomb that stuns a group for 1.3 s; stunned bloons take +4 from shurikens.
- [ ] Sticky Bomb (x-x-4): a bomb sticks to a MOAB and goes off after ~3 s for 450 plus a splash. Master Bomber (x-x-5) throws them map-wide for 3000 and stuns.
- [ ] Not done yet (don't report): Shinobi Tactics' buff to other Ninjas, Bloon Sabotage / Grand Saboteur abilities, Grandmaster's damage-percent bomb.

## Ice Monkey (key 5) and Glue Gunner (key 6)

- [ ] Ice Monkey: a light-blue ring flashes around it when it fires; bloons it hits pop one layer and the children turn blue and stop for about 1.5 s (Ceramics half as long).
- [ ] Frozen bloons can't be popped by darts, tacks or boomerangs, but bombs pop them.
- [ ] Ice Monkey ignores Lead, White, Zebra and MOABs (they don't freeze).
- [ ] Arctic Wind (0-3-0+): bloons inside the slightly-bigger-than-range circle move slower (not Lead, White, Zebra, MOABs or camo).
- [ ] Permafrost (2-x-x): bloons stay slowed after thawing.
- [ ] Glue Gunner: glued bloons turn yellowish and move at half speed; the children keep the glue for 2 more layers.
- [ ] It doesn't keep re-gluing the same bloon while unglued ones are in range; it never shoots MOABs until MOAB Glue (0-0-3).
- [ ] Corrosive Glue (2-x-x): glued bloons lose a layer every 2 s.
- [ ] The glue blob looks right (yellow, green from 3-x-x, pink on path 3).
- [ ] Side effects of the same system: Bloon Impact (Bomb 4-x-x) stuns bloons (not MOABs) for 1.4 s; MOAB Domination (Boomerang x-x-5) sets bloons on fire; MOAB Press briefly stops MOABs; Maim MOAB (Sniper 4-x-x) stops MOABs for 3 s, Cripple MOAB (5-x-x) for 7 s with +5 damage per hit.
- [ ] Not done yet (don't report): the game's real frozen/glued bloon overlays (tinted for now), Snowstorm / Absolute Zero / Glue Strike / Glue Storm abilities, Cryo Cannon / Icicles / Glue Splatter / Bloon Solver extras, Embrittlement's extra damage is in but untested.

## Look and selection

- [ ] Sniper Monkey shows its small range circle (20) when selected and while placing, even though it shoots across the whole map.
- [ ] The selected tower gets a white outline; others keep their dark outline.
- [ ] Boomerang, Sniper, Tack and Bomb now have their dark outline (it's baked into their meshes; the width is my guess).
- [ ] Every tower has a soft round shadow under it, also while placing.

## Boomerang Monkey (key 2)

- [ ] Boomerangs curve the same way as in the real game. The path is the game's own, but which side it loops to is my guess (P-PATH). If it's mirrored, say so and I'll flip it.
- [ ] The loop size looks right next to the range circle (the path isn't stretched to the target).
- [ ] Long Range Rangs (0-0-1, 0-0-2) loops wider. Kylie Boomerang (0-0-3+) flies straight out.
- [ ] Kylie Boomerang hits the same bloon more than once (it forgets hits every 0.3 s).
- [ ] Glaive Ricochet (3-0-0+): glaives bounce from bloon to bloon.
- [ ] MOAB Press (0-0-4): a second throw only at MOABs, BFBs, ZOMGs and DDTs, never BADs, and it pushes them back a bit.
- [ ] Glaive Lord (5-x-x): three glaives orbit the monkey and pop what they touch. The orbit speed is a guess.
- [ ] Every upgrade has its own monkey model, projectile art, icon and portrait.
- [ ] Not done yet (don't report): MOAB Domination's burn, Turbo Charge / Perma Charge abilities, the flip-side button.

## Drag to place

- [x] Drag a tower sideways out of the shop onto the map. Confirmed working.
- [ ] Releasing on a blocked spot (track, tree, another tower) keeps the tower in hand, so a click places it.
- [ ] Dragging back over the shop and releasing cancels.
- [ ] Dragging up and down in the shop still scrolls instead of picking up a tower.
- [ ] Clicking a tower in the shop, then clicking the map, still works.

## Keys

- [ ] 1-9 and 0 pick Dart, Boomerang, Bomb, Tack, Ice, Glue, Desperado, Sniper, Wizard and Super (shop order; the Desperado sits before the Sniper in the shop, so Sniper moved to 8 and Ninja is shop-only).

## Mod entity API and performance follow-up (2026-10-29)

- Entity ID/layer patching, damage overflow interception, fire overrides, purchase vetoes,
  and invalid-decision fallback pass the local integration checks.
- Historical performance failure (resolved by the segment/key work below): late_game_load(4,6000,1800) measured 5.382 ms average /
  20.480 ms worst through Game::step and 5.529 / 23.683 ms calling step_native directly.
  The extracted game.json SHA256 was identical for both runs. Both exceed the existing
  2.6 ms average / 10 ms worst guard. Bypassing the step event wrapper did not resolve it;
  this is not an A/B measurement of every per-hit/emission branch. Keep the guard unchanged.
- These historical timings are not an official-game comparison. See the matched
  reference/optimized measurements in benchmarks/README.md for the fix.

## Runtime subscription controls (2026-10-29)

- Passed 21 mod-related Rust checks (including local asset integration) and 6 SDK checks.
- Muted hooks preserve owner state and UI; UI callbacks re-enable them. Other owners'
  subscriptions remain active. Invalid names reject the whole command batch.
- Entity Lab starts with no interception hooks, stops panel ticks when closed, and
  removes the shield hook immediately after its fifth cancelled hit.
- This verifies the native fast-path masks, not an FPS improvement measurement.
  The native late-game finding above is resolved by the subsequent measured fix.

## Simulation and shop fixes (2026-10-29)

- Passed: 164 default renderer tests (28 deliberately ignored), 21 mod checks including
  local integration, and the 1,800-frame reference/optimized state comparison.
- Passed unchanged performance guard after the final Sandbox fix: 1.306 ms average,
  8.143 ms worst against 2.6 / 10 ms limits. Paired measurements are in benchmarks/README.md.
- Shop taps: the same ordinary tower twice within 0.3 seconds opens that tower's existing
  upgrades screen, including when unaffordable. Scrolls, pull drags and other tiles cancel
  the pair. This does not attempt to show a tower upgrade tree for heroes.
- Sandbox: base and temporary ability attacks stop at zero health; simulation continues
  and attacks resume after health is restored. The existing regression test now passes.

## Sandbox controls follow-up (2026-10-29)

- Four Sandbox checks pass: zero-health stop/resume, both tower counters reset,
  generated round selection/scaling reversal, and Sandbox-only content hooks.
- Round selection no longer clamps generated freeplay to the fixed exported list.
  Round 200 starts; returning to round 1 restores ordinary Ceramic health/children.
  Custom round sets without generated groups keep their existing bound.
- Freeplay budget arithmetic uses f64 before multiplication, preserving ordinary
  round budgets and preventing intermediate i32 overflow for large entered rounds.
- Sandbox/freeplay round labels omit the fixed-mode end round.

- Existing freeplay-generator regression passed; optimized build passed. Sandbox
  screenshot confirms the single round number and open bloon panel; no script errors.
  BTD6.exe was replaced and its SHA256 matches the tested build.

## Mod terminal and co-op entry (2026-10-29)

- Seven mod-browser checks and two co-op-menu checks pass; optimized build passed.
- Mod rows have immediate switches separate from row selection. Selecting a row shows
  its properties/settings; guest room selection remains read-only.
- Verified native screenshots: mod-terminal-final.png (Practice Start settings visible)
  and coop-terminal-final.png (real locally hosted lobby), in local_extracted/ui-cleanup-check.
- Hosted lobby shows Mods at bottom-right, clear of the bottom-left hero picker and Ready.
- Benjamin artwork/native fonts and green terminal panels reuse existing assets; no
  per-frame filesystem scans or additional texture decoding were added.

## Simplified Mods UI (2026-10-29)

- Nine mod-browser checks pass, including independent switches, conflict reporting,
  host/guest permissions, settings persistence, bounded local icon paths, and tinted
  native button clips retaining their original scale motion.
- Normal enabled-state captions/ON/OFF labels and the persistent footer are removed.
  The error notification remains available for invalid/conflicting selections.
- Custom icon fixture verifies list/details artwork, with an intentionally corrupt
  PNG falling back to the normal mod icon without a crash. Assets stay in the ignored
  ui-cleanup-check case directory. No user mod/settings files were changed by this check.
- Documentation for the optional local icon.png convention is in mods/README.md and
  website/public/mods/reference.md. Icon decoding remains lazy; the faint header signal
  uses existing native sprites, with no per-frame asset work or fullscreen effects.
- Final optimized build passed. mods-polished-final.png verifies the matching green
  utility buttons after native animation, enlarged panes, descriptions/settings, custom
  icons and corrupt-image fallback. BTD6.exe replaced; installed/build SHA256 matched.

## Native Mods menu and direct numeric entry (2026-10-29)

- Nine targeted mod-browser checks pass using an unoptimized renderer test crate
  with optimized dependencies (temporary command-line profile override to reduce
  compiler memory pressure). No permanent build-profile change was made.
- Direct setting entry checks cover typed values, persistence without enabling a
  mod, rejection of step-misaligned values, empty input, cancellation, and recovery.
- Existing selection/switch, invalid/missing mod, settings reset and guest checks pass.
- Click processing retains field/row hit rectangles while committing and rebuilding,
  so a click that saves one field can also focus the next field or select a mod.
- Final validation: 10 mod-browser tests and 2 co-op-menu tests pass. The extra
  regression verifies a click commits one numeric field and focuses the next, or
  commits the field and toggles a mod, without requiring a second click.
- Optimized build passed with a temporary single-codegen-unit override for the
  renderer to fit alongside concurrent asset extraction. Native UI screenshot
  mods-native-final.png reviewed; BTD6.exe replaced and build/install hashes matched.
- Phone-viewable screenshot: https://files.catbox.moe/3j2jxt.png

## Consistent Mods visual reference (2026-10-29)

- Ten mod-browser behavior tests pass after the styling changes.
- Standard optimized game build passed; mods-consistent-final.png reviewed at 1600x900.
- Extras/Advanced reference: round BlueBtnCircleSmall toggle geometry/endpoints,
  rectangular panel cards instead of pill list rows, rounded blue insets throughout,
  and compact inline title/icon. Numeric entry and its existing regression checks remain.
- BTD6.exe replaced and its SHA256 matched the reviewed build. The screenshot case
  now uses default mod icons; previous custom/corrupt fixture icons were removed only
  from that ignored case, leaving the user's real mods untouched.

## Mods header spacing and list descriptions (2026-10-29)

- Ten mod-browser checks pass. Standard optimized game build passed.
- Reviewed mods-header-final.png at 1600x900: title/icon and utility buttons share an
  inset header baseline; descriptions sit below mod titles and fit without touching
  the switches. Full descriptions remain in the right pane; long previews truncate.
- Installed BTD6.exe and verified its SHA256 against the reviewed build. Summary text
  is generated only during list rebuilds, preserving the existing rendering pipeline.

## Mod settings fields and compact reset (2026-10-29)

- Ten behavior tests pass, including resetting during an invalid uncommitted edit.
- Standard optimized build passed. mods-fields-final.png reviewed at 1600x900:
  compact reset beside version, complete rounded input borders with darker fill,
  matching native square steppers aligned with the fields, no large bottom button.
- BTD6.exe replaced and its hash matched the reviewed build. Focus uses a gold frame;
  input fields retain typing/Enter/Escape/paste behavior without button press scaling.

## Mods selection, search and status filters (2026-10-29)

- Thirteen mod-browser checks pass: previous behaviors plus case-insensitive multiword
  search across names/IDs/descriptions, clear/no-results, combined status/query filters,
  preserving hidden enabled mods during toggles, long-query display bounds, and keyboard
  focus moving from search into numeric settings without dropping the query.
- List and details scroll reset when changed results select a different visible mod.
  Text search refreshes cached cards without rescanning the mod directory per keystroke.
- Standard optimized build passed. Reviewed mods-search-final.png: native gold card
  selection, search bar and icon-labelled All/Enabled/Disabled filters fit above the list.
- BTD6.exe replaced; its SHA256 matched the reviewed optimized executable.


## Achievement browsing and Mods outline (2026-10-29)

- Selected mod cards retain their blue interior with an inset gold outline.
- Achievement viewport now clips cards; offscreen fixed-size cards skip descendant
  layout, texture loading and drawing, and clear stale descendant hit rectangles.
- Audited the installed GameData achievement references against the exported catalogue:
  162 source entries, 162 exported, no missing or extra IDs. Verified the final row
  (Marshmallow Maxing / Power Creep) via RRBTD6_ACHIEVEMENT_BOTTOM=1 in an autotest.
- Reviewed optimized top, bottom and Mods screenshots under
  local_extracted/ui-cleanup-check/. All 13 mod-browser, 2 grid and 2 catalogue
  checks passed. Production optimized build passed.


## Menu opening work and co-op header (2026-10-29)

- Co-op room Mods moved inside BgPanel at its top right, beside the Standard header.
  The button retains its mod count and host/guest flow; frontend clicks use the new
  shared path constant. Offline visual fixture: RRBTD6_COOP_MODS_PREVIEW=1 with
  RRBTD6_AUTOTEST (does not create or simulate a network room).
- Menu snapshots now average 8x8 framebuffer pixels on the GPU before readback,
  retaining the existing CPU blur, vignette and tint. Physical framebuffer sizes
  include Windows DPI. RRBTD6_CPU_MENU_SNAPSHOT=1 retains the reference path.
- Actual-context RRBTD6_MENU_SNAPSHOT_CHECK passed at default, 1301x733, 960x960,
  and 1800x800 windows: maximum CPU/GPU output channel difference 1/255.
- Opening sprites decode on the existing bounded four-job loader; uploads remain
  on the render thread and finish before entrance animation. Reference switch:
  RRBTD6_SERIAL_UI_WARM=1. First-group texture count remains bounded.
- Nineteen targeted tests passed: 2 co-op controls, 13 Mods, 2 catalogue, 2 grid.
  Reviewed Mods, Heroes, Maps, Profile and co-op header screenshot smoke captures.
- RRBTD6_FRONT_OPEN_AFTER=100:achievements (with screenshot autotest) exercises
  normal Home -> Achievements separately from loading directly into the page.

The GPU snapshot pipeline is exercised during loading before the first menu click,
including when startup is skipped. This avoids paying driver shader compilation
during the first Achievements/Maps/Profile transition.

Final optimized build passed; optimized pixel guard, co-op header and pause
smoke captures passed. Root BTD6.exe replaced and SHA256 matched the tested build:
D4666D5BD3EEC27865ECEA06C66AA320961CC08F9BAFBC66C261ADC1F777FF0F.
Home -> Achievements opening measured 50.453 ms reference vs 38.249 / 36.000 ms
optimized; see benchmarks/README.md for setup and remaining-hitch limits.


## Achievement card pooling (2026-10-29)

Only viewport cards plus two buffered rows are instantiated; the normal 2160-unit
viewport retains 10 card trees rather than 162. Full metadata/search remain loaded.
Grid logical ranges preserve native layout and full scroll extent. Existing cards
are recycled on scroll/search; nearby sprites prefetch using bounded workers and
cooperative render-thread uploads. Progress binds by catalogue ID, and the header
counts the full catalogue even during search. Verified top, final row, one/no
results and reset; 3 achievement checks and 2 existing grid checks passed. Final
optimized build passed and BTD6.exe was replaced (SHA256
1B06F1A8175216559374D61A71311D08CED8009A12C21F58960DBBF54B4BD09E).
This reduces instantiated card trees by 94%; no additional FPS improvement is
claimed from the single final capture (concurrent asset work affects timings).

### Steam fresh setup test (2026-10-29)
- Launch Steam BTD6 with the updated RRBTD6Installer.dll. Choose a new empty parent folder when prompted; setup creates rrbtd6 beneath it. Keep BTD6 open and follow the MelonLoader terminal bar, percentage and ETA. Time the full preparation; do not reuse prepared assets for the first-run measurement.
- Preview payload is Mods/RustTD6SetupPreview (launcher, client, tools ZIP and SHA256 manifest). Removing that optional preview folder restores the ordinary GitHub-download path; do not remove it before this test.
- Verified: optimized game build, main-menu screenshot with Trophy Store in the Achievements slot, installer C# build (0 errors), progress/helper tests 8/8, extraction-plan tests 15/15, launcher recovery tests 14 passed/1 ignored. Full clean extraction and native prompt interaction remain to be tested in Steam.
- Achievements is removed entirely, superseding earlier virtualized-menu test instructions.

### Parallel installer timing (2026-10-29)
- Steam preview now includes memory-aware parallel model batches. Repeat setup from Steam BTD6 to pick up the new tools payload. Use a new empty folder for a comparable clean first-run time. Inspect the terminal's worker count/model progress when it reaches tower preparation.
- Early ETA (e.g. 54 minutes at 1.4%) extrapolates a small sample of startup stages and is not a measured full-install time. Do not claim the under-30-minute target until a complete clean run is timed.

### Updated Steam setup test (2026-10-29, parallel maps)
- Relaunch Steam BTD6 to use the shorter setup dialog and updated tools. Map preparation now runs in parallel as well as model preparation. Allow setup to finish and record full elapsed time; component timings do not establish the full-install target.

### Setup cache test (2026-10-29)
- Next Steam setup attempt includes bounded prefab/PNG/decoded-clip reuse. Runtime performance and memory are unaffected. Already-running extraction processes keep their existing tools. Full installation time still requires a complete clean run.

### Artwork setup batching (2026-10-29)
- Next Steam setup attempt batches upgrade and later tower/hero icon/portrait exports. Sample improved 70.0 s to 10.4 s with 64 identical files. Measure the complete fresh setup to establish actual total duration; existing running installations retain their original tooling.

### Invariant animation data (2026-10-29)
- Next Steam setup uses precomputed static matrices and vertex helper arrays during animation baking. Twelve-model sample is another 10% faster with 91 identical output files; exact reference-math checks cover rotating and animated cases. Existing running extraction tools are untouched.

### Indexed clip sampling (2026-10-29)
- The next Steam setup attempt includes faster streamed-key lookup, precomputed clip binding offsets and scalar dense-sampling bounds. Twelve-model sample improved another 15%, with 91 identical files; full clean installation duration remains unmeasured.

- [ ] Monkey Village 0-0-5 (Monkey-Opolis): with no Banana Farm in the Village's range the upgrade shows "Requires Banana Farm" and can't be bought. With farms in range it costs $5,000 per farm (before discounts). Buying it removes those farms (no refund) and its crates pay more the more the farms were worth.
- [ ] Monkey Village buff icons: two Villages near a tower show each buff icon once, with no "x2".
