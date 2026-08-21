//! Blade scene setup, instancing for buildings/mechs, damage visuals, camera.

/// Thin wrapper around Blade engine scene for the city + mechs.
///
/// Next step: hold `blade_engine::Engine`, create a simple orthographic or
/// top-down camera looking at the combat grid, and instance placeholder
/// meshes (or just clear + egui) until real mech/city assets exist.
pub struct Scene {
    // engine: Option<blade_engine::Engine>,
    // camera: blade_engine::FrameCamera,
}

impl Scene {
    pub fn new() -> Self {
        Self {}
    }

    /// Placeholder for a static top-down camera aimed at the centre of an 8x8 grid.
    pub fn combat_camera() -> blade_engine::FrameCamera {
        // Looking down at roughly (3.5, 0, 3.5) from above.
        blade_engine::FrameCamera {
            transform: blade_engine::Transform {
                position: [3.5, 12.0, 3.5].into(),
                orientation: glam::Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2).into(),
            },
            fov_y: 0.9,
        }
    }
}
