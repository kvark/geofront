//! Battle stage + city overview: Kenney surface / Space Kit underground,
//! multi-part procedural mechs, Eva-style + impact cameras.

use std::collections::HashMap;

use glam::{IVec2, Mat4, Quat, Vec3};
use log::info;

use crate::combat::Mission;
use crate::units::{Mech, Team};

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

pub struct Arena {
    pub kind: ViewMode,
    /// Mech id → engine object handle (battle only).
    pub mech_handles: HashMap<u32, blade_engine::ObjectHandle>,
    /// All static stage objects (roads, buildings, corridors…) for cleanup.
    stage_handles: Vec<blade_engine::ObjectHandle>,
}

impl Arena {
    pub fn spawn(engine: &mut blade_engine::Engine, mode: ViewMode, mission: &Mission) -> Self {
        info!("Spawning arena: {:?}", mode);
        let mut stage_handles = Vec::new();
        let mut mech_handles = HashMap::new();

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
                        let handle = spawn_mech_silhouette(engine, mech);
                        mech_handles.insert(mech.id, handle);
                    }
                }
            }
            ViewMode::CityUnderground => {
                stage_handles.extend(spawn_underground_facility(engine));
            }
        }

        Self {
            kind: mode,
            mech_handles,
            stage_handles,
        }
    }

    /// Remove every object this arena owns so a new layout can be spawned.
    pub fn clear(&mut self, engine: &mut blade_engine::Engine) {
        for h in self.mech_handles.values().copied() {
            engine.remove_object(h);
        }
        self.mech_handles.clear();
        for h in self.stage_handles.drain(..) {
            engine.remove_object(h);
        }
    }

    /// Keep mech objects in sync with the tactical grid (battle only).
    pub fn sync_mechs(&self, engine: &mut blade_engine::Engine, mission: &Mission) {
        if self.kind != ViewMode::Battle {
            return;
        }
        for mech in &mission.mechs {
            let Some(&handle) = self.mech_handles.get(&mech.id) else {
                continue;
            };
            if mech.destroyed {
                engine.teleport_object(
                    handle,
                    blade_engine::Transform {
                        position: (cell_to_world(mech.position) + Vec3::Y * -4.0).into(),
                        orientation: quat_identity(),
                    },
                );
                continue;
            }
            engine.teleport_object(
                handle,
                blade_engine::Transform {
                    position: cell_to_world(mech.position).into(),
                    orientation: quat_identity(),
                },
            );
            let tint = match mech.team {
                Team::Player => [0.55, 0.75, 1.0, 1.0],
                Team::Enemy => [1.0, 0.42, 0.38, 1.0],
            };
            engine.set_color_tint(handle, tint);
        }
    }
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
                if dense {
                    1.25
                } else {
                    1.15
                }
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

/// Simple modular underground facility from Space Kit pieces.
fn spawn_underground_facility(engine: &mut blade_engine::Engine) -> Vec<blade_engine::ObjectHandle> {
    let mut handles = Vec::new();
    let placements: &[(&str, [f32; 3], f32)] = &[
        ("models/space/room-large.glb", [0.0, 0.0, 0.0], 1.0),
        ("models/space/corridor-wide.glb", [0.0, 0.0, 8.0], 1.0),
        ("models/space/corridor-wide.glb", [0.0, 0.0, -8.0], 1.0),
        ("models/space/corridor.glb", [8.0, 0.0, 0.0], 1.0),
        ("models/space/corridor.glb", [-8.0, 0.0, 0.0], 1.0),
        ("models/space/corridor-intersection.glb", [0.0, 0.0, 16.0], 1.0),
        ("models/space/corridor-intersection.glb", [0.0, 0.0, -16.0], 1.0),
        ("models/space/room-small.glb", [12.0, 0.0, 0.0], 1.0),
        ("models/space/room-small.glb", [-12.0, 0.0, 0.0], 1.0),
        ("models/space/room-wide.glb", [0.0, 0.0, 22.0], 1.0),
        ("models/space/gate.glb", [0.0, 0.0, -22.0], 1.0),
        ("models/space/gate-door.glb", [16.0, 0.0, 8.0], 1.0),
        ("models/space/stairs.glb", [-10.0, 0.0, 10.0], 1.0),
        ("models/space/corridor-corner.glb", [8.0, 0.0, 8.0], 1.0),
        ("models/space/corridor-corner.glb", [-8.0, 0.0, -8.0], 1.0),
        ("models/space/corridor-junction.glb", [0.0, 0.0, 4.0], 1.0),
        ("models/space/template-floor.glb", [4.0, -0.05, 4.0], 1.2),
        ("models/space/template-floor.glb", [-4.0, -0.05, -4.0], 1.2),
        ("models/space/template-wall.glb", [14.0, 0.0, -6.0], 1.0),
        ("models/space/room-large.glb", [0.0, 0.0, 28.0], 0.9),
    ];
    for (i, (path, pos, scale)) in placements.iter().enumerate() {
        handles.push(add_static(engine, format!("ug-{i}"), path, *pos, *scale));
    }
    handles
}

/// Multi-part procedural silhouette: legs + torso + head + arms (~2.8 u tall).
fn spawn_mech_silhouette(
    engine: &mut blade_engine::Engine,
    mech: &Mech,
) -> blade_engine::ObjectHandle {
    let color = match mech.team {
        Team::Player => [0.50, 0.68, 0.95, 1.0],
        Team::Enemy => [0.95, 0.38, 0.32, 1.0],
    };
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
    engine.add_object_with_model(
        &mech.name,
        model,
        blade_engine::Transform {
            position: pos.into(),
            orientation: quat_identity(),
        },
        blade_engine::DynamicInput::SetPosition,
    )
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
        emissive_factor: [0.0, 0.0, 0.0],
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

    // Impact blend: 0 = normal hero, 1 = peak impact
    let k = (impact_t / 1.15).clamp(0.0, 1.0);
    let k = k * k;

    let dist = 7.8 - k * 3.2;
    let side_off = 4.6 - k * 1.8;
    let height = 3.4 - k * 1.1;
    let fov = 0.72 + k * 0.18;

    let eye = focus - along * dist + side * side_off + Vec3::Y * height;
    frame_camera(eye, focus + Vec3::Y * (k * 0.4), fov)
}

/// Elevated city overview cameras.
pub fn city_camera(mode: ViewMode) -> blade_engine::FrameCamera {
    match mode {
        ViewMode::CitySurface => {
            let eye = Vec3::new(-6.0, 18.0, -4.0);
            let focus = Vec3::new(7.0, 2.0, 7.0);
            frame_camera(eye, focus, 0.70)
        }
        ViewMode::CityUnderground => {
            let eye = Vec3::new(-14.0, 12.0, -10.0);
            let focus = Vec3::new(0.0, 1.5, 4.0);
            frame_camera(eye, focus, 0.78)
        }
        ViewMode::Battle => frame_camera(Vec3::new(4.0, 12.0, 4.0), Vec3::new(7.0, 0.0, 7.0), 0.8),
    }
}
