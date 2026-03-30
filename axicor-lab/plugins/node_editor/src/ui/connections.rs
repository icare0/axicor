// ui/connections.rs
use bevy_egui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::domain::ProjectSession;
use super::node::NodeLayouts;
use crate::domain::NodeGraphUiState;

pub fn draw_all_connections(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    session: &ProjectSession,
    layouts: &NodeLayouts,
    state: &mut NodeGraphUiState,
    send_mutation: &mut impl FnMut(crate::domain::TopologyMutation),
) {
    let zoom = state.zoom;
    
    for (from, from_p, to, to_p) in &session.connections {
        if let (Some(l1), Some(l2)) = (layouts.get(from), layouts.get(to)) {
            if let (Some(&p1), Some(&p2)) = (l1.output_pins.get(from_p), l2.input_pins.get(to_p)) {
                draw_connection_line(painter, p1, p2, zoom, Color32::from_rgb(200, 120, 50));
                
                let mid = p1 + (p2 - p1) * 0.5;
                let id = ui.id().with((from, from_p, to, to_p));
                let resp = ui.interact(Rect::from_center_size(mid, Vec2::splat(15.0 * zoom)), id, egui::Sense::click());
                
                if resp.hovered() { painter.circle_filled(mid, 5.0 * zoom, Color32::YELLOW); }
                resp.context_menu(|ui| {
                    ui.label(format!("Link: {}[{}] → {}[{}]", from, from_p, to, to_p));
                    if ui.button("✂ Delete Connection").clicked() {
                        send_mutation(crate::domain::TopologyMutation::Delete(
                            crate::domain::DeleteTarget::Connection {
                                from: from.clone(),
                                from_port: from_p.clone(),
                                to: to.clone(),
                                to_port: to_p.clone(),
                            },
                            None
                        ));
                        ui.close_menu();
                    }
                });
            }
        }
    }

    if let Some((_, _, start_pos)) = &state.dragging_pin {
        if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
            draw_connection_line(painter, *start_pos, mouse, zoom, Color32::from_rgb(255, 200, 100));
        }
    }
}

pub fn draw_connection_line(painter: &egui::Painter, p1: Pos2, p2: Pos2, zoom: f32, color: Color32) {
    let cp_dist = (p2.x - p1.x).abs().max(50.0) * 0.5;
    let shape = egui::epaint::CubicBezierShape::from_points_stroke(
        [p1, p1 + Vec2::X * cp_dist, p2 - Vec2::X * cp_dist, p2],
        false, Color32::TRANSPARENT, Stroke::new(3.0 * zoom, color)
    );
    painter.add(shape);
}
