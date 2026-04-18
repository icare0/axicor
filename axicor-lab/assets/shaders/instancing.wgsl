#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::view

struct NeuronInstanceData {
    position: vec3<f32>,
    scale: f32,
    color: vec4<f32>,
};

@group(2) @binding(0)
var<storage, read> instances: array<NeuronInstanceData>;

//      ATTRIBUTE_SPHERE_ID
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) sphere_id: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    //      O(1) lookup
    let instance = instances[vertex.sphere_id];

    //    
    let local_pos = (vertex.position * instance.scale) + instance.position;

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(get_model_matrix(0u), vec4<f32>(local_pos, 1.0));
    out.color = instance.color;
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
