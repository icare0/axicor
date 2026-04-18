use crate::domain::IoInspectorState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{
    base_domain, draw_unified_header, IoWirePayload, PluginWindow, DOMAIN_IO_INSPECTOR,
};
use node_editor::domain::BrainTopologyGraph;

pub fn render_io_inspector_system(
    mut contexts: EguiContexts,
    windows: Query<(Entity, &PluginWindow)>,
    mut states: Query<&mut IoInspectorState>,
    graph: Res<BrainTopologyGraph>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else {
        return;
    };

    for (entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_IO_INSPECTOR {
            continue;
        }
        let Ok(state) = states.get_mut(entity) else {
            continue;
        };

        egui::Area::new(window.id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                let (content_rect, _) = draw_unified_header(ui, window.rect, "I/O Router");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 20, 22))
                        .show(ui, |ui| {
                            let Some(active_path) = graph.active_path.as_ref() else {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new("No active project")
                                            .color(egui::Color32::DARK_GRAY),
                                    );
                                });
                                return;
                            };
                            let Some(session) = graph.sessions.get(active_path) else {
                                return;
                            };
                            let zone = state.active_zone.as_deref().unwrap_or("");

                            if zone.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new("Select a Shard in Node Editor")
                                            .color(egui::Color32::DARK_GRAY),
                                    );
                                });
                                return;
                            }

                            // [DOD FIX]     RAM-
                            let io_data = session.shard_io.get(zone);

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if let Some(io) = io_data {
                                    // --- INPUTS ---
                                    if !io.input.is_empty() {
                                        ui.label(
                                            egui::RichText::new("INPUTS")
                                                .color(egui::Color32::from_rgb(100, 200, 255))
                                                .small(),
                                        );
                                        for matrix in &io.input {
                                            ui.collapsing(format!("M: {}", matrix.name), |ui| {
                                                for pin in &matrix.pin {
                                                    draw_capsule(ui, zone, &pin.name, true);
                                                    ui.add_space(2.0);
                                                }
                                            });
                                        }
                                        ui.add_space(8.0);
                                    }

                                    // --- OUTPUTS ---
                                    if !io.output.is_empty() {
                                        ui.label(
                                            egui::RichText::new("OUTPUTS")
                                                .color(egui::Color32::from_rgb(255, 150, 50))
                                                .small(),
                                        );
                                        for matrix in &io.output {
                                            ui.collapsing(format!("M: {}", matrix.name), |ui| {
                                                for pin in &matrix.pin {
                                                    draw_capsule(ui, zone, &pin.name, false);
                                                    ui.add_space(2.0);
                                                }
                                            });
                                        }
                                    }
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            egui::RichText::new("No I/O Data available")
                                                .color(egui::Color32::DARK_GRAY),
                                        );
                                    });
                                }
                            });
                        });
                });
            });
    }

    //
    render_flying_wire(ctx);
}

fn draw_capsule(ui: &mut egui::Ui, zone: &str, name: &str, is_input: bool) {
    let pin_color = if is_input {
        egui::Color32::from_rgb(0, 255, 100)
    } else {
        egui::Color32::from_rgb(255, 100, 0)
    };
    let bg_color = egui::Color32::from_rgb(30, 30, 32);
    let stroke_color = egui::Color32::from_rgb(45, 45, 48);

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 22.0),
        egui::Sense::click_and_drag(),
    );

    // [DOD FIX]
    if response.clicked() {
        ui.ctx().memory_mut(|m| {
            m.data.insert_temp(
                egui::Id::new(layout_api::IO_ROUTER_FOCUS_ID),
                layout_api::IoRouterFocus {
                    zone: zone.to_string(),
                    port: name.to_string(),
                    is_input,
                },
            )
        });
    }

    let visual_bg = if response.hovered() {
        egui::Color32::from_rgb(40, 40, 45)
    } else {
        bg_color
    };
    ui.painter().rect_filled(rect, 4.0, visual_bg);
    ui.painter()
        .rect_stroke(rect, 4.0, egui::Stroke::new(1.0, stroke_color));

    let pin_radius = 3.5;
    let pin_center = egui::pos2(rect.max.x - 10.0, rect.center().y);
    ui.painter()
        .circle_filled(pin_center, pin_radius, pin_color);

    let text_pos = egui::pos2(rect.min.x + 8.0, rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(200, 200, 200),
    );

    // DND Logic
    let payload_id = egui::Id::new("io_wire_drag");
    if response.drag_started() {
        ui.ctx().memory_mut(|m| {
            m.data.insert_temp(
                payload_id,
                IoWirePayload {
                    zone: zone.to_string(),
                    port: name.to_string(),
                    is_input,
                    start_pos: pin_center,
                },
            )
        });
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if response.drag_stopped() {
        ui.ctx()
            .memory_mut(|m| m.data.remove_temp::<IoWirePayload>(payload_id));
    }
}

fn render_flying_wire(ctx: &egui::Context) {
    let payload_id = egui::Id::new("io_wire_drag");
    if let Some(payload) = ctx.memory(|m| m.data.get_temp::<IoWirePayload>(payload_id)) {
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let painter =
                egui::Painter::new(ctx.clone(), egui::LayerId::debug(), egui::Rect::EVERYTHING);
            let start = payload.start_pos;
            let end = mouse_pos;
            painter.line_segment(
                [start, end],
                egui::Stroke::new(2.5, egui::Color32::from_white_alpha(150)),
            );
            painter.circle_filled(end, 4.0, egui::Color32::WHITE);
        }
    }
}
