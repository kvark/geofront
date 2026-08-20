//! Mechs (limb components, damage, equipment) and pilots.

#[derive(Debug, Clone)]
pub struct Limb {
    pub hp: f32,
    pub max_hp: f32,
}

#[derive(Debug)]
pub struct Mech {
    pub limbs: Vec<Limb>,
    // torso, weapons, systems
}

#[derive(Debug)]
pub struct Pilot {
    pub name: String,
    pub sync: f32,       // 0.0..=1.0+ with current mech
    pub loyalty: f32,    // to commander / team
    pub stress: f32,
}
