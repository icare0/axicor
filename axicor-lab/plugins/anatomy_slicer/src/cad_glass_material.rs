use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::mesh::MeshVertexBufferLayout,
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct CadGlassMaterial {
    #[uniform(0)]
    pub color: Color,
}

impl Material for CadGlassMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cad_glass.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/cad_glass.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // [DOD FIX]
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
