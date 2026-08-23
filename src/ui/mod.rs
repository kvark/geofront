//! egui screens: battle HUD, city overview controls, mode switching.

use crate::combat::{Mission, TurnPhase};
use crate::render::ViewMode;
use crate::units::{LimbKind, Team};

/// Draws the side-panel HUD and returns any action the player requested.
pub fn side_hud(
    ui: &mut egui::Ui,
    mission: &Mission,
    view_mode: ViewMode,
    selected_player: &mut u32,
    selected_enemy: &mut u32,
    selected_limb: &mut LimbKind,
) -> Option<HudAction> {
    let mut requested = None;

    ui.heading("Geofront");
    ui.label(match view_mode {
        ViewMode::Battle => "Eva-style close-up combat",
        ViewMode::CitySurface => "Surface city overview",
        ViewMode::CityUnderground => "Underground facility overview",
    });
    ui.separator();

    // Mode switcher
    ui.horizontal(|ui| {
        ui.label("View:");
        for mode in [
            ViewMode::Battle,
            ViewMode::CitySurface,
            ViewMode::CityUnderground,
        ] {
            if ui
                .selectable_label(view_mode == mode, mode.label())
                .clicked()
            {
                requested = Some(HudAction::SetView(mode));
            }
        }
    });
    ui.separator();

    ui.label("WASD move  ·  drag to look  ·  Q/E height  ·  Shift sprint  ·  wheel dolly");
    ui.separator();

    match view_mode {
        ViewMode::Battle => {
            requested = requested.or(battle_panel(
                ui,
                mission,
                selected_player,
                selected_enemy,
                selected_limb,
            ));
        }
        ViewMode::CitySurface | ViewMode::CityUnderground => {
            city_panel(ui, view_mode);
        }
    }

    requested
}

fn battle_panel(
    ui: &mut egui::Ui,
    mission: &Mission,
    selected_player: &mut u32,
    selected_enemy: &mut u32,
    selected_limb: &mut LimbKind,
) -> Option<HudAction> {
    let mut requested = None;

    ui.horizontal(|ui| {
        ui.label(format!("Turn {}", mission.turn));
        ui.separator();
        ui.label(format!("Phase: {:?}", mission.phase));
        ui.separator();
        let city_color = if mission.city_hp > 60.0 {
            egui::Color32::GREEN
        } else if mission.city_hp > 30.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        };
        ui.colored_label(
            city_color,
            format!("City protection: {:.0}%", mission.city_hp),
        );

        if mission.is_won() {
            ui.colored_label(egui::Color32::LIGHT_GREEN, "  VICTORY");
        } else if mission.is_lost() {
            ui.colored_label(egui::Color32::LIGHT_RED, "  DEFEAT");
        }
    });

    ui.separator();

    ui.columns(2, |cols| {
        cols[0].heading("Player");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Player) {
            let selected = *selected_player == m.id;
            let (hp, max) = m.total_hp();
            let label = if m.destroyed {
                format!("{} (destroyed)", m.name)
            } else {
                format!(
                    "{} @({},{})  {:.0}/{:.0}",
                    m.name, m.position.x, m.position.y, hp, max
                )
            };
            if cols[0].selectable_label(selected, label).clicked() && !m.destroyed {
                *selected_player = m.id;
            }
            if selected && !m.destroyed {
                cols[0].indent("limbs", |ui| {
                    for limb in &m.limbs {
                        let ratio = limb.damage_ratio();
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", limb.kind));
                            let color = if ratio > 0.6 {
                                egui::Color32::GREEN
                            } else if ratio > 0.25 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(
                                color,
                                format!("{:.0}/{:.0}", limb.hp, limb.max_hp),
                            );
                        });
                    }
                });
            }
        }

        cols[1].heading("Enemy");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Enemy) {
            let selected = *selected_enemy == m.id;
            let (hp, max) = m.total_hp();
            let label = if m.destroyed {
                format!("{} (destroyed)", m.name)
            } else {
                format!(
                    "{} @({},{})  {:.0}/{:.0}",
                    m.name, m.position.x, m.position.y, hp, max
                )
            };
            if cols[1].selectable_label(selected, label).clicked() && !m.destroyed {
                *selected_enemy = m.id;
            }
        }
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Target limb:");
        for limb in [
            LimbKind::Torso,
            LimbKind::LeftArm,
            LimbKind::RightArm,
            LimbKind::LeftLeg,
            LimbKind::RightLeg,
        ] {
            if ui
                .selectable_label(*selected_limb == limb, format!("{limb}"))
                .clicked()
            {
                *selected_limb = limb;
            }
        }
    });

    ui.horizontal(|ui| {
        let can_act = matches!(mission.phase, TurnPhase::Player)
            && !mission.is_won()
            && !mission.is_lost();

        if ui
            .add_enabled(can_act, egui::Button::new("Attack selected"))
            .clicked()
        {
            requested = Some(HudAction::Attack {
                attacker: *selected_player,
                target: *selected_enemy,
                limb: *selected_limb,
            });
        }
        if ui
            .add_enabled(can_act, egui::Button::new("End Turn"))
            .clicked()
        {
            requested = Some(HudAction::EndTurn);
        }
        if ui.button("Reset Mission").clicked() {
            requested = Some(HudAction::Reset);
        }
    });

    ui.separator();
    ui.heading("Combat log");
    egui::ScrollArea::vertical()
        .max_height(200.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for line in mission.log.iter().rev().take(40).rev() {
                ui.label(line);
            }
        });

    requested
}

fn city_panel(ui: &mut egui::Ui, mode: ViewMode) {
    ui.label(match mode {
        ViewMode::CitySurface => {
            "Kenney city block — commercial towers and industrial fringe.\n\
             Switch to Battle for Eva-style close combat in the street canyon."
        }
        ViewMode::CityUnderground => {
            "Space Kit facility — corridors, rooms, gates under the surface.\n\
             This is the Geofront. Surface city sits above."
        }
        ViewMode::Battle => "",
    });
    ui.separator();
    ui.label("Click the city, then WASD / drag. View buttons switch surface, underground, and battle.");
}

#[derive(Debug, Clone)]
pub enum HudAction {
    Attack {
        attacker: u32,
        target: u32,
        limb: LimbKind,
    },
    EndTurn,
    Reset,
    SetView(ViewMode),
}
