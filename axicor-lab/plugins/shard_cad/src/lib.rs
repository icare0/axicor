use bevy::prelude::*;
use layout_api::{DOMAIN_SHARD_CAD, PluginWindow, draw_unified_header};

pub struct ShardCadPlugin;

impl Plugin for ShardCadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render_shard_cad_system);
    }
}

fn render_shard_cad_system(
    mut contexts: bevy_egui::EguiContexts,
    query: Query<&PluginWindow>,
) {
    let ctx = contexts.ctx_mut();
    for window in query.iter() {
        if window.plugin_id.starts_with(DOMAIN_SHARD_CAD) && window.is_visible {
            bevy_egui::egui::Area::new(window.id)
                .fixed_pos(window.rect.min)
                .show(ctx, |ui| {
                    let rect = window.rect;
                    let (content_rect, _toolbar_rect) = draw_unified_header(ui, rect, "Shard CAD");
                    
                    ui.allocate_ui_at_rect(content_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(bevy_egui::egui::RichText::new("Shard CAD Placeholder").color(bevy_egui::egui::Color32::GRAY));
                        });
                    });
                });
        }
    }
}
