//! egui screens: NERV-style battle HUD, city overview, mode switching.

use crate::characters::{PilotDramaKind, portrait_initials, portrait_rgb};
use crate::combat::{Mission, TurnPhase};
use crate::render::ViewMode;
use crate::units::{AtFieldHudState, Facing, LimbKind, SyncBand, Team};

const NERV_ORANGE: egui::Color32 = egui::Color32::from_rgb(232, 120, 32);
const NERV_AMBER: egui::Color32 = egui::Color32::from_rgb(255, 196, 110);
const NERV_DIM: egui::Color32 = egui::Color32::from_rgb(120, 128, 138);
const NERV_LINE: egui::Color32 = egui::Color32::from_rgb(58, 36, 18);
const NERV_CYAN: egui::Color32 = egui::Color32::from_rgb(100, 210, 255);
const NERV_GREEN: egui::Color32 = egui::Color32::from_rgb(150, 220, 165);
const NERV_RED: egui::Color32 = egui::Color32::from_rgb(255, 96, 90);
const NERV_YELLOW: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);

/// Draws the side-panel HUD and returns any action the player requested.
/// Help / camera debug copy is hidden unless `show_help` is set (H or F1).
pub fn side_hud(
    ui: &mut egui::Ui,
    mission: &Mission,
    view_mode: ViewMode,
    selected_player: &mut u32,
    selected_enemy: &mut u32,
    selected_limb: &mut LimbKind,
    show_help: &mut bool,
) -> Option<HudAction> {
    let mut requested = None;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("NERV")
                .strong()
                .color(NERV_ORANGE)
                .size(16.0),
        );
        ui.label(egui::RichText::new("·").color(NERV_DIM));
        ui.label(
            egui::RichText::new("GEOFRONT")
                .strong()
                .color(NERV_AMBER)
                .size(16.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let help_label = if *show_help { "H · hide" } else { "H · help" };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(help_label).small().color(NERV_DIM))
                        .fill(egui::Color32::from_rgb(18, 20, 26))
                        .stroke(egui::Stroke::new(1.0_f32, NERV_LINE)),
                )
                .on_hover_text("Toggle camera and debug help (H or F1)")
                .clicked()
            {
                *show_help = !*show_help;
            }
        });
    });
    ui.label(
        egui::RichText::new(match view_mode {
            ViewMode::Battle => "MAGI // TACTICAL  —  STREET COMBAT",
            ViewMode::CitySurface => "MAGI // TACTICAL  —  SURFACE OVERVIEW",
            ViewMode::CityUnderground => "MAGI // TACTICAL  —  GEOFRONT FACILITY",
        })
        .small()
        .color(NERV_DIM),
    );
    rule(ui);

    ui.horizontal(|ui| {
        for mode in [
            ViewMode::Battle,
            ViewMode::CitySurface,
            ViewMode::CityUnderground,
        ] {
            let on = view_mode == mode;
            let text = egui::RichText::new(mode.label()).small().color(if on {
                NERV_AMBER
            } else {
                NERV_DIM
            });
            if ui.selectable_label(on, text).clicked() {
                requested = Some(HudAction::SetView(mode));
            }
        }
    });

    if *show_help {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "WASD move  ·  drag to look  ·  Q/E height  ·  Shift sprint  ·  wheel dolly",
            )
            .small()
            .color(NERV_DIM),
        );
        if matches!(
            view_mode,
            ViewMode::CitySurface | ViewMode::CityUnderground
        ) {
            ui.label(
                egui::RichText::new(
                    "Click the city, then WASD / drag. View buttons switch surface, underground, and battle.",
                )
                .small()
                .color(NERV_DIM),
            );
        }
    }

    rule(ui);

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
            city_panel(ui, view_mode, *show_help);
        }
    }

    requested
}

fn rule(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        egui::Sense::hover(),
    );
    ui.painter()
        .hline(rect.x_range(), rect.center().y, egui::Stroke::new(1.0_f32, NERV_LINE));
    ui.add_space(4.0);
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .small()
            .strong()
            .color(NERV_ORANGE),
    );
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
        ui.label(
            egui::RichText::new(format!("TURN {:02}", mission.turn))
                .strong()
                .color(NERV_AMBER),
        );
        ui.separator();
        let (phase_text, phase_color) = match mission.phase {
            TurnPhase::Player => ("PHASE  PLAYER", NERV_GREEN),
            TurnPhase::Enemy => ("PHASE  ENEMY", NERV_YELLOW),
        };
        ui.label(egui::RichText::new(phase_text).strong().color(phase_color));
        ui.separator();
        let city_color = if mission.city_hp > 60.0 {
            NERV_GREEN
        } else if mission.city_hp > 30.0 {
            NERV_YELLOW
        } else {
            NERV_RED
        };
        ui.label(
            egui::RichText::new(format!("CITY  {:.0}%", mission.city_hp))
                .strong()
                .color(city_color),
        );

        if mission.is_won() {
            ui.label(egui::RichText::new("  VICTORY").strong().color(NERV_GREEN));
        } else if mission.is_lost() {
            ui.label(egui::RichText::new("  DEFEAT").strong().color(NERV_RED));
        }
    });

    if let Some(flash) = &mission.pilot_flash {
        let color = match flash.kind {
            PilotDramaKind::StressSpike => egui::Color32::from_rgb(255, 170, 70),
            PilotDramaKind::Refuse => NERV_RED,
            PilotDramaKind::Steady => NERV_GREEN,
            PilotDramaKind::CoreSight => egui::Color32::from_rgb(200, 170, 255),
        };
        ui.add_space(4.0);
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

    rule(ui);

    ui.columns(2, |cols| {
        section_label(&mut cols[0], "UNIT");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Player) {
            let selected = *selected_player == m.id;
            let (hp, max) = m.total_hp();
            let pilot = m.pilot_id.and_then(|id| mission.pilot(id));
            let stressed = pilot.map(|p| p.pending_refuse).unwrap_or(false);
            let sync_mark = pilot
                .map(|p| {
                    let badge = format!("  SYNC {}%", p.sync_percent());
                    match p.sync_band() {
                        SyncBand::High => format!("{badge} +"),
                        SyncBand::Low => format!("{badge} -"),
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
                    "{} {}  ({},{})  {:.0}/{:.0}  MP{}{}{}  !",
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
                        AtFieldHudState::Ready { .. } => NERV_CYAN,
                        AtFieldHudState::Shattered { .. } => egui::Color32::from_rgb(160, 170, 190),
                    };
                    paint_at_status(&mut cols[0], state, color);
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
                                ui.label(
                                    egui::RichText::new(format!("Pilot {}", p.name)).color(NERV_AMBER),
                                );
                                paint_sync_bar(ui, p.sync, p.sync_band());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "loyalty {:.0}%",
                                            p.loyalty * 100.0
                                        ))
                                        .small()
                                        .color(NERV_DIM),
                                    );
                                    let stress_color = if p.stress >= 0.6 {
                                        NERV_RED
                                    } else if p.stress >= 0.3 {
                                        NERV_YELLOW
                                    } else {
                                        NERV_GREEN
                                    };
                                    ui.colored_label(
                                        stress_color,
                                        format!("stress {:.0}%", p.stress * 100.0),
                                    );
                                });
                            });
                        });
                        if p.pending_refuse {
                            wrapped_label(
                                &mut cols[0],
                                egui::Color32::from_rgb(255, 170, 70),
                                "! stressed — may refuse next order",
                            );
                        }
                        match p.sync_band() {
                            SyncBand::High => {
                                wrapped_label(
                                    &mut cols[0],
                                    NERV_CYAN,
                                    format!(
                                        "HIGH SYNC — strikes ×{:.2}, crit window, sees Angel cores",
                                        p.sync_damage_mult()
                                    ),
                                );
                            }
                            SyncBand::Low => {
                                wrapped_label(
                                    &mut cols[0],
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
                            ui.label(
                                egui::RichText::new(format!("{}", limb.kind))
                                    .small()
                                    .color(NERV_DIM),
                            );
                            let color = if ratio > 0.6 {
                                NERV_GREEN
                            } else if ratio > 0.25 {
                                NERV_YELLOW
                            } else {
                                NERV_RED
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

        section_label(&mut cols[1], "TARGET");
        for m in mission.mechs.iter().filter(|m| m.team == Team::Enemy) {
            let selected = *selected_enemy == m.id;
            let (hp, max) = m.total_hp();
            let at_state = m.at_field_hud_state();
            let at_mark = at_state.map(|s| s.list_mark()).unwrap_or_default();
            let tag = match m.alien {
                Some(crate::units::AlienKind::Splinter) => "  VIOLET",
                Some(crate::units::AlienKind::Mass) => "  CRIMSON",
                None => "",
            };
            let label = if m.destroyed {
                format!("{} (destroyed)", m.name)
            } else {
                format!(
                    "{} {}  ({},{})  {:.0}/{:.0}{}{}",
                    m.name,
                    m.facing.label(),
                    m.position.x,
                    m.position.y,
                    hp,
                    max,
                    at_mark,
                    tag
                )
            };
            if cols[1].selectable_label(selected, label).clicked() && !m.destroyed {
                *selected_enemy = m.id;
            }
            if selected && !m.destroyed {
                if let Some(state) = at_state {
                    let color = match state {
                        AtFieldHudState::Ready { .. } => NERV_CYAN,
                        AtFieldHudState::Shattered { .. } => egui::Color32::from_rgb(180, 140, 160),
                    };
                    paint_at_status(&mut cols[1], state, color);
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
                    wrapped_label(
                        &mut cols[1],
                        egui::Color32::from_rgb(220, 190, 255),
                        "core visible — high sync core strike",
                    );
                }
            }
        }
    });

    rule(ui);
    section_label(ui, "ORDERS");

    let can_act =
        matches!(mission.phase, TurnPhase::Player) && !mission.is_won() && !mission.is_lost();
    let mech = mission.mech(*selected_player);
    let can_move = can_act && mech.map(|m| m.can_move()).unwrap_or(false);
    let can_fire = can_act && mech.map(|m| m.can_act()).unwrap_or(false);

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
        ui.label(egui::RichText::new("Target limb").small().color(NERV_DIM));
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

    rule(ui);
    section_label(ui, "OPS LOG");
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
                    ui.colored_label(NERV_CYAN, line);
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
                    ui.label(egui::RichText::new(line).color(egui::Color32::from_rgb(190, 196, 204)));
                }
            }
        });

    requested
}


/// Label that stays inside the current column (egui columns do not clip).
fn wrapped_label(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<String>) {
    ui.add(
        egui::Label::new(egui::RichText::new(text.into()).color(color)).wrap(),
    );
}

/// AT Field charge pips drawn as shapes (egui's default font has no ● / ○).
/// Pips and the detail line stack so the UNIT column never spills into TARGET.
fn paint_at_status(ui: &mut egui::Ui, state: AtFieldHudState, color: egui::Color32) {
    let (filled, slots) = match state {
        AtFieldHudState::Ready { charges, max } => (charges, max.max(1)),
        AtFieldHudState::Shattered { .. } => (0, 1),
    };
    ui.vertical(|ui| {
        ui.set_max_width(ui.available_width());
        ui.horizontal(|ui| {
            let desired = egui::vec2(slots as f32 * 12.0 + 2.0, 12.0);
            let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            for i in 0..slots {
                let x = rect.left() + i as f32 * 12.0;
                let r = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top() + 1.0),
                    egui::vec2(10.0, 10.0),
                );
                if i < filled {
                    painter.rect_filled(r, 1.5, color);
                } else {
                    painter.rect_stroke(
                        r,
                        1.5,
                        egui::Stroke::new(1.0_f32, color),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        });
        wrapped_label(ui, color, state.detail_line());
    });
}

/// Glanceable sync ratio bar + % badge (color ramp matches SyncBand thresholds).
fn paint_sync_bar(ui: &mut egui::Ui, sync: f32, band: SyncBand) {
    let fill = match band {
        SyncBand::High => NERV_CYAN,
        SyncBand::Low => egui::Color32::from_rgb(255, 140, 70),
        SyncBand::Mid => NERV_GREEN,
    };
    let pct = (sync * 100.0).round().clamp(0.0, 100.0) as u8;
    ui.horizontal(|ui| {
        ui.colored_label(fill, format!("SYNC {pct}%"));
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
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 90)),
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
        egui::Stroke::new(1.0_f32, NERV_ORANGE)
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

fn city_panel(ui: &mut egui::Ui, mode: ViewMode, show_help: bool) {
    ui.label(
        egui::RichText::new(match mode {
            ViewMode::CitySurface => "TOKYO-3  SURFACE BLOCK",
            ViewMode::CityUnderground => "GEOFRONT  HANGAR / COMMAND",
            ViewMode::Battle => "",
        })
        .strong()
        .color(NERV_AMBER),
    );
    if show_help {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(match mode {
                ViewMode::CitySurface => {
                    "Kenney city block — commercial towers and industrial fringe.\n\
                     Switch to Battle for close-up combat in the street canyon."
                }
                ViewMode::CityUnderground => {
                    "Geofront hangar plus command, east/west wings, south airlock.\n\
                     Pieces abut on edges so floors no longer Z-fight."
                }
                ViewMode::Battle => "",
            })
            .small()
            .color(NERV_DIM),
        );
    } else {
        ui.label(
            egui::RichText::new("H or F1 — camera help and block notes")
                .small()
                .color(NERV_DIM),
        );
    }
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
