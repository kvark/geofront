//! Mechs (limb components, damage, equipment) and pilots.

use glam::IVec2;

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
        };
        m.refresh_turn();
        m
    }

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
        self.move_left = (2.0 * self.mobility()).ceil() as i32;
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

    pub fn attack_range(&self) -> i32 {
        if self.firepower() <= 0.05 {
            1
        } else {
            4
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
}

impl Pilot {
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            sync: 0.7,
            loyalty: 0.8,
            stress: 0.1,
        }
    }

    pub fn disobedience_chance(&self) -> f32 {
        let pressure = self.stress * (1.0 - self.loyalty) * (1.0 - self.sync);
        pressure.clamp(0.0, 0.6)
    }
}
