# Private co-op transport

This crate connects up to four copies of this client using encrypted QUIC via
iroh. The native Create Match / Join Match screens host and join a private lobby.
Copy Invite copies the complete temporary RR1 invite; guests paste it into Join
Match. Both copies must have identical executables and extracted game data.

Connections try direct peer communication and use an automatic relay when needed.
No port forwarding, router configuration or separate player installation is needed.
The default N0 preset uses external public discovery and relay infrastructure;
internet connectivity and availability of those services are required. This is a
development default, not a promise of production relay capacity or uptime.
Port mapping is disabled. Networking starts only when hosting or joining, on a
background worker. Queues, frame sizes, handshake work and request rates are bounded.

Invites contain a random 128-bit room secret and expire when the host leaves.
Public display names, room settings, readiness, a build/data fingerprint, gameplay
commands and diagnostic checksums are exchanged. Official account credentials and
cloud saves are never exchanged.
The host owns settings and player slots. Changing settings clears readiness.

## Implemented

- Four-player lobby membership and temporary invite copy/paste.
- Host settings, closed slots, ready state and leave/disconnect propagation.
- Protocol/build/data mismatch and incorrect invite rejection.
- Automatic relay fallback; relay-only mode for verification.
- Backend match preparation with a frozen room configuration and loading barrier.
- Host-ordered, numbered input batches covering six simulation steps each.
- Per-player input sequences and connection-assigned sender slots.
- Initial-state and applied-frame checksum comparison; divergence closes the match.
- Eight batches maximum in flight, bounded per-player inputs and a stalled-peer timeout.

## Playable prototype

Choose Standard mode and Free For All areas, create a room, and send its copied
invite to the other players. Everyone clicks Ready; the host's button becomes
Start when all connected players are ready. The loading barrier then starts the
same map and difficulty on each client. Match data is loaded independently of
each player's saved difficulty and hero selection.

Placement, ordinary upgrades, selling, targeting, selected points, pointer input,
cash collection, bank/stance controls, untargeted and point/tower-targeted abilities, round starts, speed,
auto-start and freeplay use ordered commands. Only the owner may alter a tower.
Stable tower IDs survive vector reordering; pets and sentries inherit ownership.
The adapter checks normal placement/cost/upgrade rules against the issuing player's
wallet. A foreign player's ship-deck tower prevents selling that ship.

Six-step network batches are consumed as individual 60 Hz simulation steps, not
six-step visual jumps. Acknowledgements contain the simulation diagnostic checksum
after the batch completes. Checksums stream binary fields instead of building
JSON snapshots. The ordinary single-player fixed-step loop is bypassed during
co-op, and pause/options menus continue pumping the shared simulation. Home leaves
the room. Restart is hidden until a coordinated restart protocol is implemented.

## Remaining

This is a prototype, not complete original co-op parity. Individual hero selections
and multiple heroes, area
division enforcement, sacrifices/paragons, restart, reconnect
and migration remain unfinished. Match start currently accepts only Standard / Free
For All. Diagnostic checksums do not cover every nested mutator, and this has not
established cross-platform determinism. This protocol does not connect to
official-game co-op rooms. Official cloud uploads remain disabled.

## Verification

```
cargo test --profile game -p rrbtd6-coop
cargo build --profile game -p rrbtd6-coop --bin coop-probe
python -I tools/test_coop_transport.py path/to/coop-probe.exe
python -I tools/test_coop_transport.py path/to/coop-probe.exe --match --players 4
python -I tools/test_coop_transport.py path/to/coop-probe.exe --desync
cargo test --profile game -p btd6-game coop_replay_tests
python -I tools/test_coop_game.py path/to/btd6-game.exe --players 4
cargo test --profile game -p btd6-game coop_digest_benchmark -- --ignored --nocapture
```

The probe runs two separate processes with direct-IP transport disabled. It checks
rejection of incompatible builds and incorrect secrets, joining, host settings,
guest readiness and departure. It requires internet access, uses a temporary
directory and removes its invite afterward. It does not require extracted assets.

`--match` checks 120 ordered batches and matching **command-history** checksums in
two to four separate processes. `--desync` intentionally supplies different
initial checksums and verifies both sides stop before running. The renderer replay
test requires locally extracted game data and compares two real simulations over
6,000 steps, including placement, upgrades, selling, combat and independently
drained visual effects. It compares observable combat state, not every internal
field, and does not prove full simulation determinism across all towers or machines.

The rendered-game test requires extracted local assets and runs two to four actual
windows. It places and upgrades one Dart Monkey per player, starts a round, fights
bloons and sells player two's tower. All clients must have identical diagnostic
state at shared frame 180. It saves screenshots and results under ignored
`local_extracted/coop-game-N-test/`, restores the saved map, deletes the temporary
invite, and stops only its own processes. Test-only extra cash is gated behind
explicit environment variables and is not used in ordinary co-op.

Targeted Overclock, Darkshift and Mega Mine have parity tests against the existing
single-player paths. The wire carries the owned source tower ID, its ability index,
and either a selected point or target tower ID. Invalid targets do not spend cooldown.
`economy` supplies per-player wallets, integer starting-cash remainders and
fractional shared income. Purchases and sales affect the issuing player's wallet.
Pop/round income is shared; banana pickups, bank collections and owner-only
per-round tower income pay the producer. Collecting an ally's banana still pays
its producer, including after the source tower is sold. Loans and repayment are
tracked separately per player. HUD cash is a local projection excluded from the
authoritative checksum; wallet balances and debts are included instead.

The local PE's `Simulation.DistributeCash` body (VA `0x180a762e0`) reads
`GameModel.cash` at offset `0x28`, truncates it to an integer, distributes quotient
and remainder, and applies each player's allowed double-cash flag afterward.
`Simulation.AddSharedCash` (`0x180a74da0`) divides using double precision before
calling the per-player add path. It does not floor a fractional pop share.
These addresses are build-specific: local metadata method addresses can point
inside a different build's function, so traces were aligned to the PE unwind ranges.
Special income sources such as Lead-to-Gold and Monkey Town still need a native
ownership audit. Double-cash entitlements are not enabled.

The original CoopManager player panel shows peer balances, player IDs, host marker,
Give Cash and Request Cash. Transfers use connection-assigned sender slots and
the ordered simulation stream; overspending, self-transfers and absent recipients
are rejected. Donations bypass income multipliers and recipient debt repayment.
Cash requests are synchronized and limited to one per player every five simulation
seconds. The native donation offer is floor(cash / 5), at least 100 when available,
or the remaining balance below 100. Panel slide curves come from its original
Animator export; unsupported emote/mute controls are hidden.

Re-export menu data with `tools/assets/export_coop_ui.py GAME local_extracted`
using the UnityPy environment to include CoopManager and its animation data.

Latest donation validation: ordered simulation and wire tests pass. Four rendered
clients now match through placement, upgrades, combat, sale, donation and request
commands with full-resolution textures. Earlier four-client allocation failures
were resolved by removing warmup/background loading of unused fallback models:
the tier models were already loaded and those extra copies were not being drawn.
Measured end-of-warmup private memory fell from roughly 3.2 GB to 2.0 GB per client
on this machine. Model prefetch also retains at most four staging jobs. These
measurements cover the tested map and roster, not every late-game configuration.

Set `RRBTD6_COOP_RELAY_ONLY=1` to test the game UI through the relay exclusively.
