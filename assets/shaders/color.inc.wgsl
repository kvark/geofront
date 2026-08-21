// Encode a linear value with the sRGB transfer function
fn encode_srgb(v: f32) -> f32 {
    return select(1.055 * pow(v, 1.0 / 2.4) - 0.055, v * 12.92, v <= 0.0031308);
}

fn encode_rgb(v: vec3<f32>) -> vec3<f32> {
    return vec3(encode_srgb(v.x), encode_srgb(v.y), encode_srgb(v.z));
}

fn decode_srgb(v: f32) -> f32 {
    return select(pow((v + 0.055) / 1.055, 2.4), v / 12.92, v <= 0.04045);
}

fn decode_rgb(v: vec3<f32>) -> vec3<f32> {
    return vec3(decode_srgb(v.x), decode_srgb(v.y), decode_srgb(v.z));
}
