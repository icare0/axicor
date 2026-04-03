use bevy::prelude::*;
use layout_api::{DOMAIN_BLUEPRINT_EDITOR, PluginWindow, draw_unified_header};

pub struct BlueprintEditorPlugin;

impl Plugin for BlueprintEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render_blueprint_editor_system);
    }
}

fn render_blueprint_editor_system(
    mut contexts: bevy_egui::EguiContexts,
    query: Query<&PluginWindow>,
) {
    let ctx = contexts.ctx_mut();
    for window in query.iter() {
        if window.plugin_id.starts_with(DOMAIN_BLUEPRINT_EDITOR) && window.is_visible {
            bevy_egui::egui::Area::new(window.id)
                .fixed_pos(window.rect.min)
                .show(ctx, |ui| {
                    let rect = window.rect;
                    let (content_rect, _toolbar_rect) = draw_unified_header(ui, rect, "Neuron Settings");
                    
                    ui.allocate_ui_at_rect(content_rect, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(bevy_egui::egui::RichText::new("Neuron Settings Placeholder").color(bevy_egui::egui::Color32::GRAY));
                        });
                    });
                });
        }
    }
}
