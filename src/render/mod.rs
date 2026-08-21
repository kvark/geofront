//! Blade scene setup, instancing for buildings/mechs, damage visuals, camera.

/// Thin wrapper / helpers around Blade for the combat view.
pub struct Scene;

impl Scene {
    /// Top-down camera looking at the centre of an 8×8 grid.
    pub fn combat_camera() -> blade_engine::FrameCamera {
        // Position high above the board, looking straight down.
        let eye = glam::Vec3::new(3.5, 14.0, 3.5);
        let target = glam::Vec3::new(3.5, 0.0, 3.5);
        let up = glam::Vec3::Z; // keep a consistent "north"
        let view = glam::Mat4::look_at_rh(eye, target, up);
        let world = view.inverse();
        let (_, rot, trans) = world.to_scale_rotation_translation();
        blade_engine::FrameCamera {
            transform: blade_engine::Transform {
                position: trans.into(),
                orientation: rot.into(),
            },
            fov_y: 0.85,
        }
    }
}
