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
    mission: Mission,
    arena: render::Arena,
    view_mode: render::ViewMode,
    selected_player: u32,
    selected_enemy: u32,
    selected_limb: LimbKind,
    /// Seconds remaining of impact framing after an attack.
    impact_timer: f32,
    /// Delay between queued enemy actions.
    enemy_step_timer: f32,
    fly: render::FlyCam,
    /// Held keys. look_dx/dy/wheel are per-frame and cleared after apply.
    keys: render::MoveInput,
    dragging: bool,
    last_cursor: Option<(f32, f32)>,
    last_redraw: time::Instant,
    started_at: time::Instant,
    /// Exit after this many seconds (GEOFRONT_QUIT_AFTER / GEOFRONT_SCREENSHOT).
    quit_after: Option<f32>,
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
                let width = web_window
                    .inner_width()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1100.0) as u32;
                let height = web_window
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(720.0) as u32;
                let width = width.max(1);
                let height = height.max(1);
                canvas.set_width(width);
                canvas.set_height(height);
                let _ = window.request_inner_size(PhysicalSize::new(width, height));
            }
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| body.append_child(&web_sys::Element::from(canvas.clone())).ok())
                .expect("couldn't append canvas");
            canvas.set_tab_index(0);
            let _ = canvas.focus();
            let style = canvas.style();
            let _ = style.set_property("outline", "none");
            let _ = style.set_property("touch-action", "none");
            let _ = style.set_property("cursor", "grab");
            let _ = style.set_property("width", "100%");
            let _ = style.set_property("height", "100%");
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
                x: 2.6,
                y: 2.4,
                z: 2.0,
            },
            ambient_color: mint::Vector3 {
                x: 0.16,
                y: 0.17,
                z: 0.20,
            },
            space_sky: false,
            // WebGL2 still traps on the depth pass even with raster_shadow_fs
            // (#378/#379 fix buffers + color space, not this link). Gate on wasm.
            directional_shadows: if cfg!(target_arch = "wasm32") {
                None
            } else {
                Some(blade_render::DirectionalShadowConfig {
                    resolution: 1024,
                    distance: 36.0,
                    depth: 90.0,
                    strength: 0.62,
                    normal_bias: 0.08,
                })
            },
            point_lights: Vec::new(),
        });

        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::dark());
        let egui_state =
            egui_winit::State::new(egui_context, egui::ViewportId::ROOT, &window, None, None, None);

        let mission = Mission::new_skirmish();
        let view_mode = match std::env::var("GEOFRONT_VIEW").ok().as_deref() {
            Some("battle") => render::ViewMode::Battle,
            Some("underground") => render::ViewMode::CityUnderground,
            Some("surface") => render::ViewMode::CitySurface,
            _ => render::ViewMode::CitySurface,
        };
        let quit_after = std::env::var("GEOFRONT_QUIT_AFTER")
            .ok()
            .and_then(|s| s.parse().ok())
            .or_else(|| {
                std::env::var("GEOFRONT_SCREENSHOT")
                    .ok()
                    .map(|_| 8.0)
            });
        let arena = render::Arena::spawn(&mut engine, view_mode, &mission);
        let fly = render::FlyCam::for_mode(view_mode);

        Self {
            engine,
            window,
            egui_state,
            mission,
            arena,
            view_mode,
            selected_player: 0,
            selected_enemy: 10,
            selected_limb: LimbKind::Torso,
            impact_timer: 0.0,
            enemy_step_timer: 0.0,
            fly,
            keys: render::MoveInput::default(),
            dragging: false,
            last_cursor: None,
            last_redraw: time::Instant::now(),
            started_at: time::Instant::now(),
            quit_after,
        }
    }

    fn apply_key(&mut self, key: winit::keyboard::KeyCode, down: bool) {
        use winit::keyboard::KeyCode::*;
        match key {
            KeyW => self.keys.w = down,
            KeyA => self.keys.a = down,
            KeyS => self.keys.s = down,
            KeyD => self.keys.d = down,
            KeyQ => self.keys.q = down,
            KeyE => self.keys.e = down,
            ShiftLeft | ShiftRight => self.keys.shift = down,
            _ => {}
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn sample_web_input(&mut self) {
        use wasm_bindgen::JsValue;
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Ok(keys) = js_sys::Reflect::get(&window, &JsValue::from_str("__gfKeys")) {
            let down = |code: &str| {
                js_sys::Reflect::get(&keys, &JsValue::from_str(code))
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            };
            self.keys.w = down("KeyW");
            self.keys.a = down("KeyA");
            self.keys.s = down("KeyS");
            self.keys.d = down("KeyD");
            self.keys.q = down("KeyQ") || down("Space");
            self.keys.e = down("KeyE") || down("ControlLeft") || down("ControlRight");
            self.keys.shift = down("ShiftLeft") || down("ShiftRight");
        }
        if let Ok(ptr) = js_sys::Reflect::get(&window, &JsValue::from_str("__gfPtr")) {
            let num = |name: &str| {
                js_sys::Reflect::get(&ptr, &JsValue::from_str(name))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32
            };
            let width = self.window.inner_size().width as f32;
            let over_hud = num("x") > (width - 370.0).max(0.0);
            if !over_hud {
                self.keys.look_dx += num("dx");
                self.keys.look_dy += num("dy");
                self.keys.wheel += num("wheel");
            }
            let _ = js_sys::Reflect::set(&ptr, &JsValue::from_str("dx"), &JsValue::from_f64(0.0));
            let _ = js_sys::Reflect::set(&ptr, &JsValue::from_str("dy"), &JsValue::from_f64(0.0));
            let _ = js_sys::Reflect::set(&ptr, &JsValue::from_str("wheel"), &JsValue::from_f64(0.0));
        }
        if let Ok(view) = js_sys::Reflect::get(&window, &JsValue::from_str("__gfView")) {
            if let Some(s) = view.as_string() {
                let mode = match s.as_str() {
                    "battle" => Some(render::ViewMode::Battle),
                    "surface" => Some(render::ViewMode::CitySurface),
                    "underground" => Some(render::ViewMode::CityUnderground),
                    _ => None,
                };
                if mode.is_some() {
                    let _ = js_sys::Reflect::set(
                        &window,
                        &JsValue::from_str("__gfView"),
                        &JsValue::from_str(""),
                    );
                }
                if let Some(mode) = mode {
                    self.handle_hud(ui::HudAction::SetView(mode));
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn publish_controls_probe(&self) {
        use wasm_bindgen::JsValue;
        let Some(window) = web_sys::window() else {
            return;
        };
        let cam = js_sys::Object::new();
        let set = |obj: &js_sys::Object, k: &str, v: f64| {
            let _ = js_sys::Reflect::set(obj, &JsValue::from_str(k), &JsValue::from_f64(v));
        };
        set(&cam, "yaw", self.fly.yaw as f64);
        set(&cam, "pitch", self.fly.pitch as f64);
        set(&cam, "speed", self.fly.speed as f64);
        set(&cam, "x", self.fly.pos.x as f64);
        set(&cam, "y", self.fly.pos.y as f64);
        set(&cam, "z", self.fly.pos.z as f64);
        let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__gfCam"), &cam);
    }

    fn on_event(&mut self, event: &WindowEvent) -> Result<ControlFlow, QuitEvent> {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                let down = *state == winit::event::ElementState::Pressed;
                self.apply_key(*key_code, down);
                if down && *key_code == winit::keyboard::KeyCode::Escape {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        return Err(QuitEvent);
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                let width = self.window.inner_size().width as f32;
                let over_hud = self
                    .last_cursor
                    .map(|(x, _)| x > width - 370.0)
                    .unwrap_or(false);
                self.dragging = *state == winit::event::ElementState::Pressed && !over_hud;
                if !self.dragging {
                    self.last_cursor = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                if self.dragging {
                    if let Some((lx, ly)) = self.last_cursor {
                        self.keys.look_dx += pos.0 - lx;
                        self.keys.look_dy += pos.1 - ly;
                    }
                }
                self.last_cursor = Some(pos);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let width = self.window.inner_size().width as f32;
                let over_hud = self
                    .last_cursor
                    .map(|(x, _)| x > width - 370.0)
                    .unwrap_or(false);
                if !over_hud {
                    self.keys.wheel += match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 24.0,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                    };
                }
            }
            WindowEvent::Focused(false) => {
                self.keys = render::MoveInput::default();
                self.dragging = false;
            }
            _ => {}
        }

        let response = self.egui_state.on_window_event(&self.window, event);
        if response.repaint {
            self.window.request_redraw();
        }

        match *event {
            WindowEvent::CloseRequested => return Err(QuitEvent),
            WindowEvent::RedrawRequested => {
                if self.on_draw() {
                    return Err(QuitEvent);
                }
                return Ok(ControlFlow::Poll);
            }
            _ => {}
        }
        Ok(ControlFlow::Poll)
    }

    /// Returns true when a timed capture/quit should end the process.
    fn on_draw(&mut self) -> bool {
        let now = time::Instant::now();
        let dt = (now - self.last_redraw).as_secs_f32().min(0.05);
        self.last_redraw = now;
        if let Some(limit) = self.quit_after {
            if now.duration_since(self.started_at).as_secs_f32() >= limit {
                if let Ok(path) = std::env::var("GEOFRONT_SCREENSHOT") {
                    info!("Capture window ready for {path}");
                }
                return true;
            }
        }

        #[cfg(target_arch = "wasm32")]
        self.sample_web_input();

        // Asset cooking + physics step
        self.engine.update(dt);

        if self.impact_timer > 0.0 {
            self.impact_timer = (self.impact_timer - dt).max(0.0);
        }

        // Play enemy actions one-by-one so walks/punches read as a turn.
        if self.view_mode == render::ViewMode::Battle
            && self.mission.phase == combat::TurnPhase::Enemy
        {
            self.enemy_step_timer -= dt;
            if self.enemy_step_timer <= 0.0 {
                match self.mission.step_enemy_queue() {
                    Some(Action::Attack {
                        attacker_id,
                        target_id,
                        ..
                    }) => {
                        let from = self
                            .mission
                            .mech(attacker_id)
                            .map(|m| render::cell_to_world(m.position))
                            .unwrap_or(glam::Vec3::ZERO);
                        let to = self
                            .mission
                            .mech(target_id)
                            .map(|m| render::cell_to_world(m.position))
                            .unwrap_or(from + glam::Vec3::X);
                        self.arena
                            .play_attack(&mut self.engine, attacker_id, to - from);
                        self.arena.play_hit(&mut self.engine, target_id);
                        self.impact_timer = 0.85;
                        self.enemy_step_timer = 0.85;
                    }
                    Some(Action::Move { .. }) => {
                        self.enemy_step_timer = 0.38;
                    }
                    Some(_) => {
                        self.enemy_step_timer = 0.25;
                    }
                    None => {
                        self.mission.finish_enemy_turn();
                        self.enemy_step_timer = 0.0;
                    }
                }
            }
        }

        self.fly.apply(dt, self.keys);
        self.keys.look_dx = 0.0;
        self.keys.look_dy = 0.0;
        self.keys.wheel = 0.0;

        self.arena
            .tick(&mut self.engine, &self.mission, self.selected_player, dt);
        self.arena.sync_lights(&mut self.engine, &self.mission);

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_context = self.egui_state.egui_ctx().clone();

        let mut hud_action = None;
        let egui_output = egui_context.run_ui(raw_input, |egui_ctx| {
            #[allow(deprecated)]
            {
                egui::Panel::right("hud")
                    .default_size(360.0)
                    .frame(egui::Frame::side_top_panel(&egui_ctx.style()).inner_margin(10.0))
                    .show(egui_ctx, |ui| {
                        hud_action = ui::side_hud(
                            ui,
                            &self.mission,
                            self.view_mode,
                            &mut self.selected_player,
                            &mut self.selected_enemy,
                            &mut self.selected_limb,
                        );
                    });
            }
        });

        if let Some(action) = hud_action {
            self.handle_hud(action);
        }

        self.egui_state
            .handle_platform_output(&self.window, egui_output.platform_output);

        let primitives = self
            .egui_state
            .egui_ctx()
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);

        let camera = if self.fly.piloted || self.view_mode != render::ViewMode::Battle {
            self.fly.camera()
        } else {
            render::combat_camera(
                &self.mission,
                self.selected_player,
                self.selected_enemy,
                self.impact_timer,
            )
        };

        self.engine.render(
            &camera,
            &primitives,
            &egui_output.textures_delta,
            self.window.inner_size(),
            self.window.scale_factor() as f32,
        );

        #[cfg(target_arch = "wasm32")]
        self.publish_controls_probe();
        false
    }

    fn handle_hud(&mut self, action: ui::HudAction) {
        match action {
            ui::HudAction::Attack {
                attacker,
                target,
                limb,
            } => {
                let from = self
                    .mission
                    .mech(attacker)
                    .map(|m| render::cell_to_world(m.position));
                let to = self
                    .mission
                    .mech(target)
                    .map(|m| render::cell_to_world(m.position));
                match self.mission.apply_action(Action::Attack {
                    attacker_id: attacker,
                    target_id: target,
                    limb,
                }) {
                    Ok(()) => {
                        if let (Some(f), Some(t)) = (from, to) {
                            self.arena.play_attack(&mut self.engine, attacker, t - f);
                        }
                        self.arena.play_hit(&mut self.engine, target);
                        self.impact_timer = 1.15;
                    }
                    Err(e) => {
                        self.mission.log.push(format!("Attack failed: {e}"));
                    }
                }
            }
            ui::HudAction::Step(dir) => {
                if let Some(m) = self.mission.mech(self.selected_player) {
                    let to = m.position + dir.delta();
                    if let Err(e) = self.mission.apply_action(Action::Move {
                        unit_id: self.selected_player,
                        to,
                    }) {
                        self.mission.log.push(e);
                    }
                }
            }
            ui::HudAction::Rotate(sign) => {
                if let Some(m) = self.mission.mech(self.selected_player) {
                    let facing = if sign < 0 {
                        m.facing.rotate_ccw()
                    } else {
                        m.facing.rotate_cw()
                    };
                    if let Err(e) = self.mission.apply_action(Action::Rotate {
                        unit_id: self.selected_player,
                        facing,
                    }) {
                        self.mission.log.push(e);
                    }
                }
            }
            ui::HudAction::Wait => {
                if let Err(e) = self.mission.apply_action(Action::Wait {
                    unit_id: self.selected_player,
                }) {
                    self.mission.log.push(e);
                }
            }
            ui::HudAction::EndTurn => {
                if self.mission.phase == combat::TurnPhase::Player {
                    self.mission.begin_enemy_turn();
                    self.enemy_step_timer = 0.15;
                }
            }
            ui::HudAction::Reset => {
                self.mission = Mission::new_skirmish();
                self.selected_player = 0;
                self.selected_enemy = 10;
                self.selected_limb = LimbKind::Torso;
                self.impact_timer = 0.0;
                self.enemy_step_timer = 0.0;
                self.fly = render::FlyCam::for_mode(self.view_mode);
                if self.view_mode == render::ViewMode::Battle {
                    self.arena.clear(&mut self.engine);
                    self.arena =
                        render::Arena::spawn(&mut self.engine, self.view_mode, &self.mission);
                }
            }
            ui::HudAction::SetView(mode) => {
                if mode != self.view_mode {
                    self.arena.clear(&mut self.engine);
                    self.view_mode = mode;
                    self.impact_timer = 0.0;
                    self.enemy_step_timer = 0.0;
                    self.arena = render::Arena::spawn(&mut self.engine, mode, &self.mission);
                    self.fly = render::FlyCam::for_mode(mode);
                }
            }
        }
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
