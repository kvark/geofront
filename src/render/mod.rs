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
/// Three corners leave two MAX_LOCAL_LIGHTS slots for cyan/magenta neon.
const SODIUM_CELLS: [(i32, i32); 2] = [(1, 2), (6, 5)];

/// Cool anime accent light (cell + RGB). One neon leaves slots for VFX and
/// keeps Blade's 1-light reservoir from washing sodium pools to noise.
const NEON_LIGHTS: [(i32, i32, [f32; 3]); 1] = [
    (4, 1, [0.3, 0.95, 1.0]), // cyan
];

/// Default player strike length when no profile is supplied.
const DEFAULT_STRIKE_SECS: f32 = 0.55;

/// Brief hit flash / sodium kick (seconds remaining at onset).
const IMPACT_FLASH_SECS: f32 = 0.32;
/// Sharp camera punch when the strike lands.
const IMPACT_PUNCH_SECS: f32 = 0.16;

/// 1 at onset, quadratic falloff — lights, tints, and camera punch share this.
pub fn impact_envelope(remaining: f32, duration: f32) -> f32 {
    let k = (remaining / duration.max(1e-4)).clamp(0.0, 1.0);
    k * k
}

/// Attack / telegraph color language so Angels read apart from mech sodium.
#[derive(Debug, Clone, Copy)]
struct StrikePalette {
    light: [f32; 3],
    light_i: f32,
    light_r: f32,
    spark_ray: u32,
    spark_a: u32,
    spark_b: u32,
    telegraph: [f32; 3],
    telegraph_i: f32,
    telegraph_r: f32,
    telegraph_height: f32,
    punch: [f32; 3],
    punch_i: f32,
    tint_build: [f32; 3],
    /// Kick warm sodium street lamps on impact (mechs only).
    sodium_kick: bool,
}

fn strike_palette(kind: Option<AlienKind>) -> StrikePalette {
    match kind {
        // Fast harasser: electric magenta / violet.
        Some(AlienKind::Splinter) => StrikePalette {
            light: [0.95, 0.28, 1.0],
            light_i: 980.0,
            light_r: 8.5,
            spark_ray: 0xFF_FF_66_EE,
            spark_a: 0xFF_FF_CC_FF,
            spark_b: 0xFF_AA_22_CC,
            telegraph: [0.75, 0.22, 1.0],
            telegraph_i: 140.0,
            telegraph_r: 4.2,
            telegraph_height: 2.05,
            punch: [0.85, 0.25, 1.0],
            punch_i: 260.0,
            tint_build: [0.35, 0.15, 0.95],
            sodium_kick: false,
        },
        // Slow pressure: bone-pale + crimson core.
        Some(AlienKind::Mass) => StrikePalette {
            light: [1.0, 0.22, 0.18],
            light_i: 1200.0,
            light_r: 11.0,
            spark_ray: 0xFF_44_55_FF,
            spark_a: 0xFF_AA_CC_FF,
            spark_b: 0xFF_22_33_CC,
            telegraph: [1.0, 0.18, 0.12],
            telegraph_i: 160.0,
            telegraph_r: 6.5,
            telegraph_height: 1.55,
            punch: [1.0, 0.28, 0.14],
            punch_i: 300.0,
            tint_build: [0.85, 0.12, 0.08],
            sodium_kick: false,
        },
        // Player / generic mechs: warm sodium strike language.
        None => StrikePalette {
            light: [1.0, 0.78, 0.32],
            light_i: 1100.0,
            light_r: 9.5,
            spark_ray: 0xFF_FF_EE_88,
            spark_a: 0xFF_FF_FF_CC,
            spark_b: 0xFF_FF_AA_44,
            telegraph: [0.45, 0.35, 1.0],
            telegraph_i: 90.0,
            telegraph_r: 5.0,
            telegraph_height: 1.4,
            punch: [1.0, 0.55, 0.18],
            punch_i: 220.0,
            tint_build: [0.55, 0.25, 0.35],
            sodium_kick: true,
        },
    }
}

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
    /// Cached at spawn so attack lights/tints know Angel vs mech language.
    alien: Option<AlienKind>,
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
    hit_duration: f32,
    punch_dir: Vec3,
    /// Incoming strike direction (attacker → target) for knockback / sparks.
    hit_dir: Vec3,
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
    dir: Vec3,
    /// Strike lands on AT Field (cyan deflect; no Hit clip).
    at_field: bool,
    shattered: bool,
    /// Attacker language for the deferred impact flash.
    attacker_kind: Option<AlienKind>,
}

struct ImpactFlash {
    pos: Vec3,
    t: f32,
    duration: f32,
    dir: Vec3,
    /// Cool cyan AT Field deflect (vs warm sodium / Angel hit).
    at_field: bool,
    /// `Some` = Angel strike palette; `None` = mech sodium (ignored when at_field).
    kind: Option<AlienKind>,
}

pub struct Arena {
    pub kind: ViewMode,
    visuals: HashMap<u32, MechVisual>,
    stage_handles: Vec<blade_engine::ObjectHandle>,
    pending_hits: Vec<PendingHit>,
    impact_flashes: Vec<ImpactFlash>,
    /// Remaining camera-punch seconds (sharp kick at impact).
    punch_t: f32,
    /// Brief AT Field ring flare (unit_id → seconds remaining).
    at_field_pulse: HashMap<u32, f32>,
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
            impact_flashes: Vec::new(),
            punch_t: 0.0,
            at_field_pulse: HashMap::new(),
            light_handles: Vec::new(),
        }
    }

    pub fn clear(&mut self, engine: &mut blade_engine::Engine) {
        for vis in self.visuals.values() {
            engine.remove_object(vis.handle);
        }
        self.visuals.clear();
        self.pending_hits.clear();
        self.impact_flashes.clear();
        self.punch_t = 0.0;
        self.at_field_pulse.clear();
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
        self.play_attack_ex(engine, id, toward, profile, hit_target, false, false)
    }

    /// Like [`play_attack`], but when `at_field` the deferred contact is a cyan
    /// deflect (and optional shatter) instead of a Hit reaction + sodium punch.
    pub fn play_attack_ex(
        &mut self,
        engine: &mut blade_engine::Engine,
        id: u32,
        toward: Vec3,
        profile: AttackAnim,
        hit_target: Option<u32>,
        at_field: bool,
        shattered: bool,
    ) -> f32 {
        let mut strike_dir = Vec3::Z;
        let mut attacker_kind = None;
        if let Some(v) = self.visuals.get_mut(&id) {
            if v.death_locked {
                return 0.0;
            }
            attacker_kind = v.alien;
            let dir = Vec3::new(toward.x, 0.0, toward.z);
            v.punch_dir = if dir.length_squared() > 1e-4 {
                dir.normalize()
            } else {
                Vec3::Z
            };
            strike_dir = v.punch_dir;
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
                dir: strike_dir,
                at_field,
                shattered,
                attacker_kind,
            });
        }
        profile.total()
    }

    pub fn play_hit(
        &mut self,
        engine: &mut blade_engine::Engine,
        id: u32,
        duration: f32,
        dir: Vec3,
    ) {
        if let Some(v) = self.visuals.get_mut(&id) {
            if v.death_locked || v.clip == MechClip::Death {
                return;
            }
            v.hit = duration.max(0.1);
            v.hit_duration = duration.max(0.1);
            if dir.length_squared() > 1e-4 {
                v.hit_dir = dir.normalize();
            }
            set_clip(engine, v, MechClip::Hit, false);
        }
    }

    /// Flash + camera punch at the contact point. Fires even if the target is already a wreck.
    fn spawn_impact(&mut self, target_id: u32, dir: Vec3, kind: Option<AlienKind>) {
        let pos = self
            .visuals
            .get(&target_id)
            .map(|v| v.pos + Vec3::Y * 1.35)
            .unwrap_or(Vec3::Y * 1.35);
        self.impact_flashes.push(ImpactFlash {
            pos,
            t: IMPACT_FLASH_SECS,
            duration: IMPACT_FLASH_SECS,
            dir,
            at_field: false,
            kind,
        });
        self.punch_t = self.punch_t.max(IMPACT_PUNCH_SECS);
    }

    /// Cyan AT Field deflect flash + ring pulse (no camera punch).
    pub fn spawn_at_field_fx(&mut self, target_id: u32, shattered: bool) {
        let pos = self
            .visuals
            .get(&target_id)
            .map(|v| v.pos + Vec3::Y * 1.45)
            .unwrap_or(Vec3::Y * 1.45);
        let dur = if shattered { 0.55 } else { 0.38 };
        self.impact_flashes.push(ImpactFlash {
            pos,
            t: dur,
            duration: dur,
            dir: Vec3::Y,
            at_field: true,
            kind: None,
        });
        let pulse = if shattered { 0.7 } else { 0.45 };
        self.at_field_pulse
            .entry(target_id)
            .and_modify(|t| *t = (*t).max(pulse))
            .or_insert(pulse);
    }


    pub fn camera_punch(&self) -> f32 {
        self.punch_t
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
                // Warm sodium + cool neon accents. Budget: 2 sodium + 1 neon
                // so punch/telegraph/wreck/AT-Field still fit under MAX=8.
                // Lavapipe reads pools weakly on flat asphalt — bright, wide,
                // slightly low so the street gets the lobe.
                // Only mech (sodium-language) impacts kick the street lamps —
                // Angel hits keep their own magenta/crimson ground flash.
                let kick = self
                    .impact_flashes
                    .iter()
                    .filter(|f| strike_palette(f.kind).sodium_kick)
                    .map(|f| impact_envelope(f.t, f.duration))
                    .fold(0.0f32, f32::max);
                let sodium = [1.0, 0.52, 0.12];
                let sodium_i = 920.0 * (1.0 + 2.8 * kick);
                let sodium_r = 20.0 + 5.0 * kick;
                for &(x, z) in &SODIUM_CELLS {
                    let p = cell_to_world(IVec2::new(x, z));
                    push(&mut lights, [p.x, 1.95, p.z], sodium, sodium_i, sodium_r);
                }
                let neon_i = 420.0 * (1.0 + 1.3 * kick);
                let neon_r = 14.0 + 3.0 * kick;
                for &(x, z, color) in &NEON_LIGHTS {
                    let p = cell_to_world(IVec2::new(x, z));
                    // Raise neon slightly so façades catch a cool wash.
                    push(&mut lights, [p.x, 2.55, p.z], color, neon_i, neon_r);
                }
            }
        }

        if self.kind == ViewMode::Battle {
            // Impact pulse first so it wins a slot under the 8-light cap.
            for flash in &self.impact_flashes {
                let k = impact_envelope(flash.t, flash.duration);
                let p = flash.pos + flash.dir * 0.2;
                // Low, wide — lavapipe reads ground pools better than high points.
                let pal = strike_palette(flash.kind);
                let (color, intensity, range) = if flash.at_field {
                    ([0.35, 0.85, 1.35], 950.0 * k, 10.0)
                } else {
                    (pal.light, pal.light_i * k, pal.light_r)
                };
                push(&mut lights, [p.x, 0.55, p.z], color, intensity, range);
            }
            // Idle AT Field aura on Mass while charges remain.
            for mech in &mission.mechs {
                if mech.destroyed || mech.at_field == 0 {
                    continue;
                }
                let Some(vis) = self.visuals.get(&mech.id) else {
                    continue;
                };
                let pulse = self
                    .at_field_pulse
                    .get(&mech.id)
                    .copied()
                    .unwrap_or(0.0);
                let boost = 1.0 + 2.4 * (pulse / 0.7).clamp(0.0, 1.0);
                push(
                    &mut lights,
                    [vis.pos.x, 1.55, vis.pos.z],
                    [0.25, 0.75, 1.2],
                    55.0 * boost,
                    5.5,
                );
            }
            for vis in self.visuals.values() {
                if vis.punch > 0.0 {
                    let pal = strike_palette(vis.alien);
                    let flash = vis.pos + Vec3::Y * 1.6 + vis.punch_dir * 0.8;
                    let k = (vis.punch / vis.punch_duration.max(0.01)).clamp(0.0, 1.0);
                    push(
                        &mut lights,
                        flash.into(),
                        pal.punch,
                        pal.punch_i * k,
                        6.0,
                    );
                }
            }
            for vis in self.visuals.values() {
                if vis.telegraph > 0.0 {
                    let pal = strike_palette(vis.alien);
                    let k = (vis.telegraph / vis.telegraph_duration.max(0.01)).clamp(0.0, 1.0);
                    // Charge glow builds as wind-up completes (k goes 1→0).
                    let build = 1.0 - k;
                    let glow =
                        vis.pos + Vec3::Y * pal.telegraph_height - vis.punch_dir * 0.35;
                    push(
                        &mut lights,
                        glow.into(),
                        pal.telegraph,
                        pal.telegraph_i * build,
                        pal.telegraph_r,
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

        // Deferred hit reactions + impact VFX land when the strike starts.
        let mut due = Vec::new();
        self.pending_hits.retain_mut(|ph| {
            ph.delay -= dt;
            if ph.delay <= 0.0 {
                due.push((ph.target_id, ph.duration, ph.dir, ph.at_field, ph.shattered, ph.attacker_kind));
                false
            } else {
                true
            }
        });
        for (tid, dur, dir, at_field, shattered, kind) in due {
            if at_field {
                self.spawn_at_field_fx(tid, shattered);
            } else {
                self.play_hit(engine, tid, dur, dir);
                self.spawn_impact(tid, dir, kind);
            }
        }

        self.punch_t = (self.punch_t - dt).max(0.0);
        for flash in &mut self.impact_flashes {
            flash.t = (flash.t - dt).max(0.0);
        }
        self.impact_flashes.retain(|f| f.t > 0.0);
        for t in self.at_field_pulse.values_mut() {
            *t = (*t - dt).max(0.0);
        }
        self.at_field_pulse.retain(|_, t| *t > 0.0);

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
            let knock = if vis.hit > 0.0 {
                vis.hit_dir * (0.28 * impact_envelope(vis.hit, vis.hit_duration))
            } else {
                Vec3::ZERO
            };
            let pos = vis.pos + Vec3::Y * bob_y + vis.punch_dir * lean + knock;
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
            if mech.at_field > 0 && !mech.destroyed {
                // Cool cyan veil while AT Field is up.
                let flare = self
                    .at_field_pulse
                    .get(&mech.id)
                    .map(|t| (*t / 0.7).clamp(0.0, 1.0))
                    .unwrap_or(0.0);
                tint[0] = (tint[0] * 0.72 + 0.15 + 0.55 * flare).min(2.4);
                tint[1] = (tint[1] * 0.85 + 0.45 + 0.85 * flare).min(3.0);
                tint[2] = (tint[2] + 0.95 + 1.4 * flare).min(4.2);
            }
            if vis.telegraph > 0.0 {
                let build = 1.0 - (vis.telegraph / vis.telegraph_duration.max(0.01));
                let tb = strike_palette(vis.alien).tint_build;
                tint[0] = (tint[0] + tb[0] * build).min(2.8);
                tint[1] = (tint[1] + tb[1] * build).min(2.6);
                tint[2] = (tint[2] + tb[2] * build).min(3.2);
            }
            if vis.hit > 0.0 {
                // Hot white-orange pop so the beat reads under lavapipe.
                let flash = impact_envelope(vis.hit, vis.hit_duration);
                tint[0] = (tint[0] + 3.2 * flash).min(5.0);
                tint[1] = (tint[1] + 2.4 * flash).min(4.4);
                tint[2] = (tint[2] + 1.1 * flash).min(3.2);
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
        draw_at_field_rings(engine, mission, &self.visuals, &self.at_field_pulse);
        draw_angel_silhouettes(engine, mission, &self.visuals);
        draw_impact_sparks(engine, &self.impact_flashes);
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

fn draw_impact_sparks(engine: &mut blade_engine::Engine, flashes: &[ImpactFlash]) {
    if flashes.is_empty() {
        return;
    }
    let mut lines = Vec::new();
    for flash in flashes {
        let k = impact_envelope(flash.t, flash.duration);
        if k < 0.04 {
            continue;
        }
        let pal = strike_palette(flash.kind);
        let (ray_color, tip_a, tip_b, rays) = if flash.at_field {
            (0xFF_66_EE_FFu32, 0xFF_AA_FF_FFu32, 0xFF_33_99_FFu32, 14)
        } else {
            (
                pal.spark_ray,
                pal.spark_a,
                pal.spark_b,
                match flash.kind {
                    Some(AlienKind::Mass) => 12,
                    Some(AlienKind::Splinter) => 14,
                    None => 10,
                },
            )
        };
        for i in 0..rays {
            let a = (i as f32) * std::f32::consts::TAU / rays as f32 + flash.t * 11.0;
            let lift = if i % 2 == 0 { 0.55 } else { 0.12 };
            let dir = Vec3::new(a.cos(), lift, a.sin());
            let len = 0.35 + 1.15 * k;
            lines.push(blade_render::DebugLine {
                a: blade_render::DebugPoint {
                    pos: flash.pos.into(),
                    color: ray_color,
                },
                b: blade_render::DebugPoint {
                    pos: (flash.pos + dir * len).into(),
                    color: ray_color,
                },
            });
        }
        let across = flash.dir.cross(Vec3::Y).normalize_or_zero() * (0.75 * k);
        let along = flash.dir * (0.55 * k);
        for (d0, d1) in [
            (across, -across),
            (along, -along),
            (Vec3::Y * 0.55 * k, Vec3::Y * -0.12),
        ] {
            lines.push(blade_render::DebugLine {
                a: blade_render::DebugPoint {
                    pos: (flash.pos + d0).into(),
                    color: tip_a,
                },
                b: blade_render::DebugPoint {
                    pos: (flash.pos + d1).into(),
                    color: tip_b,
                },
            });
        }
    }
    engine.add_debug_lines(&lines);
}

/// Octagon / hex-ish AT Field silhouette around charged Mass units.
fn draw_at_field_rings(
    engine: &mut blade_engine::Engine,
    mission: &Mission,
    visuals: &HashMap<u32, MechVisual>,
    pulses: &HashMap<u32, f32>,
) {
    let mut lines = Vec::new();
    for mech in &mission.mechs {
        if mech.destroyed || mech.at_field == 0 {
            continue;
        }
        let Some(vis) = visuals.get(&mech.id) else {
            continue;
        };
        let flare = pulses
            .get(&mech.id)
            .map(|t| (*t / 0.7).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let radius = 1.15 + 0.35 * flare + 0.08 * (mech.at_field as f32);
        let y0 = 0.35;
        let y1 = 2.15 + 0.4 * flare;
        let color = if flare > 0.15 {
            0xFF_AA_FF_FF
        } else {
            0xFF_44_CC_EE
        };
        let sides = 8;
        for ring_y in [y0, (y0 + y1) * 0.5, y1] {
            for i in 0..sides {
                let a0 = (i as f32) * std::f32::consts::TAU / sides as f32;
                let a1 = ((i + 1) as f32) * std::f32::consts::TAU / sides as f32;
                let p0 = vis.pos + Vec3::new(a0.cos() * radius, ring_y, a0.sin() * radius);
                let p1 = vis.pos + Vec3::new(a1.cos() * radius, ring_y, a1.sin() * radius);
                lines.push(blade_render::DebugLine {
                    a: blade_render::DebugPoint {
                        pos: p0.into(),
                        color,
                    },
                    b: blade_render::DebugPoint {
                        pos: p1.into(),
                        color,
                    },
                });
            }
        }
        // Vertical struts so the cage reads under lavapipe.
        for i in 0..sides {
            if i % 2 != 0 {
                continue;
            }
            let a = (i as f32) * std::f32::consts::TAU / sides as f32;
            let p0 = vis.pos + Vec3::new(a.cos() * radius, y0, a.sin() * radius);
            let p1 = vis.pos + Vec3::new(a.cos() * radius, y1, a.sin() * radius);
            lines.push(blade_render::DebugLine {
                a: blade_render::DebugPoint {
                    pos: p0.into(),
                    color: 0xFF_33_AA_DD,
                },
                b: blade_render::DebugPoint {
                    pos: p1.into(),
                    color,
                },
            });
        }
    }
    if !lines.is_empty() {
        engine.add_debug_lines(&lines);
    }
}

/// Soft Angel silhouette without new art: Mass = wide crimson cage,
/// Splinter = tall magenta spines. Telegraph inflates the ring.
fn draw_angel_silhouettes(
    engine: &mut blade_engine::Engine,
    mission: &Mission,
    visuals: &HashMap<u32, MechVisual>,
) {
    let mut lines = Vec::new();
    for mech in &mission.mechs {
        if mech.destroyed {
            continue;
        }
        let Some(kind) = mech.alien else {
            continue;
        };
        let Some(vis) = visuals.get(&mech.id) else {
            continue;
        };
        let charge = if vis.telegraph > 0.0 {
            1.0 - (vis.telegraph / vis.telegraph_duration.max(0.01))
        } else if vis.punch > 0.0 {
            (vis.punch / vis.punch_duration.max(0.01)).clamp(0.0, 1.0) * 0.55
        } else {
            0.0
        };
        match kind {
            AlienKind::Mass => {
                let radius = 1.25 + 0.45 * charge;
                let y0 = 0.22;
                let y1 = 2.35 + 0.55 * charge;
                let color = if charge > 0.2 {
                    0xFF_66_88_FF
                } else {
                    0xFF_33_44_CC
                };
                let sides = 6;
                for ring_y in [y0, (y0 + y1) * 0.5, y1] {
                    for i in 0..sides {
                        let a0 = (i as f32) * std::f32::consts::TAU / sides as f32;
                        let a1 = ((i + 1) as f32) * std::f32::consts::TAU / sides as f32;
                        let p0 = vis.pos + Vec3::new(a0.cos() * radius, ring_y, a0.sin() * radius);
                        let p1 = vis.pos + Vec3::new(a1.cos() * radius, ring_y, a1.sin() * radius);
                        lines.push(blade_render::DebugLine {
                            a: blade_render::DebugPoint {
                                pos: p0.into(),
                                color,
                            },
                            b: blade_render::DebugPoint {
                                pos: p1.into(),
                                color,
                            },
                        });
                    }
                }
                for i in 0..sides {
                    if i % 2 != 0 {
                        continue;
                    }
                    let a = (i as f32) * std::f32::consts::TAU / sides as f32;
                    let p0 = vis.pos + Vec3::new(a.cos() * radius, y0, a.sin() * radius);
                    let p1 = vis.pos + Vec3::new(a.cos() * radius, y1, a.sin() * radius);
                    lines.push(blade_render::DebugLine {
                        a: blade_render::DebugPoint {
                            pos: p0.into(),
                            color: 0xFF_22_22_AA,
                        },
                        b: blade_render::DebugPoint {
                            pos: p1.into(),
                            color,
                        },
                    });
                }
            }
            AlienKind::Splinter => {
                let radius = 0.55 + 0.25 * charge;
                let y0 = 0.15;
                let y1 = 2.85 + 0.7 * charge;
                let color = if charge > 0.2 {
                    0xFF_FF_88_FF
                } else {
                    0xFF_CC_44_DD
                };
                let mid = (y0 + y1) * 0.45;
                for i in 0..4 {
                    let a0 = (i as f32) * std::f32::consts::TAU / 4.0 + 0.2 * charge;
                    let a1 = ((i + 1) as f32) * std::f32::consts::TAU / 4.0 + 0.2 * charge;
                    let p0 = vis.pos + Vec3::new(a0.cos() * radius, mid, a0.sin() * radius);
                    let p1 = vis.pos + Vec3::new(a1.cos() * radius, mid, a1.sin() * radius);
                    lines.push(blade_render::DebugLine {
                        a: blade_render::DebugPoint {
                            pos: p0.into(),
                            color,
                        },
                        b: blade_render::DebugPoint {
                            pos: p1.into(),
                            color,
                        },
                    });
                }
                for i in 0..4 {
                    let a = (i as f32) * std::f32::consts::TAU / 4.0;
                    let base = vis.pos
                        + Vec3::new(a.cos() * radius * 0.55, y0, a.sin() * radius * 0.55);
                    let tip = vis.pos
                        + Vec3::new(a.cos() * radius * 0.2, y1, a.sin() * radius * 0.2);
                    lines.push(blade_render::DebugLine {
                        a: blade_render::DebugPoint {
                            pos: base.into(),
                            color: 0xFF_88_22_AA,
                        },
                        b: blade_render::DebugPoint {
                            pos: tip.into(),
                            color,
                        },
                    });
                }
                let apex = vis.pos + Vec3::Y * y1;
                let core = vis.pos + Vec3::Y * mid;
                lines.push(blade_render::DebugLine {
                    a: blade_render::DebugPoint {
                        pos: core.into(),
                        color: 0xFF_FF_CC_FF,
                    },
                    b: blade_render::DebugPoint {
                        pos: apex.into(),
                        color,
                    },
                });
            }
        }
    }
    if !lines.is_empty() {
        engine.add_debug_lines(&lines);
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
    // Kenney road tiles are authored 1×1; CELL is 2. Scale to CELL so asphalt
    // covers the plaza. Darken + desaturate so sodium/neon/sky pop (less lilac).
    let plaza_tint = [0.09, 0.085, 0.08, 1.0];
    let sodium_pad = [2.6, 1.35, 0.28, 1.0];
    let cyan_pad = [0.55, 1.35, 1.55, 1.0];
    let magenta_pad = [1.55, 0.45, 1.25, 1.0];
    let road_scale = CELL; // 1×1 mesh → 2×2 cell cover
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
                road_scale,
            );
            let tint = if SODIUM_CELLS.iter().any(|&(lx, lz)| lx == x && lz == z) {
                sodium_pad
            } else if let Some((_, _, col)) = NEON_LIGHTS
                .iter()
                .find(|&&(lx, lz, _)| lx == x && lz == z)
            {
                if col[2] > col[0] {
                    cyan_pad
                } else {
                    magenta_pad
                }
            } else {
                plaza_tint
            };
            engine.set_color_tint(h, tint);
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
            4.6,
        );
        engine.set_color_tint(h, [3.2, 1.55, 0.35, 1.0]);
        handles.push(h);
    }
    // Neon accent fixtures — same mesh, cool tint for anime night read.
    for (i, &(x, z, color)) in NEON_LIGHTS.iter().enumerate() {
        let pos = cell_to_world(IVec2::new(x, z));
        let h = add_static(
            engine,
            format!("neon-lamp-{i}"),
            "models/roads/light-square.glb",
            [pos.x, 0.05, pos.z],
            3.8,
        );
        engine.set_color_tint(h, [color[0] * 2.8, color[1] * 2.8, color[2] * 2.8, 1.0]);
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
    // One extra ring vs prior Tokyo-3 pass — taller / tighter canyon.
    let margin = if dense { 4 } else { 2 };

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
                    ring >= 2 && ring <= 4
                } else {
                    ring >= 1 && ring <= 4
                }
            } else {
                ring == 2 && (x + z) % 2 == 0
            };
            if !place {
                continue;
            }

            // Dark ground pad under each tower so dusk sky doesn't leak
            // through Kenney footprints as lilac voids.
            let pos = cell_to_world(IVec2::new(x, z));
            let pad = add_static(
                engine,
                format!("bld-pad-{i}"),
                "models/roads/tile-low.glb",
                [pos.x, -1.02, pos.z],
                CELL * 1.05,
            );
            engine.set_color_tint(pad, [0.22, 0.20, 0.24, 1.0]);
            handles.push(pad);

            let path = if ring >= 3 && (i % 4 == 0) {
                industrial[i % industrial.len()]
            } else if i % 7 == 0 {
                midrise[i % midrise.len()]
            } else {
                skyscrapers[i % skyscrapers.len()]
            };

            let scale = if path.contains("skyscraper") {
                let base = if dense { 1.95 } else { 1.35 };
                base + ((i % 5) as f32) * 0.22
            } else if path.contains("chimney") {
                1.65
            } else if dense {
                1.45
            } else {
                1.1
            };
            let h = add_static(
                engine,
                format!("bld-{i}"),
                path,
                [pos.x, 0.0, pos.z],
                scale,
            );
            // Night façade tints: cool dusk walls + occasional warm/cyan
            // "lit floor" boosts (shader also adds procedural window glitter).
            let tint = match i % 5 {
                0 => [0.62, 0.52, 0.40, 1.0],       // warm dusk wall
                1 => [0.48, 0.46, 0.52, 1.0],       // cool-neutral dusk
                2 => [0.72, 0.50, 0.38, 1.0],       // sodium-washed façade
                3 => [1.15, 0.78, 0.35, 1.0],       // lit warm floors
                _ => [0.45, 0.68, 0.78, 1.0],       // sparse cyan neon catch
            };
            engine.set_color_tint(h, tint);
            handles.push(h);
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
    // Angels exaggerate presence without new meshes: Mass bulk, Splinter spindly.
    match mech.alien {
        Some(AlienKind::Mass) => 0.68,
        Some(AlienKind::Splinter) => 0.28,
        None => 0.4,
    }
}

fn mech_tint(mech: &Mech, pulse: f32) -> [f32; 4] {
    // High gain + saturated team colors so mechs stay heroic against the
    // darker Tokyo-3 twilight (Quaternius albedo ~0.2–0.35 + Reinhard).
    // Cool player / warm enemy split reads through sodium + cyan neon.
    // Angels lean bone/crimson (Mass) and magenta-violet (Splinter) so they
    // do not read as recolored enemy mechs.
    match mech.alien {
        Some(AlienKind::Mass) => [2.55 * pulse, 1.65 * pulse, 1.35 * pulse, 1.0],
        Some(AlienKind::Splinter) => [1.65 * pulse, 0.85 * pulse, 2.75 * pulse, 1.0],
        None => match mech.team {
            Team::Player => [1.45 * pulse, 2.05 * pulse, 2.55 * pulse, 1.0],
            Team::Enemy => [2.55 * pulse, 1.2, 0.88, 1.0],
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
    let vis = MechVisual {
        handle,
        alien: mech.alien,
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
        hit_duration: 0.48,
        punch_dir: Vec3::Z,
        hit_dir: Vec3::Z,
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
/// pull into a tighter, more dramatic impact framing. `punch_t` is a
/// short extra kick at the moment the strike lands.
pub fn combat_camera(
    mission: &Mission,
    selected_player: u32,
    selected_enemy: u32,
    impact_t: f32,
    punch_t: f32,
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
    let punch = impact_envelope(punch_t, IMPACT_PUNCH_SECS);

    // Lower ¾ anime combat frame: stay inside the 8×8 so canyon walls + dusk
    // sky sillhouette the lane. Pull back enough that horizon glow reads
    // between towers; punch is a sharp FOV/dolly jolt (hit-stop beat).
    let dist = 7.8 - k * 1.6 - punch * 0.7;
    let jolt = (punch_t * 53.0).sin() * punch * 0.24;
    let side_off = 2.55 - k * 0.55 + jolt;
    let height = 1.75 - k * 0.2 + punch * 0.08;
    let fov = 0.82 + k * 0.12 + punch * 0.10;

    let eye = focus - along * dist + side * side_off + Vec3::Y * height;
    // Look toward upper façades / dusk band for anime street sillhouette.
    frame_camera(eye, focus + Vec3::Y * (1.05 + k * 0.45), fov)
}

/// Elevated city overview cameras — also used to seed FlyCam.
pub fn city_camera(mode: ViewMode) -> blade_engine::FrameCamera {
    match mode {
        ViewMode::CitySurface => {
            // Hero canyon overview: lower ¾ into the dusk street, sky glow
            // between towers (Eva city establishing beat).
            let eye = Vec3::new(-5.5, 8.2, -3.5);
            let focus = Vec3::new(7.5, 4.2, 7.5);
            frame_camera(eye, focus, 0.76)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_envelope_hot_at_onset() {
        assert!((impact_envelope(0.22, 0.22) - 1.0).abs() < 1e-5);
        assert!(impact_envelope(0.0, 0.22) < 0.01);
        let mid = impact_envelope(0.11, 0.22);
        assert!(mid > 0.20 && mid < 0.30, "mid={mid}");
    }

    #[test]
    fn angel_strike_palette_differs_from_mech() {
        let mech = strike_palette(None);
        let splinter = strike_palette(Some(AlienKind::Splinter));
        let mass = strike_palette(Some(AlienKind::Mass));
        assert!(mech.sodium_kick);
        assert!(!splinter.sodium_kick);
        assert!(!mass.sodium_kick);
        assert!(splinter.light[2] > splinter.light[1]);
        assert!(mass.light[0] > mass.light[2]);
        assert!(mech.light[1] > 0.5);
        assert!(splinter.telegraph_height > mass.telegraph_height);
    }

    #[test]
    fn angel_scale_presence() {
        let splinter = Mech::new_alien(1, AlienKind::Splinter, IVec2::ZERO);
        let mass = Mech::new_alien(2, AlienKind::Mass, IVec2::ZERO);
        let player = Mech::new_player(3, "Coil", IVec2::ZERO);
        assert!(mech_scale(&mass) > mech_scale(&player));
        assert!(mech_scale(&splinter) < mech_scale(&player));
    }
}
