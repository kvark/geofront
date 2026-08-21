struct CameraParams {
    position: vec3<f32>,
    depth: f32,
    orientation: vec4<f32>,
    fov: vec2<f32>,
    target_size: vec2<u32>,
}

fn get_ray_direction(ndc: vec2<f32>, cam: CameraParams) -> vec3<f32> {
    let basis = quaternion_to_matrix(cam.orientation);
    let dir_cam = vec3<f32>(
        ndc.x * cam.fov.x,
        ndc.y * cam.fov.y,
        -1.0,
    );
    return normalize(basis * dir_cam);
}

fn screen_to_ndc(pixel: vec2<f32>, target_size: vec2<u32>) -> vec2<f32> {
    let size = vec2<f32>(target_size);
    return vec2<f32>(
        (pixel.x + 0.5) / size.x * 2.0 - 1.0,
        1.0 - (pixel.y + 0.5) / size.y * 2.0,
    );
}
