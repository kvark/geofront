//! Turn manager, actions, telegraphs (optional), external support requests.

use crate::units::{AlienKind, Facing, LimbKind, Mech, Pilot, Team, HEAVY_LIMB_RATIO};
use glam::IVec2;

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
    Move {
        unit_id: u32,
        to: IVec2,
    },
    Rotate {
        unit_id: u32,
        facing: Facing,
    },
    Attack {
        attacker_id: u32,
        target_id: u32,
        limb: LimbKind,
    },
    Wait {
        unit_id: u32,
    },
}

/// Outcome of a legal order. `Skipped` is a pilot refuse (unit locks up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    Done,
    Skipped,
}

/// One-shot presentation hooks drained by the battle view after `apply_action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatFx {
    /// Cyan AT Field deflect on the target (no limb damage).
    AtFieldAbsorb { target_id: u32 },
    /// Last charge spent — field shatters.
    AtFieldBreak { target_id: u32 },
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
    /// Presentation FX produced by the last applied action(s).
    pub pending_fx: Vec<CombatFx>,
}

impl Mission {
    pub fn new_skirmish() -> Self {
        let grid = Grid::new(8, 8);
        // Opening distances keep smoke/tests able to connect: Splinter (range 5)
        // and Mass (range 2) sit on the east side facing player mechs.
        let mut mechs = vec![
            Mech::new_player(0, "Coil", IVec2::new(2, 3)),
            Mech::new_player(1, "Bastion", IVec2::new(2, 4)),
            Mech::new_alien(10, AlienKind::Splinter, IVec2::new(5, 3)),
            Mech::new_alien(11, AlienKind::Mass, IVec2::new(5, 5)),
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
            pending_fx: Vec::new(),
        }
    }

    /// Drain presentation FX produced since the last call.
    pub fn take_fx(&mut self) -> Vec<CombatFx> {
        std::mem::take(&mut self.pending_fx)
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

    pub fn pilot_mut(&mut self, id: u32) -> Option<&mut Pilot> {
        self.pilots.iter_mut().find(|p| p.id == id)
    }

    pub fn is_won(&self) -> bool {
        self.living_mechs(Team::Enemy).count() == 0
    }

    pub fn is_lost(&self) -> bool {
        self.living_mechs(Team::Player).count() == 0 || self.city_hp <= 0.0
    }

    pub fn occupied(&self, pos: IVec2, ignore_id: Option<u32>) -> bool {
        self.mechs
            .iter()
            .any(|m| !m.destroyed && m.position == pos && Some(m.id) != ignore_id)
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

    pub fn apply_action(&mut self, action: Action) -> Result<ApplyResult, String> {
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
                {
                    let mech = self.mech(unit_id).ok_or("Unknown unit")?;
                    if !mech.can_move() {
                        return Err("No move left".into());
                    }
                    let delta = to - mech.position;
                    if delta.x.abs() + delta.y.abs() != 1 {
                        return Err("Must step one tile (orthogonal)".into());
                    }
                }
                if matches!(team, Team::Player) && self.try_refuse(unit_id, "step") {
                    return Ok(ApplyResult::Skipped);
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                let facing = Facing::from_delta(to - mech.position).unwrap_or(mech.facing);
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
                {
                    let mech = self.mech(unit_id).ok_or("Unknown unit")?;
                    if mech.destroyed || mech.acted {
                        return Err("Cannot rotate".into());
                    }
                }
                if matches!(team, Team::Player) && self.try_refuse(unit_id, "turn") {
                    return Ok(ApplyResult::Skipped);
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
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

                let (attacker_pos, facing, firepower, name, range, destroyed) = {
                    let a = self.mech(attacker_id).ok_or("Unknown attacker")?;
                    (
                        a.position,
                        a.facing,
                        a.firepower(),
                        a.name.clone(),
                        a.attack_range(),
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

                if matches!(team, Team::Player) && self.try_refuse(attacker_id, "attack") {
                    return Ok(ApplyResult::Skipped);
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
                let toward = IVec2::new(
                    (tpos.x - attacker_pos.x).signum(),
                    (tpos.y - attacker_pos.y).signum(),
                );
                let flanked = look != toward && dist > 0;
                let mut dmg = (25.0 * firepower).max(5.0);
                if !flanked {
                    dmg *= 1.15;
                }
                let before_limb = target.limb_ratio(limb);
                let before_near = target.is_near_death();

                // AT Field: absorb before limbs. Mass starts with charges.
                let absorbed = target.try_absorb_at_field();
                let remaining_field = target.at_field;
                let tname = target.name.clone();
                let was_enemy = matches!(target.team, Team::Enemy);
                let mut destroyed_t = false;
                if absorbed {
                    self.pending_fx.push(CombatFx::AtFieldAbsorb { target_id });
                    if remaining_field == 0 {
                        self.pending_fx.push(CombatFx::AtFieldBreak { target_id });
                        self.log.push(format!(
                            "{name} strikes {tname} — AT Field absorbs, then SHATTERS!"
                        ));
                    } else {
                        self.log.push(format!(
                            "{name} strikes {tname} — AT Field absorbs! ({remaining_field} left)"
                        ));
                    }
                } else {
                    target.apply_damage(limb, dmg);
                    destroyed_t = target.destroyed;
                    self.log.push(format!(
                        "{name} attacked {tname} ({limb}) for {dmg:.0} dmg{}",
                        if destroyed_t { " — DESTROYED" } else { "" }
                    ));
                }

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
                if destroyed_t && was_enemy {
                    self.city_hp = (self.city_hp + 2.0).min(100.0);
                }
                self.maybe_stress_spike(target_id, limb, before_limb, before_near);
            }
            Action::Wait { unit_id } => {
                let team = self.team_of(unit_id).ok_or("Unknown unit")?;
                if !self.phase_allows(team) {
                    return Err("Not this team's turn".into());
                }
                {
                    let mech = self.mech(unit_id).ok_or("Unknown unit")?;
                    if mech.destroyed {
                        return Err("Destroyed".into());
                    }
                }
                if matches!(team, Team::Player) && self.try_refuse(unit_id, "wait") {
                    return Ok(ApplyResult::Skipped);
                }
                let mech = self.mech_mut(unit_id).ok_or("Unknown unit")?;
                mech.acted = true;
                mech.move_left = 0;
                let name = mech.name.clone();
                self.log.push(format!("{name} holds position."));
            }
        }
        Ok(ApplyResult::Done)
    }

    /// Deterministic 0–1 roll so tests and autoplay stay repeatable.
    fn order_roll(turn: u32, unit_id: u32) -> f32 {
        ((turn.wrapping_mul(17) + unit_id) % 100) as f32 / 100.0
    }

    /// If a trauma spike armed a refuse, consume it. Returns true when the order is skipped.
    fn try_refuse(&mut self, unit_id: u32, order: &str) -> bool {
        let Some(pid) = self.mech(unit_id).and_then(|m| m.pilot_id) else {
            return false;
        };
        let Some(idx) = self.pilots.iter().position(|p| p.id == pid) else {
            return false;
        };
        if !self.pilots[idx].pending_refuse {
            return false;
        }
        let chance = self.pilots[idx].refuse_chance();
        let pname = self.pilots[idx].name.clone();
        let mname = self
            .mech(unit_id)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let roll = Self::order_roll(self.turn, unit_id);
        self.pilots[idx].pending_refuse = false;
        if chance > 0.0 && roll < chance {
            if let Some(m) = self.mech_mut(unit_id) {
                m.acted = true;
                m.move_left = 0;
            }
            self.log
                .push(format!("{pname} refuses the {order} — {mname} locks up."));
            true
        } else {
            self.log
                .push(format!("{pname} steadies and follows the {order}."));
            false
        }
    }

    /// Spike player-pilot stress when a limb is wrecked or the mech is near death.
    fn maybe_stress_spike(
        &mut self,
        target_id: u32,
        limb: LimbKind,
        before_limb: f32,
        before_near: bool,
    ) {
        let Some(mech) = self.mech(target_id) else {
            return;
        };
        if !matches!(mech.team, Team::Player) {
            return;
        }
        let Some(pid) = mech.pilot_id else {
            return;
        };
        let after_limb = mech.limb_ratio(limb);
        let now_near = mech.is_near_death();
        let heavy = before_limb > HEAVY_LIMB_RATIO && after_limb <= HEAVY_LIMB_RATIO;
        let near = !before_near && now_near;
        if !heavy && !near {
            return;
        }
        let mech_name = mech.name.clone();
        let reason = if near && heavy {
            format!("{limb} wrecked, near-death")
        } else if near {
            "near-death".to_string()
        } else {
            format!("{limb} wrecked")
        };
        let Some(pilot) = self.pilot_mut(pid) else {
            return;
        };
        let pname = pilot.name.clone();
        pilot.spike_from_hit();
        let stress_pct = (pilot.stress * 100.0).round();
        self.log.push(format!(
            "{pname} reels in {mech_name} — {reason}; stress {stress_pct:.0}% (may refuse next order)"
        ));
    }

    fn refresh_team(&mut self, team: Team) {
        for m in self.mechs.iter_mut().filter(|m| m.team == team) {
            m.refresh_turn();
        }
    }

    fn step_toward(&self, from: IVec2, to: IVec2, eid: u32) -> Option<IVec2> {
        let dx = (to.x - from.x).signum();
        let dy = (to.y - from.y).signum();
        let candidates = if dx != 0 && dy != 0 {
            [
                IVec2::new(from.x + dx, from.y),
                IVec2::new(from.x, from.y + dy),
            ]
        } else if dx != 0 {
            [IVec2::new(from.x + dx, from.y), IVec2::new(from.x, from.y)]
        } else {
            [IVec2::new(from.x, from.y + dy), IVec2::new(from.x, from.y)]
        };
        for next in candidates {
            if next != from && self.grid.in_bounds(next) && !self.occupied(next, Some(eid)) {
                return Some(next);
            }
        }
        None
    }

    fn step_away(&self, from: IVec2, threat: IVec2, eid: u32) -> Option<IVec2> {
        let mut best: Option<(IVec2, i32)> = None;
        for dir in [Facing::North, Facing::East, Facing::South, Facing::West] {
            let next = from + dir.delta();
            if !self.grid.in_bounds(next) || self.occupied(next, Some(eid)) {
                continue;
            }
            let dist = Grid::manhattan(next, threat);
            if best.map(|(_, d)| dist > d).unwrap_or(true) {
                best = Some((next, dist));
            }
        }
        best.map(|(p, _)| p)
    }

    fn plan_enemy_actions(
        &self,
        eid: u32,
        mut epos: IVec2,
        mut mp: i32,
        range: i32,
        kind: Option<AlienKind>,
        tid: u32,
        tpos: IVec2,
    ) -> Vec<Action> {
        let mut queue = Vec::new();
        let limb = self
            .mech(eid)
            .map(|m| m.preferred_attack_limb())
            .unwrap_or(LimbKind::Torso);

        match kind {
            Some(AlienKind::Splinter) => {
                // Kite when pressed, otherwise hold the edge of range.
                while mp > 0 {
                    let dist = Grid::manhattan(epos, tpos);
                    if dist <= 2 {
                        if let Some(next) = self.step_away(epos, tpos, eid) {
                            queue.push(Action::Move {
                                unit_id: eid,
                                to: next,
                            });
                            epos = next;
                            mp -= 1;
                            continue;
                        }
                    }
                    if dist > range {
                        if let Some(next) = self.step_toward(epos, tpos, eid) {
                            queue.push(Action::Move {
                                unit_id: eid,
                                to: next,
                            });
                            epos = next;
                            mp -= 1;
                            continue;
                        }
                    }
                    break;
                }
                let dist = Grid::manhattan(epos, tpos);
                if dist <= range {
                    queue.push(Action::Attack {
                        attacker_id: eid,
                        target_id: tid,
                        limb,
                    });
                } else {
                    queue.push(Action::Wait { unit_id: eid });
                }
            }
            Some(AlienKind::Mass) | None => {
                // Close and smash (generic enemies share Mass pressure).
                while mp > 0 && Grid::manhattan(epos, tpos) > range {
                    if let Some(next) = self.step_toward(epos, tpos, eid) {
                        queue.push(Action::Move {
                            unit_id: eid,
                            to: next,
                        });
                        epos = next;
                        mp -= 1;
                    } else {
                        break;
                    }
                }
                let dist = Grid::manhattan(epos, tpos);
                if dist <= range {
                    queue.push(Action::Attack {
                        attacker_id: eid,
                        target_id: tid,
                        limb,
                    });
                } else {
                    queue.push(Action::Wait { unit_id: eid });
                }
            }
        }
        queue
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
            let Some(enemy) = self.mech(eid) else {
                continue;
            };
            let epos = enemy.position;
            let mp = enemy.move_left;
            let range = enemy.attack_range();
            let kind = enemy.alien;
            let name = enemy.name.clone();
            let (tid, tpos) = player_positions
                .iter()
                .min_by_key(|(_, p)| Grid::manhattan(epos, *p))
                .copied()
                .unwrap();

            match kind {
                Some(AlienKind::Splinter) => {
                    self.log.push(format!("{name} kites and harasses."));
                }
                Some(AlienKind::Mass) => {
                    self.log.push(format!("{name} advances under pressure."));
                }
                None => {}
            }

            let planned = self.plan_enemy_actions(eid, epos, mp, range, kind, tid, tpos);
            self.enemy_queue.extend(planned);
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
                if matches!(
                    self.apply_action(Action::Attack {
                        attacker_id: pid,
                        target_id: eid,
                        limb: LimbKind::Torso,
                    }),
                    Ok(ApplyResult::Done)
                ) {
                    player_hits += 1;
                }
            }
            self.end_player_turn();
        }
        self.log.push(format!("SMOKE: player hits={player_hits}"));
        if self.is_won() {
            self.log.push("SMOKE: Mission won.".into());
        } else if self.is_lost() {
            self.log.push("SMOKE: Mission lost.".into());
        } else {
            self.log
                .push(format!("SMOKE: Stopped after {} turns.", self.turn));
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
            m.log
                .iter()
                .any(|l| l.contains("SMOKE: player hits=") && !l.ends_with("hits=0")),
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

    #[test]
    fn skirmish_spawns_alien_archetypes() {
        let m = Mission::new_skirmish();
        let splinter = m.mech(10).unwrap();
        let mass = m.mech(11).unwrap();
        assert_eq!(splinter.alien, Some(AlienKind::Splinter));
        assert_eq!(mass.alien, Some(AlienKind::Mass));
        assert_eq!(splinter.attack_range(), 5);
        assert_eq!(mass.attack_range(), 2);
    }

    #[test]
    fn splinter_kites_when_adjacent() {
        let mut m = Mission::new_skirmish();
        // Place Splinter next to Coil.
        m.mech_mut(10).unwrap().position = IVec2::new(3, 3);
        m.mech_mut(0).unwrap().position = IVec2::new(2, 3);
        m.mech_mut(10).unwrap().refresh_turn();
        m.begin_enemy_turn();
        assert!(
            m.log.iter().any(|l| l.contains("kites")),
            "log: {:?}",
            m.log
        );
        let first_move = m.enemy_queue.iter().find_map(|a| match a {
            Action::Move { unit_id: 10, to } => Some(*to),
            _ => None,
        });
        let to = first_move.expect("Splinter should queue a retreat step");
        assert!(Grid::manhattan(to, IVec2::new(2, 3)) > 1);
    }

    #[test]
    fn mass_closes_when_out_of_range() {
        let mut m = Mission::new_skirmish();
        // Mass far from nearest player, short range 2.
        m.mech_mut(11).unwrap().position = IVec2::new(7, 7);
        m.mech_mut(0).unwrap().position = IVec2::new(1, 1);
        m.mech_mut(1).unwrap().position = IVec2::new(1, 2);
        m.mech_mut(11).unwrap().refresh_turn();
        m.begin_enemy_turn();
        assert!(
            m.log.iter().any(|l| l.contains("advances")),
            "log: {:?}",
            m.log
        );
        assert!(
            m.enemy_queue
                .iter()
                .any(|a| matches!(a, Action::Move { unit_id: 11, .. })),
            "queue: {:?}",
            m.enemy_queue
        );
    }

    #[test]
    fn mass_starts_with_at_field() {
        let m = Mission::new_skirmish();
        assert_eq!(m.mech(11).unwrap().at_field, 2);
        assert_eq!(m.mech(10).unwrap().at_field, 0);
        assert_eq!(m.mech(0).unwrap().at_field, 0);
    }

    #[test]
    fn mass_at_field_absorbs_then_takes_damage() {
        let mut m = Mission::new_skirmish();
        // Pull Mass into Coil range (opening spawn is 5 tiles away).
        m.mech_mut(11).unwrap().position = IVec2::new(4, 3);
        let torso_before = m
            .mech(11)
            .unwrap()
            .limbs
            .iter()
            .find(|l| l.kind == LimbKind::Torso)
            .unwrap()
            .hp;
        assert!(m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 11,
                limb: LimbKind::Torso,
            })
            .is_ok());
        assert_eq!(m.mech(11).unwrap().at_field, 1);
        let torso_mid = m
            .mech(11)
            .unwrap()
            .limbs
            .iter()
            .find(|l| l.kind == LimbKind::Torso)
            .unwrap()
            .hp;
        assert_eq!(torso_mid, torso_before, "first hit should be absorbed");
        assert!(
            m.pending_fx
                .iter()
                .any(|fx| matches!(fx, CombatFx::AtFieldAbsorb { target_id: 11 })),
            "fx: {:?}",
            m.pending_fx
        );
        assert!(m
            .apply_action(Action::Attack {
                attacker_id: 1,
                target_id: 11,
                limb: LimbKind::Torso,
            })
            .is_ok());
        assert_eq!(m.mech(11).unwrap().at_field, 0);
        assert!(
            m.pending_fx
                .iter()
                .any(|fx| matches!(fx, CombatFx::AtFieldBreak { target_id: 11 })),
            "fx: {:?}",
            m.pending_fx
        );
        let torso_after_break = m
            .mech(11)
            .unwrap()
            .limbs
            .iter()
            .find(|l| l.kind == LimbKind::Torso)
            .unwrap()
            .hp;
        assert_eq!(torso_after_break, torso_before, "shatter absorb still blocks");

        m.end_player_turn();
        m.take_fx();
        assert!(m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 11,
                limb: LimbKind::Torso,
            })
            .is_ok());
        let torso_hurt = m
            .mech(11)
            .unwrap()
            .limbs
            .iter()
            .find(|l| l.kind == LimbKind::Torso)
            .unwrap()
            .hp;
        assert!(torso_hurt < torso_before, "third hit should wound");
    }

    #[test]
    fn chip_damage_does_not_spike_stress() {
        let mut m = Mission::new_skirmish();
        m.phase = TurnPhase::Enemy;
        let stress0 = m.pilot(0).unwrap().stress;
        // Splinter-sized chip on a fresh arm (~29) leaves it above 25%.
        let before = m.mech(0).unwrap().limb_ratio(LimbKind::LeftArm);
        m.mech_mut(0).unwrap().apply_damage(LimbKind::LeftArm, 20.0);
        m.maybe_stress_spike(0, LimbKind::LeftArm, before, false);
        assert_eq!(m.pilot(0).unwrap().stress, stress0);
        assert!(!m.pilot(0).unwrap().pending_refuse);
        assert!(m.log.iter().all(|l| !l.contains("reels")));
    }

    #[test]
    fn wrecked_limb_spikes_stress_and_logs() {
        let mut m = Mission::new_skirmish();
        let stress0 = m.pilot(0).unwrap().stress;
        let before = m.mech(0).unwrap().limb_ratio(LimbKind::LeftArm);
        m.mech_mut(0).unwrap().apply_damage(LimbKind::LeftArm, 50.0);
        assert!(m.mech(0).unwrap().limb_ratio(LimbKind::LeftArm) <= 0.25);
        m.maybe_stress_spike(0, LimbKind::LeftArm, before, false);
        let p = m.pilot(0).unwrap();
        assert!(p.stress > stress0);
        assert!(p.pending_refuse);
        assert!(
            m.log
                .iter()
                .any(|l| l.contains("reels") && l.contains("L.Arm")),
            "log: {:?}",
            m.log
        );
    }

    #[test]
    fn near_death_spikes_stress() {
        let mut m = Mission::new_skirmish();
        let before = m.mech(0).unwrap().limb_ratio(LimbKind::Torso);
        m.mech_mut(0).unwrap().apply_damage(LimbKind::Torso, 75.0);
        assert!(m.mech(0).unwrap().is_near_death());
        m.maybe_stress_spike(0, LimbKind::Torso, before, false);
        assert!(m.pilot(0).unwrap().pending_refuse);
        assert!(
            m.log.iter().any(|l| l.contains("near-death")),
            "log: {:?}",
            m.log
        );
    }

    #[test]
    fn pending_refuse_skips_next_order_once() {
        let mut m = Mission::new_skirmish();
        // Turn 1 / unit 0 roll is 0.17; maxed panic clamps to 0.55 → refuse.
        {
            let p = m.pilot_mut(0).unwrap();
            p.stress = 1.0;
            p.loyalty = 0.0;
            p.sync = 0.2;
            p.pending_refuse = true;
        }
        let torso = m
            .mech(10)
            .unwrap()
            .limbs
            .iter()
            .find(|l| l.kind == LimbKind::Torso)
            .unwrap()
            .hp;
        let result = m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 10,
                limb: LimbKind::Torso,
            })
            .unwrap();
        assert_eq!(result, ApplyResult::Skipped);
        assert!(m.mech(0).unwrap().acted);
        assert!(!m.pilot(0).unwrap().pending_refuse);
        assert_eq!(
            m.mech(10)
                .unwrap()
                .limbs
                .iter()
                .find(|l| l.kind == LimbKind::Torso)
                .unwrap()
                .hp,
            torso,
            "refused attack must not deal damage"
        );
        assert!(
            m.log.iter().any(|l| l.contains("refuses the attack")),
            "log: {:?}",
            m.log
        );

        // Flag consumed — next player turn the same unit fires.
        m.mech_mut(0).unwrap().refresh_turn();
        let result = m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 10,
                limb: LimbKind::Torso,
            })
            .unwrap();
        assert_eq!(result, ApplyResult::Done);
        assert!(
            m.mech(10)
                .unwrap()
                .limbs
                .iter()
                .find(|l| l.kind == LimbKind::Torso)
                .unwrap()
                .hp
                < torso
        );
    }

    #[test]
    fn pending_refuse_can_obey_and_still_consume() {
        let mut m = Mission::new_skirmish();
        // Turn 5 / unit 0 roll is 0.85, above any refuse chance → obey.
        m.turn = 5;
        {
            let p = m.pilot_mut(0).unwrap();
            p.spike_from_hit();
        }
        let result = m
            .apply_action(Action::Attack {
                attacker_id: 0,
                target_id: 10,
                limb: LimbKind::Torso,
            })
            .unwrap();
        assert_eq!(result, ApplyResult::Done);
        assert!(!m.pilot(0).unwrap().pending_refuse);
        assert!(
            m.log.iter().any(|l| l.contains("steadies and follows")),
            "log: {:?}",
            m.log
        );
    }

    fn autoplay_loop(m: &mut Mission, max_turns: u32) {
        for _ in 0..max_turns {
            if m.is_won() || m.is_lost() {
                return;
            }
            let player_ids: Vec<u32> = m.living_mechs(Team::Player).map(|x| x.id).collect();
            for pid in player_ids {
                if !m.mech(pid).map(|x| x.can_act()).unwrap_or(false) {
                    continue;
                }
                let from = m.mech(pid).unwrap().position;
                let Some(eid) = m
                    .living_mechs(Team::Enemy)
                    .min_by_key(|e| Grid::manhattan(from, e.position))
                    .map(|e| e.id)
                else {
                    break;
                };
                let in_range = {
                    let a = m.mech(pid).unwrap();
                    let e = m.mech(eid).unwrap();
                    Grid::manhattan(a.position, e.position) <= a.attack_range()
                };
                if in_range {
                    let _ = m.apply_action(Action::Attack {
                        attacker_id: pid,
                        target_id: eid,
                        limb: LimbKind::Torso,
                    });
                } else {
                    let _ = m.apply_action(Action::Wait { unit_id: pid });
                }
            }
            if m.is_won() || m.is_lost() {
                return;
            }
            m.end_player_turn();
        }
    }

    #[test]
    fn autoplay_style_still_wins_with_at_field() {
        let mut m = Mission::new_skirmish();
        autoplay_loop(&mut m, 20);
        assert!(m.is_won(), "expected victory with AT Field; log={:?}", m.log);
    }

    #[test]
    fn autoplay_style_still_wins_with_stress_skips() {
        let mut m = Mission::new_skirmish();
        autoplay_loop(&mut m, 20);
        assert!(
            m.is_won(),
            "expected victory even if a pilot skips; log={:?}",
            m.log
        );
    }
}
