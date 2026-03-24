use bevy::prelude::*;
use bevy::render::render_resource::*;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CornerMaskMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(0)]
    pub pivot: Vec2,
}

impl UiMaterial for CornerMaskMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ui_corner_mask.wgsl".into()
    }
}
