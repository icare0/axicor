use bevy::prelude::*;
use bevy::render::render_resource::*;
use crate::domain::NeuronInstanceData;

use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use crate::domain::ATTRIBUTE_SPHERE_ID;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct NeuronInstanceMaterial {
    #[storage(0, read_only)]
    pub instances: Vec<NeuronInstanceData>,
}

impl Material for NeuronInstanceMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/instancing.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/instancing.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        //   :     u32 ID.
        //    UV.
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_SPHERE_ID.at_shader_location(1),
        ])?;
        
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
