use bevy::prelude::*;
use layout_api::{DOMAIN_ANATOMY_SLICER, PluginWindow, draw_unified_header};

pub struct AnatomySlicerPlugin;

impl Plugin for AnatomySlicerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render_anatomy_slicer_system);
    }
}

fn render_anatomy_slicer_system(
    mut contexts: bevy_egui::EguiContexts,
    query: Query<&PluginWindow>,
) {
    let ctx = contexts.ctx_mut();
    for window in query.iter() {
        if window.plugin_id.starts_with(DOMAIN_ANATOMY_SLICER) && window.is_visible {
            bevy_egui::egui::Area::new(window.id)
                .fixed_pos(window.rect.min)
                .show(ctx, |ui| {
                    let rect = window.rect;
                    let (content_rect, _toolbar_rect) = draw_unified_header(ui, rect, "Shard Slicer");
                    
                    ui.allocate_ui_at_rect(content_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(bevy_egui::egui::RichText::new("Shard Slicer Placeholder").color(bevy_egui::egui::Color32::GRAY));
                        });
                    });
                });
        }
    }
}
