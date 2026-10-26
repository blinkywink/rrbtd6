//! Bloon definitions and structure-of-arrays runtime storage.
//!
//! Movement fields follow `Simulation.Bloons.Bloon` (offsets in
//! research/04-native-analysis.md §5). Children, armour, mutators and status
//! effects are not modelled yet.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BloonTypeId(pub u16);

/// Static per-type data, a subset of `Models.Bloons.BloonModel`.
#[derive(Clone, Debug)]
pub struct BloonDef {
    pub id: String,
    /// `BloonModel.speedFrames` (+0xE8): track units per frame. Taken
    /// verbatim from decoded data; never derived from `speed` here.
    pub speed_frames: f32,
    /// Collision radius (`Collidable.Radius`, Bloon +0x150), used by the
    /// leak and spawn-clamp tests. HYPOTHESIS H-BLOON-1: initialised from
    /// `BloonModel.radius`.
    pub radius: f32,
    /// `BloonModel.maxHealth`.
    pub max_health: i32,
    /// `BloonModel.GetTotalLeakDamage(...)` result, before
    /// `Simulation.CalcDamageFromBloon` (H-LEAK-2).
    pub total_leak_damage: f32,
    pub is_moab: bool,
    pub is_boss: bool,
    pub rotate_to_follow_path: bool,
}

/// Live bloons, one column per field, kept in **creation order** to mirror
/// the original `process` list (removal is order-preserving).
#[derive(Default, Debug)]
pub struct Bloons {
    pub id: Vec<u32>,
    pub kind: Vec<BloonTypeId>,
    pub path: Vec<u16>,
    pub distance: Vec<f32>,
    pub speed_frames: Vec<f32>,
    pub radius: Vec<f32>,
    /// Per-point `bloonSpeedMultiplier` (Bloon +0x200); 1.0 by default.
    pub track_speed_mult: Vec<f32>,
    pub health: Vec<i32>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
    pub rotation: Vec<f32>,
    pub spawn_round: Vec<i32>,
    pub emission_index: Vec<i32>,
    /// Marked during a frame, compacted at its end.
    pub dead: Vec<bool>,
}

macro_rules! columns {
    ($m:ident, $self:ident $(, $arg:expr)*) => {{
        $self.id.$m($($arg),*);
        $self.kind.$m($($arg),*);
        $self.path.$m($($arg),*);
        $self.distance.$m($($arg),*);
        $self.speed_frames.$m($($arg),*);
        $self.radius.$m($($arg),*);
        $self.track_speed_mult.$m($($arg),*);
        $self.health.$m($($arg),*);
        $self.x.$m($($arg),*);
        $self.y.$m($($arg),*);
        $self.z.$m($($arg),*);
        $self.rotation.$m($($arg),*);
        $self.spawn_round.$m($($arg),*);
        $self.emission_index.$m($($arg),*);
        $self.dead.$m($($arg),*);
    }};
}

impl Bloons {
    pub fn with_capacity(n: usize) -> Self {
        let mut b = Self::default();
        columns!(reserve, b, n);
        b
    }

    pub fn len(&self) -> usize {
        self.id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push(&mut self, id: u32, kind: BloonTypeId, path: u16, distance: f32, def: &BloonDef, round: i32, emission: i32) {
        self.id.push(id);
        self.kind.push(kind);
        self.path.push(path);
        self.distance.push(distance);
        self.speed_frames.push(def.speed_frames);
        self.radius.push(def.radius);
        self.track_speed_mult.push(1.0);
        self.health.push(def.max_health);
        self.x.push(0.0);
        self.y.push(0.0);
        self.z.push(0.0);
        self.rotation.push(0.0);
        self.spawn_round.push(round);
        self.emission_index.push(emission);
        self.dead.push(false);
    }

    /// Removes every bloon marked dead, preserving the order of the rest.
    /// Allocation-free.
    pub fn compact(&mut self) {
        if !self.dead.iter().any(|&d| d) {
            return;
        }
        let mut w = 0;
        for r in 0..self.len() {
            if !self.dead[r] {
                if w != r {
                    self.id[w] = self.id[r];
                    self.kind[w] = self.kind[r];
                    self.path[w] = self.path[r];
                    self.distance[w] = self.distance[r];
                    self.speed_frames[w] = self.speed_frames[r];
                    self.radius[w] = self.radius[r];
                    self.track_speed_mult[w] = self.track_speed_mult[r];
                    self.health[w] = self.health[r];
                    self.x[w] = self.x[r];
                    self.y[w] = self.y[r];
                    self.z[w] = self.z[r];
                    self.rotation[w] = self.rotation[r];
                    self.spawn_round[w] = self.spawn_round[r];
                    self.emission_index[w] = self.emission_index[r];
                    self.dead[w] = false;
                }
                w += 1;
            }
        }
        columns!(truncate, self, w);
    }
}
