struct CameraParams {
    position: vec3<f32>,
    depth_range: vec2<f32>,
    orientation: vec4<f32>,
    fov: vec2<f32>,
    target_size: vec2<u32>,
}

struct ScreenRay {
    origin: vec3<f32>,
    dir: vec3<f32>,
}

fn get_ray(pixel: vec2<f32>, camera: CameraParams) -> ScreenRay {
    let half_size = 0.5 * vec2<f32>(camera.target_size);
    let ndc = (pixel - half_size) / half_size;
    let cs_near = vec4<f32>(ndc * camera.fov, -1.0, 1.0);
    // use inverse transform of the camera without projection matrix
    let ws_near = qrot(camera.orientation, cs_near.xyz) + camera.position;
    return ScreenRay(camera.position, normalize(ws_near - camera.position));
}

fn get_projection_matrix(camera: CameraParams) -> mat4x4<f32> {
    let m00 = 1.0 / camera.fov.x;
    let m11 = 1.0 / camera.fov.y;
    let m22 = camera.depth_range.y / (camera.depth_range.x - camera.depth_range.y);
    let m23 = camera.depth_range.x * camera.depth_range.y / (camera.depth_range.x - camera.depth_range.y);
    return mat4x4(
        m00, 0.0, 0.0, 0.0,
        0.0, m11, 0.0, 0.0,
        0.0, 0.0, m22, -1.0,
        0.0, 0.0, m23, 0.0,
    );
}

fn get_view_matrix(camera: CameraParams) -> mat4x4<f32> {
    let inv_orient = qinv(camera.orientation);
    let basis = mat3x3(
        qrot(inv_orient, vec3(1.0, 0.0, 0.0)),
        qrot(inv_orient, vec3(0.0, 1.0, 0.0)),
        qrot(inv_orient, vec3(0.0, 0.0, 1.0)),
    );
    return mat4x4(
        vec4(basis[0], 0.0),
        vec4(basis[1], 0.0),
        vec4(basis[2], 0.0),
        vec4(-camera.position * basis, 1.0),
    );
}
