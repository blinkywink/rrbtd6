//! The fixed-step simulation, run either inline or on a worker thread while the main thread draws
//! the world.
//!
//! Where the steps ran inline the main loop now copies the game (`Game::view`, the frame's display
//! queues moving to the copy), hands the real one to the worker and draws the world from the copy;
//! before the HUD (whose actions change the game) it takes the real one back. The real game sees
//! the same calls in the same order as when stepped inline (input, steps, HUD actions, input ...),
//! so the simulation is unchanged; only the world picture shows the state before the frame's
//! steps.
//! RRBTD6_SYNC_SIM=1: step inline, as before.

use crate::game::{Game, FPS};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Under load (a huge pop: one step can take 40 ms) a frame runs steps for at most this long, at
/// least one, and lets the time it couldn't get to go: the game slows for a moment instead of the
/// next frame owing more steps (each as heavy), the frame after more still. The steps and their
/// order are the same; only how many fit in a frame.
pub const STEP_BUDGET: f64 = 0.010;

/// What a frame's stepping needs besides the game.
#[derive(Clone)]
pub struct StepJob {
    /// Seconds of simulation owed (the main loop's accumulator).
    pub acc: f32,
    pub budget: bool,
    pub auto_start: bool,
    pub root: PathBuf,
}

/// What it leaves: the accumulator after it, and (total step seconds, steps, worst step).
#[derive(Clone, Copy, Default)]
pub struct StepDone {
    pub acc: f32,
    pub stats: (f64, u64, f64),
}

/// Runs the steps the accumulator owes (Fixed-step simulation, P-TIME).
pub fn run_steps(game: &mut Game, job: &StepJob) -> StepDone {
    let mut acc = job.acc;
    let mut stats = (0.0f64, 0u64, 0.0f64);
    let frame_sim_t0 = std::time::Instant::now();
    while acc >= 1.0 / FPS {
        if job.budget && frame_sim_t0.elapsed().as_secs_f64() >= STEP_BUDGET {
            acc = acc.min(1.0 / FPS);
            break;
        }
        let t_step = std::time::Instant::now();
        let completed_before = game.round;
        game.step();
        round_completed(&job.root, game, completed_before);
        let dt_step = t_step.elapsed().as_secs_f64();
        stats.0 += dt_step;
        stats.1 += 1;
        stats.2 = stats.2.max(dt_step);
        // Like the BTD6 setting: the first round is started by hand.
        if job.auto_start && !game.sandbox && game.round > 0 {
            game.start_round();
        }
        acc -= 1.0 / FPS;
    }
    StepDone { acc, stats }
}

/// After a step that ended a round: InGame.RoundEnd -> DrainXpAndCheckLevelUp (the round's Monkey
/// XP into the profile) and the local profile's highest round / experience.
fn round_completed(root: &Path, game: &mut Game, completed_before: usize) {
    if game.round == completed_before || std::env::var_os("RRBTD6_AUTOTEST").is_some() {
        return;
    }
    crate::tower_xp::drain(root, game);
    if !game.sandbox && crate::frontend::account_preview::is_local() {
        let profile_root = root.to_path_buf();
        let completed_round = game.round;
        let xp = ((game.round.saturating_sub(completed_before)) as u64).saturating_mul(100);
        std::thread::spawn(move || {
            if crate::local_profile::update(&profile_root, |p| {
                p.highest_round = p.highest_round.max(completed_round);
                crate::local_profile::add_experience(p, xp);
            })
            .is_ok()
            {
                crate::frontend::account_preview::refresh_local(&profile_root);
            }
        });
    }
}

const _: fn() = || {
    fn send<T: Send>() {}
    send::<Game>();
};

/// A persistent thread that runs `run_steps` on the real game, which it owns while it runs (the
/// main loop draws a copy meanwhile, `Game::view`), and one that frees the copies off the main
/// thread.
pub struct SimWorker {
    to: mpsc::Sender<(Box<Game>, StepJob)>,
    from: mpsc::Receiver<(Box<Game>, StepDone)>,
    drop_to: mpsc::Sender<Game>,
    running: bool,
}

impl SimWorker {
    pub fn new() -> Self {
        let (to, rx) = mpsc::channel::<(Box<Game>, StepJob)>();
        let (tx, from) = mpsc::channel();
        std::thread::Builder::new()
            .name("sim".into())
            // The sim's deep pop cascades recurse (the tests run with a 256 MB stack too).
            .stack_size(256 << 20)
            .spawn(move || {
                for (mut game, job) in rx {
                    let done = run_steps(&mut game, &job);
                    if tx.send((game, done)).is_err() {
                        break;
                    }
                }
            })
            .expect("sim thread");
        let (drop_to, drops) = mpsc::channel::<Game>();
        std::thread::Builder::new()
            .name("sim-view-drop".into())
            .spawn(move || {
                for view in drops {
                    drop(view);
                }
            })
            .expect("view drop thread");
        SimWorker { to, from, drop_to, running: false }
    }

    /// Starts the frame's steps on the real game, leaving `game` holding `view` (its copy) until
    /// `finish` puts the real one back.
    pub fn start(&mut self, game: &mut Game, view: Game, job: StepJob) {
        assert!(!self.running);
        let real = std::mem::replace(game, view);
        self.to.send((Box::new(real), job)).expect("sim thread gone");
        self.running = true;
    }

    pub fn running(&self) -> bool {
        self.running
    }

    /// Waits for the steps and puts the real game back in `game` (the copy is freed elsewhere).
    pub fn finish(&mut self, game: &mut Game) -> StepDone {
        assert!(self.running);
        self.running = false;
        // A panic in the step (the worker gone) is the game's panic: raise it here.
        let (real, done) = self.from.recv().expect("simulation thread panicked");
        let view = std::mem::replace(game, *real);
        if let Err(e) = self.drop_to.send(view) {
            drop(e.0);
        }
        done
    }
}
