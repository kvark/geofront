//! Geofront – mecha tactical city defence and management.

mod base;
mod characters;
mod combat;
mod render;
mod ui;
mod units;
mod world;

use combat::{Action, Mission};
use log::info;
use units::{LimbKind, Team};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).expect("logger");
    }

    info!("Geofront starting");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        run_smoke();
        return;
    }

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run");
}

fn run_smoke() {
    info!("Running combat smoke test");
    let mut mission = Mission::new_skirmish();
    mission.smoke_run(6);
    for line in &mission.log {
        println!("{line}");
    }
    if mission.is_won() {
        info!("Smoke finished: WIN");
    } else if mission.is_lost() {
        info!("Smoke finished: LOSS");
    } else {
        info!("Smoke finished: ongoing (turn {})", mission.turn);
    }
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    egui_state: Option<egui_winit::State>,
    egui_ctx: egui::Context,
    mission: Option<Mission>,
    selected_player: u32,
    selected_enemy: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Geofront — Mecha Tactical")
                    .with_inner_size(winit::dpi::LogicalSize::new(960.0, 640.0)),
            )
            .expect("window");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys as _;
            let canvas = window.canvas().expect("winit canvas");
            // Keep a stable id for any future Blade integration
            canvas.set_id("geofront-canvas");
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
                .expect("couldn't append canvas");
        }

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        self.window = Some(window);
        self.egui_state = Some(egui_state);
        self.mission = Some(Mission::new_skirmish());
        self.selected_player = 0;
        self.selected_enemy = 10;

        info!("Window + egui ready (Blade 3D scene next)");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(egui_state) = self.egui_state.as_mut() else {
            return;
        };

        let response = egui_state.on_window_event(window, &event);
        if response.repaint {
            window.request_redraw();
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.draw_ui(window);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl App {
    fn draw_ui(&mut self, window: &Window) {
        let Some(egui_state) = self.egui_state.as_mut() else {
            return;
        };
        let Some(mission) = self.mission.as_mut() else {
            return;
        };

        let raw_input = egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Geofront");
                ui.label("Mecha tactical city defence — combat MVP (Blade 3D next)");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(format!("Turn {}", mission.turn));
                    ui.label(format!("Phase: {:?}", mission.phase));
                    ui.label(format!("City protection: {:.0}%", mission.city_hp));
                    if mission.is_won() {
                        ui.colored_label(egui::Color32::GREEN, "VICTORY");
                    } else if mission.is_lost() {
                        ui.colored_label(egui::Color32::RED, "DEFEAT");
                    }
                });

                ui.separator();
                ui.heading("Units");
                for m in &mission.mechs {
                    if m.destroyed {
                        continue;
                    }
                    let (hp, max) = m.total_hp();
                    let team = match m.team {
                        Team::Player => "P",
                        Team::Enemy => "E",
                    };
                    ui.label(format!(
                        "[{team}] {} @ ({}, {})  HP {:.0}/{:.0}  mob {:.0}%  fire {:.0}%",
                        m.name,
                        m.position.x,
                        m.position.y,
                        hp,
                        max,
                        m.mobility() * 100.0,
                        m.firepower() * 100.0
                    ));
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("End Turn").clicked() && !mission.is_won() && !mission.is_lost() {
                        mission.end_player_turn();
                    }
                    if ui.button("Reset Mission").clicked() {
                        *mission = Mission::new_skirmish();
                    }
                    if ui.button("Player attack torso").clicked() && !mission.is_won() && !mission.is_lost() {
                        let _ = mission.apply_action(Action::Attack {
                            attacker_id: self.selected_player,
                            target_id: self.selected_enemy,
                            limb: LimbKind::Torso,
                        });
                    }
                });

                ui.separator();
                ui.heading("Combat log");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in mission.log.iter().rev().take(30).rev() {
                            ui.label(line);
                        }
                    });
            });
        });

        egui_state.handle_platform_output(window, full_output.platform_output);
        // Note: without a full wgpu/Blade renderer we don't paint egui meshes yet.
        // The window + input loop is live; next step is Blade Engine to present frames.
        let _ = full_output;
        window.request_redraw();
    }
}
