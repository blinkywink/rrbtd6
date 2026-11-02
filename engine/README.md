# btd6-sim

A headless, deterministic simulation core: the first version of the simulation, kept for the
[benchmarks](../benchmarks/README.md) and the [compatibility tests](../tests/observations/README.md).

**The game doesn't use this crate.** The simulation you play is
[`renderer/src/game.rs`](../renderer/src/game.rs), so game behaviour changes belong there.

Rules for this crate are at the top of [`src/lib.rs`](src/lib.rs), and what has been verified is in
[`specs/compatibility.md`](../specs/compatibility.md).
