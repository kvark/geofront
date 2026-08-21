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
use units::LimbKind;
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
    selected_limb: LimbKind,
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
                    .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 720.0)),
            )
            .expect("window");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys as _;
            let canvas = window.canvas().expect("winit canvas");
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
        self.selected_limb = LimbKind::Torso;

        info!("Window ready. Interactive combat HUD active (Blade presentation next).");
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
                self.draw_ui();
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
    fn draw_ui(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(egui_state) = self.egui_state.as_mut() else {
            return;
        };
        let Some(mission) = self.mission.as_mut() else {
            return;
        };

        let raw_input = egui_state.take_egui_input(window);
        let mut hud_action = None;

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                hud_action = ui::battle_hud(
                    ui,
                    mission,
                    &mut self.selected_player,
                    &mut self.selected_enemy,
                    &mut self.selected_limb,
                );
            });
        });

        // Apply any requested action after the UI borrow ends.
        if let Some(action) = hud_action {
            match action {
                ui::HudAction::Attack {
                    attacker,
                    target,
                    limb,
                } => {
                    if let Err(e) = mission.apply_action(Action::Attack {
                        attacker_id: attacker,
                        target_id: target,
                        limb,
                    }) {
                        mission.log.push(format!("Attack failed: {e}"));
                    }
                }
                ui::HudAction::EndTurn => {
                    mission.end_player_turn();
                }
                ui::HudAction::Reset => {
                    *mission = Mission::new_skirmish();
                    self.selected_player = 0;
                    self.selected_enemy = 10;
                    self.selected_limb = LimbKind::Torso;
                }
            }
        }

        egui_state.handle_platform_output(window, full_output.platform_output);

        // TODO: tessellate + Blade Engine::render so the HUD actually appears.
        // Until assets/shaders are present and Engine is wired, the window
        // processes input but does not present egui meshes.
        let _ = full_output;
        window.request_redraw();
    }
}
