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
    for (entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_IO_INSPECTOR { continue; }
        
        let Some(ctx) = contexts.try_ctx_mut() else { return };
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
                        .inner_margin(8.0)
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
                                for port in &inputs { draw_capsule(ui, zone, port, true); ui.add_space(2.0); }
                                
                                if !inputs.is_empty() && !outputs.is_empty() {
                                    ui.add_space(6.0);
                                    ui.painter().hline(ui.cursor().min.x..=ui.max_rect().max.x, ui.cursor().min.y, egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45)));
                                    ui.add_space(10.0);
                                }
                                
                                for port in &outputs { draw_capsule(ui, zone, port, false); ui.add_space(2.0); }
                            });
                        });
                });
            });
    }

    // Глобальный рендер летящего шнура (Cross-Plugin Cable)
    let payload_id = egui::Id::new("io_wire_drag");
    let Some(ctx) = contexts.try_ctx_mut() else { return };
    if let Some(payload) = ctx.memory(|m| m.data.get_temp::<IoWirePayload>(payload_id)) {
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cable_overlay")));
            let start = payload.start_pos;
            let end = mouse_pos;
            
            let cp1 = start + egui::vec2(50.0, 0.0);
            let cp2 = end - egui::vec2(50.0, 0.0);
            
            let stroke_color = if payload.is_input { egui::Color32::from_rgb(50, 160, 80) } else { egui::Color32::from_rgb(180, 60, 60) };
            
            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                points: [start, cp1, cp2, end],
                closed: false,
                fill: egui::Color32::TRANSPARENT,
                stroke: egui::Stroke::new(2.5, stroke_color.gamma_multiply(0.6)),
            }));
            
            painter.circle_filled(end, 4.0, egui::Color32::WHITE);
        }
    }
}

fn draw_capsule(ui: &mut egui::Ui, zone: &str, name: &str, is_input: bool) {
    let pin_color = if is_input { egui::Color32::from_rgb(0, 255, 100) } else { egui::Color32::from_rgb(255, 100, 0) };
    let bg_color = egui::Color32::from_rgb(30, 30, 32);
    let stroke_color = egui::Color32::from_rgb(45, 45, 48);

    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::click_and_drag());

    // Фон капсулы
    let visual_bg = if response.hovered() { egui::Color32::from_rgb(40, 40, 45) } else { bg_color };
    ui.painter().rect_filled(rect, 4.0, visual_bg);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, stroke_color));

    // Отрисовка Пина (Индикатора) - Всегда СПРАВА
    let pin_radius = 3.5;
    let pin_center = egui::pos2(rect.max.x - 10.0, rect.center().y);
    ui.painter().circle_filled(pin_center, pin_radius, pin_color);

    // Отрисовка Текста - Всегда СЛЕВА
    let text_pos = egui::pos2(rect.min.x + 8.0, rect.center().y);
    ui.painter().text(text_pos, egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), egui::Color32::from_rgb(200, 200, 200));

    // Логика DND
    let payload_id = egui::Id::new("io_wire_drag");
    if response.drag_started() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(payload_id, IoWirePayload {
            zone: zone.to_string(),
            port: name.to_string(),
            is_input,
            start_pos: pin_center,
        }));
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if response.drag_stopped() {
        ui.ctx().memory_mut(|m| m.data.remove_temp::<IoWirePayload>(payload_id));
    }
}
