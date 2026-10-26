//! Headless BTD6-compatible simulation core.
//!
//! Design rules (see `specs/compatibility.md`):
//! - Simulation advances in whole frames, mirroring the original
//!   `Simulation.SimulationTime.elapsed: int` and `Process(int elapsed)` API.
//! - Every behaviour that is not yet verified against the original game is
//!   tagged `HYPOTHESIS <ID>` in the source and listed in the compatibility
//!   matrix. Do not remove a tag without linking evidence.
//! - No heap allocation inside [`sim::Simulation::step`] once storage has
//!   warmed up (checked by `benchmarks`).

pub mod bloons;
pub mod data;
pub mod mapbuild;
pub mod math;
pub mod path;
pub mod rng;
pub mod rounds;
pub mod sim;
pub mod time;
