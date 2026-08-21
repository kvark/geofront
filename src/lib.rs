//! Geofront – mecha tactical city defence and management.

mod base;
mod characters;
mod combat;
mod render;
mod ui;
mod units;
mod world;

use std::path::PathBuf;

use combat::{Action, Mission};
use log::info;
use units::LimbKind;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

#[cfg(not(target_arch = "wasm32"))]
use std::time;
#[cfg(target_arch = "wasm32")]
use web_time as time;

pub fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
        mount_embedded_assets();
    }

    info!("Geofront starting");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        run_smoke();
        return;
    }

    // On native, shaders must exist on disk. On WASM they are embedded into the VFS.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let shaders = assets_dir().join("shaders");
        if !shaders.is_dir() {
            eprintln!(
                "Missing assets/shaders.\n\
                 Run: ./scripts/fetch-shaders.sh\n\
                 Or:  cp -r ../redline/assets/shaders ./assets/shaders\n\
                 Smoke test still works without shaders:\n\
                   cargo run -- --smoke"
            );
            std::process::exit(1);
        }
    }

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App { game: None };
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

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Embed `assets/` into Blade's VFS so WASM can load shaders/models without a filesystem.
#[cfg(target_arch = "wasm32")]
fn mount_embedded_assets() {
    use include_dir::{Dir, include_dir};
    static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    fn walk(dir: &Dir, root: &std::path::Path) {
        for file in dir.files() {
            blade_engine::vfs::mount(root.join(file.path()), file.contents().to_vec());
        }
        for child in dir.dirs() {
            walk(child, root);
        }
    }
    walk(&ASSETS, &root);
    info!("Mounted embedded assets into VFS");
}

struct QuitEvent;

struct Game {
    engine: blade_engine::Engine,
    window: Window,
    egui_state: egui_winit::State,
    egui_viewport_id: egui::ViewportId,
    mission: Mission,
    selected_player: u32,
    selected_enemy: u32,
    selected_limb: LimbKind,
    last_redraw: time::Instant,
}

impl Drop for Game {
    fn drop(&mut self) {
        self.engine.destroy();
    }
}

impl Game {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        info!("Initializing Geofront (Blade + combat)");

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Geofront — Mecha Tactical")
                    .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 720.0)),
            )
            .expect("window");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::dpi::PhysicalSize;
            use winit::platform::web::WindowExtWebSys as _;
            let canvas = window.canvas().expect("winit canvas");
            canvas.set_id(blade_graphics::CANVAS_ID);
            if let Some(web_window) = web_sys::window() {
                let width = web_window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1100.0) as u32;
                let height = web_window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(720.0) as u32;
                let width = width.max(1);
                let height = height.max(1);
                canvas.set_width(width);
                canvas.set_height(height);
                let _ = window.request_inner_size(PhysicalSize::new(width, height));
            }
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| body.append_child(&web_sys::Element::from(canvas)).ok())
                .expect("couldn't append canvas");
        }

        let assets = assets_dir();
        let ray_trace = std::env::var_os("GEOFRONT_RT").is_some();

        let mut engine = blade_engine::Engine::new(
            blade_engine::Presentation::Window(&window),
            &blade_engine::config::Engine {
                shader_path: assets.join("shaders").to_string_lossy().into_owned(),
                data_path: assets.to_string_lossy().into_owned(),
                cache_path: "asset-cache".to_string(),
                time_step: 0.01,
                render_backend: if ray_trace {
                    blade_engine::config::RenderBackend::RayTracer
                } else {
                    blade_engine::config::RenderBackend::Rasterizer
                },
                gui_enabled: true,
            },
        );

        engine.set_raster_config(blade_render::RasterConfig {
            clear_color: blade_graphics::TextureColor::OpaqueBlack,
            light_dir: mint::Vector3 {
                x: 0.3,
                y: 0.8,
                z: 0.4,
            },
            light_color: mint::Vector3 {
                x: 1.8,
                y: 1.6,
                z: 1.4,
            },
            ambient_color: mint::Vector3 {
                x: 0.04,
                y: 0.045,
                z: 0.06,
            },
            space_sky: false,
        });

        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::dark());
        let egui_viewport_id = egui_context.viewport_id();
        let egui_state =
            egui_winit::State::new(egui_context, egui_viewport_id, &window, None, None, None);

        Self {
            engine,
            window,
            egui_state,
            egui_viewport_id,
            mission: Mission::new_skirmish(),
            selected_player: 0,
            selected_enemy: 10,
            selected_limb: LimbKind::Torso,
            last_redraw: time::Instant::now(),
        }
    }

    fn on_event(&mut self, event: &WindowEvent) -> Result<ControlFlow, QuitEvent> {
        let response = self.egui_state.on_window_event(&self.window, event);
        if response.repaint {
            self.window.request_redraw();
        }
        if response.consumed {
            return Ok(ControlFlow::Poll);
        }

        match *event {
            WindowEvent::CloseRequested => return Err(QuitEvent),
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if key_code == winit::keyboard::KeyCode::Escape {
                    return Err(QuitEvent);
                }
            }
            WindowEvent::RedrawRequested => {
                let delay = self.on_draw();
                return Ok(ControlFlow::wait_duration(delay));
            }
            _ => {}
        }
        Ok(ControlFlow::Poll)
    }

    fn on_draw(&mut self) -> time::Duration {
        self.last_redraw = time::Instant::now();

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_context = self.egui_state.egui_ctx().clone();

        let mut hud_action = None;
        let egui_output = egui_context.run_ui(raw_input, |egui_ctx| {
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                hud_action = ui::battle_hud(
                    ui,
                    &self.mission,
                    &mut self.selected_player,
                    &mut self.selected_enemy,
                    &mut self.selected_limb,
                );
            });
        });

        if let Some(action) = hud_action {
            match action {
                ui::HudAction::Attack {
                    attacker,
                    target,
                    limb,
                } => {
                    if let Err(e) = self.mission.apply_action(Action::Attack {
                        attacker_id: attacker,
                        target_id: target,
                        limb,
                    }) {
                        self.mission.log.push(format!("Attack failed: {e}"));
                    }
                }
                ui::HudAction::EndTurn => {
                    self.mission.end_player_turn();
                }
                ui::HudAction::Reset => {
                    self.mission = Mission::new_skirmish();
                    self.selected_player = 0;
                    self.selected_enemy = 10;
                    self.selected_limb = LimbKind::Torso;
                }
            }
        }

        self.egui_state
            .handle_platform_output(&self.window, egui_output.platform_output);

        let primitives = self
            .egui_state
            .egui_ctx()
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);

        let camera = render::Scene::combat_camera();
        self.engine.render(
            &camera,
            &primitives,
            &egui_output.textures_delta,
            self.window.inner_size(),
            self.window.scale_factor() as f32,
        );

        egui_output.viewport_output[&self.egui_viewport_id].repaint_delay
    }
}

struct App {
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.is_none() {
            self.game = Some(Game::new(event_loop));
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(game) = self.game.as_ref() {
            game.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        match game.on_event(&event) {
            Ok(control_flow) => event_loop.set_control_flow(control_flow),
            Err(QuitEvent) => event_loop.exit(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web_start {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn start() {
        crate::run();
    }
}
