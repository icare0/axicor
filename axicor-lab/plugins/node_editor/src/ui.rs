use bevy_egui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};

pub fn render_editor_ui(
    ui: &mut egui::Ui,
    rect: Rect,
    graph: &BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    mut _send_mutation: impl FnMut(TopologyMutation),
    mut _send_save: impl FnMut(),
    mut _send_bake: impl FnMut(),
) {
    // 1. Управление камерой канваса (Pan & Zoom)
    let interact_response = ui.interact(rect, ui.id().with("canvas_bg"), egui::Sense::click_and_drag());
    
    // Панорамирование средней или левой кнопкой мыши (по фону)
    if interact_response.dragged_by(egui::PointerButton::Middle) || 
       (interact_response.dragged_by(egui::PointerButton::Primary) && ui.ctx().dragged_id().is_none()) {
        state.pan += interact_response.drag_delta();
    }
    
    // Зум колесиком
    if ui.rect_contains_pointer(rect) {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = state.zoom;
            state.zoom = (state.zoom + scroll * 0.005).clamp(0.2, 5.0);
            
            // Компенсация панорамирования для зума в центр экрана
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let zoom_ratio = state.zoom / old_zoom;
                state.pan = mouse_pos.to_vec2() - (mouse_pos.to_vec2() - state.pan) * zoom_ratio;
            }
        }
    }

    let painter = ui.painter_at(rect);

    // Хелперы для перевода координат
    let node_size = Vec2::new(140.0, 50.0) * state.zoom;
    let to_screen = |pos: Pos2| -> Pos2 {
        rect.min + (pos.to_vec2() * state.zoom) + state.pan
    };

    // --- PASS 1: CALC ---
    // Вычисляем экранные габариты всех зон
    let mut screen_rects = std::collections::HashMap::new();
    for (i, zone) in graph.zones.iter().enumerate() {
        let local_pos = state.node_positions.entry(zone.clone())
            .or_insert_with(|| Pos2::new(100.0 + (i as f32 * 200.0), 150.0));
        
        let screen_pos = to_screen(*local_pos);
        let node_rect = Rect::from_min_size(screen_pos, node_size);
        screen_rects.insert(zone.clone(), node_rect);
    }

    // --- PASS 2: BACKGROUND (Связи / Ghost Axons) ---
    for (from, to) in &graph.connections {
        if let (Some(r1), Some(r2)) = (screen_rects.get(from), screen_rects.get(to)) {
            let p1 = r1.right_center();
            let p2 = r2.left_center();
            
            // Расчет контрольных точек для красивой кривой Безье
            let control_scale = (p2.x - p1.x).abs().max(50.0) * 0.5;
            
            let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                [p1, p1 + Vec2::X * control_scale, p2 - Vec2::X * control_scale, p2],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.0 * state.zoom, Color32::from_rgb(200, 100, 50)), // Оранжевые нити связей
            );
            painter.add(shape);
        }
    }

    // --- PASS 3: FOREGROUND (Ноды) ---
    for zone in &graph.zones {
        let node_rect = screen_rects[zone];
        
        // Рисуем подложку ноды (темно-серая)
        painter.rect_filled(
            node_rect,
            6.0 * state.zoom,
            Color32::from_rgb(40, 40, 40)
        );
        
        // Рисуем обводку
        painter.rect_stroke(
            node_rect,
            6.0 * state.zoom,
            Stroke::new(1.5 * state.zoom, Color32::from_rgb(100, 150, 200))
        );
        
        // Текст названия зоны
        painter.text(
            node_rect.center(),
            egui::Align2::CENTER_CENTER,
            zone,
            egui::FontId::proportional(16.0 * state.zoom),
            Color32::WHITE
        );

        // Интерактивность: Перетаскивание ноды левой кнопкой мыши
        let node_id = ui.id().with(zone);
        let node_response = ui.interact(node_rect, node_id, egui::Sense::drag());
        
        if node_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        
        if node_response.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if let Some(pos) = state.node_positions.get_mut(zone) {
                *pos += node_response.drag_delta() / state.zoom;
            }
        }
    }
}
