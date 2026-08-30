//! Turn manager, actions, telegraphs (optional), external support requests.

use glam::IVec2;
use crate::units::{Facing, LimbKind, Mech, Pilot, Team};

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
    /// One orthogonal step.
    Move { unit_id: u32, to: IVec2 },
    Rotate { unit_id: u32, facing: Facing },
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
    /// Enemy actions still to play out (one at a time for presentation).
    pub enemy_queue: Vec<Action>,
}

impl Mission {
    pub fn new_skirmish() -> Self {
        let grid = Grid::new(8, 8);
        // Opening Manhattan distances are ≤ attack_range (4) so the first
        // player volley and the smoke test can actually connect.
        let mut mechs = vec![
            Mech::new_player(0, "Coil", IVec2::new(2, 3)),
            Mech::new_player(1, "Bastion", IVec2::new(2, 4)),
            Mech::new_enemy(10, "Razor", IVec2::new(5, 3)),
            Mech::new_enemy(11, "Husk", IVec2::new(5, 5)),
        ];
        let pilots = vec![Pilot::new(0, "Nori"), Pilot::new(1, "Vesper")];
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
            enemy_queue: Vec::new(),
        }
    }

    pub fn living_mechs(&self, team: Team) -> impl Iterator<Item = &Mech> {
        self.mechs
            .iter()
            .filter(move |m| m.team == team && !m.destroyed)
    }

    pub fn mech_mut(&mut self, id: u32) -> Option<&mut Mech> {
        self.mechs.iter_mut().find(|m| m.id == id)
    }

    pub fn mech(&self, id: u32) -> Option<&Mech> {
        self.mechs.iter().find(|m| m.id == id)
    }

    pub fn pilot(&self, id: u32) -> Option<&Pilot> {
        self.pilots.iter().find(|p| p.id == id)
    }

    pub fn is_won(&self) -> bool {
        self.living_mechs(Team::Enemy).count() == 0
    }

    pub fn is_lost(&self) -> bool {
        self.living_mechs(Team::Player).count() == 0 || self.city_hp <= 0.0
    }

    pub fn occupied(&self, pos: IVec2, ignore_id: Option<u32>) -> bool {
        self.mechs.iter().any(|m| {
            !m.destroyed && m.position == pos && Some(m.id) != ignore_id
        })
    }

    fn team_of(&self, id: u32) -> Option<Team> {
        self.mech(id).map(|m| m.team)
    }

    fn phase_allows(&self, team: Team) -> bool {
        match self.phase {
            TurnPhase::Player => matches!(team, Team::Player),
            TurnPhase::Enemy => matches!(team, Team::Enemy),
        }
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::Move { unit_id, to } => {
                if !self.grid.in_bounds(to) {
                    return Err("Out of bounds".into());
                }
                if self.occupied(to, Some(unit_id)) {
                    return Err("Tile occupied".into());
                }
                let team = self.team_of(unit_id).ok_or("Unknown unit")?;
                if !self.phase_allows(team) {
                    return Err("Not this team's turn".into());
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                if !mech.can_move() {
                    return Err("No move left".into());
                }
                let delta = to - mech.position;
                if delta.x.abs() + delta.y.abs() != 1 {
                    return Err("Must step one tile (orthogonal)".into());
                }
                let facing = Facing::from_delta(delta).unwrap_or(mech.facing);
                mech.position = to;
                mech.facing = facing;
                mech.move_left -= 1;
                let name = mech.name.clone();
                let face = mech.facing.label();
                let mp = mech.move_left;
                self.log.push(format!(
                    "{name} steps to ({}, {}) facing {face}  [{mp} MP]",
                    to.x, to.y
                ));
            }
            Action::Rotate { unit_id, facing } => {
                let team = self.team_of(unit_id).ok_or("Unknown unit")?;
                if !self.phase_allows(team) {
                    return Err("Not this team's turn".into());
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                if mech.destroyed || mech.acted {
                    return Err("Cannot rotate".into());
                }
                mech.facing = facing;
                let name = mech.name.clone();
                let label = facing.label();
                self.log.push(format!("{name} turns {label}"));
            }
            Action::Attack {
                attacker_id,
                target_id,
                limb,
            } => {
                let team = self.team_of(attacker_id).ok_or("Unknown attacker")?;
                if !self.phase_allows(team) {
                    return Err("Not this team's turn".into());
                }

                let (attacker_pos, facing, firepower, name, range, pilot_id, destroyed) = {
                    let a = self.mech(attacker_id).ok_or("Unknown attacker")?;
                    (
                        a.position,
                        a.facing,
                        a.firepower(),
                        a.name.clone(),
                        a.attack_range(),
                        a.pilot_id,
                        a.destroyed,
                    )
                };
                if destroyed {
                    return Err("Attacker destroyed".into());
                }
                {
                    let a = self.mech(attacker_id).ok_or("Unknown attacker")?;
                    if !a.can_act() {
                        return Err("Already acted".into());
                    }
                }

                if matches!(team, Team::Player) {
                    if let Some(pid) = pilot_id {
                        if let Some(pilot) = self.pilot(pid) {
                            let chance = pilot.disobedience_chance();
                            let roll = ((self.turn.wrapping_mul(17) + attacker_id) % 100) as f32
                                / 100.0;
                            if chance > 0.02 && roll < chance {
                                if let Some(a) = self.mech_mut(attacker_id) {
                                    a.acted = true;
                                }
                                self.log.push(format!(
                                    "{name} hesitates (sync break) and holds fire."
                                ));
                                return Ok(());
                            }
                        }
                    }
                }

                let target = self.mech_mut(target_id).ok_or("Unknown target")?;
                if target.destroyed {
                    return Err("Target already destroyed".into());
                }
                let tpos = target.position;
                let dist = Grid::manhattan(attacker_pos, tpos);
                if dist > range {
                    return Err("Out of range".into());
                }
                let look = facing.delta();
                let toward = IVec2::new((tpos.x - attacker_pos.x).signum(), (tpos.y - attacker_pos.y).signum());
                let flanked = look != toward && dist > 0;
                let mut dmg = (25.0 * firepower).max(5.0);
                if !flanked {
                    dmg *= 1.15;
                }
                target.apply_damage(limb, dmg);
                let destroyed_t = target.destroyed;
                let tname = target.name.clone();
                let was_enemy = matches!(target.team, Team::Enemy);
                if let Some(a) = self.mech_mut(attacker_id) {
                    a.acted = true;
                    a.move_left = 0;
                    // Face the target if it was a cardinal shot.
                    if let Some(f) = Facing::from_delta(IVec2::new(
                        (tpos.x - attacker_pos.x).signum(),
                        (tpos.y - attacker_pos.y).signum(),
                    )) {
                        if (tpos.x - attacker_pos.x).abs() == 0
                            || (tpos.y - attacker_pos.y).abs() == 0
                        {
                            a.facing = f;
                        }
                    }
                }
                self.log.push(format!(
                    "{name} attacked {tname} ({limb}) for {dmg:.0} dmg{}",
                    if destroyed_t { " — DESTROYED" } else { "" }
                ));
                if destroyed_t && was_enemy {
                    self.city_hp = (self.city_hp + 2.0).min(100.0);
                }
            }
            Action::Wait { unit_id } => {
                let team = self.team_of(unit_id).ok_or("Unknown unit")?;
                if !self.phase_allows(team) {
                    return Err("Not this team's turn".into());
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                if mech.destroyed {
                    return Err("Destroyed".into());
                }
                mech.acted = true;
                mech.move_left = 0;
                let name = mech.name.clone();
                self.log.push(format!("{name} holds position."));
            }
        }
        Ok(())
    }

    fn refresh_team(&mut self, team: Team) {
        for m in self.mechs.iter_mut().filter(|m| m.team == team) {
            m.refresh_turn();
        }
    }

    /// Plan enemy moves into `enemy_queue`. Presentation plays them one by one.
    pub fn begin_enemy_turn(&mut self) {
        self.phase = TurnPhase::Enemy;
        self.enemy_queue.clear();
        self.refresh_team(Team::Enemy);

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
            let mut epos = enemy.position;
            let mut mp = enemy.move_left;
            let range = enemy.attack_range();
            let (tid, tpos) = player_positions
                .iter()
                .min_by_key(|(_, p)| Grid::manhattan(epos, *p))
                .copied()
                .unwrap();

            while mp > 0 && Grid::manhattan(epos, tpos) > range {
                let dx = (tpos.x - epos.x).signum();
                let dy = (tpos.y - epos.y).signum();
                let next = if dx != 0 {
                    IVec2::new(epos.x + dx, epos.y)
                } else {
                    IVec2::new(epos.x, epos.y + dy)
                };
                if !self.grid.in_bounds(next) || self.occupied(next, Some(eid)) {
                    break;
                }
                self.enemy_queue.push(Action::Move {
                    unit_id: eid,
                    to: next,
                });
                epos = next;
                mp -= 1;
            }

            let dist = Grid::manhattan(epos, tpos);
            if dist <= range {
                self.enemy_queue.push(Action::Attack {
                    attacker_id: eid,
                    target_id: tid,
                    limb: LimbKind::Torso,
                });
            } else {
                self.enemy_queue.push(Action::Wait { unit_id: eid });
            }
        }
        self.log.push("Enemy phase.".into());
    }

    /// Apply the next queued enemy action. Returns it so the view can animate.
    pub fn step_enemy_queue(&mut self) -> Option<Action> {
        if self.enemy_queue.is_empty() {
            return None;
        }
        let action = self.enemy_queue.remove(0);
        let _ = self.apply_action(action.clone());
        Some(action)
    }

    pub fn finish_enemy_turn(&mut self) {
        self.phase = TurnPhase::Player;
        self.turn += 1;
        self.refresh_team(Team::Player);
        if self.living_mechs(Team::Enemy).count() > 0 {
            self.city_hp = (self.city_hp - 3.0).max(0.0);
            self.log.push(format!(
                "City took collateral damage. Protection: {:.0}%",
                self.city_hp
            ));
        }
        self.log.push(format!("Turn {} — your move.", self.turn));
    }

    /// Instant full enemy resolution (smoke tests / skip anim).
    pub fn end_player_turn(&mut self) {
        self.begin_enemy_turn();
        while self.step_enemy_queue().is_some() {}
        self.finish_enemy_turn();
    }

    pub fn smoke_run(&mut self, max_turns: u32) {
        let mut player_hits = 0u32;
        for _ in 0..max_turns {
            if self.is_won() || self.is_lost() {
                break;
            }
            let enemy_ids: Vec<u32> = self.living_mechs(Team::Enemy).map(|m| m.id).collect();
            let player_ids: Vec<u32> = self.living_mechs(Team::Player).map(|m| m.id).collect();
            for pid in player_ids {
                let Some(attacker) = self.mech(pid) else {
                    continue;
                };
                let from = attacker.position;
                let Some(&eid) = enemy_ids.iter().min_by_key(|&&id| {
                    self.mech(id)
                        .map(|m| Grid::manhattan(from, m.position))
                        .unwrap_or(i32::MAX)
                }) else {
                    continue;
                };
                if self.apply_action(Action::Attack {
                    attacker_id: pid,
                    target_id: eid,
                    limb: LimbKind::Torso,
                })
                .is_ok()
                {
                    player_hits += 1;
                }
            }
            self.end_player_turn();
        }
        self.log
            .push(format!("SMOKE: player hits={player_hits}"));
        if self.is_won() {
            self.log.push("SMOKE: Mission won.".into());
        } else if self.is_lost() {
            self.log.push("SMOKE: Mission lost.".into());
        } else {
            self.log.push(format!("SMOKE: Stopped after {} turns.", self.turn));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthogonal_step_and_action_lock() {
        let mut m = Mission::new_skirmish();
        let start = m.mech(0).unwrap().position;
        assert!(m
            .apply_action(Action::Move {
                unit_id: 0,
                to: start + IVec2::new(1, 0)
            })
            .is_ok());
        assert_eq!(m.mech(0).unwrap().facing, Facing::East);
        // Diagonal illegal
        let pos = m.mech(0).unwrap().position;
        assert!(m
            .apply_action(Action::Move {
                unit_id: 0,
                to: pos + IVec2::new(1, 1)
            })
            .is_err());
        assert!(m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 10,
                limb: LimbKind::Torso,
            })
            .is_ok());
        // Cannot move after acting
        let pos = m.mech(0).unwrap().position;
        assert!(m
            .apply_action(Action::Move {
                unit_id: 0,
                to: pos + IVec2::new(1, 0)
            })
            .is_err());
    }

    #[test]
    fn opening_attack_in_range() {
        let mut m = Mission::new_skirmish();
        let a = m.mech(0).unwrap();
        let b = m.mech(10).unwrap();
        assert!(Grid::manhattan(a.position, b.position) <= a.attack_range());
        assert!(m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 10,
                limb: LimbKind::Torso,
            })
            .is_ok());
    }

    #[test]
    fn smoke_lands_a_player_attack() {
        let mut m = Mission::new_skirmish();
        m.smoke_run(6);
        assert!(
            m.log.iter().any(|l| l.contains("SMOKE: player hits=")
                && !l.ends_with("hits=0")),
            "smoke log: {:?}",
            m.log
        );
        assert!(m
            .log
            .iter()
            .any(|l| l.contains("Coil attacked") || l.contains("Bastion attacked")));
    }

    #[test]
    fn smoke_does_not_panic() {
        let mut m = Mission::new_skirmish();
        m.smoke_run(4);
        assert!(!m.log.is_empty());
    }
}
