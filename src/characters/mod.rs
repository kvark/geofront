//! Loyalty, opinions, sync ratios, dialog state machines.
//!
//! Short Evangelion-flavored combat-log / HUD lines for pilot psychology beats
//! (stress spike arms refuse, refuse fires, or the pilot steadies), plus
//! procedural portrait placeholders (tint + initials) for the battle HUD.

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
    /// Sparse high-sync Angel core / pattern-sight read.
    CoreSight,
}

/// Brief HUD banner for the last pilot drama beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotHudFlash {
    pub kind: PilotDramaKind,
    pub pilot_id: u32,
    pub pilot_name: String,
    /// Combat-log narration (tests / scrollback).
    pub line: String,
    /// One-line spoken beat shown with the portrait.
    pub dialog: String,
}

impl PilotHudFlash {
    pub fn new(kind: PilotDramaKind, pilot: &Pilot, line: impl Into<String>) -> Self {
        Self {
            kind,
            pilot_id: pilot.id,
            pilot_name: pilot.name.clone(),
            line: line.into(),
            dialog: dialog_beat(kind, pilot),
        }
    }

    /// Compact caption for the HUD strip.
    pub fn caption(&self) -> &'static str {
        match self.kind {
            PilotDramaKind::StressSpike => "⚠ PILOT STRESS SPIKE",
            PilotDramaKind::Refuse => "⛔ ORDER REFUSED",
            PilotDramaKind::Steady => "✓ PILOT STEADIED",
            PilotDramaKind::CoreSight => "✦ PATTERN SIGHT",
        }
    }
}

/// Procedural portrait tint from pilot id (no art packs).
pub fn portrait_rgb(pilot_id: u32) -> [u8; 3] {
    match pilot_id % 6 {
        0 => [72, 168, 210],  // Nori — cool cyan
        1 => [210, 110, 170], // Vesper — magenta
        2 => [120, 190, 110],
        3 => [220, 170, 70],
        4 => [150, 130, 220],
        _ => [200, 120, 90],
    }
}

/// One or two initials for the silhouette plaque.
pub fn portrait_initials(name: &str) -> String {
    let mut parts = name.split_whitespace().filter(|s| !s.is_empty());
    let first = parts.next().and_then(|s| s.chars().next());
    let second = parts.next().and_then(|s| s.chars().next());
    match (first, second) {
        (Some(a), Some(b)) => format!(
            "{}{}",
            a.to_ascii_uppercase(),
            b.to_ascii_uppercase()
        ),
        (Some(a), None) => a.to_ascii_uppercase().to_string(),
        _ => "?".into(),
    }
}

/// Short first-person cockpit line for the drama flash (not a full dialog system).
pub fn dialog_beat(kind: PilotDramaKind, pilot: &Pilot) -> String {
    match kind {
        PilotDramaKind::StressSpike => stress_dialog(pilot),
        PilotDramaKind::Refuse => refuse_dialog(pilot),
        PilotDramaKind::Steady => steady_dialog(pilot),
        PilotDramaKind::CoreSight => core_dialog(pilot),
    }
}

fn stress_dialog(pilot: &Pilot) -> String {
    if pilot.stress >= 0.75 {
        "The plug's screaming— make it stop!".into()
    } else if pilot.sync <= SYNC_LOW {
        "Sync's slipping… I can feel it fray.".into()
    } else if pilot.loyalty < 0.45 {
        "You felt that too, right? Don't lie.".into()
    } else {
        "It hit the field— I'm still here.".into()
    }
}

fn refuse_dialog(pilot: &Pilot) -> String {
    if pilot.sync <= SYNC_LOW {
        "I can't move— the sync's gone!".into()
    } else if pilot.stress >= 0.8 {
        "I said I can't— don't order me!".into()
    } else if pilot.loyalty < 0.4 {
        "Not this. Find another pilot.".into()
    } else {
        "…I won't. Not yet.".into()
    }
}

fn steady_dialog(pilot: &Pilot) -> String {
    if pilot.loyalty >= 0.7 || pilot.sync >= SYNC_HIGH {
        "…I'm fine. Moving.".into()
    } else {
        "Okay. Okay— following.".into()
    }
}

fn core_dialog(pilot: &Pilot) -> String {
    if pilot.sync >= 0.95 {
        "There— the pattern. I see the core.".into()
    } else {
        "Something's flashing… there.".into()
    }
}

/// Short Eva-flavored combat-log line. `detail` is the trauma reason or order verb.
pub fn drama_line(kind: PilotDramaKind, pilot: &Pilot, mech: &str, detail: &str) -> String {
    let stress_pct = (pilot.stress * 100.0).round();
    match kind {
        PilotDramaKind::StressSpike => stress_spike_line(pilot, mech, detail, stress_pct),
        PilotDramaKind::Refuse => refuse_line(pilot, mech, detail),
        PilotDramaKind::Steady => steady_line(pilot, detail),
        PilotDramaKind::CoreSight => core_sight_line(pilot, detail),
    }
}

fn core_sight_line(pilot: &Pilot, detail: &str) -> String {
    let sync_pct = (pilot.sync * 100.0).round();
    format!(
        "{}: \"{}\" — sees the pattern (sync {:.0}%) — {}.",
        pilot.name,
        dialog_beat(PilotDramaKind::CoreSight, pilot),
        sync_pct,
        detail
    )
}

fn stress_spike_line(pilot: &Pilot, mech: &str, reason: &str, stress_pct: f32) -> String {
    // Flavor by cockpit pressure; always keep the refuse hint for HUD/tests.
    let quote = dialog_beat(PilotDramaKind::StressSpike, pilot);
    if pilot.stress >= 0.75 {
        format!(
            "{}: \"{}\" — {}; stress {:.0}% (may refuse next order)",
            pilot.name, quote, reason, stress_pct
        )
    } else if pilot.sync <= SYNC_LOW {
        format!(
            "{}: \"{}\" — sync fraying in {}, {}; stress {:.0}% (may refuse next order)",
            pilot.name, quote, mech, reason, stress_pct
        )
    } else if pilot.loyalty < 0.45 {
        format!(
            "{}: \"{}\" — snaps at Command from {}, {}; stress {:.0}% (may refuse next order)",
            pilot.name, quote, mech, reason, stress_pct
        )
    } else {
        format!(
            "{}: \"{}\" — reels in {}, {}; stress {:.0}% (may refuse next order)",
            pilot.name, quote, mech, reason, stress_pct
        )
    }
}

fn refuse_line(pilot: &Pilot, mech: &str, order: &str) -> String {
    let quote = dialog_beat(PilotDramaKind::Refuse, pilot);
    if pilot.sync <= SYNC_LOW {
        format!(
            "{}: \"{}\" — sync collapse — refuses the {}! {} locks up.",
            pilot.name, quote, order, mech
        )
    } else if pilot.stress >= 0.8 {
        format!(
            "{}: \"{}\" — refuses the {}! {} freezes.",
            pilot.name, quote, order, mech
        )
    } else if pilot.loyalty < 0.4 {
        format!(
            "{}: \"{}\" — overrides Command — refuses the {}; {} locks up.",
            pilot.name, quote, order, mech
        )
    } else {
        format!(
            "{}: \"{}\" — refuses the {}; {} locks up.",
            pilot.name, quote, order, mech
        )
    }
}

fn steady_line(pilot: &Pilot, order: &str) -> String {
    let quote = dialog_beat(PilotDramaKind::Steady, pilot);
    if pilot.loyalty >= 0.7 || pilot.sync >= SYNC_HIGH {
        format!(
            "{}: \"{}\" — bites down and steadies — follows the {}.",
            pilot.name, quote, order
        )
    } else {
        format!(
            "{}: \"{}\" — steadies and follows the {}.",
            pilot.name, quote, order
        )
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
        assert!(line.contains("reels") || line.contains("entry plug") || line.contains("snaps") || line.contains("\""));
        assert!(line.contains("may refuse next order"));
        assert!(line.contains("L.Arm"));
        assert!(line.contains(&dialog_beat(PilotDramaKind::StressSpike, &p)));
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
        assert!(line.contains(&dialog_beat(PilotDramaKind::Refuse, &p)));
    }

    #[test]
    fn steady_line_mentions_steadies() {
        let p = Pilot::new(0, "Nori");
        let line = drama_line(PilotDramaKind::Steady, &p, "Coil", "attack");
        assert!(line.contains("steadies"));
        assert!(line.contains("attack"));
        assert!(line.contains("I'm fine") || line.contains("Okay"));
    }

    #[test]
    fn hud_flash_captions() {
        let p = Pilot::new(1, "Vesper");
        let spike = PilotHudFlash::new(PilotDramaKind::StressSpike, &p, "x");
        assert!(spike.caption().contains("STRESS"));
        assert_eq!(spike.pilot_id, 1);
        assert!(!spike.dialog.is_empty());
        let refuse = PilotHudFlash::new(PilotDramaKind::Refuse, &p, "x");
        assert!(refuse.caption().contains("REFUSED"));
        let steady = PilotHudFlash::new(PilotDramaKind::Steady, &p, "x");
        assert!(steady.caption().contains("STEADIED"));
        let core = PilotHudFlash::new(PilotDramaKind::CoreSight, &p, "x");
        assert!(core.caption().contains("PATTERN"));
    }

    #[test]
    fn high_stress_spike_uses_entry_plug_flavor() {
        let mut p = Pilot::new(0, "Nori");
        p.stress = 0.9;
        let line = drama_line(PilotDramaKind::StressSpike, &p, "Coil", "near-death");
        assert!(line.contains("plug") || line.contains("screaming"));
        assert!(line.contains("may refuse next order"));
        assert_eq!(
            dialog_beat(PilotDramaKind::StressSpike, &p),
            "The plug's screaming— make it stop!"
        );
    }

    #[test]
    fn portrait_initials_and_tints_differ_per_pilot() {
        assert_eq!(portrait_initials("Nori"), "N");
        assert_eq!(portrait_initials("Vesper Lane"), "VL");
        assert_ne!(portrait_rgb(0), portrait_rgb(1));
    }

    #[test]
    fn core_sight_dialog_is_sparse_pattern_line() {
        let mut p = Pilot::new(0, "Nori");
        p.sync = 0.95;
        let d = dialog_beat(PilotDramaKind::CoreSight, &p);
        assert!(d.contains("pattern") || d.contains("core") || d.contains("flashing"));
        let line = drama_line(PilotDramaKind::CoreSight, &p, "Coil", "Splinter, Mass cores flash");
        assert!(line.contains("sees the pattern"));
        assert!(line.contains(&d));
    }
}
