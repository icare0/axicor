use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::input::ButtonInput;
use crate::layout::data::{PluginGeometry, PluginWindow};

pub fn sync_plugin_geometry_system(
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&PluginGeometry, &PluginWindow, &mut Projection), Changed<PluginGeometry>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // DOD: Block VRAM reallocation in Hot Loop during window resizing
    if mouse_input.pressed(MouseButton::Left) {
        return; 
    }

    for (geometry, plugin, mut projection) in query.iter_mut() {
        let size = geometry.size;
        if size.x <= 2.0 || size.y <= 2.0 { continue; }

        // 1. Update Perspective Projection
        if let Projection::Perspective(ref mut perspective) = *projection {
            perspective.aspect_ratio = size.x / size.y;
        }

        // 2. Resize RTT Image
        if let Some(texture_handle) = &plugin.texture {
            if let Some(image) = images.get_mut(texture_handle) {
                let current_size = image.texture_descriptor.size;
                if current_size.width != size.x as u32 || current_size.height != size.y as u32 {
                    image.resize(Extent3d {
                        width: size.x as u32,
                        height: size.y as u32,
                        depth_or_array_layers: 1,
                    });
                }
            }
        }
    }
}
