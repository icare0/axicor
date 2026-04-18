use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{PluginWindow, draw_unified_header, base_domain, DOMAIN_MATRIX_EDITOR};
use crate::domain::MatrixEditorState;

pub fn render_matrix_editor_system(
    mut contexts: EguiContexts,
    windows: Query<(Entity, &PluginWindow)>,
    _states: Query<&MatrixEditorState>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (_entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_MATRIX_EDITOR { continue; }

        egui::Area::new(window.id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                let (content_rect, _) = draw_unified_header(ui, window.rect, "Matrix Editor");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(" UI in Quarantine \nWaiting for UX overhaul.")
                                .color(egui::Color32::DARK_GRAY)
                                .size(16.0)
                        );
                    });
                });
            });
    }
}
