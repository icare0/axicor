use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use crate::layout::data::{PluginGeometry, PluginWindow, WindowDragState};

pub fn sync_plugin_geometry_system(
    mut images: ResMut<Assets<Image>>,
    drag_state: Res<WindowDragState>,
    query: Query<(&PluginWindow, &PluginGeometry)>,
) {
    // DOD: Debounce. Жестко блокируем реаллокацию VRAM во время движения сплиттеров.
    if drag_state.is_dragging { return; }

    for (window, geom) in query.iter() {
        // Clamp protection: WGPU паникует при создании текстуры нулевого размера
        if geom.size.x < 100.0 || geom.size.y < 100.0 { continue; } 

        if let Some(handle) = &window.texture {
            if let Some(image) = images.get_mut(handle) {
                let current_size = image.texture_descriptor.size;
                let target_x = geom.size.x as u32;
                let target_y = geom.size.y as u32;

                // Сравниваем физический размер текстуры с реальными габаритами окна
                if current_size.width != target_x || current_size.height != target_y {
                    image.resize(Extent3d {
                        width: target_x,
                        height: target_y,
                        depth_or_array_layers: 1,
                    });
                }
            }
        }
    }
}
