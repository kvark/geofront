//! Battle stage + city overview: Kenney surface / Space Kit underground,
//! Quaternius skinned GLB mechs (Idle/Walk/Punch/Death), street/hangar lights.

use std::collections::HashMap;

use glam::{IVec2, Mat4, Quat, Vec3};
use log::info;

use crate::combat::Mission;
use crate::units::{AlienKind, Facing, Mech, Team};

/// World units per tactical grid cell (matches Kenney road tile width).
pub const CELL: f32 = 2.0;

/// Sodium street-lamp grid cells (shared by locals + warm ground pads).
const SODIUM_CELLS: [(i32, i32); 5] = [(1, 1), (1, 6), (6, 1), (6, 6), (3, 4)];

/// Default player strike length when no profile is supplied.
const DEFAULT_STRIKE_SECS: f32 = 0.55;

/// Per-attacker wind-up + strike timing (aliens differ).
#[derive(Debug, Clone, Copy)]
pub struct AttackAnim {
    pub telegraph: f32,
    pub strike: f32,
    pub lunge: f32,
    pub anim_speed: f32,
    pub hit_duration: f32,
}

impl AttackAnim {
    pub fn for_mech(mech: &Mech) -> Self {
        match mech.alien {
            // Snappy poke: short wind-up, fast clip, longer lunge travel.
            Some(AlienKind::Splinter) => Self {
                telegraph: 0.12,
                strike: 0.36,
                lunge: 0.78,
                anim_speed: 1.55,
                hit_duration: 0.32,
            },
            // Heavy slam: long telegraph, slow punch, deep lunge.
            Some(AlienKind::Mass) => Self {
                telegraph: 0.45,
                strike: 0.78,
                lunge: 1.05,
                anim_speed: 0.78,
                hit_duration: 0.7,
            },
            None => Self {
                telegraph: 0.22,
                strike: DEFAULT_STRIKE_SECS,
                lunge: 0.55,
                anim_speed: 1.15,
                hit_duration: 0.48,
            },
        }
    }

    pub fn total(self) -> f32 {
        self.telegraph + self.strike
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum MechClip {
    Idle,
    Walk,
    Punch,
    Death,
    Hit,
}

struct MechVisual {
    handle: blade_engine::ObjectHandle,
    pos: Vec3,
    yaw: f32,
    bob: f32,
    /// Wind-up before the Punch clip / forward lunge.
    telegraph: f32,
    telegraph_duration: f32,
    /// Remaining strike time (Punch clip + forward lunge).
    punch: f32,
    punch_duration: f32,
    lunge_amp: f32,
    punch_anim_speed: f32,
    hit: f32,
    punch_dir: Vec3,
    clip: MechClip,
    /// Once Death plays, never restart or leave it.
    death_locked: bool,
    /// Seconds since death lock; used to freeze the last pose.
    death_age: f32,
    death_frozen: bool,
    /// George inserts Tall clips, so Walk is 16 instead of 15.
    walk_index: usize,
}

struct PendingHit {
    target_id: u32,
    delay: f32,
    duration: f32,
}

pub struct Arena {
    pub kind: ViewMode,
    visuals: HashMap<u32, MechVisual>,
    stage_handles: Vec<blade_engine::ObjectHandle>,
    pending_hits: Vec<PendingHit>,
    /// Persistent local-light handles (Blade tip uses handle-based LocalLight).
    light_handles: Vec<blade_engine::LightHandle>,
}

impl Arena {
    pub fn spawn(engine: &mut blade_engine::Engine, mode: ViewMode, mission: &Mission) -> Self {
        info!("Spawning arena: {:?}", mode);
        let mut stage_handles = Vec::new();
        let mut visuals = HashMap::new();

        match mode {
            ViewMode::Battle | ViewMode::CitySurface => {
                stage_handles.extend(spawn_surface_roads(engine, 8, 8));
                // Battle needs the denser skyscraper canyon; surface overview
                // uses the same packing so screenshots match the Eva mood.
                stage_handles.extend(spawn_surface_buildings(engine, 8, 8, true));
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
            pending_hits: Vec::new(),
            light_handles: Vec::new(),
        }
    }

    pub fn clear(&mut self, engine: &mut blade_engine::Engine) {
        for vis in self.visuals.values() {
            engine.remove_object(vis.handle);
        }
        self.visuals.clear();
        self.pending_hits.clear();
        for h in self.stage_handles.drain(..) {
            engine.remove_object(h);
        }
        for h in self.light_handles.drain(..) {
            engine.remove_light(h);
        }
    }

    /// Begin an attack with wind-up. Hit reaction is deferred until the strike lands.
    /// Returns total presentation seconds (telegraph + strike) for timers.
    pub fn play_attack(
        &mut self,
        engine: &mut blade_engine::Engine,
        id: u32,
        toward: Vec3,
        profile: AttackAnim,
        hit_target: Option<u32>,
    ) -> f32 {
        if let Some(v) = self.visuals.get_mut(&id) {
            if v.death_locked {
                return 0.0;
            }
            let dir = Vec3::new(toward.x, 0.0, toward.z);
            v.punch_dir = if dir.length_squared() > 1e-4 {
                dir.normalize()
            } else {
                Vec3::Z
            };
            v.telegraph_duration = profile.telegraph.max(0.01);
            v.telegraph = profile.telegraph;
            v.punch_duration = profile.strike.max(0.01);
            v.punch = 0.0;
            v.lunge_amp = profile.lunge;
            v.punch_anim_speed = profile.anim_speed;
            v.hit = 0.0;
            // Wind-up holds Idle (lean is positional); Punch starts when telegraph ends.
            set_clip(engine, v, MechClip::Idle, true);
        }
        if let Some(tid) = hit_target {
            self.pending_hits.push(PendingHit {
                target_id: tid,
                delay: profile.telegraph,
                duration: profile.hit_duration,
            });
        }
        profile.total()
    }

    pub fn play_hit(&mut self, engine: &mut blade_engine::Engine, id: u32, duration: f32) {
        if let Some(v) = self.visuals.get_mut(&id) {
            if v.death_locked || v.clip == MechClip::Death {
                return;
            }
            v.hit = duration.max(0.1);
            set_clip(engine, v, MechClip::Hit, false);
        }
    }

    /// Street lamps / hangar fixtures + a brief impact flash.
    ///
    /// Blade tip keeps local lights as handle-owned `LocalLight` entries
    /// (color is unit RGB, intensity is radiant peak). We rebuild the set
    /// each frame so punch/telegraph flashes stay in sync with anims.
    pub fn sync_lights(&mut self, engine: &mut blade_engine::Engine, mission: &Mission) {
        let mut lights: Vec<blade_render::LocalLight> = Vec::new();
        let push = |lights: &mut Vec<blade_render::LocalLight>,
                    pos: [f32; 3],
                    color: [f32; 3],
                    intensity: f32,
                    range: f32| {
            if lights.len() >= blade_render::MAX_LOCAL_LIGHTS {
                return;
            }
            lights.push(blade_render::LocalLight {
                position: pos.into(),
                color: color.into(),
                intensity,
                range,
                angular: blade_render::LightAngularProfile::Omnidirectional,
            });
        };

        match self.kind {
            ViewMode::CityUnderground => {
                for pos in [
                    [0.0, 3.2, 0.0],
                    [0.0, 3.0, 14.0],
                    [0.0, 3.0, 28.0],
                    [20.0, 3.0, 0.0],
                    [-20.0, 3.0, 0.0],
                    [0.0, 2.4, -10.0],
                ] {
                    push(&mut lights, pos, [1.0, 0.82, 0.55], 55.0, 14.0);
                }
            }
            ViewMode::CitySurface | ViewMode::Battle => {
                // Warm sodium street lamps. Cap at 5 so punch/telegraph/wreck
                // flashes still fit under MAX_LOCAL_LIGHTS=8.
                // Color is deep orange (not creamy white) for Tokyo-3 night.
                // Lavapipe reads pools weakly on flat Kenney asphalt — brighter,
                // larger falloff, slightly lower so the street gets the lobe.
                let sodium = [1.0, 0.55, 0.14];
                for &(x, z) in &SODIUM_CELLS {
                    let p = cell_to_world(IVec2::new(x, z));
                    push(&mut lights, [p.x, 2.05, p.z], sodium, 420.0, 17.5);
                }
            }
        }

        if self.kind == ViewMode::Battle {
            for vis in self.visuals.values() {
                if vis.punch > 0.0 {
                    let flash = vis.pos + Vec3::Y * 1.6 + vis.punch_dir * 0.8;
                    let k = (vis.punch / vis.punch_duration.max(0.01)).clamp(0.0, 1.0);
                    push(
                        &mut lights,
                        flash.into(),
                        [1.0, 0.55, 0.18],
                        220.0 * k,
                        6.0,
                    );
                }
            }
            for vis in self.visuals.values() {
                if vis.telegraph > 0.0 {
                    let k = (vis.telegraph / vis.telegraph_duration.max(0.01)).clamp(0.0, 1.0);
                    // Charge glow builds as wind-up completes (k goes 1→0).
                    let build = 1.0 - k;
                    let glow = vis.pos + Vec3::Y * 1.4 - vis.punch_dir * 0.35;
                    push(
                        &mut lights,
                        glow.into(),
                        [0.45, 0.35, 1.0],
                        90.0 * build,
                        5.0,
                    );
                }
            }
            for mech in &mission.mechs {
                if mech.destroyed {
                    let p = cell_to_world(mech.position);
                    push(&mut lights, [p.x, 0.6, p.z], [1.0, 0.28, 0.1], 18.0, 4.0);
                }
            }
        }

        // Reuse handles when the count is stable; otherwise recreate.
        if self.light_handles.len() == lights.len() {
            for (handle, light) in self.light_handles.iter().zip(lights.iter()) {
                engine.set_light(*handle, *light);
            }
        } else {
            for h in self.light_handles.drain(..) {
                engine.remove_light(h);
            }
            for light in lights {
                self.light_handles.push(engine.add_light(light));
            }
        }
    }

    /// Lerp mechs toward grid cells, bob while walking, telegraph lean + lunge on attack.
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

        // Deferred hit reactions land when the strike starts (end of telegraph).
        let mut due = Vec::new();
        self.pending_hits.retain_mut(|ph| {
            ph.delay -= dt;
            if ph.delay <= 0.0 {
                due.push((ph.target_id, ph.duration));
                false
            } else {
                true
            }
        });
        for (tid, dur) in due {
            self.play_hit(engine, tid, dur);
        }

        for mech in &mission.mechs {
            let Some(vis) = self.visuals.get_mut(&mech.id) else {
                continue;
            };
            // Bind skinned clips once GLB cooks finish (safe to call every frame).
            engine.ensure_animation_models(vis.handle);
            // Wrecks stay on the tile — a slight slump, not a fall-through.
            let target = if mech.destroyed {
                cell_to_world(mech.position) + Vec3::Y * -0.18
            } else {
                cell_to_world(mech.position)
            };
            let to_target = target - vis.pos;
            let dist = to_target.length();
            let speed = if mech.destroyed { 5.0 } else { 9.0 };
            if dist > 0.02 {
                vis.pos += to_target.normalize() * (speed * dt).min(dist);
                if !mech.destroyed {
                    vis.bob += dt * 10.0;
                }
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
            if !vis.death_locked {
                vis.yaw += dy * (1.0 - (-12.0 * dt).exp());
            }

            // Telegraph → strike transition.
            if vis.telegraph > 0.0 {
                vis.telegraph = (vis.telegraph - dt).max(0.0);
                if vis.telegraph <= 0.0 && !vis.death_locked {
                    vis.punch = vis.punch_duration;
                    set_clip_with_speed(
                        engine,
                        vis,
                        MechClip::Punch,
                        false,
                        vis.punch_anim_speed,
                    );
                }
            } else if vis.punch > 0.0 {
                vis.punch = (vis.punch - dt).max(0.0);
            }
            if vis.hit > 0.0 {
                vis.hit = (vis.hit - dt).max(0.0);
            }

            let bob_y = if mech.destroyed {
                0.0
            } else {
                vis.bob.sin() * 0.08 * (dist * 2.0).min(1.0)
            };

            // Wind-up leans back; strike lunges forward then recovers.
            let lean = if vis.telegraph > 0.0 {
                let t = 1.0 - (vis.telegraph / vis.telegraph_duration.max(0.01));
                -vis.lunge_amp * 0.35 * t
            } else if vis.punch > 0.0 {
                let t = (1.0 - vis.punch / vis.punch_duration.max(0.01)).clamp(0.0, 1.0);
                if t < 0.4 {
                    (t / 0.4) * vis.lunge_amp
                } else {
                    (1.0 - (t - 0.4) / 0.6) * vis.lunge_amp
                }
            } else {
                0.0
            };
            let pos = vis.pos + Vec3::Y * bob_y + vis.punch_dir * lean;
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
            let mut tint = mech_tint(mech, pulse);
            if vis.telegraph > 0.0 {
                let build = 1.0 - (vis.telegraph / vis.telegraph_duration.max(0.01));
                tint[0] = (tint[0] + 0.55 * build).min(2.2);
                tint[1] = (tint[1] + 0.25 * build).min(2.0);
                tint[2] = (tint[2] + 0.35 * build).min(2.2);
            }
            engine.set_color_tint(vis.handle, tint);

            if mech.destroyed {
                if !vis.death_locked {
                    vis.death_locked = true;
                    vis.death_age = 0.0;
                    vis.death_frozen = false;
                    vis.telegraph = 0.0;
                    vis.punch = 0.0;
                    vis.hit = 0.0;
                    set_clip_with_speed(engine, vis, MechClip::Death, false, 0.95);
                } else {
                    vis.death_age += dt;
                    // Freeze on the fallen pose so the clip cannot restart or idle-pop.
                    if !vis.death_frozen && vis.death_age >= 1.15 {
                        freeze_clip(engine, vis);
                        vis.death_frozen = true;
                    }
                }
                continue;
            }

            let want = if vis.telegraph > 0.0 {
                MechClip::Idle
            } else if vis.punch > 0.0 {
                MechClip::Punch
            } else if vis.hit > 0.0 {
                MechClip::Hit
            } else if dist > 0.08 {
                MechClip::Walk
            } else {
                MechClip::Idle
            };
            if matches!(want, MechClip::Punch) {
                // Punch already started with the right speed at telegraph end.
            } else {
                set_clip(
                    engine,
                    vis,
                    want,
                    matches!(want, MechClip::Idle | MechClip::Walk),
                );
            }
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
    // Plaza asphalt reads too lilac under cool twilight ambient — desaturate
    // so dusk sky keeps the chroma lead. Warm-tint cells under sodium lamps
    // so pads read as lit ground even when lavapipe softens local pools.
    let plaza_tint = [0.72, 0.70, 0.64, 1.0];
    let pad_tint = [1.55, 0.95, 0.42, 1.0];
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
            let h = add_static(
                engine,
                format!("road-{x}-{z}"),
                path,
                [pos.x, -1.0, pos.z],
                1.0,
            );
            let lamp_pad = SODIUM_CELLS.iter().any(|&(lx, lz)| lx == x && lz == z);
            engine.set_color_tint(h, if lamp_pad { pad_tint } else { plaza_tint });
            handles.push(h);
        }
    }
    // Visual sodium fixtures (Kenney light-square is ~0.6m; scale up to street).
    for (i, &(x, z)) in SODIUM_CELLS.iter().enumerate() {
        let pos = cell_to_world(IVec2::new(x, z));
        let h = add_static(
            engine,
            format!("sodium-lamp-{i}"),
            "models/roads/light-square.glb",
            [pos.x, 0.0, pos.z],
            4.2,
        );
        engine.set_color_tint(h, [2.4, 1.25, 0.35, 1.0]);
        handles.push(h);
    }
    handles
}

fn spawn_surface_buildings(
    engine: &mut blade_engine::Engine,
    width: i32,
    height: i32,
    dense: bool,
) -> Vec<blade_engine::ObjectHandle> {
    // Prefer skyscrapers so the 8×8 street reads as a Tokyo-3 canyon.
    // Buildings sit *outside* the fight grid (outer rings only) so the street
    // stays clear and the low hero camera isn't buried in rim walls.
    let skyscrapers = [
        "models/commercial/building-skyscraper-a.glb",
        "models/commercial/building-skyscraper-b.glb",
        "models/commercial/building-skyscraper-c.glb",
        "models/commercial/building-skyscraper-d.glb",
        "models/commercial/building-skyscraper-e.glb",
    ];
    let midrise = [
        "models/commercial/building-a.glb",
        "models/commercial/building-c.glb",
        "models/commercial/building-e.glb",
        "models/commercial/building-i.glb",
        "models/commercial/building-l.glb",
        "models/commercial/building-n.glb",
    ];
    let industrial = [
        "models/industrial/building-a.glb",
        "models/industrial/building-d.glb",
        "models/industrial/building-h.glb",
        "models/industrial/chimney-large.glb",
    ];

    let mut handles = Vec::new();
    let mut i = 0usize;
    let margin = if dense { 3 } else { 2 };

    for x in -margin..width + margin {
        for z in -margin..height + margin {
            let outer = x < 0 || z < 0 || x >= width || z >= height;
            if !outer {
                // Fight grid stays open street — canyon walls are outside.
                continue;
            }
            let ring = (-x)
                .max(x - (width - 1))
                .max((-z).max(z - (height - 1)));
            // Camera approaches from the west (behind players); keep that side
            // buffered. Pack N/S/E walls closer (ring 1) for canyon framing.
            let west_approach = x < 0;
            let place = if dense {
                if west_approach {
                    ring == 2 || ring == 3
                } else {
                    ring >= 1 && ring <= 3
                }
            } else {
                ring == 2 && (x + z) % 2 == 0
            };
            if !place {
                continue;
            }

            let path = if ring >= 3 && (i % 4 == 0) {
                industrial[i % industrial.len()]
            } else if i % 6 == 0 {
                midrise[i % midrise.len()]
            } else {
                skyscrapers[i % skyscrapers.len()]
            };

            let scale = if path.contains("skyscraper") {
                let base = if dense { 1.7 } else { 1.25 };
                base + ((i % 4) as f32) * 0.2
            } else if path.contains("chimney") {
                1.4
            } else if dense {
                1.25
            } else {
                1.05
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
    match mech.alien {
        Some(AlienKind::Mass) => "models/mechs/George.glb",
        Some(AlienKind::Splinter) => "models/mechs/Leela.glb",
        None => match (mech.team, mech.id % 2) {
            (Team::Player, 0) => "models/mechs/Stan.glb",
            (Team::Player, _) => "models/mechs/Mike.glb",
            (Team::Enemy, 0) => "models/mechs/George.glb",
            (Team::Enemy, _) => "models/mechs/Leela.glb",
        },
    }
}

fn mech_scale(mech: &Mech) -> f32 {
    // Quaternius pack is authored at ~7m; 0.4 puts player feet on a 2-unit cell.
    match mech.alien {
        Some(AlienKind::Mass) => 0.55,
        Some(AlienKind::Splinter) => 0.32,
        None => 0.4,
    }
}

fn mech_tint(mech: &Mech, pulse: f32) -> [f32; 4] {
    // High gain + saturated team colors so mechs stay readable against the
    // darker Tokyo-3 twilight (Quaternius albedo ~0.2–0.35 + Reinhard).
    match mech.alien {
        Some(AlienKind::Mass) => [2.45 * pulse, 1.05, 0.75, 1.0],
        Some(AlienKind::Splinter) => [1.35 * pulse, 1.45 * pulse, 2.55 * pulse, 1.0],
        None => match mech.team {
            Team::Player => [1.55 * pulse, 1.95 * pulse, 2.35 * pulse, 1.0],
            Team::Enemy => [2.35 * pulse, 1.15, 0.95, 1.0],
        },
    }
}

fn clip_index(clip: MechClip, walk_index: usize) -> usize {
    match clip {
        MechClip::Idle => 5,
        MechClip::Walk => walk_index,
        MechClip::Punch => 10,
        MechClip::Death => 1,
        MechClip::Hit => 3,
    }
}

fn set_clip(
    engine: &mut blade_engine::Engine,
    vis: &mut MechVisual,
    clip: MechClip,
    looping: bool,
) {
    let speed = if matches!(clip, MechClip::Punch | MechClip::Hit | MechClip::Death) {
        1.15
    } else {
        1.0
    };
    set_clip_with_speed(engine, vis, clip, looping, speed);
}

fn set_clip_with_speed(
    engine: &mut blade_engine::Engine,
    vis: &mut MechVisual,
    clip: MechClip,
    looping: bool,
    speed: f32,
) {
    if vis.death_locked && clip != MechClip::Death {
        return;
    }
    // Sticky clips: skip no-op. Punch may re-fire from the start on a new attack.
    if vis.clip == clip && !matches!(clip, MechClip::Punch) {
        return;
    }
    vis.clip = clip;
    let mut player = blade_engine::AnimationPlayer::new(clip_index(clip, vis.walk_index));
    player.looping = looping;
    player.speed = speed;
    engine.set_animation(vis.handle, Some(player));
}

fn freeze_clip(engine: &mut blade_engine::Engine, vis: &mut MechVisual) {
    // Re-apply Death at speed 0 so the wreck stops ticking back toward bind pose.
    let mut player = blade_engine::AnimationPlayer::new(clip_index(MechClip::Death, vis.walk_index));
    player.looping = false;
    player.speed = 0.0;
    engine.set_animation(vis.handle, Some(player));
    vis.clip = MechClip::Death;
}

fn spawn_mech(engine: &mut blade_engine::Engine, mech: &Mech) -> MechVisual {
    let path = mech_glb_path(mech);
    let scale = mech_scale(mech);
    let pos = cell_to_world(mech.position);
    let yaw = mech.facing.yaw();
    let q = Quat::from_rotation_y(yaw);
    let handle = engine.add_object(
        &blade_engine::config::Object {
            name: mech.name.clone(),
            visuals: vec![blade_engine::config::Visual {
                model: path.into(),
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
            orientation: mint::Quaternion {
                s: q.w,
                v: [q.x, q.y, q.z].into(),
            },
        },
        blade_engine::DynamicInput::SetPosition,
    );
    let walk_index = if path.contains("George") { 16 } else { 15 };
    let mut vis = MechVisual {
        handle,
        pos,
        yaw,
        bob: 0.0,
        telegraph: 0.0,
        telegraph_duration: 0.22,
        punch: 0.0,
        punch_duration: DEFAULT_STRIKE_SECS,
        lunge_amp: 0.55,
        punch_anim_speed: 1.15,
        hit: 0.0,
        punch_dir: Vec3::Z,
        clip: MechClip::Hit, // force the first set_clip to apply Idle
        death_locked: false,
        death_age: 0.0,
        death_frozen: false,
        walk_index,
    };
    // Defer Idle bind until after `Engine::update` flushes GLB cooks; binding here
    // races cooking and leaves animation_model unset (permanent T-pose).
    // `clip: Hit` forces the first tick's set_clip to apply Idle.
    vis
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

/// Low hero camera. When `impact_t` is > 0 (seconds remaining),
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

    let focus = (p + e) * 0.5 + Vec3::Y * 1.35;
    let along = (e - p).normalize_or_zero();
    let side = along.cross(Vec3::Y).normalize_or_zero();

    let k = (impact_t / 1.15).clamp(0.0, 1.0);
    let k = k * k;

    // Over-the-shoulder street shot: stay inside the 8×8 so outer canyon
    // walls frame the lane instead of clipping the lens. Slightly raised /
    // pulled back so the near hero mech isn't cropped mid-shot.
    let dist = 6.7 - k * 1.6;
    let side_off = 3.0 - k * 0.75;
    let height = 2.95 - k * 0.55;
    let fov = 0.74 + k * 0.10;

    let eye = focus - along * dist + side * side_off + Vec3::Y * height;
    // Look slightly above foot level so the shot reads more horizontal.
    frame_camera(eye, focus + Vec3::Y * (0.45 + k * 0.30), fov)
}

/// Elevated city overview cameras — also used to seed FlyCam.
pub fn city_camera(mode: ViewMode) -> blade_engine::FrameCamera {
    match mode {
        ViewMode::CitySurface => {
            // Street-canyon overview rather than high noon god-cam.
            let eye = Vec3::new(-4.0, 11.0, -2.5);
            let focus = Vec3::new(7.0, 3.5, 7.0);
            frame_camera(eye, focus, 0.72)
        }
        ViewMode::CityUnderground => {
            let eye = Vec3::new(-18.0, 16.0, -16.0);
            let focus = Vec3::new(0.0, 1.5, 8.0);
            frame_camera(eye, focus, 0.78)
        }
        ViewMode::Battle => FlyCam::from_eye_focus(
            Vec3::new(-1.5, 3.2, 11.0),
            Vec3::new(7.0, 1.6, 7.0),
        )
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
