use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{PluginWindow, draw_unified_header, base_domain, DOMAIN_ANATOMY_SLICER};
use crate::domain::AnatomySlicerState;
use node_editor::domain::BrainTopologyGraph;

pub fn render_anatomy_slicer_system(
    mut contexts: EguiContexts,
    windows: Query<(Entity, &PluginWindow)>,
    mut states: Query<&mut AnatomySlicerState>,
    graph: Res<BrainTopologyGraph>,
) {
    for (entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_ANATOMY_SLICER { continue; }
        
        let rtt_id = {
            let state = states.get(entity).ok();
            state.and_then(|s| s.shard_rtt.as_ref().map(|h| contexts.add_image(h.clone())))
        };

        let Some(ctx) = contexts.try_ctx_mut() else { return };
        let Ok(mut state) = states.get_mut(entity) else { continue };

        egui::Area::new(window.id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                let (content_rect, _) = draw_unified_header(ui, window.rect, "Shard Slicer");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    let full_rect = ui.available_rect_before_wrap();
                    
                    // 1. Main 3D Viewport (CAD)
                    if let Some(tex_id) = rtt_id {
                        ui.painter().image(tex_id, full_rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    } else {
                        ui.painter().rect_filled(full_rect, 0.0, egui::Color32::from_rgb(12, 12, 14));
                    }

                    state.cad_viewport_size = full_rect.size();
                    state.cad_viewport_rect = Some(full_rect);

                    // 2. Info Panel Overlay (Anatomy)
                    let panel_width = 220.0;
                    let panel_rect = egui::Rect::from_min_max(
                        egui::pos2(full_rect.max.x - panel_width - 12.0, full_rect.min.y + 12.0),
                        egui::pos2(full_rect.max.x - 12.0, full_rect.max.y - 12.0)
                    );

                    ui.allocate_ui_at_rect(panel_rect, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_premultiplied(18, 18, 20, 200))
                            .rounding(6.0)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(20)))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                let Some(zone) = &state.active_zone else {
                                    ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("No Shard Selected").color(egui::Color32::DARK_GRAY)); });
                                    return;
                                };

                                let Some(active_path) = &graph.active_path else { return; };
                                let Some(session) = graph.sessions.get(active_path) else { return; };
                                let Some(anatomy) = session.shard_anatomies.get(zone) else { return; };

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.label(egui::RichText::new(format!("ANATOMY: {}", zone)).color(egui::Color32::from_rgb(200, 150, 255)).strong());
                                    ui.add_space(8.0);

                                    for layer in &anatomy.layers {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&layer.name).color(egui::Color32::WHITE));
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{:.1}%", layer.height_pct * 100.0)).color(egui::Color32::GRAY));
                                            });
                                        });
                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(40, 40, 45));
                                        let mut fill_rect = rect;
                                        fill_rect.set_width(rect.width() * layer.height_pct);
                                        ui.painter().rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(100, 200, 255));
                                        ui.add_space(12.0);
                                    }
                                });
                            });
                    });
                });
            });
    }
}
