struct Surface {
    albedo: vec3<f32>,
    roughness: f32,
    normal: vec3<f32>,
    metallic: f32,
    motion: vec2<f32>,
    depth: f32,
}

fn read_surface(gbuf: texture_2d<f32>, depth: texture_depth_2d, coords: vec2<i32>) -> Surface {
    let g = textureLoad(gbuf, coords, 0);
    let d = textureLoad(depth, coords, 0);
    var s: Surface;
    s.albedo = g.xyz;
    s.roughness = g.w;
    // normal/motion packed in other targets in full pipeline
    s.normal = vec3(0.0, 1.0, 0.0);
    s.metallic = 0.0;
    s.motion = vec2(0.0);
    s.depth = d;
    return s;
}
