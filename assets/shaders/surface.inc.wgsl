struct Surface {
    albedo: vec3<f32>,
    roughness: f32,
    normal: vec3<f32>,
    metallic: f32,
    emissive: vec3<f32>,
    occlusion: f32,
}

fn surface_eval_diffuse(s: Surface, n_dot_l: f32) -> vec3<f32> {
    return s.albedo * (1.0 - s.metallic) * max(n_dot_l, 0.0) / 3.14159265;
}
