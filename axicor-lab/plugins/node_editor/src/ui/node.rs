// ui/node.rs
use bevy_egui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;
use crate::domain::{NodeGraphUiState, BrainTopologyGraph};
use super::canvas::CanvasTransform;

// --- Константы ноды ---
const NODE_WIDTH:      f32 = 180.0;
const HEADER_HEIGHT:   f32 = 24.0;
const ROW_HEIGHT:      f32 = 20.0;
const PIN_RADIUS:      f32 = 5.0;
const SHADOW_OFFSET:   f32 = 4.0;
const CORNER_RADIUS:   f32 = 6.0;
const PIN_HIT_SCALE:   f32 = 3.0;

const CLR_HEADER:      Color32 = Color32::from_rgb(45, 55, 70);
const CLR_BODY:        Color32 = Color32::from_rgb(30, 30, 35);
const CLR_BORDER:      Color32 = Color32::from_rgb(60, 70, 85);
const CLR_SHADOW:      Color32 = Color32::from_black_alpha(100);
const CLR_PIN_IN:      Color32 = Color32::from_rgb(100, 200, 255);
const CLR_PIN_OUT:     Color32 = Color32::from_rgb(255, 150, 50);
const CLR_PIN_LABEL:   Color32 = Color32::LIGHT_GRAY;
const CLR_PIN_HOVER:   Color32 = Color32::WHITE;

// ---------------------------------------------------------------------------
// Типы
// ---------------------------------------------------------------------------

/// Геометрия одной ноды — вычисляется в PASS 1, используется в PASS 2 и 3.
pub struct NodeLayout {
    pub screen_rect: Rect,
    pub header_rect: Rect,
    pub body_rect:   Rect,
    pub input_pins:  HashMap<String, Pos2>,
    pub output_pins: HashMap<String, Pos2>,
}

/// Геометрия всех нод за кадр.
pub type NodeLayouts = HashMap<String, NodeLayout>;

// ---------------------------------------------------------------------------
// PASS 1 — вычисление геометрии
// ---------------------------------------------------------------------------

pub fn calc_all_layouts(
    graph: &BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    transform: &CanvasTransform,
) -> NodeLayouts {
    let mut layouts = NodeLayouts::with_capacity(graph.zones.len());

    for (i, zone) in graph.zones.iter().enumerate() {
        let local_pos = *state.node_positions
            .entry(zone.clone())
            .or_insert_with(|| Pos2::new(100.0 + i as f32 * 250.0, 150.0));

        let inputs = graph.node_inputs.get(zone).cloned().unwrap_or_default();
        let outputs = graph.node_outputs.get(zone).cloned().unwrap_or_default();

        layouts.insert(zone.clone(), calc_node_layout(local_pos, &inputs, &outputs, transform));
    }

    layouts
}

fn calc_node_layout(local_pos: Pos2, inputs: &[String], outputs: &[String], t: &CanvasTransform) -> NodeLayout {
    let header_h = HEADER_HEIGHT * t.zoom;
    let row_h    = ROW_HEIGHT    * t.zoom;
    // +1 строка для кнопки "+ add"
    let rows     = (inputs.len() + 1).max(outputs.len() + 1) as f32; 
    let body_h   = rows * row_h + 16.0 * t.zoom;

    let screen_pos  = t.to_screen(local_pos);
    let node_size   = Vec2::new(NODE_WIDTH * t.zoom, header_h + body_h);
    let screen_rect = Rect::from_min_size(screen_pos, node_size);
    let header_rect = Rect::from_min_size(screen_pos, Vec2::new(node_size.x, header_h));
    let body_rect   = Rect::from_min_max(
        Pos2::new(screen_rect.min.x, screen_rect.min.y + header_h),
        screen_rect.max,
    );

    let input_pins = inputs.iter().enumerate().map(|(idx, port)| {
        let y = body_rect.top() + 12.0 * t.zoom + idx as f32 * row_h;
        (port.clone(), Pos2::new(body_rect.left(), y))
    }).collect();

    let output_pins = outputs.iter().enumerate().map(|(idx, port)| {
        let y = body_rect.top() + 12.0 * t.zoom + idx as f32 * row_h;
        (port.clone(), Pos2::new(body_rect.right(), y))
    }).collect();

    NodeLayout { screen_rect, header_rect, body_rect, input_pins, output_pins }
}

// ---------------------------------------------------------------------------
// PASS 3 — отрисовка + интерактивность
// ---------------------------------------------------------------------------

pub fn draw_all_nodes(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    graph: &mut BrainTopologyGraph,
    layouts: &NodeLayouts,
    state: &mut NodeGraphUiState,
) {
    let zones = graph.zones.clone();
    for zone in &zones {
        let Some(layout) = layouts.get(zone) else { continue };
        draw_node(painter, ui, graph, zone, layout, state);
    }
}

fn draw_node(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    graph: &mut BrainTopologyGraph,
    zone: &str,
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
) {
    let zoom = state.zoom;
    let NodeLayout { screen_rect, header_rect, body_rect, .. } = *layout;

    let is_selected = state.selected_node.as_deref() == Some(zone);
    draw_node_shape(painter, screen_rect, header_rect, body_rect, zone, zoom, is_selected);
    handle_node_drag(ui, graph, zone, screen_rect, state);
    
    let inputs = graph.node_inputs.get(zone).cloned().unwrap_or_default();
    let outputs = graph.node_outputs.get(zone).cloned().unwrap_or_default();

    draw_input_pins(painter, ui, graph, zone, &inputs, layout, state);
    draw_output_pins(painter, ui, graph, zone, &outputs, layout, state);
}

fn draw_node_shape(
    painter: &egui::Painter,
    screen_rect: Rect,
    header_rect: Rect,
    body_rect: Rect,
    label: &str,
    zoom: f32,
    is_selected: bool, // ДОБАВЛЕНО
) {
    let r = CORNER_RADIUS * zoom;

    // Тень
    painter.rect_filled(
        screen_rect.translate(Vec2::splat(SHADOW_OFFSET * zoom)),
        r, CLR_SHADOW,
    );
    // Хедер
    painter.rect_filled(header_rect,
        egui::Rounding { nw: r, ne: r, sw: 0.0, se: 0.0 }, CLR_HEADER);
    painter.text(header_rect.center(), egui::Align2::CENTER_CENTER,
        label, egui::FontId::proportional(14.0 * zoom), Color32::WHITE);
    // Тело
    painter.rect_filled(body_rect,
        egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: r }, CLR_BODY);
    // DOD FIX: Яркая обводка, если нода выделена
    let border_color = if is_selected { Color32::GOLD } else { CLR_BORDER };
    let border_width = if is_selected { 2.0 * zoom } else { 1.0 * zoom };
    painter.rect_stroke(screen_rect, r, Stroke::new(border_width, border_color));
}

fn handle_node_drag(
    ui: &mut egui::Ui,
    graph: &mut BrainTopologyGraph, // ДОБАВЛЕНО
    zone: &str,
    screen_rect: Rect,
    state: &mut NodeGraphUiState,
) {
    // DOD FIX: Используем click_and_drag, чтобы ЛКМ регистрировал клики!
    let response = ui.interact(screen_rect, ui.id().with(zone), egui::Sense::click_and_drag());

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(pos) = state.node_positions.get_mut(zone) {
            *pos += response.drag_delta() / state.zoom;
        }
    }
    // DOD FIX: Мгновенное выделение по клику ЛКМ
    if response.clicked() {
        state.selected_node = Some(zone.to_string());
    }

    response.context_menu(|ui| {
        ui.label(format!("Node: {}", zone));
        ui.separator();
        
        if ui.button("⚙ Properties").clicked() { 
            state.selected_node = Some(zone.to_string()); 
            ui.close_menu(); 
        }
        
        if ui.button("🗑 Delete Node").clicked() { 
            // DOD FIX: Физическое удаление зоны и всех привязанных к ней аксонов
            let z = zone.to_string();
            graph.zones.retain(|x| x != &z);
            graph.connections.retain(|(f, _, t, _)| f != &z && t != &z);
            graph.node_inputs.remove(&z);
            graph.node_outputs.remove(&z);
            state.node_positions.remove(&z);
            
            graph.is_dirty = true; // ДОБАВЛЕНО
            
            // Сбрасываем выделение, если удалили выбранную ноду
            if state.selected_node.as_deref() == Some(zone) {
                state.selected_node = None;
            }
            ui.close_menu(); 
        }
    });
}

fn draw_input_pins(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    graph: &mut BrainTopologyGraph,
    zone: &str,
    inputs: &[String],
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
) {
    let zoom = state.zoom;
    let r = PIN_RADIUS * zoom;

    for (_idx, port) in inputs.iter().enumerate() {
        let Some(&pin_pos) = layout.input_pins.get(port) else { continue };
        draw_pin(painter, pin_pos, r, CLR_PIN_IN);
        painter.text(
            pin_pos + Vec2::new(10.0 * zoom, 0.0),
            egui::Align2::LEFT_CENTER,
            port, egui::FontId::proportional(12.0 * zoom), CLR_PIN_LABEL,
        );

        let hit = Rect::from_center_size(pin_pos, Vec2::splat(r * PIN_HIT_SCALE));
        
        // DOD FIX: rect_contains_pointer обходит блокировку hover() во время Drag!
        let is_hovered = ui.rect_contains_pointer(hit);
        if is_hovered {
            painter.circle_stroke(pin_pos, r * 1.5, Stroke::new(2.0, CLR_PIN_HOVER));
        }

        if is_hovered && ui.input(|i| i.pointer.any_released()) {
            if let Some((src_zone, src_port, _)) = state.dragging_pin.clone() {
                if src_zone != zone {
                    state.pending_connection = Some((src_zone, src_port, zone.to_string(), port.clone()));
                }
            }
            state.dragging_pin = None; // Успешный коннект сбрасывает Drag
        }
    }

    // Отрисовка кнопки "+ add"
    let plus_y = layout.body_rect.top() + 12.0 * zoom + (inputs.len() as f32) * 20.0 * zoom;
    let plus_pos = Pos2::new(layout.body_rect.left(), plus_y);
    draw_pin(painter, plus_pos, r * 0.8, Color32::DARK_GRAY);
    painter.text(plus_pos + Vec2::new(10.0 * zoom, 0.0), egui::Align2::LEFT_CENTER, "+ add", egui::FontId::proportional(11.0 * zoom), Color32::GRAY);
    
    if ui.interact(Rect::from_center_size(plus_pos, Vec2::splat(r * 2.0)), ui.id().with((zone, "add_in")), egui::Sense::click()).clicked() {
        let new_port = format!("in_{}", inputs.len() + 1);
        graph.node_inputs.get_mut(zone).unwrap().push(new_port);
        graph.is_dirty = true; // ДОБАВЛЕНО
    }
}

fn draw_output_pins(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    graph: &mut BrainTopologyGraph,
    zone: &str,
    outputs: &[String],
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
) {
    let zoom = state.zoom;
    let r = PIN_RADIUS * zoom;

    for (_idx, port) in outputs.iter().enumerate() {
        let Some(&pin_pos) = layout.output_pins.get(port) else { continue };
        draw_pin(painter, pin_pos, r, CLR_PIN_OUT);
        painter.text(
            pin_pos - Vec2::new(10.0 * zoom, 0.0),
            egui::Align2::RIGHT_CENTER,
            port, egui::FontId::proportional(12.0 * zoom), CLR_PIN_LABEL,
        );

        let hit = Rect::from_center_size(pin_pos, Vec2::splat(r * PIN_HIT_SCALE));
        
        // DOD FIX: click_and_drag поддерживает и "липкий" клик, и удержание
        let out_response = ui.interact(hit, ui.id().with((zone, "out", port)), egui::Sense::click_and_drag());
        
        if ui.rect_contains_pointer(hit) { 
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair); 
            painter.circle_stroke(pin_pos, r * 1.5, Stroke::new(2.0, CLR_PIN_HOVER));
        }
        
        if out_response.drag_started() || out_response.clicked() {
            state.dragging_pin = Some((zone.to_string(), port.clone(), pin_pos));
        }
    }

    // Отрисовка кнопки "+ add"
    let plus_y = layout.body_rect.top() + 12.0 * zoom + (outputs.len() as f32) * 20.0 * zoom;
    let plus_pos = Pos2::new(layout.body_rect.right(), plus_y);
    draw_pin(painter, plus_pos, r * 0.8, Color32::DARK_GRAY);
    painter.text(plus_pos - Vec2::new(10.0 * zoom, 0.0), egui::Align2::RIGHT_CENTER, "+ add", egui::FontId::proportional(11.0 * zoom), Color32::GRAY);
    
    if ui.interact(Rect::from_center_size(plus_pos, Vec2::splat(r * 2.0)), ui.id().with((zone, "add_out")), egui::Sense::click()).clicked() {
        let new_port = format!("out_{}", outputs.len() + 1);
        graph.node_outputs.get_mut(zone).unwrap().push(new_port);
        graph.is_dirty = true; // ДОБАВЛЕНО
    }
}

#[inline]
fn draw_pin(painter: &egui::Painter, pos: Pos2, r: f32, color: Color32) {
    painter.circle_filled(pos, r, color);
    painter.circle_stroke(pos, r, Stroke::new(1.0, Color32::BLACK));
}
