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
    pub limbs: Vec<Limb>,
    pub pilot_id: Option<u32>,
    pub destroyed: bool,
}

impl Mech {
    pub fn new_player(id: u32, name: impl Into<String>, pos: IVec2) -> Self {
        Self {
            id,
            name: name.into(),
            team: Team::Player,
            position: pos,
            limbs: vec![
                Limb::new(LimbKind::Torso, 100.0),
                Limb::new(LimbKind::LeftArm, 60.0),
                Limb::new(LimbKind::RightArm, 60.0),
                Limb::new(LimbKind::LeftLeg, 50.0),
                Limb::new(LimbKind::RightLeg, 50.0),
            ],
            pilot_id: None,
            destroyed: false,
        }
    }

    pub fn new_enemy(id: u32, name: impl Into<String>, pos: IVec2) -> Self {
        Self {
            id,
            name: name.into(),
            team: Team::Enemy,
            position: pos,
            limbs: vec![
                Limb::new(LimbKind::Torso, 80.0),
                Limb::new(LimbKind::LeftArm, 40.0),
                Limb::new(LimbKind::RightArm, 40.0),
                Limb::new(LimbKind::LeftLeg, 35.0),
                Limb::new(LimbKind::RightLeg, 35.0),
            ],
            pilot_id: None,
            destroyed: false,
        }
    }

    pub fn limb_mut(&mut self, kind: LimbKind) -> Option<&mut Limb> {
        self.limbs.iter_mut().find(|l| l.kind == kind)
    }

    pub fn apply_damage(&mut self, kind: LimbKind, amount: f32) {
        if let Some(limb) = self.limb_mut(kind) {
            limb.hp = (limb.hp - amount).max(0.0);
        }
        // Destroyed if torso is gone or all limbs gone.
        let torso_ok = self
            .limbs
            .iter()
            .any(|l| l.kind == LimbKind::Torso && l.is_functional());
        if !torso_ok {
            self.destroyed = true;
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
}

#[derive(Debug)]
pub struct Pilot {
    pub id: u32,
    pub name: String,
    pub sync: f32,    // 0.0..=1.0+ with current mech
    pub loyalty: f32, // to commander / team
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

    /// Chance the pilot will refuse or half-heartedly follow a risky order.
    pub fn disobedience_chance(&self) -> f32 {
        let pressure = self.stress * (1.0 - self.loyalty) * (1.0 - self.sync);
        pressure.clamp(0.0, 0.6)
    }
}
