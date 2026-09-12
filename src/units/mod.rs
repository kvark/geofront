//! Mechs (limb components, damage, equipment), aliens, and pilots.

use glam::IVec2;

/// Limb HP ratio at or below this is a wrecked limb (stress trigger).
pub const HEAVY_LIMB_RATIO: f32 = 0.25;
/// Torso HP ratio that counts as near-death.
pub const NEAR_DEATH_TORSO_RATIO: f32 = 0.30;
/// Whole-mech HP ratio that counts as near-death.
pub const NEAR_DEATH_TOTAL_RATIO: f32 = 0.35;
/// Stress added on a trauma spike.
pub const STRESS_SPIKE: f32 = 0.35;
/// Sync lost on a trauma spike (floor 0.2).
pub const SYNC_DIP: f32 = 0.06;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimbKind {
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl std::fmt::Display for LimbKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimbKind::Torso => write!(f, "Torso"),
            LimbKind::LeftArm => write!(f, "L.Arm"),
            LimbKind::RightArm => write!(f, "R.Arm"),
            LimbKind::LeftLeg => write!(f, "L.Leg"),
            LimbKind::RightLeg => write!(f, "R.Leg"),
        }
    }
}

/// Cardinal facing on the tactical grid. +X is east, +Y (world +Z) is north.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    pub fn delta(self) -> IVec2 {
        match self {
            Facing::North => IVec2::new(0, 1),
            Facing::East => IVec2::new(1, 0),
            Facing::South => IVec2::new(0, -1),
            Facing::West => IVec2::new(-1, 0),
        }
    }

    /// Yaw so a +Z-facing mesh looks along this heading.
    pub fn yaw(self) -> f32 {
        match self {
            Facing::North => 0.0,
            Facing::East => std::f32::consts::FRAC_PI_2,
            Facing::South => std::f32::consts::PI,
            Facing::West => -std::f32::consts::FRAC_PI_2,
        }
    }

    pub fn from_delta(d: IVec2) -> Option<Self> {
        match (d.x.signum(), d.y.signum()) {
            (0, 1) => Some(Facing::North),
            (1, 0) => Some(Facing::East),
            (0, -1) => Some(Facing::South),
            (-1, 0) => Some(Facing::West),
            _ => None,
        }
    }

    pub fn rotate_cw(self) -> Self {
        match self {
            Facing::North => Facing::East,
            Facing::East => Facing::South,
            Facing::South => Facing::West,
            Facing::West => Facing::North,
        }
    }

    pub fn rotate_ccw(self) -> Self {
        match self {
            Facing::North => Facing::West,
            Facing::West => Facing::South,
            Facing::South => Facing::East,
            Facing::East => Facing::North,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Facing::North => "N",
            Facing::East => "E",
            Facing::South => "S",
            Facing::West => "W",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Limb {
    pub kind: LimbKind,
    pub hp: f32,
    pub max_hp: f32,
}

impl Limb {
    pub fn new(kind: LimbKind, max_hp: f32) -> Self {
        Self {
            kind,
            hp: max_hp,
            max_hp,
        }
    }

    pub fn is_functional(&self) -> bool {
        self.hp > 0.0
    }

    pub fn damage_ratio(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
}

/// Alien combat archetypes. Placeholder visuals; distinct stats + AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlienKind {
    /// Fragile harasser: kites, chips limbs at long range.
    Splinter,
    /// Slow pressure: closes and smashes at short range.
    Mass,
}

impl AlienKind {
    pub fn label(self) -> &'static str {
        match self {
            AlienKind::Splinter => "Splinter",
            AlienKind::Mass => "Mass",
        }
    }
}

#[derive(Debug)]
pub struct Mech {
    pub id: u32,
    pub name: String,
    pub team: Team,
    pub position: IVec2,
    pub facing: Facing,
    pub limbs: Vec<Limb>,
    pub pilot_id: Option<u32>,
    pub destroyed: bool,
    /// Orthogonal steps remaining this turn.
    pub move_left: i32,
    /// Attack or Wait already spent this turn.
    pub acted: bool,
    /// `Some` for alien enemies; player mechs stay `None`.
    pub alien: Option<AlienKind>,
    /// Remaining AT Field absorbs (Mass starts charged; Eva-ish deflect).
    pub at_field: u8,
}

impl Mech {
    pub fn new_player(id: u32, name: impl Into<String>, pos: IVec2) -> Self {
        let mut m = Self {
            id,
            name: name.into(),
            team: Team::Player,
            position: pos,
            facing: Facing::East,
            limbs: vec![
                Limb::new(LimbKind::Torso, 100.0),
                Limb::new(LimbKind::LeftArm, 60.0),
                Limb::new(LimbKind::RightArm, 60.0),
                Limb::new(LimbKind::LeftLeg, 50.0),
                Limb::new(LimbKind::RightLeg, 50.0),
            ],
            pilot_id: None,
            destroyed: false,
            move_left: 0,
            acted: false,
            alien: None,
            at_field: 0,
        };
        m.refresh_turn();
        m
    }

    /// Generic enemy mech (legacy). Prefer [`Self::new_alien`].
    pub fn new_enemy(id: u32, name: impl Into<String>, pos: IVec2) -> Self {
        let mut m = Self {
            id,
            name: name.into(),
            team: Team::Enemy,
            position: pos,
            facing: Facing::West,
            limbs: vec![
                Limb::new(LimbKind::Torso, 80.0),
                Limb::new(LimbKind::LeftArm, 40.0),
                Limb::new(LimbKind::RightArm, 40.0),
                Limb::new(LimbKind::LeftLeg, 35.0),
                Limb::new(LimbKind::RightLeg, 35.0),
            ],
            pilot_id: None,
            destroyed: false,
            move_left: 0,
            acted: false,
            alien: None,
            at_field: 0,
        };
        m.refresh_turn();
        m
    }

    pub fn new_alien(id: u32, kind: AlienKind, pos: IVec2) -> Self {
        let (limbs, facing) = match kind {
            AlienKind::Splinter => (
                vec![
                    Limb::new(LimbKind::Torso, 55.0),
                    Limb::new(LimbKind::LeftArm, 50.0),
                    Limb::new(LimbKind::RightArm, 50.0),
                    Limb::new(LimbKind::LeftLeg, 55.0),
                    Limb::new(LimbKind::RightLeg, 55.0),
                ],
                Facing::West,
            ),
            AlienKind::Mass => (
                vec![
                    Limb::new(LimbKind::Torso, 120.0),
                    Limb::new(LimbKind::LeftArm, 25.0),
                    Limb::new(LimbKind::RightArm, 25.0),
                    Limb::new(LimbKind::LeftLeg, 70.0),
                    Limb::new(LimbKind::RightLeg, 70.0),
                ],
                Facing::West,
            ),
        };
        let mut m = Self {
            id,
            name: kind.label().into(),
            team: Team::Enemy,
            position: pos,
            facing,
            limbs,
            pilot_id: None,
            destroyed: false,
            move_left: 0,
            acted: false,
            alien: Some(kind),
            // Mass pressure: two AT Field absorbs before limbs take damage.
            at_field: if matches!(kind, AlienKind::Mass) { 2 } else { 0 },
        };
        m.refresh_turn();
        m
    }

    pub fn refresh_turn(&mut self) {
        if self.destroyed {
            self.move_left = 0;
            self.acted = true;
            return;
        }
        let mult = match self.alien {
            Some(AlienKind::Splinter) => 3.0,
            Some(AlienKind::Mass) => 1.0,
            None => 2.0,
        };
        self.move_left = (mult * self.mobility()).ceil() as i32;
        self.move_left = self.move_left.max(1);
        self.acted = false;
    }

    pub fn can_move(&self) -> bool {
        !self.destroyed && !self.acted && self.move_left > 0
    }

    pub fn can_act(&self) -> bool {
        !self.destroyed && !self.acted
    }

    pub fn limb_mut(&mut self, kind: LimbKind) -> Option<&mut Limb> {
        self.limbs.iter_mut().find(|l| l.kind == kind)
    }

    pub fn apply_damage(&mut self, kind: LimbKind, amount: f32) {
        if let Some(limb) = self.limb_mut(kind) {
            limb.hp = (limb.hp - amount).max(0.0);
        }
        let torso_ok = self
            .limbs
            .iter()
            .any(|l| l.kind == LimbKind::Torso && l.is_functional());
        if !torso_ok {
            self.destroyed = true;
            self.move_left = 0;
            self.acted = true;
        }
    }

    /// Spend one AT Field charge. Returns `true` if the strike was absorbed.
    pub fn try_absorb_at_field(&mut self) -> bool {
        if self.destroyed || self.at_field == 0 {
            return false;
        }
        self.at_field -= 1;
        true
    }

    pub fn mobility(&self) -> f32 {
        let legs: f32 = self
            .limbs
            .iter()
            .filter(|l| matches!(l.kind, LimbKind::LeftLeg | LimbKind::RightLeg))
            .map(|l| l.damage_ratio())
            .sum();
        (legs / 2.0).clamp(0.0, 1.0)
    }

    pub fn firepower(&self) -> f32 {
        let arms: f32 = self
            .limbs
            .iter()
            .filter(|l| matches!(l.kind, LimbKind::LeftArm | LimbKind::RightArm))
            .map(|l| l.damage_ratio())
            .sum();
        (arms / 2.0).clamp(0.0, 1.0)
    }

    pub fn total_hp(&self) -> (f32, f32) {
        let cur: f32 = self.limbs.iter().map(|l| l.hp).sum();
        let max: f32 = self.limbs.iter().map(|l| l.max_hp).sum();
        (cur, max)
    }

    pub fn limb_ratio(&self, kind: LimbKind) -> f32 {
        self.limbs
            .iter()
            .find(|l| l.kind == kind)
            .map(Limb::damage_ratio)
            .unwrap_or(0.0)
    }

    pub fn total_hp_ratio(&self) -> f32 {
        let (cur, max) = self.total_hp();
        if max <= 0.0 {
            0.0
        } else {
            (cur / max).clamp(0.0, 1.0)
        }
    }

    pub fn is_near_death(&self) -> bool {
        self.limb_ratio(LimbKind::Torso) <= NEAR_DEATH_TORSO_RATIO
            || self.total_hp_ratio() <= NEAR_DEATH_TOTAL_RATIO
    }

    pub fn attack_range(&self) -> i32 {
        if self.firepower() <= 0.05 {
            return 1;
        }
        match self.alien {
            Some(AlienKind::Splinter) => 5,
            Some(AlienKind::Mass) => 2,
            None => 4,
        }
    }

    /// Preferred limb when this unit attacks (aliens differ).
    pub fn preferred_attack_limb(&self) -> LimbKind {
        match self.alien {
            Some(AlienKind::Splinter) => LimbKind::LeftArm,
            Some(AlienKind::Mass) => LimbKind::Torso,
            None => LimbKind::Torso,
        }
    }
}

#[derive(Debug)]
pub struct Pilot {
    pub id: u32,
    pub name: String,
    pub sync: f32,
    pub loyalty: f32,
    pub stress: f32,
    /// After a trauma spike, the next order may be refused once.
    pub pending_refuse: bool,
}

impl Pilot {
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            sync: 0.7,
            loyalty: 0.8,
            stress: 0.1,
            pending_refuse: false,
        }
    }

    pub fn disobedience_chance(&self) -> f32 {
        let pressure = self.stress * (1.0 - self.loyalty) * (1.0 - self.sync);
        pressure.clamp(0.0, 0.6)
    }

    /// Chance to skip the next order. Zero unless a spike armed `pending_refuse`.
    pub fn refuse_chance(&self) -> f32 {
        if !self.pending_refuse {
            return 0.0;
        }
        // Visible Eva beat (~20–55%) that still respects loyalty / sync.
        let panic = 0.22 + 0.45 * self.stress * (1.15 - self.loyalty) * (1.1 - 0.5 * self.sync);
        panic.clamp(0.18, 0.55)
    }

    /// Spike stress after a wrecked limb or near-death; arm a one-shot refuse.
    pub fn spike_from_hit(&mut self) {
        self.stress = (self.stress + STRESS_SPIKE).clamp(0.0, 1.0);
        self.sync = (self.sync - SYNC_DIP).clamp(0.2, 1.0);
        self.pending_refuse = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pilot_does_not_refuse() {
        let p = Pilot::new(0, "Nori");
        assert_eq!(p.refuse_chance(), 0.0);
        assert!(p.disobedience_chance() < 0.02);
        assert!(!p.pending_refuse);
    }

    #[test]
    fn trauma_spike_arms_refuse() {
        let mut p = Pilot::new(0, "Nori");
        let sync0 = p.sync;
        p.spike_from_hit();
        assert!(p.pending_refuse);
        assert!((p.stress - 0.45).abs() < 1e-5);
        assert!(p.sync < sync0);
        assert!(p.refuse_chance() >= 0.18);
        assert!(p.refuse_chance() <= 0.55);
    }

    #[test]
    fn near_death_from_torso() {
        let mut m = Mech::new_player(0, "Coil", IVec2::new(0, 0));
        assert!(!m.is_near_death());
        m.apply_damage(LimbKind::Torso, 75.0);
        assert!(m.limb_ratio(LimbKind::Torso) <= NEAR_DEATH_TORSO_RATIO);
        assert!(m.is_near_death());
    }
}
