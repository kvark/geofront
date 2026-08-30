//! Battle stage + city overview: Kenney surface / Space Kit underground,
//! Quaternius skinned GLB mechs (Idle/Walk/Punch/Death), street/hangar lights.

use std::collections::HashMap;

use glam::{IVec2, Mat4, Quat, Vec3};
use log::info;

use crate::combat::Mission;
use crate::units::{Facing, Mech, Team};

/// World units per tactical grid cell (matches Kenney road tile width).
pub const CELL: f32 = 2.0;

/// Punch clip + lunge + muzzle flash share one duration.
const PUNCH_SECS: f32 = 0.55;

/// Map grid cell → world position (Y-up). Feet on the road / floor surface.
pub fn cell_to_world(pos: IVec2) -> Vec3 {
    Vec3::new(pos.x as f32 * CELL, 0.0, pos.y as f32 * CELL)
}
