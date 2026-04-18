#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable, Default, Debug, ShaderType)]
#[repr(C)]
#[allow(unused)]
pub struct NeuronInstanceData {
    pub position: [f32; 3],
    pub scale: f32,
    pub color: [f32; 4],
}

#[derive(Resource, Default)]
pub struct NeuronInstances {
    pub data: Vec<NeuronInstanceData>,
    pub selected: Option<usize>,
}

#[derive(Resource, Default)]
pub struct TopologyGraph {
    pub padded_n: usize,
    pub targets: Vec<u32>,              //   dendrite_targets (Columnar)
    pub soma_to_axon: Vec<u32>,         //  soma_id -> axon_id
    pub axon_segments: Vec<Vec<Vec3>>,  //    
    pub soma_positions: Vec<Vec3>,      //  3D  
    pub traced_entity: Option<Entity>,  // ID    
    pub last_selected: Option<usize>,   //   
    pub compact_to_dense: Vec<usize>,   // :  UI-  RAM-
    pub axon_to_soma: Vec<usize>,      // :    
    pub global_axon_mat: Handle<StandardMaterial>, // : Handle     X-Ray 
    pub soma_mat: Handle<crate::systems::material::NeuronInstanceMaterial>, // : Handle  
}

#[derive(Component)]
pub struct ShardGeometry {
    pub viewport: Entity,
}

#[derive(Component)]
pub struct ViewportCamera {
    pub viewport: Entity,
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, // Rotation around Y
    pub beta: f32,  // Rotation up/down
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            viewport: Entity::PLACEHOLDER,
            target: Vec3::ZERO,
            radius: 40.0,
            alpha: std::f32::consts::PI / 4.0,
            beta: 0.5,
        }
    }
}

#[derive(Event, Clone, Debug)]
pub struct ZoneSelectedEvent {
    pub project_name: String,
    pub shard_name: String,
}

use bevy::render::mesh::MeshVertexAttribute;
use bevy::render::render_resource::VertexFormat;

pub const ATTRIBUTE_SPHERE_ID: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Sphere_Id", 1337, VertexFormat::Uint32);
