// Encode a linear value with the sRGB transfer function.
//
// Only needed when writing to a non-sRGB texture (or the swapchain without sRGB format).
fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        return linear * 12.92;
    }
    return 1.055 * pow(linear, 1.0 / 2.4) - 0.055;
}

fn linear_to_srgb_vec3(linear: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb(linear.x),
        linear_to_srgb(linear.y),
        linear_to_srgb(linear.z),
    );
}
