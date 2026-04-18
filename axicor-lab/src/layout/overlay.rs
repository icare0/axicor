use bevy_egui::egui;
use crate::layout::domain::WindowDragState;
use layout_api::{DragIntent, WindowDragRequest};

pub fn draw_drag_intent_overlay(
    ui: &mut egui::Ui,
    drag_state: &WindowDragState,
    drag_request: &WindowDragRequest,
    tiles: &bevy::utils::HashMap<egui_tiles::TileId, egui::Rect>,
) {
    let overlay_color = egui::Color32::from_white_alpha(15);
    
    // [DOD FIX] Tooltip LayerPainter        
    let fg_painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.id().with("drag_preview")));

    match &drag_state.intent {
        DragIntent::Split { axis, insert_before, .. } => {
            if let Some(src_tile) = drag_state.source_tile {
                if let Some(&rect) = tiles.get(&src_tile) {
                    draw_split_preview(&fg_painter, rect, *axis, *insert_before, drag_request.current_pos, overlay_color);
                }
            }
        }
        DragIntent::Merge { victim } => {
            if let Some(&victim_rect) = tiles.get(victim) {
                fg_painter.rect_filled(victim_rect.shrink(5.0), 10.0, overlay_color);
            }
        }
        DragIntent::Swap { victim } => {
            if let Some(&victim_rect) = tiles.get(victim) {
                fg_painter.rect_stroke(victim_rect.shrink(5.0), 8.0,
                    egui::Stroke::new(2.0, egui::Color32::from_white_alpha(80)));
            }
        }
        _ => {}
    }
}

fn draw_split_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    axis: egui_tiles::LinearDir,
    insert_before: bool,
    pos: egui::Pos2,
    color: egui::Color32,
) {
    if axis == egui_tiles::LinearDir::Horizontal {
        let x = pos.x.clamp(rect.min.x + 100.0, rect.max.x - 100.0);
        let split_rect = if insert_before { egui::Rect::from_min_max(rect.left_top(), egui::pos2(x, rect.max.y)) }
                         else { egui::Rect::from_min_max(egui::pos2(x, rect.min.y), rect.right_bottom()) };
        painter.rect_filled(split_rect.shrink(5.0), 5.0, color);
        painter.line_segment([egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)], egui::Stroke::new(3.0, egui::Color32::WHITE));
    } else {
        let y = pos.y.clamp(rect.min.y + 100.0, rect.max.y - 100.0);
        let split_rect = if insert_before { egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.max.x, y)) }
                         else { egui::Rect::from_min_max(egui::pos2(rect.min.x, y), rect.right_bottom()) };
        painter.rect_filled(split_rect.shrink(5.0), 5.0, color);
        painter.line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], egui::Stroke::new(3.0, egui::Color32::WHITE));
    }
}

pub fn split_half(rect: egui::Rect, axis: egui_tiles::LinearDir, fraction: f32) -> (egui::Rect, egui::Rect) {
    if axis == egui_tiles::LinearDir::Horizontal {
        let x = rect.min.x + rect.width() * fraction;
        (egui::Rect::from_min_max(rect.left_top(), egui::pos2(x, rect.max.y)),
         egui::Rect::from_min_max(egui::pos2(x, rect.min.y), rect.right_bottom()))
    } else {
        let y = rect.min.y + rect.height() * fraction;
        (egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.max.x, y)),
         egui::Rect::from_min_max(egui::pos2(rect.min.x, y), rect.right_bottom()))
    }
}
