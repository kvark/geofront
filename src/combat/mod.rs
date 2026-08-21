//! Turn manager, actions, telegraphs (optional), external support requests.

use glam::IVec2;
use crate::units::{LimbKind, Mech, Pilot, Team};

#[derive(Debug)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
}

impl Grid {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn in_bounds(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width && pos.y < self.height
    }

    pub fn manhattan(a: IVec2, b: IVec2) -> i32 {
        (a.x - b.x).abs() + (a.y - b.y).abs()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Player,
    Enemy,
}

#[derive(Debug, Clone)]
pub enum Action {
    Move { unit_id: u32, to: IVec2 },
    Attack { attacker_id: u32, target_id: u32, limb: LimbKind },
    Wait { unit_id: u32 },
}

#[derive(Debug)]
pub struct Mission {
    pub grid: Grid,
    pub mechs: Vec<Mech>,
    pub pilots: Vec<Pilot>,
    pub phase: TurnPhase,
    pub turn: u32,
    pub city_hp: f32,
    pub log: Vec<String>,
}

impl Mission {
    pub fn new_skirmish() -> Self {
        let grid = Grid::new(8, 8);
        let mut mechs = vec![
            Mech::new_player(0, "Unit-01", IVec2::new(1, 3)),
            Mech::new_player(1, "Unit-02", IVec2::new(1, 4)),
            Mech::new_enemy(10, "Angel-A", IVec2::new(6, 3)),
            Mech::new_enemy(11, "Angel-B", IVec2::new(6, 5)),
        ];
        let pilots = vec![
            Pilot::new(0, "Shinji"),
            Pilot::new(1, "Asuka"),
        ];
        mechs[0].pilot_id = Some(0);
        mechs[1].pilot_id = Some(1);

        Self {
            grid,
            mechs,
            pilots,
            phase: TurnPhase::Player,
            turn: 1,
            city_hp: 100.0,
            log: vec!["Mission start: defend the city block.".into()],
        }
    }

    pub fn living_mechs(&self, team: Team) -> impl Iterator<Item = &Mech> {
        self.mechs.iter().filter(move |m| m.team == team && !m.destroyed)
    }

    pub fn mech_mut(&mut self, id: u32) -> Option<&mut Mech> {
        self.mechs.iter_mut().find(|m| m.id == id)
    }

    pub fn mech(&self, id: u32) -> Option<&Mech> {
        self.mechs.iter().find(|m| m.id == id)
    }

    pub fn is_won(&self) -> bool {
        self.living_mechs(Team::Enemy).count() == 0
    }

    pub fn is_lost(&self) -> bool {
        self.living_mechs(Team::Player).count() == 0 || self.city_hp <= 0.0
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::Move { unit_id, to } => {
                if !self.grid.in_bounds(to) {
                    return Err("Out of bounds".into());
                }
                if self.mechs.iter().any(|m| !m.destroyed && m.position == to) {
                    return Err("Tile occupied".into());
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                if mech.destroyed {
                    return Err("Unit destroyed".into());
                }
                let dist = Grid::manhattan(mech.position, to);
                let max_move = (2.0 * mech.mobility()).ceil() as i32;
                if dist > max_move.max(1) {
                    return Err(format!("Too far (max {max_move})"));
                }
                let name = mech.name.clone();
                mech.position = to;
                self.log.push(format!("{name} moved to ({}, {})", to.x, to.y));
            }
            Action::Attack {
                attacker_id,
                target_id,
                limb,
            } => {
                let (attacker_pos, firepower, name) = {
                    let a = self.mech(attacker_id).ok_or("Unknown attacker")?;
                    if a.destroyed {
                        return Err("Attacker destroyed".into());
                    }
                    (a.position, a.firepower(), a.name.clone())
                };
                let target = self.mech_mut(target_id).ok_or("Unknown target")?;
                if target.destroyed {
                    return Err("Target already destroyed".into());
                }
                let dist = Grid::manhattan(attacker_pos, target.position);
                if dist > 4 {
                    return Err("Out of range".into());
                }
                let base_dmg = 25.0 * firepower;
                let dmg = base_dmg.max(5.0);
                target.apply_damage(limb, dmg);
                let destroyed = target.destroyed;
                let tname = target.name.clone();
                let was_enemy = matches!(target.team, Team::Enemy);
                self.log.push(format!(
                    "{name} attacked {tname} ({limb}) for {dmg:.0} dmg{}",
                    if destroyed { " — DESTROYED" } else { "" }
                ));
                if destroyed && was_enemy {
                    self.city_hp = (self.city_hp + 2.0).min(100.0);
                }
            }
            Action::Wait { unit_id } => {
                if let Some(m) = self.mech(unit_id) {
                    self.log.push(format!("{} waits.", m.name));
                }
            }
        }
        Ok(())
    }

    pub fn run_enemy_turn(&mut self) {
        let player_positions: Vec<(u32, IVec2)> = self
            .living_mechs(Team::Player)
            .map(|m| (m.id, m.position))
            .collect();
        if player_positions.is_empty() {
            return;
        }

        let enemy_ids: Vec<u32> = self.living_mechs(Team::Enemy).map(|m| m.id).collect();
        for eid in enemy_ids {
            let Some(enemy) = self.mech(eid) else { continue };
            let epos = enemy.position;
            let (tid, tpos) = player_positions
                .iter()
                .min_by_key(|(_, p)| Grid::manhattan(epos, *p))
                .copied()
                .unwrap();
            let dist = Grid::manhattan(epos, tpos);
            if dist <= 4 {
                let _ = self.apply_action(Action::Attack {
                    attacker_id: eid,
                    target_id: tid,
                    limb: LimbKind::Torso,
                });
            } else {
                let dx = (tpos.x - epos.x).signum();
                let dy = (tpos.y - epos.y).signum();
                let next = if dx != 0 {
                    IVec2::new(epos.x + dx, epos.y)
                } else {
                    IVec2::new(epos.x, epos.y + dy)
                };
                let _ = self.apply_action(Action::Move {
                    unit_id: eid,
                    to: next,
                });
            }
        }
    }

    pub fn end_player_turn(&mut self) {
        self.phase = TurnPhase::Enemy;
        self.run_enemy_turn();
        self.phase = TurnPhase::Player;
        self.turn += 1;
        if self.living_mechs(Team::Enemy).count() > 0 {
            self.city_hp = (self.city_hp - 3.0).max(0.0);
            self.log.push(format!(
                "City took collateral damage. Protection: {:.0}%",
                self.city_hp
            ));
        }
    }

    pub fn smoke_run(&mut self, max_turns: u32) {
        for _ in 0..max_turns {
            if self.is_won() || self.is_lost() {
                break;
            }
            let enemy_ids: Vec<u32> = self.living_mechs(Team::Enemy).map(|m| m.id).collect();
            let player_ids: Vec<u32> = self.living_mechs(Team::Player).map(|m| m.id).collect();
            for pid in player_ids {
                if let Some(&eid) = enemy_ids.first() {
                    let _ = self.apply_action(Action::Attack {
                        attacker_id: pid,
                        target_id: eid,
                        limb: LimbKind::Torso,
                    });
                }
            }
            self.end_player_turn();
        }
        if self.is_won() {
            self.log.push("SMOKE: Mission won.".into());
        } else if self.is_lost() {
            self.log.push("SMOKE: Mission lost.".into());
        } else {
            self.log.push(format!("SMOKE: Stopped after {} turns.", self.turn));
        }
    }
}
