#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

struct CadGlassMaterial {
    color: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> material: CadGlassMaterial;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vertex(
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let model = get_model_matrix(instance_index);
    let world_pos = model * vec4<f32>(position, 1.0);
    out.clip_position = view.view_proj * world_pos;
    out.world_normal = normalize((model * vec4<f32>(normal, 0.0)).xyz);
    out.world_position = world_pos.xyz;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let cam_pos = view.world_position.xyz;
    let view_dir = normalize(cam_pos - in.world_position);
    let normal = normalize(in.world_normal);

    // Fresnel   ,  
    let fresnel = pow(1.0 - abs(dot(normal, view_dir)), 3.0);

    //    Rust (material.color.a)
    let base_alpha = material.color.a;
    let alpha = clamp(base_alpha + fresnel * 0.5, 0.0, 1.0);

    //      
    let edge_highlight = vec3<f32>(1.0) * fresnel * 0.4;
    let col = material.color.rgb + edge_highlight;

    return vec4<f32>(col, alpha);
}
