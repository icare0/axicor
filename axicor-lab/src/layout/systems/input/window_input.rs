use crate::layout::domain::WindowDragState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{TopologyCache, WindowDragRequest};

const BORDER_SHRINK: f32 = 3.0;

pub fn window_input_system(
    mut contexts: EguiContexts,
    mut drag_request: ResMut<WindowDragRequest>,
    topology: Res<TopologyCache>,
    drag_state: Res<WindowDragState>,
) {
    let pointer = contexts.ctx_mut().input(|i| i.pointer.clone());

    if pointer.any_pressed() && !drag_state.is_dragging {
        if let Some(pos) = pointer.interact_pos() {
            for (&tile_id, &rect) in &topology.tiles {
                if edge_triggers(rect).iter().any(|t| t.contains(pos)) {
                    drag_request.active = true;
                    drag_request.source = layout_api::DragSource::EdgeTrigger;
                    drag_request.start_pos = pos;
                    drag_request.target_tile = Some(tile_id);
                    break;
                }
            }
        }
    }

    if drag_request.active {
        if let Some(pos) = pointer.interact_pos() {
            drag_request.current_pos = pos;
        }
        if pointer.any_released() {
            drag_request.active = false;
        }
    }
}

// DOD FIX: - (8x8)
pub fn edge_triggers(rect: egui::Rect) -> [egui::Rect; 4] {
    let r = rect.shrink(BORDER_SHRINK);
    let s = 8.0;

    [
        egui::Rect::from_min_size(r.left_top(), egui::vec2(s, s)),
        egui::Rect::from_min_size(egui::pos2(r.max.x - s, r.min.y), egui::vec2(s, s)),
        egui::Rect::from_min_size(egui::pos2(r.min.x, r.max.y - s), egui::vec2(s, s)),
        egui::Rect::from_min_size(egui::pos2(r.max.x - s, r.max.y - s), egui::vec2(s, s)),
    ]
}
