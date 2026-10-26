# Things to test by hand

Added without a hands-on check. Tick them off (or note what's wrong) when you play. Newest at the top.

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
- [ ] Engineer Overclock (0-4-0) / Ultraboost (0-5-0): press the ability, then click another tower. It attacks much faster for 60 s (shorter on tier 4 and 5 towers). Ultraboost also leaves a small permanent speed-up that stacks up to 10 times. Right-click or Esc cancels. No "pick a tower" highlight yet.
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
- [ ] Not done yet (don't report): Overclock / Ultraboost, Sentry Champion explosion on expiry, the full-trap picture.

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
