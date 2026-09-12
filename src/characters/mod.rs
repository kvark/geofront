//! Loyalty, opinions, sync ratios, dialog state machines.
//!
//! Short Evangelion-flavored combat-log / HUD lines for pilot psychology beats
//! (stress spike arms refuse, refuse fires, or the pilot steadies).

use crate::units::{Pilot, SYNC_HIGH, SYNC_LOW};

/// Kind of pilot psychology beat shown in the combat log / HUD flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PilotDramaKind {
    /// Trauma spike armed a one-shot refuse.
    StressSpike,
    /// Pilot skipped the ordered action.
    Refuse,
    /// Armed refuse was consumed but the pilot obeyed.
    Steady,
}

/// Brief HUD banner for the last pilot drama beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotHudFlash {
    pub kind: PilotDramaKind,
    pub pilot_name: String,
    pub line: String,
}

impl PilotHudFlash {
    pub fn new(kind: PilotDramaKind, pilot: &Pilot, line: impl Into<String>) -> Self {
        Self {
            kind,
            pilot_name: pilot.name.clone(),
            line: line.into(),
        }
    }

    /// Compact caption for the HUD strip.
    pub fn caption(&self) -> &'static str {
        match self.kind {
            PilotDramaKind::StressSpike => "⚠ PILOT STRESS SPIKE",
            PilotDramaKind::Refuse => "⛔ ORDER REFUSED",
            PilotDramaKind::Steady => "✓ PILOT STEADIED",
        }
    }
}

/// Short Eva-flavored combat-log line. `detail` is the trauma reason or order verb.
pub fn drama_line(kind: PilotDramaKind, pilot: &Pilot, mech: &str, detail: &str) -> String {
    let stress_pct = (pilot.stress * 100.0).round();
    match kind {
        PilotDramaKind::StressSpike => stress_spike_line(pilot, mech, detail, stress_pct),
        PilotDramaKind::Refuse => refuse_line(pilot, mech, detail),
        PilotDramaKind::Steady => steady_line(pilot, detail),
    }
}

fn stress_spike_line(pilot: &Pilot, mech: &str, reason: &str, stress_pct: f32) -> String {
    // Flavor by cockpit pressure; always keep the refuse hint for HUD/tests.
    if pilot.stress >= 0.75 {
        format!(
            "{}: entry plug screaming — {}; stress {:.0}% (may refuse next order)",
            pilot.name, reason, stress_pct
        )
    } else if pilot.sync <= SYNC_LOW {
        format!(
            "{} reels in {} — sync fraying, {}; stress {:.0}% (may refuse next order)",
            pilot.name, mech, reason, stress_pct
        )
    } else if pilot.loyalty < 0.45 {
        format!(
            "{} snaps at Command from {} — {}; stress {:.0}% (may refuse next order)",
            pilot.name, mech, reason, stress_pct
        )
    } else {
        format!(
            "{} reels in {} — {}; stress {:.0}% (may refuse next order)",
            pilot.name, mech, reason, stress_pct
        )
    }
}

fn refuse_line(pilot: &Pilot, mech: &str, order: &str) -> String {
    if pilot.sync <= SYNC_LOW {
        format!(
            "{}: sync collapse — refuses the {}! {} locks up.",
            pilot.name, order, mech
        )
    } else if pilot.stress >= 0.8 {
        format!(
            "{}: I can't— refuses the {}! {} freezes.",
            pilot.name, order, mech
        )
    } else if pilot.loyalty < 0.4 {
        format!(
            "{} overrides Command — refuses the {}; {} locks up.",
            pilot.name, order, mech
        )
    } else {
        format!("{} refuses the {} — {} locks up.", pilot.name, order, mech)
    }
}

fn steady_line(pilot: &Pilot, order: &str) -> String {
    if pilot.loyalty >= 0.7 || pilot.sync >= SYNC_HIGH {
        format!(
            "{} bites down and steadies — follows the {}.",
            pilot.name, order
        )
    } else {
        format!("{} steadies and follows the {}.", pilot.name, order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Pilot;

    #[test]
    fn stress_spike_line_mentions_refuse_hint() {
        let p = Pilot::new(0, "Nori");
        let line = drama_line(PilotDramaKind::StressSpike, &p, "Coil", "L.Arm wrecked");
        assert!(line.contains("Nori"));
        assert!(line.contains("reels") || line.contains("entry plug") || line.contains("snaps"));
        assert!(line.contains("may refuse next order"));
        assert!(line.contains("L.Arm"));
    }

    #[test]
    fn refuse_line_locks_up_and_names_order() {
        let mut p = Pilot::new(0, "Nori");
        p.stress = 1.0;
        p.loyalty = 0.0;
        p.sync = 0.2;
        let line = drama_line(PilotDramaKind::Refuse, &p, "Coil", "attack");
        assert!(line.contains("refuses the attack"));
        assert!(line.contains("locks up") || line.contains("freezes"));
        assert!(line.contains("sync collapse") || line.contains("I can't"));
    }

    #[test]
    fn steady_line_mentions_steadies() {
        let p = Pilot::new(0, "Nori");
        let line = drama_line(PilotDramaKind::Steady, &p, "Coil", "attack");
        assert!(line.contains("steadies"));
        assert!(line.contains("attack"));
    }

    #[test]
    fn hud_flash_captions() {
        let p = Pilot::new(1, "Vesper");
        let spike = PilotHudFlash::new(PilotDramaKind::StressSpike, &p, "x");
        assert!(spike.caption().contains("STRESS"));
        let refuse = PilotHudFlash::new(PilotDramaKind::Refuse, &p, "x");
        assert!(refuse.caption().contains("REFUSED"));
        let steady = PilotHudFlash::new(PilotDramaKind::Steady, &p, "x");
        assert!(steady.caption().contains("STEADIED"));
    }

    #[test]
    fn high_stress_spike_uses_entry_plug_flavor() {
        let mut p = Pilot::new(0, "Nori");
        p.stress = 0.9;
        let line = drama_line(PilotDramaKind::StressSpike, &p, "Coil", "near-death");
        assert!(line.contains("entry plug"));
        assert!(line.contains("may refuse next order"));
    }
}
