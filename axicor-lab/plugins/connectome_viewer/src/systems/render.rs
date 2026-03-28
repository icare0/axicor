use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::{PluginWindow, base_domain, DOMAIN_VIEWPORT};

pub fn render_connectome_viewer_system(
    mut contexts: EguiContexts,
    window_query: Query<&PluginWindow>,
) {
    // 1. Сначала собираем данные для отрисовки (и регистрируем текстуры), 
    // чтобы не конфликтовать с заимствованием ctx_mut()
    let mut render_items = Vec::new();
    for window in window_query.iter() {
        if !window.is_visible { continue; }
        if base_domain(&window.plugin_id) != DOMAIN_VIEWPORT { continue; }

        let texture_id = window.texture.as_ref().map(|handle| contexts.add_image(handle.clone()));
        render_items.push((window.id, window.rect, texture_id));
    }

    if render_items.is_empty() { return; }

    // 2. Только теперь берем контекст egui
    let Some(ctx) = contexts.try_ctx_mut() else { return; };

    for (id, rect, texture_id) in render_items {
        let area_id = format!("ConnectomePortal_{:?}", id);
        egui::Area::new(area_id.into())
            .fixed_pos(rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(rect);
                
                let (content_rect, _) = layout_api::draw_unified_header(ui, rect, "Connectome Viewer");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    if let Some(tid) = texture_id {
                        ui.add(
                            egui::Image::new(egui::load::SizedTexture::new(tid, content_rect.size()))
                                .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 10.0, se: 10.0 })
                        );
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new("VRAM Allocating...").color(egui::Color32::DARK_GRAY));
                        });
                    }
                });
            });
    }
}
