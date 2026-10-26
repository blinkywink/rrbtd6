//! Rounds and emissions (research/04-native-analysis.md §6).
//!
//! VERIFIED-DISASM: emission times are in **frames** relative to the round
//! start. `AddEmissions` adds `(float)time.elapsed` with an f32 add, and
//! `Spawner.Process` fires an emission when `time <= (float)elapsed`,
//! scanning every pending emission in list order.
//!
//! The original rescans every pending emission on every frame. We precompute
//! each emission's due frame and stable-sort by it, which gives the same
//! spawns in the same order (ties within a frame keep list order) at O(1)
//! amortised cost per frame.

use crate::bloons::BloonTypeId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emission {
    /// `BloonEmissionModel.time`, frames from round start.
    pub time: f32,
    pub kind: BloonTypeId,
}

/// `Models.Rounds.BloonGroupModel`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloonGroup {
    pub kind: BloonTypeId,
    pub start: f32,
    pub end: f32,
    pub count: i32,
}

impl BloonGroup {
    /// `BloonGroupModel.GetEmissions` @0x1810CC280, f32 operation order kept.
    pub fn emissions(&self) -> impl Iterator<Item = Emission> + '_ {
        let span = self.end - self.start;
        let (step, offset) = if self.count == 1 {
            (span / 1.0, span * 0.5)
        } else {
            (span / (self.count - 1) as f32, 0.0)
        };
        (0..self.count.max(0)).map(move |i| Emission { time: ((i as f32) * step + self.start) + offset, kind: self.kind })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Round {
    /// In the original list order.
    pub emissions: Vec<Emission>,
}

impl Round {
    pub fn from_emissions(emissions: Vec<Emission>) -> Self {
        Self { emissions }
    }

    /// HYPOTHESIS H-ROUND-2: when a round only has groups, the emission
    /// list is the groups' emissions concatenated in group order. Prefer
    /// `RoundModel.emissions` from decoded data where present.
    pub fn from_groups(groups: &[BloonGroup]) -> Self {
        Self { emissions: groups.iter().flat_map(|g| g.emissions()).collect() }
    }

    /// `roundDuration` = max emission time (frames).
    pub fn duration(&self) -> f32 {
        self.emissions.iter().fold(0.0f32, |m, e| if e.time > m { e.time } else { m })
    }
}

/// First frame `f` with `e_time <= (f as f32)`, where
/// `e_time = (start as f32) + time` (f32 add, as in `AddEmissions`).
pub fn due_frame(start: i32, time: f32) -> i32 {
    let abs = start as f32 + time;
    if abs.is_nan() {
        return i32::MAX; // NaN never compares <=; never fires.
    }
    let mut f = if abs > i32::MAX as f32 { i32::MAX } else { abs.ceil() as i32 };
    while f > i32::MIN && abs <= (f - 1) as f32 {
        f -= 1;
    }
    while f < i32::MAX && abs > f as f32 {
        f += 1;
    }
    f
}

#[derive(Clone, Copy, Debug)]
pub struct Pending {
    pub due: i32,
    pub kind: BloonTypeId,
    pub emission_index: i32,
}

/// One active `RoundData`.
#[derive(Clone, Debug)]
pub struct ActiveRound {
    pub round: i32,
    pub start_frame: i32,
    pub duration: f32,
    pending: Vec<Pending>,
    next: usize,
}

impl ActiveRound {
    pub fn new(round: i32, start_frame: i32, r: &Round) -> Self {
        let mut pending: Vec<Pending> = r
            .emissions
            .iter()
            .enumerate()
            .map(|(i, e)| Pending { due: due_frame(start_frame, e.time), kind: e.kind, emission_index: i as i32 })
            .collect();
        pending.sort_by_key(|p| p.due); // stable: keeps list order within a frame
        Self { round, start_frame, duration: r.duration(), pending, next: 0 }
    }

    /// Advances past every emission due at `elapsed` and returns their
    /// index range (original order). Read them with [`Self::pending_at`].
    pub fn advance_due(&mut self, elapsed: i32) -> core::ops::Range<usize> {
        let a = self.next;
        while self.next < self.pending.len() && self.pending[self.next].due <= elapsed {
            self.next += 1;
        }
        a..self.next
    }

    pub fn pending_at(&self, i: usize) -> Pending {
        self.pending[i]
    }

    /// Emissions that fire at frame `elapsed`, in original order.
    pub fn take_due(&mut self, elapsed: i32) -> &[Pending] {
        let r = self.advance_due(elapsed);
        &self.pending[r]
    }

    pub fn emitting(&self) -> bool {
        self.next < self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: BloonTypeId = BloonTypeId(0);

    #[test]
    fn group_expansion_matches_original_formula() {
        let g = BloonGroup { kind: K, start: 0.0, end: 100.0, count: 5 };
        let t: Vec<f32> = g.emissions().map(|e| e.time).collect();
        assert_eq!(t, vec![0.0, 25.0, 50.0, 75.0, 100.0]);
        let single = BloonGroup { kind: K, start: 10.0, end: 30.0, count: 1 };
        assert_eq!(single.emissions().map(|e| e.time).collect::<Vec<_>>(), vec![20.0]);
    }

    #[test]
    fn due_frame_is_first_frame_reaching_time() {
        assert_eq!(due_frame(100, 0.0), 100);
        assert_eq!(due_frame(100, 0.5), 101);
        assert_eq!(due_frame(100, 1.0), 101);
        // Brute-force agreement with the original comparison.
        for start in [0, 7, 12345, 16_777_000] {
            for time in [0.0f32, 0.25, 1.0, 3.7, 99.99, 1234.5] {
                let f = due_frame(start, time);
                let abs = start as f32 + time;
                assert!(abs <= f as f32);
                assert!(!(abs <= (f - 1) as f32));
            }
        }
    }

    #[test]
    fn same_frame_emissions_keep_list_order() {
        let r = Round::from_emissions(vec![
            Emission { time: 5.2, kind: BloonTypeId(1) },
            Emission { time: 5.1, kind: BloonTypeId(2) },
            Emission { time: 1.0, kind: BloonTypeId(3) },
        ]);
        let mut a = ActiveRound::new(1, 0, &r);
        assert_eq!(a.take_due(1).iter().map(|p| p.kind.0).collect::<Vec<_>>(), vec![3]);
        assert!(a.take_due(5).is_empty());
        assert_eq!(a.take_due(6).iter().map(|p| p.kind.0).collect::<Vec<_>>(), vec![1, 2]);
        assert!(!a.emitting());
    }
}
