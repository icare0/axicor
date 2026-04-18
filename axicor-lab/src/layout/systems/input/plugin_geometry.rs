use bevy::prelude::*;
use bevy_egui::egui;
use bevy::render::render_resource::Extent3d;
use crate::layout::domain::WindowDragState;
use crate::layout::systems::window::{create_plugin_render_target, spawn_pane_entity};
use layout_api::{PluginWindow, PluginGeometry, AllocatedPanes};
use bevy::utils::HashSet;

const MIN_TEXTURE_SIZE: f32 = 10.0;

pub fn sync_plugin_geometry_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    drag_state: Res<WindowDragState>,
    allocated: Res<AllocatedPanes>,
    mut query: Query<(Entity, &mut PluginWindow, &mut PluginGeometry)>,
    mut known_ids: Local<HashSet<String>>, // DOD FIX: Zero-allocation capacity retention
) {
    if drag_state.is_dragging { return; }

    known_ids.clear(); //    

    for (_, mut window, mut geom) in query.iter_mut() {
        known_ids.insert(window.plugin_id.clone()); //  ,    

        let Some(rect) = allocated.rects.get(&window.plugin_id) else { continue };

        window.rect = *rect;
        window.id = egui::Id::new(&window.plugin_id);
        geom.size = Vec2::new(rect.width(), rect.height());
        if geom.size.x < MIN_TEXTURE_SIZE || geom.size.y < MIN_TEXTURE_SIZE { continue; }

        let tx = geom.size.x as u32;
        let ty = geom.size.y as u32;

        match &window.texture {
            Some(handle) => {
                if let Some(image) = images.get_mut(handle) {
                    let s = image.texture_descriptor.size;
                    if s.width != tx || s.height != ty {
                        image.resize(Extent3d { width: tx, height: ty, depth_or_array_layers: 1 });
                    }
                }
            }
            None => {
                window.texture = Some(create_plugin_render_target(&mut images, tx, ty));
            }
        }
    }

    for (plugin_id, rect) in &allocated.rects {
        if known_ids.contains(plugin_id) { continue; }
        info!("[WM] Auto-spawning missing ECS entity for: {}", plugin_id);
        spawn_pane_entity(&mut commands, &mut images, plugin_id, rect.width(), rect.height());
    }
}
