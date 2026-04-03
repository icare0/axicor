use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use crate::domain::AnatomySlicerState;

pub fn allocate_vram_system(
    mut query: Query<&mut AnatomySlicerState>,
    mut images: ResMut<Assets<Image>>,
) {
    for mut state in query.iter_mut() {
        if state.active_zone.is_none() || state.shard_rtt.is_some() || state.cad_viewport_size.x <= 10.0 {
            continue;
        }

        let target_w = state.cad_viewport_size.x as u32;
        let target_h = state.cad_viewport_size.y as u32;
        let size = Extent3d { width: target_w, height: target_h, depth_or_array_layers: 1 };

        let mut image = Image::new_fill(
            size, TextureDimension::D2, &[1, 2, 3, 255], TextureFormat::Bgra8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        state.shard_rtt = Some(images.add(image));
        
        info!("[VRAM] Allocated {}x{} for CAD Inspector", target_w, target_h);
    }
}

pub fn sync_vram_system(
    query: Query<&AnatomySlicerState>,
    mut images: ResMut<Assets<Image>>,
) {
    for state in query.iter() {
        if let Some(handle) = &state.shard_rtt {
            let size = state.cad_viewport_size;
            if size.x <= 10.0 || size.y <= 10.0 { continue; }

            let target_w = size.x as u32;
            let target_h = size.y as u32;

            let mut needs_resize = false;
            if let Some(image) = images.get(handle) {
                if image.texture_descriptor.size.width != target_w || image.texture_descriptor.size.height != target_h {
                    needs_resize = true;
                }
            }

            if needs_resize {
                if let Some(image) = images.get_mut(handle) {
                    image.resize(Extent3d { width: target_w, height: target_h, depth_or_array_layers: 1 });
                }
            }
        }
    }
}
