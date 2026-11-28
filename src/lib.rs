//! Geofront – mecha tactical city defence and management.

mod base;
mod characters;
mod combat;
mod render;
mod ui;
mod units;
mod world;

use std::path::PathBuf;

use combat::{Action, ApplyResult, CombatFx, Mission};
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

    // Native shaders come from blade_render::shader_dir() (+ optional overlays).
    // WASM mounts a build.rs-merged tree into the VFS.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let stock = blade_render::shader_dir();
        if !stock.is_dir() {
            eprintln!(
                "Missing blade-render WGSL at {}\n\
                 Smoke test still works without shaders:\n\
                   cargo run -- --smoke",
                stock.display()
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

/// Embed `assets/` (models) plus build.rs-merged WGSL into Blade's VFS for WASM.
#[cfg(target_arch = "wasm32")]
fn mount_embedded_assets() {
    use include_dir::{include_dir, Dir};
    static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");
    // Stock blade-render/code + assets/shaders overlays (see build.rs).
    static MERGED_SHADERS: Dir = include_dir!("$OUT_DIR/merged-shaders");
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
    // Overlay/complete shaders at assets/shaders so Engine shader_path stays stable.
    let shader_root = root.join("shaders");
    for file in MERGED_SHADERS.files() {
        blade_engine::vfs::mount(shader_root.join(file.path()), file.contents().to_vec());
    }
    info!("Mounted embedded assets + merged shaders into VFS");
}

/// Native: `blade_render::shader_dir()` stock WGSL, with any `assets/shaders/*.wgsl`
/// overlays layered into `asset-cache/shaders` (Tokyo-3 `raster.wgsl` today).
#[cfg(not(target_arch = "wasm32"))]
fn native_shader_path(overlay_dir: &std::path::Path) -> String {
    use std::fs;
    let stock = blade_render::shader_dir();
    let has_overlay = fs::read_dir(overlay_dir)
        .ok()
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    == Some("wgsl")
            })
        })
        .unwrap_or(false);
    if !has_overlay {
        return stock.to_string_lossy().into_owned();
    }
    let dest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("asset-cache/shaders");
    fs::create_dir_all(&dest).expect("asset-cache/shaders");
    for entry in fs::read_dir(&stock).expect("blade shader_dir") {
        let entry = entry.expect("stock entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        let name = entry.file_name();
        let overlay = overlay_dir.join(&name);
        let src = if overlay.is_file() { overlay } else { path };
        fs::copy(&src, dest.join(&name)).unwrap_or_else(|e| {
            panic!("shader layer {} -> {}: {e}", src.display(), dest.display())
        });
    }
    dest.to_string_lossy().into_owned()
}

fn initial_view_mode() -> render::ViewMode {
    let from_env = std::env::var("GEOFRONT_VIEW").ok();
    #[cfg(target_arch = "wasm32")]
    let from_web = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| {
            let q = q.trim_start_matches('?');
            q.split('&').find_map(|pair| {
                let mut it = pair.splitn(2, '=');
                match (it.next(), it.next()) {
                    (Some("view"), Some(v)) => Some(v.to_string()),
                    _ => None,
                }
            })
        });
    #[cfg(not(target_arch = "wasm32"))]
    let from_web: Option<String> = None;
    match from_env.as_deref().or(from_web.as_deref()) {
        Some("battle") => render::ViewMode::Battle,
        Some("underground") => render::ViewMode::CityUnderground,
        Some("surface") => render::ViewMode::CitySurface,
        _ => render::ViewMode::CitySurface,
    }
}


/// Tokyo-3 dusk raster grade. Battle uses a darker canyon key so sodium
/// pools stay the bright points; surface overview keeps the existing dusk
/// look (already reads correctly from above).
fn dusk_raster_config(battle: bool) -> blade_render::RasterConfig {
    // Lower evening sun, more grazing for long canyon shadows.
    let light_dir = mint::Vector3 {
        x: 0.78,
        y: 0.22,
        z: 0.12,
    };
    let (light_color, ambient_color, shadow_strength) = if battle {
        // Dimmer warm key + tighter indigo fill. The close street camera
        // was blowing midtones toward noon under the surface-grade key.
        (
            mint::Vector3 {
                x: 2.25,
                y: 0.72,
                z: 0.20,
            },
            mint::Vector3 {
                x: 0.024,
                y: 0.022,
                z: 0.058,
            },
            0.90,
        )
    } else {
        (
            mint::Vector3 {
                x: 8.6,
                y: 3.15,
                z: 0.92,
            },
            mint::Vector3 {
                x: 0.055,
                y: 0.05,
                z: 0.12,
            },
            0.72,
        )
    };
    blade_render::RasterConfig {
        clear_color: blade_graphics::TextureColor::OpaqueBlack,
        light_dir,
        light_color,
        ambient_color,
        space_sky: false,
        // Tight ortho around the 8×8 plaza; dual receiver bias from
        // blade#390 keeps skinned Quaternius mechs lit under lavapipe.
        directional_shadows: Some(blade_render::DirectionalShadowConfig {
            resolution: 2048,
            distance: 48.0,
            depth: 180.0,
            strength: shadow_strength,
            normal_bias: 0.15,
            depth_bias: 0.07,
        }),
    }
}

struct QuitEvent;

/// Autoplay (lavapipe smoke) plays animations and turn pacing this much
/// faster than real time. Combat resolution is unchanged.
const AUTOPLAY_TIME_SCALE: f32 = 1.6;

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
    hud_width_points: f32,
    last_redraw: time::Instant,
    started_at: time::Instant,
    /// Exit after this many seconds (GEOFRONT_QUIT_AFTER / GEOFRONT_SCREENSHOT).
    quit_after: Option<f32>,
    /// When set, script a battle to victory for lavapipe playtests.
    autoplay: bool,
    /// Cooldown before the next autoplay action.
    autoplay_wait: f32,
    /// Seconds waited at start for asset cooks / first Idle bind.
    autoplay_warmup: f32,
    autoplay_done: bool,
    /// H / F1 toggles the camera/debug help block on the NERV HUD.
    show_help: bool,
}

// Winit cursor coordinates are physical pixels, while the HUD is laid out in egui points.
fn pointer_in_hud(x: f32, width_pixels: f32, hud_points: f32, pixels_per_point: f32) -> bool {
    x >= (width_pixels - hud_points * pixels_per_point).max(0.0)
}

#[cfg(test)]
#[test]
fn pointer_routing_accounts_for_hud_scale() {
    assert!(pointer_in_hud(500.0, 760.0, 360.0, 1.0));
    assert!(!pointer_in_hud(399.0, 760.0, 360.0, 1.0));
    assert!(pointer_in_hud(1000.0, 1520.0, 360.0, 2.0));
    assert!(!pointer_in_hud(799.0, 1520.0, 360.0, 2.0));
    assert!(pointer_in_hud(0.0, 300.0, 360.0, 1.0));
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
            use winit::dpi::LogicalSize;
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
                let size =
                    LogicalSize::new(width.max(1), height.max(1)).to_physical(window.scale_factor());
                canvas.set_width(size.width);
                canvas.set_height(size.height);
                let _ = window.request_inner_size(size);
            }
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| {
                    body.append_child(&web_sys::Element::from(canvas.clone()))
                        .ok()
                })
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
                shader_path: {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        native_shader_path(&assets.join("shaders"))
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        // Merged stock+overlay mounted here in mount_embedded_assets.
                        assets.join("shaders").to_string_lossy().into_owned()
                    }
                },
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

        // Tokyo-3 dusk grade: low warm key, tight indigo fill, contact
        // shadows on (Blade skinned-receiver bias). Battle gets a darker
        // canyon key so sodium pools stay the bright points.
        engine.set_raster_config(dusk_raster_config(
            initial_view_mode() == render::ViewMode::Battle,
        ));

        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::dark());
        let egui_state = egui_winit::State::new(
            egui_context,
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        let mission = Mission::new_skirmish();
        let view_mode = initial_view_mode();
        let autoplay = std::env::var_os("GEOFRONT_AUTOPLAY").is_some();
        // Denser Tokyo-3 scenes need ~150s under lavapipe on a fast runner and
        // >200s on the slower GitHub hosts (main 90d35b2 hit the 200s quit
        // before VICTORY). CI still passes QUIT_AFTER=120 until workflow OAuth can
        // bump smoke.yml; its step timeout is 8 min, so floor at 360s.
        let mut quit_after = std::env::var("GEOFRONT_QUIT_AFTER")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .or_else(|| {
                std::env::var("GEOFRONT_SCREENSHOT")
                    .ok()
                    .map(|_| 8.0_f32)
            });
        if autoplay {
            let floor = 360.0_f32;
            quit_after = Some(quit_after.map(|t| f32::max(t, floor)).unwrap_or(floor));
        }

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
            hud_width_points: 360.0,
            last_redraw: time::Instant::now(),
            started_at: time::Instant::now(),
            quit_after,
            autoplay,
            autoplay_wait: 0.0,
            autoplay_warmup: 2.5,
            autoplay_done: false,
            show_help: false,
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
            KeyH | F1 if down => self.show_help = !self.show_help,
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
        let mode = match self.view_mode {
            render::ViewMode::Battle => "battle",
            render::ViewMode::CitySurface => "surface",
            render::ViewMode::CityUnderground => "underground",
        };
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("__gfMode"),
            &JsValue::from_str(mode),
        );
    }

    fn cursor_over_hud(&self) -> bool {
        let scale = egui_winit::pixels_per_point(self.egui_state.egui_ctx(), &self.window);
        self.last_cursor.is_some_and(|(x, _)| {
            pointer_in_hud(
                x,
                self.window.inner_size().width as f32,
                self.hud_width_points,
                scale,
            )
        })
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
                self.dragging =
                    *state == winit::event::ElementState::Pressed && !self.cursor_over_hud();
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
                if self.last_cursor.is_some() && !self.cursor_over_hud() {
                    let wheel = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y * 24.0,
                        winit::event::MouseScrollDelta::PixelDelta(p) => {
                            if cfg!(target_arch = "wasm32") {
                                p.y as f32 / self.window.scale_factor() as f32
                            } else {
                                p.y as f32
                            }
                        }
                    };
                    // Web DOM and winit use opposite wheel signs. Retain the old DOM direction.
                    self.keys.wheel += if cfg!(target_arch = "wasm32") { -wheel } else { wheel };
                }
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                use winit::platform::web::WindowExtWebSys as _;
                if let Some(canvas) = self.window.canvas() {
                    if canvas.width() != size.width {
                        canvas.set_width(size.width);
                    }
                    if canvas.height() != size.height {
                        canvas.set_height(size.height);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
                self.dragging = false;
            }
            WindowEvent::Focused(false) => {
                self.last_cursor = None;
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
        let raw_dt = (now - self.last_redraw).as_secs_f32();
        // Interactive play clamps dt so a hitch never skips an animation. The
        // lavapipe smoke runs at a few fps, where that clamp made game time
        // crawl behind wall time; autoplay keeps up with the wall clock and
        // plays at 1.6x (presentation pacing only, same combat resolution).
        let dt = if self.autoplay {
            raw_dt.min(0.25) * AUTOPLAY_TIME_SCALE
        } else {
            raw_dt.min(0.05)
        };
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
                        let profile = self
                            .mission
                            .mech(attacker_id)
                            .map(render::AttackAnim::for_mech)
                            .unwrap_or(render::AttackAnim {
                                telegraph: 0.22,
                                strike: 0.55,
                                lunge: 0.55,
                                anim_speed: 1.15,
                                hit_duration: 0.48,
                            });
                        let absorbed = self.mission.pending_fx.iter().any(|fx| {
                            matches!(fx, CombatFx::AtFieldAbsorb { target_id: t } if *t == target_id)
                        });
                        let shattered = self.mission.pending_fx.iter().any(|fx| {
                            matches!(fx, CombatFx::AtFieldBreak { target_id: t } if *t == target_id)
                        });
                        let total = if absorbed {
                            self.arena.play_attack_ex(
                                &mut self.engine,
                                attacker_id,
                                to - from,
                                profile,
                                Some(target_id),
                                true,
                                shattered,
                            )
                        } else {
                            self.arena.play_attack(
                                &mut self.engine,
                                attacker_id,
                                to - from,
                                profile,
                                Some(target_id),
                            )
                        };
                        self.drain_combat_fx();
                        self.impact_timer = total.max(0.85);
                        self.enemy_step_timer = (total + 0.12).max(0.5);
                    }
                    Some(Action::Move { .. }) => {
                        self.enemy_step_timer = 0.38;
                    }
                    Some(_) => {
                        self.enemy_step_timer = 0.25;
                    }
                    None => {
                        self.mission.finish_enemy_turn();
                        self.drain_combat_fx();
                        self.enemy_step_timer = 0.0;
                    }
                }
            }
        }

        if self.autoplay {
            self.tick_autoplay(dt);
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
            let panel = egui::SidePanel::right("hud")
                .exact_width(360.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(8, 10, 14))
                        .inner_margin(12.0)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(232, 120, 32),
                        )),
                )
                .show(egui_ctx, |ui| {
                    // Pilot alerts and help can push combat orders below a short viewport.
                    hud_action = egui::ScrollArea::vertical()
                        .id_salt("tactical-hud")
                        .show(ui, |ui| {
                            ui::side_hud(
                                ui,
                                &self.mission,
                                self.view_mode,
                                &mut self.selected_player,
                                &mut self.selected_enemy,
                                &mut self.selected_limb,
                                &mut self.show_help,
                            )
                        })
                        .inner;
                });
            self.hud_width_points =
                (egui_ctx.viewport_rect().right() - panel.response.rect.left()).max(0.0);
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
                self.arena.camera_punch(),
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

    /// Scripted skirmish for lavapipe: warm up, attack, end turn, until VICTORY/DEFEAT.
    fn tick_autoplay(&mut self, dt: f32) {
        if self.view_mode != render::ViewMode::Battle {
            self.handle_hud(ui::HudAction::SetView(render::ViewMode::Battle));
            return;
        }
        if self.autoplay_warmup > 0.0 {
            self.autoplay_warmup -= dt;
            return;
        }
        if self.mission.is_won() || self.mission.is_lost() {
            if !self.autoplay_done {
                self.autoplay_done = true;
                if self.mission.is_won() {
                    info!("AUTOPLAY: VICTORY on turn {}", self.mission.turn);
                } else {
                    info!("AUTOPLAY: DEFEAT on turn {}", self.mission.turn);
                }
                // Quit shortly after the verdict; the autoplay floor is only a cap.
                let soon = self.started_at.elapsed().as_secs_f32() + 2.0;
                self.quit_after = Some(self.quit_after.map_or(soon, |t| t.min(soon)));
            }
            return;
        }
        if self.mission.phase != combat::TurnPhase::Player {
            return;
        }
        if self.autoplay_wait > 0.0 {
            self.autoplay_wait -= dt;
            return;
        }

        // Prefer attacking with the selected living player; otherwise any living player.
        let attacker = self
            .mission
            .mech(self.selected_player)
            .filter(|m| !m.destroyed && matches!(m.team, units::Team::Player) && m.can_act())
            .map(|m| m.id)
            .or_else(|| {
                self.mission
                    .living_mechs(units::Team::Player)
                    .find(|m| m.can_act())
                    .map(|m| m.id)
            });

        if let Some(aid) = attacker {
            self.selected_player = aid;
            let from = self.mission.mech(aid).map(|m| m.position).unwrap();
            let target = self
                .mission
                .living_mechs(units::Team::Enemy)
                .min_by_key(|e| combat::Grid::manhattan(from, e.position))
                .map(|e| e.id);
            if let Some(tid) = target {
                self.selected_enemy = tid;
                let in_range = {
                    let a = self.mission.mech(aid).unwrap();
                    let e = self.mission.mech(tid).unwrap();
                    combat::Grid::manhattan(a.position, e.position) <= a.attack_range()
                };
                if in_range {
                    self.handle_hud(ui::HudAction::Attack {
                        attacker: aid,
                        target: tid,
                        limb: LimbKind::Torso,
                    });
                    self.autoplay_wait = 1.35;
                    return;
                }
            }
            // No shot available for this unit — mark acted via Wait.
            self.handle_hud(ui::HudAction::Wait);
            self.autoplay_wait = 0.35;
            return;
        }

        // Everyone acted (or cannot) — end the player phase.
        self.handle_hud(ui::HudAction::EndTurn);
        self.autoplay_wait = 0.4;
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
                    Ok(ApplyResult::Done) => {
                        let profile = self
                            .mission
                            .mech(attacker)
                            .map(render::AttackAnim::for_mech)
                            .unwrap_or(render::AttackAnim {
                                telegraph: 0.22,
                                strike: 0.55,
                                lunge: 0.55,
                                anim_speed: 1.15,
                                hit_duration: 0.48,
                            });
                        let absorbed = self.mission.pending_fx.iter().any(|fx| {
                            matches!(fx, CombatFx::AtFieldAbsorb { target_id: t } if *t == target)
                        });
                        let shattered = self.mission.pending_fx.iter().any(|fx| {
                            matches!(fx, CombatFx::AtFieldBreak { target_id: t } if *t == target)
                        });
                        if let (Some(f), Some(t)) = (from, to) {
                            let total = if absorbed {
                                self.arena.play_attack_ex(
                                    &mut self.engine,
                                    attacker,
                                    t - f,
                                    profile,
                                    Some(target),
                                    true,
                                    shattered,
                                )
                            } else {
                                self.arena.play_attack(
                                    &mut self.engine,
                                    attacker,
                                    t - f,
                                    profile,
                                    Some(target),
                                )
                            };
                            self.impact_timer = total.max(1.0);
                        } else {
                            self.impact_timer = profile.total().max(1.0);
                        }
                        self.drain_combat_fx();
                    }
                    Ok(ApplyResult::Skipped) => {
                        // Pilot locked up — log already records the refuse.
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
                    self.engine
                        .set_raster_config(dusk_raster_config(mode == render::ViewMode::Battle));
                    self.arena = render::Arena::spawn(&mut self.engine, mode, &self.mission);
                    self.fly = render::FlyCam::for_mode(mode);
                }
            }
        }
    }

    fn drain_combat_fx(&mut self) {
        for fx in self.mission.take_fx() {
            match fx {
                CombatFx::CoreFlash { target_id } => {
                    self.arena.spawn_core_flash(target_id);
                }
                CombatFx::CoreCrack { target_id } => {
                    self.arena.spawn_core_crack(target_id);
                }
                _ => {}
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
