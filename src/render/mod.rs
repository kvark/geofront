//! Battle stage + city overview: Kenney surface / Space Kit underground,
//! Quaternius GLB mechs with lerp/bob/lunge, Eva-style + impact cameras.

use std::collections::HashMap;

use glam::{IVec2, Mat4, Quat, Vec3};
use log::info;

use crate::combat::Mission;
use crate::units::{Facing, Mech, Team};

/// World units per tactical grid cell (matches Kenney road tile width).
pub const CELL: f32 = 2.0;

/// Map grid cell → world position (Y-up). Feet on the road / floor surface.
pub fn cell_to_world(pos: IVec2) -> Vec3 {
    Vec3::new(pos.x as f32 * CELL, 0.0, pos.y as f32 * CELL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Close-up tactical combat in the surface city block.
    Battle,
    /// Elevated overview of the surface city.
    CitySurface,
    /// Elevated overview of the underground Geofront facility.
    CityUnderground,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Battle => "Battle",
            ViewMode::CitySurface => "City — Surface",
            ViewMode::CityUnderground => "City — Underground",
        }
    }
}

struct MechVisual {
    handle: blade_engine::ObjectHandle,
    pos: Vec3,
    yaw: f32,
    bob: f32,
    punch: f32,
    punch_dir: Vec3,
}

pub struct Arena {
    pub kind: ViewMode,
    visuals: HashMap<u32, MechVisual>,
    stage_handles: Vec<blade_engine::ObjectHandle>,
}

impl Arena {
    pub fn spawn(engine: &mut blade_engine::Engine, mode: ViewMode, mission: &Mission) -> Self {
        info!("Spawning arena: {:?}", mode);
        let mut stage_handles = Vec::new();
        let mut visuals = HashMap::new();

        match mode {
            ViewMode::Battle | ViewMode::CitySurface => {
                stage_handles.extend(spawn_surface_roads(engine, 8, 8));
                stage_handles.extend(spawn_surface_buildings(
                    engine,
                    8,
                    8,
                    mode == ViewMode::CitySurface,
                ));
                if mode == ViewMode::Battle {
                    for mech in &mission.mechs {
                        let vis = spawn_mech(engine, mech);
                        visuals.insert(mech.id, vis);
                    }
                }
            }
            ViewMode::CityUnderground => {
                stage_handles.extend(spawn_underground_facility(engine));
            }
        }

        Self {
            kind: mode,
            visuals,
            stage_handles,
        }
    }

    pub fn clear(&mut self, engine: &mut blade_engine::Engine) {
        for vis in self.visuals.values() {
            engine.remove_object(vis.handle);
        }
        self.visuals.clear();
        for h in self.stage_handles.drain(..) {
            engine.remove_object(h);
        }
    }

    pub fn play_attack(&mut self, id: u32, toward: Vec3) {
        if let Some(v) = self.visuals.get_mut(&id) {
            v.punch = 0.38;
            let dir = Vec3::new(toward.x, 0.0, toward.z);
            v.punch_dir = if dir.length_squared() > 1e-4 {
                dir.normalize()
            } else {
                Vec3::Z
            };
        }
    }

    /// Lerp mechs toward grid cells, bob while walking, lunge on attack.
    pub fn tick(
        &mut self,
        engine: &mut blade_engine::Engine,
        mission: &Mission,
        selected: u32,
        dt: f32,
    ) {
        if self.kind != ViewMode::Battle {
            return;
        }
        for mech in &mission.mechs {
            let Some(vis) = self.visuals.get_mut(&mech.id) else {
                continue;
            };
            let target = if mech.destroyed {
                cell_to_world(mech.position) + Vec3::Y * -3.2
            } else {
                cell_to_world(mech.position)
            };
            let to_target = target - vis.pos;
            let dist = to_target.length();
            let speed = if mech.destroyed { 5.0 } else { 9.0 };
            if dist > 0.02 {
                vis.pos += to_target.normalize() * (speed * dt).min(dist);
                vis.bob += dt * 10.0;
            } else {
                vis.pos = target;
                vis.bob *= (1.0 - dt * 8.0).max(0.0);
            }

            let want_yaw = mech.facing.yaw();
            let mut dy = want_yaw - vis.yaw;
            while dy > std::f32::consts::PI {
                dy -= std::f32::consts::TAU;
            }
            while dy < -std::f32::consts::PI {
                dy += std::f32::consts::TAU;
            }
            vis.yaw += dy * (1.0 - (-12.0 * dt).exp());

            if vis.punch > 0.0 {
                vis.punch = (vis.punch - dt).max(0.0);
            }

            let bob_y = if mech.destroyed {
                0.0
            } else {
                vis.bob.sin() * 0.08 * (dist * 2.0).min(1.0)
            };
            let lunge_k = if vis.punch > 0.0 {
                // Ease out punch: forward then back.
                let t = 1.0 - vis.punch / 0.38;
                let s = if t < 0.45 {
                    (t / 0.45) * 0.55
                } else {
                    (1.0 - (t - 0.45) / 0.55) * 0.55
                };
                s
            } else {
                0.0
            };
            let pos = vis.pos + Vec3::Y * bob_y + vis.punch_dir * lunge_k;
            let q = Quat::from_rotation_y(vis.yaw);
            engine.teleport_object(
                vis.handle,
                blade_engine::Transform {
                    position: pos.into(),
                    orientation: mint::Quaternion {
                        s: q.w,
                        v: [q.x, q.y, q.z].into(),
                    },
                },
            );

            let pulse = if mech.id == selected && !mech.destroyed {
                1.0 + 0.08 * (vis.bob * 0.35).sin().abs()
            } else {
                1.0
            };
            let tint = match mech.team {
                Team::Player => [0.85 * pulse, 1.05 * pulse, 1.35 * pulse, 1.0],
                Team::Enemy => [1.35 * pulse, 0.55, 0.45, 1.0],
            };
            engine.set_color_tint(vis.handle, tint);
        }

        draw_tactical_overlay(engine, mission, selected);
    }
}

fn draw_tactical_overlay(engine: &mut blade_engine::Engine, mission: &Mission, selected: u32) {
    let Some(mech) = mission.mech(selected) else {
        return;
    };
    if mech.destroyed || !matches!(mission.phase, crate::combat::TurnPhase::Player) {
        return;
    }

    let mut lines = Vec::new();
    let y = 0.04;
    let half = CELL * 0.46;

    let push_quad = |lines: &mut Vec<blade_render::DebugLine>, c: Vec3, color: u32| {
        let pts = [
            [c.x - half, y, c.z - half],
            [c.x + half, y, c.z - half],
            [c.x + half, y, c.z + half],
            [c.x - half, y, c.z + half],
        ];
        for i in 0..4 {
            lines.push(blade_render::DebugLine {
                a: blade_render::DebugPoint {
                    pos: pts[i],
                    color,
                },
                b: blade_render::DebugPoint {
                    pos: pts[(i + 1) % 4],
                    color,
                },
            });
        }
    };

    // Movement tiles
    if mech.can_move() {
        for dir in [Facing::North, Facing::East, Facing::South, Facing::West] {
            let to = mech.position + dir.delta();
            if mission.grid.in_bounds(to) && !mission.occupied(to, Some(mech.id)) {
                push_quad(&mut lines, cell_to_world(to), 0x88_FF_CC_44);
            }
        }
    }

    // Attack range ring on enemies
    let range = mech.attack_range();
    for other in mission.living_mechs(Team::Enemy) {
        let color = if crate::combat::Grid::manhattan(mech.position, other.position) <= range {
            0xFF_66_55_AA
        } else {
            0x88_44_33_55
        };
        push_quad(&mut lines, cell_to_world(other.position), color);
    }

    // Facing arrow
    let origin = cell_to_world(mech.position) + Vec3::Y * 0.15;
    let fwd = Vec3::new(mech.facing.delta().x as f32, 0.0, mech.facing.delta().y as f32);
    let tip = origin + fwd * 1.15;
    lines.push(blade_render::DebugLine {
        a: blade_render::DebugPoint {
            pos: origin.into(),
            color: 0xFF_EE_88_FF,
        },
        b: blade_render::DebugPoint {
            pos: tip.into(),
            color: 0xFF_EE_88_FF,
        },
    });

    engine.add_debug_lines(&lines);
}

fn quat_identity() -> mint::Quaternion<f32> {
    mint::Quaternion {
        s: 1.0,
        v: [0.0, 0.0, 0.0].into(),
    }
}

fn add_static(
    engine: &mut blade_engine::Engine,
    name: impl Into<String>,
    model: &str,
    pos: [f32; 3],
    scale: f32,
) -> blade_engine::ObjectHandle {
    engine.add_object(
        &blade_engine::config::Object {
            name: name.into(),
            visuals: vec![blade_engine::config::Visual {
                model: model.into(),
                scale,
                pos: [0.0; 3].into(),
                rot: [0.0; 3].into(),
                front_face: blade_engine::config::FrontFace::default(),
            }],
            colliders: vec![],
            additional_mass: None,
        },
        blade_engine::Transform {
            position: pos.into(),
            orientation: quat_identity(),
        },
        blade_engine::DynamicInput::Empty,
    )
}

fn spawn_surface_roads(
    engine: &mut blade_engine::Engine,
    width: i32,
    height: i32,
) -> Vec<blade_engine::ObjectHandle> {
    let mut handles = Vec::new();
    for z in 0..height {
        for x in 0..width {
            let path = if (x + z) % 5 == 0 {
                "models/roads/road-crossroad.glb"
            } else if x % 2 == 0 {
                "models/roads/road-straight.glb"
            } else {
                "models/roads/road-crossing.glb"
            };
            let pos = cell_to_world(IVec2::new(x, z));
            handles.push(add_static(
                engine,
                format!("road-{x}-{z}"),
                path,
                [pos.x, -1.0, pos.z],
                1.0,
            ));
        }
    }
    handles
}

fn spawn_surface_buildings(
    engine: &mut blade_engine::Engine,
    width: i32,
    height: i32,
    dense: bool,
) -> Vec<blade_engine::ObjectHandle> {
    let commercial = [
        "models/commercial/building-skyscraper-a.glb",
        "models/commercial/building-skyscraper-c.glb",
        "models/commercial/building-skyscraper-e.glb",
        "models/commercial/building-a.glb",
        "models/commercial/building-c.glb",
        "models/commercial/building-e.glb",
        "models/commercial/building-i.glb",
        "models/commercial/building-l.glb",
    ];
    let industrial = [
        "models/industrial/building-a.glb",
        "models/industrial/building-d.glb",
        "models/industrial/building-h.glb",
        "models/industrial/chimney-large.glb",
        "models/industrial/detail-tank.glb",
    ];

    let mut handles = Vec::new();
    let mut i = 0usize;
    let margin = if dense { 3 } else { 2 };

    for x in -margin..width + margin {
        for z in -margin..height + margin {
            let outer = x < 0 || z < 0 || x >= width || z >= height;
            let edge = x == 0 || z == 0 || x == width - 1 || z == height - 1;
            if !outer && !edge {
                continue;
            }
            if !outer && !dense && (x + z) % 3 != 0 {
                continue;
            }
            if dense && !outer && (x + z) % 2 != 0 {
                continue;
            }
            let path = if outer && (i % 3 == 0) {
                industrial[i % industrial.len()]
            } else {
                commercial[i % commercial.len()]
            };
            let scale = if path.contains("skyscraper") {
                if dense { 1.25 } else { 1.15 }
            } else {
                1.0
            };
            let pos = cell_to_world(IVec2::new(x, z));
            handles.push(add_static(
                engine,
                format!("bld-{i}"),
                path,
                [pos.x, 0.0, pos.z],
                scale,
            ));
            i += 1;
        }
    }
    handles
}

/// Modular Geofront: pieces abut on edges, never share floor area (avoids Z-fight).
///
/// Kenney Space Kit extents (XZ):
/// - room-large 20×20, room-small 12×12, corridor 4×4, corridor-wide 8×8,
///   intersection 4×4, gate 4.2×1.4
fn spawn_underground_facility(engine: &mut blade_engine::Engine) -> Vec<blade_engine::ObjectHandle> {
    let mut handles = Vec::new();
    // Hangar at origin occupies x,z ∈ [-10, 10]
    let placements: &[(&str, [f32; 3], f32)] = &[
        ("models/space/room-large.glb", [0.0, 0.0, 0.0], 1.0),
        // North spine: hangar z=10 → wide corridor 8 tall, center z=14
        ("models/space/corridor-wide.glb", [0.0, 0.0, 14.0], 1.0),
        // z=18 → intersection 4, center z=20
        ("models/space/corridor-intersection.glb", [0.0, 0.0, 20.0], 1.0),
        // z=22 → command room-small 12, center z=28
        ("models/space/room-small.glb", [0.0, 0.0, 28.0], 1.0),
        // East spur: hangar x=10 → corridor 4, center x=12
        ("models/space/corridor.glb", [12.0, 0.0, 0.0], 1.0),
        ("models/space/room-small.glb", [20.0, 0.0, 0.0], 1.0),
        // West spur
        ("models/space/corridor.glb", [-12.0, 0.0, 0.0], 1.0),
        ("models/space/room-small.glb", [-20.0, 0.0, 0.0], 1.0),
        // South airlock: hangar z=-10, gate depth 1.4, center z=-10.7
        ("models/space/gate.glb", [0.0, 0.0, -10.7], 1.0),
        ("models/space/gate-door.glb", [0.05, 0.0, -11.35], 1.0),
        // Side stair well east of north hall, outside hangar/room footprints
        ("models/space/stairs.glb", [16.0, 0.0, 14.0], 1.0),
        ("models/space/corridor-corner.glb", [12.0, 0.0, 14.0], 1.0),
    ];
    for (i, (path, pos, scale)) in placements.iter().enumerate() {
        handles.push(add_static(engine, format!("ug-{i}"), path, *pos, *scale));
    }
    handles
}

fn mech_glb_path(mech: &Mech) -> &'static str {
    match (mech.team, mech.id % 2) {
        (Team::Player, 0) => "models/mechs/Stan.glb",
        (Team::Player, _) => "models/mechs/Mike.glb",
        (Team::Enemy, 0) => "models/mechs/George.glb",
        (Team::Enemy, _) => "models/mechs/Leela.glb",
    }
}

fn spawn_mech(engine: &mut blade_engine::Engine, mech: &Mech) -> MechVisual {
    let color = match mech.team {
        Team::Player => [0.50, 0.68, 0.95, 1.0],
        Team::Enemy => [0.95, 0.38, 0.32, 1.0],
    };
    // Quaternius GLBs are in assets/models/mechs (skinned). Blade's raster path
    // currently draws them unlit/black, so battle uses a readable silhouette.
    // Swap to mech_glb_path() once skin/PBR lighting lands.
    let _ = mech_glb_path(mech);
    let model = engine.create_model(
        &format!("mech-{}", mech.id),
        vec![
            box_geo("leg-l", color, [-0.28, 0.55, 0.0], [0.16, 0.55, 0.18]),
            box_geo("leg-r", color, [0.28, 0.55, 0.0], [0.16, 0.55, 0.18]),
            box_geo("torso", color, [0.0, 1.45, 0.0], [0.42, 0.55, 0.28]),
            box_geo("head", color, [0.0, 2.25, 0.05], [0.22, 0.22, 0.22]),
            box_geo("arm-l", color, [-0.58, 1.55, 0.0], [0.12, 0.42, 0.12]),
            box_geo("arm-r", color, [0.58, 1.55, 0.0], [0.12, 0.42, 0.12]),
            box_geo("pad-l", color, [-0.48, 1.85, 0.0], [0.18, 0.12, 0.18]),
            box_geo("pad-r", color, [0.48, 1.85, 0.0], [0.18, 0.12, 0.18]),
        ],
    );
    let pos = cell_to_world(mech.position);
    let yaw = mech.facing.yaw();
    let q = Quat::from_rotation_y(yaw);
    let handle = engine.add_object_with_model(
        &mech.name,
        model,
        blade_engine::Transform {
            position: pos.into(),
            orientation: mint::Quaternion {
                s: q.w,
                v: [q.x, q.y, q.z].into(),
            },
        },
        blade_engine::DynamicInput::SetPosition,
    );
    MechVisual {
        handle,
        pos,
        yaw,
        bob: 0.0,
        punch: 0.0,
        punch_dir: Vec3::Z,
    }
}

fn box_geo(
    name: &str,
    color: [f32; 4],
    center: [f32; 3],
    half: [f32; 3],
) -> blade_render::ProceduralGeometry {
    let [cx, cy, cz] = center;
    let [hx, hy, hz] = half;
    let corners = [
        [cx - hx, cy - hy, cz - hz],
        [cx + hx, cy - hy, cz - hz],
        [cx + hx, cy + hy, cz - hz],
        [cx - hx, cy + hy, cz - hz],
        [cx - hx, cy - hy, cz + hz],
        [cx + hx, cy - hy, cz + hz],
        [cx + hx, cy + hy, cz + hz],
        [cx - hx, cy + hy, cz + hz],
    ];
    let faces: [([usize; 4], [f32; 3]); 6] = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ([5, 4, 7, 6], [0.0, 0.0, 1.0]),
        ([4, 0, 3, 7], [-1.0, 0.0, 0.0]),
        ([1, 5, 6, 2], [1.0, 0.0, 0.0]),
        ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
        ([4, 5, 1, 0], [0.0, -1.0, 0.0]),
    ];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (quad, n) in faces {
        let base = vertices.len() as u32;
        for &ci in &quad {
            let p = corners[ci];
            vertices.push(blade_render::Vertex {
                position: p,
                bitangent_sign: 1.0,
                tex_coords: [0.0, 0.0],
                normal: encode_normal(n),
                tangent: encode_normal([1.0, 0.0, 0.0]),
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    blade_render::ProceduralGeometry {
        name: name.into(),
        vertices,
        indices,
        base_color_factor: color,
        metalness: 0.40,
        roughness: 0.42,
        emissive_factor: [0.04, 0.05, 0.06],
    }
}

fn encode_normal(v: [f32; 3]) -> u32 {
    let pack = |f: f32| -> u32 {
        let c = (f.clamp(-1.0, 1.0) * 127.0 + 0.5) as i8 as u8 as u32;
        c
    };
    pack(v[0]) | (pack(v[1]) << 8) | (pack(v[2]) << 16)
}

fn frame_camera(eye: Vec3, focus: Vec3, fov_y: f32) -> blade_engine::FrameCamera {
    let view = Mat4::look_at_rh(eye, focus, Vec3::Y);
    let world = view.inverse();
    let (_, rot, trans) = world.to_scale_rotation_translation();
    let q: Quat = rot;
    blade_engine::FrameCamera {
        transform: blade_engine::Transform {
            position: trans.into(),
            orientation: mint::Quaternion {
                s: q.w,
                v: [q.x, q.y, q.z].into(),
            },
        },
        fov_y,
    }
}

/// Eva-style low hero camera. When `impact_t` is > 0 (seconds remaining),
/// pull into a tighter, more dramatic impact framing.
pub fn combat_camera(
    mission: &Mission,
    selected_player: u32,
    selected_enemy: u32,
    impact_t: f32,
) -> blade_engine::FrameCamera {
    let player = mission
        .mechs
        .iter()
        .find(|m| m.id == selected_player && !m.destroyed)
        .or_else(|| {
            mission
                .mechs
                .iter()
                .find(|m| m.team == Team::Player && !m.destroyed)
        });
    let enemy = mission
        .mechs
        .iter()
        .find(|m| m.id == selected_enemy && !m.destroyed)
        .or_else(|| {
            mission
                .mechs
                .iter()
                .find(|m| m.team == Team::Enemy && !m.destroyed)
        });

    let p = player
        .map(|m| cell_to_world(m.position))
        .unwrap_or(Vec3::new(2.0, 0.0, 6.0));
    let e = enemy
        .map(|m| cell_to_world(m.position))
        .unwrap_or(Vec3::new(12.0, 0.0, 8.0));

    let focus = (p + e) * 0.5 + Vec3::Y * 1.5;
    let along = (e - p).normalize_or_zero();
    let side = along.cross(Vec3::Y).normalize_or_zero();

    let k = (impact_t / 1.15).clamp(0.0, 1.0);
    let k = k * k;

    let dist = 7.8 - k * 3.2;
    let side_off = 4.6 - k * 1.8;
    let height = 3.4 - k * 1.1;
    let fov = 0.72 + k * 0.18;

    let eye = focus - along * dist + side * side_off + Vec3::Y * height;
    frame_camera(eye, focus + Vec3::Y * (k * 0.4), fov)
}

/// Elevated city overview cameras — also used to seed FlyCam.
pub fn city_camera(mode: ViewMode) -> blade_engine::FrameCamera {
    match mode {
        ViewMode::CitySurface => {
            let eye = Vec3::new(-6.0, 18.0, -4.0);
            let focus = Vec3::new(7.0, 2.0, 7.0);
            frame_camera(eye, focus, 0.70)
        }
        ViewMode::CityUnderground => {
            let eye = Vec3::new(-18.0, 16.0, -16.0);
            let focus = Vec3::new(0.0, 1.5, 8.0);
            frame_camera(eye, focus, 0.78)
        }
        ViewMode::Battle => FlyCam::from_eye_focus(Vec3::new(-2.0, 3.4, 9.0), Vec3::new(7.0, 1.5, 7.0))
            .camera(),
    }
}

/// Held WASD / look / wheel for one frame. Gameplay reads this, not raw events.
#[derive(Clone, Copy, Default)]
pub struct MoveInput {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub q: bool,
    pub e: bool,
    pub shift: bool,
    pub look_dx: f32,
    pub look_dy: f32,
    pub wheel: f32,
}

impl MoveInput {
    pub fn has_look(self) -> bool {
        self.look_dx != 0.0 || self.look_dy != 0.0
    }

    pub fn has_move(self) -> bool {
        self.w || self.a || self.s || self.d || self.q || self.e || self.wheel != 0.0
    }
}

/// FPS fly camera. Yaw 0 looks along −Z; +yaw is CCW about +Y.
/// W/S along forward_xz, D/A along right_xz (A = screen-left, D = screen-right).
pub struct FlyCam {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub piloted: bool,
}

impl FlyCam {
    pub fn for_mode(mode: ViewMode) -> Self {
        let (eye, focus) = match mode {
            ViewMode::CitySurface | ViewMode::CityUnderground => {
                // Keep FlyCam seed in lockstep with city_camera.
                let _ = city_camera(mode);
                match mode {
                    ViewMode::CitySurface => (Vec3::new(-6.0, 18.0, -4.0), Vec3::new(7.0, 2.0, 7.0)),
                    _ => (Vec3::new(-18.0, 16.0, -16.0), Vec3::new(0.0, 1.5, 8.0)),
                }
            }
            ViewMode::Battle => (Vec3::new(-2.0, 3.4, 9.0), Vec3::new(7.0, 1.5, 7.0)),
        };
        let mut cam = Self::from_eye_focus(eye, focus);
        cam.piloted = mode != ViewMode::Battle;
        cam
    }

    pub fn from_eye_focus(eye: Vec3, focus: Vec3) -> Self {
        let dir = (focus - eye).normalize_or_zero();
        let pitch = dir.y.clamp(-0.99, 0.99).asin();
        let yaw = (-dir.x).atan2(-dir.z);
        Self {
            pos: eye,
            yaw,
            pitch,
            speed: 0.0,
            piloted: false,
        }
    }

    /// Ground-plane forward. yaw=0 → −Z.
    pub fn forward_xz(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    /// Ground-plane right. yaw=0 → +X.
    pub fn right_xz(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    pub fn look_dir(&self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(
            -self.yaw.sin() * cp,
            self.pitch.sin(),
            -self.yaw.cos() * cp,
        )
    }

    pub fn apply(&mut self, dt: f32, input: MoveInput) {
        if input.has_look() || input.has_move() {
            self.piloted = true;
        }
        self.yaw -= input.look_dx * 0.005;
        self.pitch = (self.pitch - input.look_dy * 0.004).clamp(-1.2, 1.2);

        let sprint = if input.shift { 2.4 } else { 1.0 };
        let speed = 14.0 * sprint;
        let f = self.forward_xz();
        let r = self.right_xz();
        let mut wish = Vec3::ZERO;
        if input.w {
            wish += f;
        }
        if input.s {
            wish -= f;
        }
        if input.d {
            wish += r;
        }
        if input.a {
            wish -= r;
        }
        if input.q {
            wish += Vec3::Y;
        }
        if input.e {
            wish -= Vec3::Y;
        }
        if wish.length_squared() > 1e-6 {
            wish = wish.normalize();
        }
        self.pos += wish * speed * dt;
        self.pos += self.look_dir() * (-input.wheel * 0.025);
        self.speed = wish.length() * speed;
    }

    pub fn camera(&self) -> blade_engine::FrameCamera {
        frame_camera(self.pos, self.pos + self.look_dir(), 0.85)
    }
}
