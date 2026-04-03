use bevy::prelude::*;
use layout_api::{DOMAIN_MATRIX_EDITOR, PluginWindow, draw_unified_header};

pub struct MatrixEditorPlugin;

impl Plugin for MatrixEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render_matrix_editor_system);
    }
}

fn render_matrix_editor_system(
    mut contexts: bevy_egui::EguiContexts,
    query: Query<&PluginWindow>,
) {
    let ctx = contexts.ctx_mut();
    for window in query.iter() {
        if window.plugin_id.starts_with(DOMAIN_MATRIX_EDITOR) && window.is_visible {
            bevy_egui::egui::Area::new(window.id)
                .fixed_pos(window.rect.min)
                .order(bevy_egui::egui::Order::Middle)
                .show(ctx, |ui| {
                    ui.set_clip_rect(window.rect);
                    let (content_rect, _) = draw_unified_header(ui, window.rect, "Matrix Editor");

                    ui.allocate_ui_at_rect(content_rect, |ui| {
                        bevy_egui::egui::Frame::none()
                            .fill(bevy_egui::egui::Color32::from_rgb(18, 18, 20))
                            .show(ui, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(bevy_egui::egui::RichText::new("Matrix Editor Placeholder").color(bevy_egui::egui::Color32::GRAY));
                                });
                            });
                    });
                });
        }
    }
}
