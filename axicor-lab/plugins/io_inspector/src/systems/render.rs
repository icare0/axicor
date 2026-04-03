use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{PluginWindow, draw_unified_header, base_domain, DOMAIN_IO_INSPECTOR, IoWirePayload};
use crate::domain::IoInspectorState;
use node_editor::domain::BrainTopologyGraph;

pub fn render_io_inspector_system(
    mut contexts: EguiContexts,
    windows: Query<(Entity, &PluginWindow)>,
    mut states: Query<&mut IoInspectorState>,
    graph: Res<BrainTopologyGraph>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_IO_INSPECTOR { continue; }
        let Ok(state) = states.get_mut(entity) else { continue };

        egui::Area::new(window.id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                let (content_rect, _) = draw_unified_header(ui, window.rect, "I/O Router");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(18, 18, 20))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let Some(zone) = &state.active_zone else {
                                ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("No Shard Selected").color(egui::Color32::DARK_GRAY)); });
                                return;
                            };

                            let Some(active_path) = &graph.active_path else { return; };
                            let Some(session) = graph.sessions.get(active_path) else { return; };

                            let inputs = session.node_inputs.get(zone).cloned().unwrap_or_default();
                            let outputs = session.node_outputs.get(zone).cloned().unwrap_or_default();

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.label(egui::RichText::new("INPUT MATRICES").color(egui::Color32::from_rgb(100, 200, 255)).strong());
                                ui.add_space(6.0);
                                for port in &inputs { draw_capsule(ui, zone, port, true); ui.add_space(4.0); }

                                ui.add_space(24.0);

                                ui.label(egui::RichText::new("OUTPUT MATRICES").color(egui::Color32::from_rgb(255, 150, 50)).strong());
                                ui.add_space(6.0);
                                for port in &outputs { draw_capsule(ui, zone, port, false); ui.add_space(4.0); }
                            });
                        });
                });
            });
    }

    // Глобальный рендер летящего шнура (Cross-Plugin Cable)
    let payload_id = egui::Id::new("io_wire_drag");
    if let Some(payload) = ctx.memory(|m| m.data.get_temp::<IoWirePayload>(payload_id)) {
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cable_overlay")));
            let start = payload.start_pos;
            let end = mouse_pos;
            
            // Кривая Безье для шнура
            let cp1 = start + egui::vec2(50.0, 0.0);
            let cp2 = end - egui::vec2(50.0, 0.0);
            
            let stroke_color = if payload.is_input { egui::Color32::from_rgb(50, 160, 80) } else { egui::Color32::from_rgb(180, 60, 60) };
            
            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                points: [start, cp1, cp2, end],
                closed: false,
                fill: egui::Color32::TRANSPARENT,
                stroke: egui::Stroke::new(2.5, stroke_color.gamma_multiply(0.6)),
            }));
            
            // Светящаяся точка на конце
            painter.circle_filled(end, 4.0, egui::Color32::WHITE);
        }
    }
}

fn draw_capsule(ui: &mut egui::Ui, zone: &str, name: &str, is_input: bool) {
    let bg_color = if is_input { egui::Color32::from_rgb(25, 45, 35) } else { egui::Color32::from_rgb(45, 25, 25) };
    let stroke_color = if is_input { egui::Color32::from_rgb(50, 160, 80) } else { egui::Color32::from_rgb(180, 60, 60) };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click_and_drag());

    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, egui::Color32::from_rgb(50, 50, 55));
    } else {
        ui.painter().rect_filled(rect, 6.0, bg_color);
    }
    ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(1.0, stroke_color));

    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(13.0), egui::Color32::WHITE);

    // [DOD FIX] Логика DND: запись в глобальный блэкборд egui
    let payload_id = egui::Id::new("io_wire_drag");
    if response.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(payload_id, IoWirePayload {
            zone: zone.to_string(),
            port: name.to_string(),
            is_input,
            start_pos: rect.center(),
        }));
    }
    if response.drag_stopped() {
        ui.ctx().memory_mut(|m| m.data.remove::<IoWirePayload>(payload_id));
    }
}
