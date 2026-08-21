#include "brdf.inc.wgsl"
#include "color.inc.wgsl"

struct RasterFrameParams {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // direction towards the light
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    // w component is a flag for the procedural space sky
    ambient_color: vec4<f32>,
    // x: environment map enabled, y: the surface needs sRGB encoding
    settings: vec4<f32>,
}

struct RasterDrawParams {
    model: mat4x4<f32>,
    normal_quat: vec4<f32>,
    base_color_factor: vec4<f32>,
    emissive_factor: vec4<f32>,
    // x: metallic, y: roughness
    material_factors: vec4<f32>,
    // uv transform
    uv_transform: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: RasterFrameParams;
@group(1) @binding(0) var<uniform> draw: RasterDrawParams;
@group(1) @binding(1) var base_color_texture: texture_2d<f32>;
@group(1) @binding(2) var base_color_sampler: sampler;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var normal_sampler: sampler;
@group(2) @binding(0) var env_map: texture_cube<f32>;
@group(2) @binding(1) var env_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) tc: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) tc: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = (draw.model * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_pos = frame.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    let n = qrot(draw.normal_quat, in.normal);
    let t = qrot(draw.normal_quat, in.tangent.xyz);
    out.normal = n;
    out.tangent = t;
    out.bitangent = in.tangent.w * cross(n, t);
    out.tc = in.tc * draw.uv_transform.xy + draw.uv_transform.zw;
    return out;
}

fn encode_surface_color(color: vec3<f32>, do_srgb: bool) -> vec3<f32> {
    if (do_srgb) {
        return encode_rgb(color);
    }
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(base_color_texture, base_color_sampler, in.tc);
    let albedo = base.xyz * draw.base_color_factor.xyz;
    let metallic = draw.material_factors.x;
    let roughness = draw.material_factors.y;

    var N = normalize(in.normal);
    let T = normalize(in.tangent);
    let B = normalize(in.bitangent);
    let nmap = textureSample(normal_texture, normal_sampler, in.tc).xyz * 2.0 - 1.0;
    N = normalize(mat3x3(T, B, N) * nmap);

    let V = normalize(frame.camera_pos.xyz - in.world_pos);
    let L = normalize(frame.light_dir.xyz);
    let H = normalize(V + L);

    let NdotL = max(dot(N, L), 0.0);
    let NdotV = max(dot(N, V), 0.001);
    let NdotH = max(dot(N, H), 0.0);
    let VdotH = max(dot(V, H), 0.0);

    let f0 = mix(vec3(0.04), albedo, metallic);
    let diffuse = albedo * (1.0 - metallic) / 3.14159265;
    let spec = brdf_specular(f0, roughness, NdotH, NdotV, NdotL, VdotH);

    var color = (diffuse + spec) * frame.light_color.xyz * NdotL;
    color += albedo * frame.ambient_color.xyz;
    color += draw.emissive_factor.xyz;

    if (frame.settings.x > 0.5) {
        let R = reflect(-V, N);
        let env = textureSample(env_map, env_sampler, R).xyz;
        color += env * f0 * (1.0 - roughness);
    }

    // simple tone map
    let mapped = color / (color + vec3(1.0));
    return vec4<f32>(encode_surface_color(mapped, frame.settings.y > 0.5), 1.0);
}
