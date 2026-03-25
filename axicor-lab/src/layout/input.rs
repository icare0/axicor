use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use crate::layout::data::{PluginWindow, PluginGeometry, WindowDragState};

pub fn sync_plugin_geometry_system(
    mut images: ResMut<Assets<Image>>,
    drag_state: Res<WindowDragState>,
    // DOD FIX: Запрашиваем мутабельную камеру для инъекции таргета
    mut query: Query<(Entity, &mut PluginWindow, &PluginGeometry, Option<&mut Camera>)>,
) {
    if drag_state.is_dragging { return; }

    for (_entity, mut window, geom, mut camera_opt) in query.iter_mut() {
        if geom.size.x < 100.0 || geom.size.y < 100.0 { continue; } 

        let target_x = geom.size.x as u32;
        let target_y = geom.size.y as u32;

        if window.texture.is_none() {
            // 1. Первая аллокация VRAM-буфера
            let handle = crate::layout::systems::create_plugin_render_target(&mut images, target_x, target_y);
            window.texture = Some(handle.clone());

            // 2. Жесткая привязка 3D-Оптики к локальному буферу (Zero Overlap)
            if let Some(mut cam) = camera_opt {
                cam.target = RenderTarget::Image(handle);
                cam.is_active = true; // Воскрешаем камеру: теперь она рисует строго в свою текстуру
            }
        } else if let Some(handle) = &window.texture {
            // 3. Штатный ресайз
            if let Some(image) = images.get_mut(handle) {
                let current_size = image.texture_descriptor.size;
                if current_size.width != target_x || current_size.height != target_y {
                    image.resize(bevy::render::render_resource::Extent3d {
                        width: target_x,
                        height: target_y,
                        depth_or_array_layers: 1,
                    });
                }
            }
        }
    }
}
