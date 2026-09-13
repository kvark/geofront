//! egui screens: battle HUD, city overview controls, mode switching.

use crate::characters::{PilotDramaKind, portrait_initials, portrait_rgb};
use crate::combat::{Mission, TurnPhase};
use crate::render::ViewMode;
use crate::units::{AtFieldHudState, Facing, LimbKind, SyncBand, Team};

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
        ViewMode::Battle => "Close-up street combat",
        ViewMode::CitySurface => "Surface city overview",
        ViewMode::CityUnderground => "Underground facility overview",
    });
    ui.separator();

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
        let phase_text = match mission.phase {
            TurnPhase::Player => "Your phase",
            TurnPhase::Enemy => "Enemy phase",
        };
        ui.label(phase_text);
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

    if let Some(flash) = &mission.pilot_flash {
        let color = match flash.kind {
            PilotDramaKind::StressSpike => egui::Color32::from_rgb(255, 170, 70),
            PilotDramaKind::Refuse => egui::Color32::from_rgb(255, 90, 90),
            PilotDramaKind::Steady => egui::Color32::from_rgb(140, 220, 160),
            PilotDramaKind::CoreSight => egui::Color32::from_rgb(200, 170, 255),
        };
        ui.horizontal(|ui| {
            let stressed = matches!(
                flash.kind,
                PilotDramaKind::StressSpike | PilotDramaKind::Refuse
            );
            paint_pilot_portrait(
                ui,
                flash.pilot_id,
                &flash.pilot_name,
                44.0,
                stressed,
                matches!(flash.kind, PilotDramaKind::CoreSight),
            );
            ui.vertical(|ui| {
                ui.colored_label(color, flash.caption());
                ui.label(
                    egui::RichText::new(format!(r#"{}: "{}""#, flash.pilot_name, flash.dialog))
                        .strong()
                        .color(color),
                );
                ui.small(&flash.line);
            });
        });
    }

    ui.separator();

    ui.columns(2, |cols| {
        cols[0].heading("Player");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Player) {
            let selected = *selected_player == m.id;
            let (hp, max) = m.total_hp();
            let pilot = m.pilot_id.and_then(|id| mission.pilot(id));
            let stressed = pilot.map(|p| p.pending_refuse).unwrap_or(false);
            let sync_mark = pilot
                .map(|p| {
                    let badge = format!("  sync{}%", p.sync_percent());
                    match p.sync_band() {
                        SyncBand::High => format!("{badge}⚡"),
                        SyncBand::Low => format!("{badge}↓"),
                        SyncBand::Mid => badge,
                    }
                })
                .unwrap_or_default();
            let at_state = m.at_field_hud_state();
            let at_mark = at_state.map(|s| s.list_mark()).unwrap_or_default();
            let label = if m.destroyed {
                format!("{} (destroyed)", m.name)
            } else if stressed {
                format!(
                    "{} {}  ({},{})  {:.0}/{:.0}  MP{}{}{}  ⚠",
                    m.name,
                    m.facing.label(),
                    m.position.x,
                    m.position.y,
                    hp,
                    max,
                    m.move_left,
                    at_mark,
                    sync_mark
                )
            } else {
                format!(
                    "{} {}  ({},{})  {:.0}/{:.0}  MP{}{}{}",
                    m.name,
                    m.facing.label(),
                    m.position.x,
                    m.position.y,
                    hp,
                    max,
                    m.move_left,
                    at_mark,
                    sync_mark
                )
            };
            if cols[0].selectable_label(selected, label).clicked() && !m.destroyed {
                *selected_player = m.id;
            }
            if selected && !m.destroyed {
                if let Some(state) = at_state {
                    let color = match state {
                        AtFieldHudState::Ready { .. } => egui::Color32::from_rgb(100, 210, 255),
                        AtFieldHudState::Shattered { .. } => egui::Color32::from_rgb(160, 170, 190),
                    };
                    cols[0].colored_label(color, state.detail_line());
                }
                if let Some(pid) = m.pilot_id {
                    if let Some(p) = mission.pilot(pid) {
                        cols[0].horizontal(|ui| {
                            paint_pilot_portrait(
                                ui,
                                p.id,
                                &p.name,
                                36.0,
                                p.pending_refuse || p.stress >= 0.6,
                                p.can_see_core(),
                            );
                            ui.vertical(|ui| {
                                ui.label(format!("Pilot {}", p.name));
                                paint_sync_bar(ui, p.sync, p.sync_band());
                                ui.horizontal(|ui| {
                                    ui.label(format!("loyalty {:.0}%", p.loyalty * 100.0));
                                    let stress_color = if p.stress >= 0.6 {
                                        egui::Color32::from_rgb(255, 90, 90)
                                    } else if p.stress >= 0.3 {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::LIGHT_GREEN
                                    };
                                    ui.colored_label(
                                        stress_color,
                                        format!("stress {:.0}%", p.stress * 100.0),
                                    );
                                });
                            });
                        });
                        if p.pending_refuse {
                            cols[0].colored_label(
                                egui::Color32::from_rgb(255, 170, 70),
                                "⚠ stressed — may refuse next order",
                            );
                        }
                        match p.sync_band() {
                            SyncBand::High => {
                                cols[0].colored_label(
                                    egui::Color32::from_rgb(100, 220, 255),
                                    format!(
                                        "⚡ high sync — strikes ×{:.2}, crit window, sees Angel cores",
                                        p.sync_damage_mult()
                                    ),
                                );
                            }
                            SyncBand::Low => {
                                cols[0].colored_label(
                                    egui::Color32::from_rgb(255, 140, 70),
                                    format!("sync frayed — strikes ×{:.2}", p.sync_damage_mult()),
                                );
                            }
                            SyncBand::Mid => {}
                        }
                    }
                }
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
                            ui.colored_label(color, format!("{:.0}/{:.0}", limb.hp, limb.max_hp));
                        });
                    }
                });
                let status = if m.acted {
                    "acted"
                } else if m.can_move() {
                    "can move / attack"
                } else {
                    "can attack or wait"
                };
                cols[0].small(status);
            }
        }

        cols[1].heading("Enemy");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Enemy) {
            let selected = *selected_enemy == m.id;
            let (hp, max) = m.total_hp();
            let at_state = m.at_field_hud_state();
            let at_mark = at_state.map(|s| s.list_mark()).unwrap_or_default();
            let label = if m.destroyed {
                format!("{} (destroyed)", m.name)
            } else {
                format!(
                    "{} {}  ({},{})  {:.0}/{:.0}{}",
                    m.name,
                    m.facing.label(),
                    m.position.x,
                    m.position.y,
                    hp,
                    max,
                    at_mark
                )
            };
            if cols[1].selectable_label(selected, label).clicked() && !m.destroyed {
                *selected_enemy = m.id;
            }
            if selected && !m.destroyed {
                if let Some(state) = at_state {
                    let color = match state {
                        AtFieldHudState::Ready { .. } => egui::Color32::from_rgb(100, 210, 255),
                        AtFieldHudState::Shattered { .. } => egui::Color32::from_rgb(180, 140, 160),
                    };
                    cols[1].colored_label(color, state.detail_line());
                }
            }
            if selected && !m.destroyed && m.alien.is_some() {
                let sees = mission
                    .mech(*selected_player)
                    .and_then(|pm| pm.pilot_id)
                    .and_then(|id| mission.pilot(id))
                    .map(|p| p.can_see_core())
                    .unwrap_or(false);
                if sees {
                    cols[1].colored_label(
                        egui::Color32::from_rgb(220, 190, 255),
                        "core visible — high sync core strike",
                    );
                }
            }
        }
    });

    ui.separator();

    let can_act =
        matches!(mission.phase, TurnPhase::Player) && !mission.is_won() && !mission.is_lost();
    let mech = mission.mech(*selected_player);
    let can_move = can_act && mech.map(|m| m.can_move()).unwrap_or(false);
    let can_fire = can_act && mech.map(|m| m.can_act()).unwrap_or(false);

    ui.label("Step (1 tile) / face");
    ui.horizontal(|ui| {
        let mut pick = None;
        let step = |ui: &mut egui::Ui, label: &str, dir: Facing, enabled: bool| {
            ui.add_enabled(enabled, egui::Button::new(label))
                .clicked()
                .then_some(HudAction::Step(dir))
        };
        pick = pick.or(step(ui, "N", Facing::North, can_move));
        pick = pick.or(step(ui, "W", Facing::West, can_move));
        pick = pick.or(step(ui, "E", Facing::East, can_move));
        pick = pick.or(step(ui, "S", Facing::South, can_move));
        ui.separator();
        if ui.add_enabled(can_fire, egui::Button::new("↺")).clicked() {
            pick = Some(HudAction::Rotate(-1));
        }
        if ui.add_enabled(can_fire, egui::Button::new("↻")).clicked() {
            pick = Some(HudAction::Rotate(1));
        }
        if pick.is_some() {
            requested = pick;
        }
    });

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
        if ui
            .add_enabled(can_fire, egui::Button::new("Attack"))
            .clicked()
        {
            requested = Some(HudAction::Attack {
                attacker: *selected_player,
                target: *selected_enemy,
                limb: *selected_limb,
            });
        }
        if ui
            .add_enabled(can_fire, egui::Button::new("Wait"))
            .clicked()
        {
            requested = Some(HudAction::Wait);
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
                let at_recharge = line.contains("AT Field recharges");
                let at_shatter = line.contains("SHATTERS") || line.contains("AT Field shattered");
                let at_absorb = line.contains("AT Field absorbs");
                let drama = line.contains("may refuse next order")
                    || line.contains("refuses the")
                    || line.contains("steadies")
                    || line.contains("sees the pattern");
                if at_shatter {
                    ui.colored_label(egui::Color32::from_rgb(180, 220, 255), line);
                } else if at_recharge || at_absorb {
                    ui.colored_label(egui::Color32::from_rgb(100, 210, 255), line);
                } else if drama {
                    let color = if line.contains("refuses the") {
                        egui::Color32::from_rgb(255, 120, 120)
                    } else if line.contains("may refuse") {
                        egui::Color32::from_rgb(255, 180, 90)
                    } else if line.contains("sees the pattern") {
                        egui::Color32::from_rgb(200, 170, 255)
                    } else {
                        egui::Color32::from_rgb(160, 220, 170)
                    };
                    ui.colored_label(color, line);
                } else {
                    ui.label(line);
                }
            }
        });

    requested
}

/// Glanceable sync ratio bar + % badge (color ramp matches SyncBand thresholds).
fn paint_sync_bar(ui: &mut egui::Ui, sync: f32, band: SyncBand) {
    let fill = match band {
        SyncBand::High => egui::Color32::from_rgb(100, 220, 255),
        SyncBand::Low => egui::Color32::from_rgb(255, 140, 70),
        SyncBand::Mid => egui::Color32::from_rgb(140, 210, 150),
    };
    let pct = (sync * 100.0).round().clamp(0.0, 100.0) as u8;
    ui.horizontal(|ui| {
        ui.colored_label(fill, format!("sync {pct}%"));
        let desired = egui::vec2(88.0, 10.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(28, 32, 40));
        let mut filled = rect;
        filled.set_width(rect.width() * sync.clamp(0.0, 1.0));
        painter.rect_filled(filled, 2.0, fill);
        // Threshold ticks at low (45%) and high (85%).
        for t in [0.45_f32, 0.85] {
            let x = rect.left() + rect.width() * t;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 90)),
            );
        }
    });
}

/// Procedural pilot plaque: tinted silhouette + initials (no art packs).
fn paint_pilot_portrait(

    ui: &mut egui::Ui,
    pilot_id: u32,
    name: &str,
    size: f32,
    stressed: bool,
    high_sync: bool,
) {
    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let rgb = portrait_rgb(pilot_id);
    let mut fill = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    if stressed {
        fill = egui::Color32::from_rgb(
            ((rgb[0] as u16 * 2 + 255) / 3) as u8,
            (rgb[1] as u16 * 2 / 3) as u8,
            (rgb[2] as u16 * 2 / 3) as u8,
        );
    }
    let stroke = if high_sync {
        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(180, 230, 255))
    } else if stressed {
        egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 120, 80))
    } else {
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(30, 30, 40))
    };
    painter.rect(
        rect,
        5.0,
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    // Head + shoulders silhouette.
    let c = rect.center();
    let head_r = size * 0.18;
    let head = egui::pos2(c.x, rect.top() + size * 0.32);
    painter.circle_filled(head, head_r, egui::Color32::from_rgba_unmultiplied(20, 22, 28, 200));
    let shoulder = egui::Rect::from_center_size(
        egui::pos2(c.x, rect.top() + size * 0.72),
        egui::vec2(size * 0.62, size * 0.38),
    );
    painter.rect_filled(
        shoulder,
        size * 0.2,
        egui::Color32::from_rgba_unmultiplied(20, 22, 28, 200),
    );
    let initials = portrait_initials(name);
    painter.text(
        c,
        egui::Align2::CENTER_CENTER,
        initials,
        egui::FontId::proportional((size * 0.38).clamp(11.0, 18.0)),
        egui::Color32::WHITE,
    );
}

fn city_panel(ui: &mut egui::Ui, mode: ViewMode) {
    ui.label(match mode {
        ViewMode::CitySurface => {
            "Kenney city block — commercial towers and industrial fringe.\n\
             Switch to Battle for close-up combat in the street canyon."
        }
        ViewMode::CityUnderground => {
            "Geofront hangar plus command, east/west wings, south airlock.\n\
             Pieces abut on edges so floors no longer Z-fight."
        }
        ViewMode::Battle => "",
    });
    ui.separator();
    ui.label(
        "Click the city, then WASD / drag. View buttons switch surface, underground, and battle.",
    );
}

#[derive(Debug, Clone)]
pub enum HudAction {
    Attack {
        attacker: u32,
        target: u32,
        limb: LimbKind,
    },
    Step(Facing),
    Rotate(i8),
    Wait,
    EndTurn,
    Reset,
    SetView(ViewMode),
}
