// ui/node.rs
use bevy::prelude::Entity;
use bevy_egui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;
use crate::domain::{NodeGraphUiState, ProjectSession, TopologyMutation};
use super::canvas::CanvasTransform;

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

#[derive(Clone, Copy, PartialEq)]
pub enum NodeType { Shard, EnvRX, EnvTX }

pub struct NodeLayout {
    pub node_type: NodeType,
    pub screen_rect: Rect,
    pub header_rect: Rect,
    pub body_rect:   Rect,
    pub input_pins:  HashMap<String, Pos2>,
    pub output_pins: HashMap<String, Pos2>,
}

pub type NodeLayouts = HashMap<String, NodeLayout>;

pub fn calc_all_layouts(
    session: &ProjectSession,
    state: &mut NodeGraphUiState,
    transform: &CanvasTransform,
) -> NodeLayouts {
    let capacity = session.zones.len() + session.env_rx_nodes.len() + session.env_tx_nodes.len();
    let mut layouts = NodeLayouts::with_capacity(capacity);

    let groups: [(&Vec<String>, NodeType, f32); 3] = [
        (&session.zones, NodeType::Shard, 150.0),
        (&session.env_rx_nodes, NodeType::EnvRX, 50.0),
        (&session.env_tx_nodes, NodeType::EnvTX, 250.0),
    ];

    for (nodes, node_type, default_y) in groups {
        for (i, zone) in nodes.iter().enumerate() {
            let node_id = session.zone_ids.get(zone).cloned().unwrap_or_else(|| zone.clone());

            let local_pos = *state.node_positions
                .entry(zone.clone())
                .or_insert_with(|| {
                    if let Some(&(x, y)) = session.layout_cache.get(&node_id) {
                        Pos2::new(x, y)
                    } else {
                        Pos2::new(100.0 + i as f32 * 250.0, default_y)
                    }
                });

            let inputs = session.node_inputs.get(zone).cloned().unwrap_or_default();
            let outputs = session.node_outputs.get(zone).cloned().unwrap_or_default();

            layouts.insert(zone.clone(), calc_node_layout(node_type, local_pos, &inputs, &outputs, transform));
        }
    }

    layouts
}

fn calc_node_layout(node_type: NodeType, local_pos: Pos2, inputs: &[String], outputs: &[String], t: &CanvasTransform) -> NodeLayout {
    let header_h = HEADER_HEIGHT * t.zoom;
    let row_h    = ROW_HEIGHT    * t.zoom;
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

    NodeLayout { node_type, screen_rect, header_rect, body_rect, input_pins, output_pins }
}

pub fn draw_all_nodes(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    session: &mut ProjectSession,
    layouts: &NodeLayouts,
    state: &mut NodeGraphUiState,
    send_mutation: &mut impl FnMut(TopologyMutation),
    send_context_menu: &mut impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: Entity,
) {
    let mut all_nodes = session.zones.clone();
    all_nodes.extend(session.env_rx_nodes.clone());
    all_nodes.extend(session.env_tx_nodes.clone());

    for node in &all_nodes {
        let Some(layout) = layouts.get(node) else { continue };
        draw_node(painter, ui, session, node, layout, state, send_mutation, send_context_menu, target_window);
    }
}

fn draw_node(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    session: &mut ProjectSession,
    zone: &str,
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
    send_mutation: &mut impl FnMut(TopologyMutation),
    send_context_menu: &mut impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: Entity,
) {
    let zoom = state.zoom;
    let NodeLayout { screen_rect, header_rect, body_rect, .. } = *layout;

    let node_id = session.zone_ids.get(zone).cloned().unwrap_or_else(|| zone.to_string());
    let is_selected = state.selected_node_id.as_ref() == Some(&node_id);

    draw_node_shape(painter, layout.node_type, screen_rect, header_rect, body_rect, zoom, is_selected);
    
    // --- Header / Rename Logic ---
    if state.renaming_zone.as_deref() == Some(zone) {
        ui.allocate_ui_at_rect(header_rect, |ui| {
            // [DOD FIX] Запрещаем пробелы в именах зон (важно для путей в файловой системе)
            state.rename_buffer.retain(|c| !c.is_whitespace());

            let edit = ui.add(egui::TextEdit::singleline(&mut state.rename_buffer)
                .frame(false)
                .text_color(Color32::WHITE)
                .horizontal_align(egui::Align::Center));

            edit.request_focus();

            if edit.lost_focus() {
                // [DOD FIX] Подтверждение строго по Enter. Клик мимо = отмена.
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let new_name = state.rename_buffer.clone();
                    if !new_name.is_empty() && new_name != zone {
                        send_mutation(TopologyMutation::Rename(crate::domain::RenameTarget::Shard {
                            old_name: zone.to_string(),
                            new_name,
                            id: node_id.clone(),
                        }, None));
                    }
                }
                state.renaming_zone = None;
            }
        });
    } else {
        painter.text(header_rect.center(), egui::Align2::CENTER_CENTER, zone, egui::FontId::proportional(14.0 * zoom), Color32::WHITE);
    }

    handle_node_drag(ui, session, zone, screen_rect, state, send_context_menu, target_window);
    
    let inputs = session.node_inputs.get(zone).cloned().unwrap_or_default();
    let outputs = session.node_outputs.get(zone).cloned().unwrap_or_default();

    if layout.node_type != NodeType::EnvRX {
        draw_input_pins(painter, ui, session, zone, &inputs, layout, state, send_mutation, send_context_menu, target_window);
    }
    if layout.node_type != NodeType::EnvTX {
        draw_output_pins(painter, ui, session, zone, &outputs, layout, state, send_mutation, send_context_menu, target_window);
    }
}

fn draw_node_shape(
    painter: &egui::Painter,
    node_type: NodeType,
    screen_rect: Rect,
    header_rect: Rect,
    body_rect: Rect,
    zoom: f32,
    is_selected: bool,
) {
    let r = CORNER_RADIUS * zoom;

    let (header_color, border_base) = match node_type {
        NodeType::EnvRX => (Color32::from_rgb(35, 65, 45), Color32::from_rgb(50, 160, 80)),
        NodeType::EnvTX => (Color32::from_rgb(65, 35, 35), Color32::from_rgb(180, 60, 60)),
        NodeType::Shard => (CLR_HEADER, CLR_BORDER),
    };

    painter.rect_filled(screen_rect.translate(Vec2::splat(SHADOW_OFFSET * zoom)), r, CLR_SHADOW);
    painter.rect_filled(header_rect, egui::Rounding { nw: r, ne: r, sw: 0.0, se: 0.0 }, header_color);
    painter.rect_filled(body_rect, egui::Rounding { nw: 0.0, ne: 0.0, sw: r, se: r }, CLR_BODY);

    let border_color = if is_selected { Color32::GOLD } else { border_base };
    let border_width = if is_selected { 2.0 * zoom } else { 1.0 * zoom };
    painter.rect_stroke(screen_rect, r, Stroke::new(border_width, border_color));
}

fn handle_node_drag(
    ui: &mut egui::Ui,
    session: &mut ProjectSession,
    zone: &str,
    screen_rect: Rect,
    state: &mut NodeGraphUiState,
    send_context_menu: &mut impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: Entity,
) {
    let node_id = session.zone_ids.get(zone).cloned().unwrap_or_else(|| zone.to_string());
    let response = ui.interact(screen_rect, ui.id().with(&node_id), egui::Sense::click_and_drag());

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(pos) = state.node_positions.get_mut(zone) {
            *pos += response.drag_delta() / state.zoom;
            // [DOD FIX] Обновляем кэш в RAM для выживания при переключении вкладок.
            // При этом НЕ ставим is_dirty = true, чтобы не дергать AST-компилятор!
            if let Some(id) = session.zone_ids.get(zone) {
                session.layout_cache.insert(id.clone(), (pos.x, pos.y));
            }
        }
    }
    if response.clicked() {
        state.selected_node_id = Some(node_id);
    }

    if response.secondary_clicked() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            let label_suffix = if state.level == crate::domain::EditorLevel::Model { "Department" } else { "Shard" };
            
            send_context_menu(layout_api::OpenContextMenuEvent {
                target_window,
                position: pos,
                actions: vec![
                    layout_api::MenuAction {
                        action_id: format!("node_editor.delete_node|{}", zone),
                        label: format!("🗑 Delete {}", label_suffix),
                    },
                    layout_api::MenuAction {
                        action_id: format!("node_editor.start_rename|{}", zone),
                        label: "📝 Rename".into(),
                    },
                ],
            });
        }
    }
}

fn draw_input_pins(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    _session: &mut ProjectSession,
    zone: &str,
    inputs: &[String],
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
    send_mutation: &mut impl FnMut(TopologyMutation),
    send_context_menu: &mut impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: Entity,
) {
    let zoom = state.zoom;
    let r = PIN_RADIUS * zoom;

    for port in inputs {
        let Some(&pin_pos) = layout.input_pins.get(port) else { continue };
        draw_pin(painter, pin_pos, r, CLR_PIN_IN);
        
        let is_editing = state.renaming_port.as_ref() == Some(&(zone.to_string(), true, port.clone()));
        if is_editing {
            let edit_rect = Rect::from_min_size(pin_pos + Vec2::new(10.0 * zoom, -8.0 * zoom), Vec2::new(60.0 * zoom, 16.0 * zoom));
            ui.allocate_ui_at_rect(edit_rect, |ui| {
                // [DOD FIX] Жестко вырезаем пробелы, чтобы они не попали в AST и FNV хэши
                state.rename_buffer.retain(|c| !c.is_whitespace());

                let edit = ui.add(egui::TextEdit::singleline(&mut state.rename_buffer).frame(false).text_color(CLR_PIN_LABEL));
                edit.request_focus();

                if edit.lost_focus() {
                    // [DOD FIX] Подтверждение только по Enter. Клик мимо = отмена.
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let new_name = state.rename_buffer.clone();
                        if !new_name.is_empty() && new_name != *port {
                            send_mutation(TopologyMutation::Rename(crate::domain::RenameTarget::IoPin { zone: zone.to_string(), is_input: true, old_name: port.clone(), new_name }, None));
                        }
                    }
                    state.renaming_port = None;
                }
            });
        } else {
            painter.text(pin_pos + Vec2::new(10.0 * zoom, 0.0), egui::Align2::LEFT_CENTER, port, egui::FontId::proportional(12.0 * zoom), CLR_PIN_LABEL);
        }

        let hit = Rect::from_center_size(pin_pos, Vec2::splat(r * PIN_HIT_SCALE));
        let resp = ui.interact(hit, ui.id().with((zone, "in", port)), egui::Sense::click_and_drag());
        if ui.rect_contains_pointer(hit) { painter.circle_stroke(pin_pos, r * 1.5, Stroke::new(2.0, CLR_PIN_HOVER)); }

        if ui.rect_contains_pointer(hit) && ui.input(|i| i.pointer.any_released()) {
            if let Some((src_zone, src_port, _)) = state.dragging_pin.clone() {
                if src_zone != zone {
                    send_mutation(TopologyMutation::AddConnection { from: src_zone, from_port: src_port, to: zone.to_string(), to_port: port.clone() });
                }
            }
            state.dragging_pin = None;
        }

        if resp.secondary_clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                send_context_menu(layout_api::OpenContextMenuEvent {
                    target_window, position: pos, actions: vec![
                        layout_api::MenuAction { action_id: format!("node_editor.start_rename_port|{}|1|{}", zone, port), label: "📝 Rename Port".into() },
                        layout_api::MenuAction { action_id: format!("node_editor.delete_port|{}|1|{}", zone, port), label: "🗑 Delete Port".into() },
                    ]
                });
            }
        }
    }

    let plus_y = layout.body_rect.top() + 12.0 * zoom + (inputs.len() as f32) * 20.0 * zoom;
    let plus_pos = Pos2::new(layout.body_rect.left(), plus_y);
    draw_pin(painter, plus_pos, r * 0.8, Color32::DARK_GRAY);
    painter.text(plus_pos + Vec2::new(10.0 * zoom, 0.0), egui::Align2::LEFT_CENTER, "+", egui::FontId::proportional(12.0 * zoom), Color32::DARK_GRAY);
    if ui.interact(Rect::from_center_size(plus_pos, Vec2::splat(r * 2.0)), ui.id().with((zone, "add_in")), egui::Sense::click()).clicked() {
        send_mutation(TopologyMutation::AddIoMatrix { zone: zone.to_string(), is_input: true, name: format!("in_{}", inputs.len() + 1) });
    }
}

fn draw_output_pins(
    painter: &egui::Painter,
    ui: &mut egui::Ui,
    _session: &mut ProjectSession,
    zone: &str,
    outputs: &[String],
    layout: &NodeLayout,
    state: &mut NodeGraphUiState,
    send_mutation: &mut impl FnMut(TopologyMutation),
    send_context_menu: &mut impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: Entity,
) {
    let zoom = state.zoom;
    let r = PIN_RADIUS * zoom;

    for port in outputs {
        let Some(&pin_pos) = layout.output_pins.get(port) else { continue };
        draw_pin(painter, pin_pos, r, CLR_PIN_OUT);
        
        let is_editing = state.renaming_port.as_ref() == Some(&(zone.to_string(), false, port.clone()));
        if is_editing {
            let edit_rect = Rect::from_min_max(pin_pos - Vec2::new(70.0 * zoom, 8.0 * zoom), pin_pos - Vec2::new(10.0 * zoom, -8.0 * zoom));
            ui.allocate_ui_at_rect(edit_rect, |ui| {
                // [DOD FIX] Жестко вырезаем пробелы
                state.rename_buffer.retain(|c| !c.is_whitespace());

                let edit = ui.add(egui::TextEdit::singleline(&mut state.rename_buffer).frame(false).text_color(CLR_PIN_LABEL).horizontal_align(egui::Align::RIGHT));
                edit.request_focus();

                if edit.lost_focus() {
                    // [DOD FIX] Подтверждение только по Enter. Клик мимо = отмена.
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let new_name = state.rename_buffer.clone();
                        if !new_name.is_empty() && new_name != *port {
                            send_mutation(TopologyMutation::Rename(crate::domain::RenameTarget::IoPin { zone: zone.to_string(), is_input: false, old_name: port.clone(), new_name }, None));
                        }
                    }
                    state.renaming_port = None;
                }
            });
        } else {
            painter.text(pin_pos - Vec2::new(10.0 * zoom, 0.0), egui::Align2::RIGHT_CENTER, port, egui::FontId::proportional(12.0 * zoom), CLR_PIN_LABEL);
        }

        let hit = Rect::from_center_size(pin_pos, Vec2::splat(r * PIN_HIT_SCALE));
        let out_response = ui.interact(hit, ui.id().with((zone, "out", port)), egui::Sense::click_and_drag());
        if ui.rect_contains_pointer(hit) { ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing); painter.circle_stroke(pin_pos, r * 1.5, Stroke::new(2.0, CLR_PIN_HOVER)); }
        if out_response.drag_started() || out_response.clicked() { state.dragging_pin = Some((zone.to_string(), port.clone(), pin_pos)); }

        if out_response.secondary_clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                send_context_menu(layout_api::OpenContextMenuEvent {
                    target_window, position: pos, actions: vec![
                        layout_api::MenuAction { action_id: format!("node_editor.start_rename_port|{}|0|{}", zone, port), label: "📝 Rename Port".into() },
                        layout_api::MenuAction { action_id: format!("node_editor.delete_port|{}|0|{}", zone, port), label: "🗑 Delete Port".into() },
                    ]
                });
            }
        }
    }

    let plus_y = layout.body_rect.top() + 12.0 * zoom + (outputs.len() as f32) * 20.0 * zoom;
    let plus_pos = Pos2::new(layout.body_rect.right(), plus_y);
    draw_pin(painter, plus_pos, r * 0.8, Color32::DARK_GRAY);
    painter.text(plus_pos - Vec2::new(10.0 * zoom, 0.0), egui::Align2::RIGHT_CENTER, "+", egui::FontId::proportional(12.0 * zoom), Color32::DARK_GRAY);
    if ui.interact(Rect::from_center_size(plus_pos, Vec2::splat(r * 2.0)), ui.id().with((zone, "add_out")), egui::Sense::click()).clicked() {
        send_mutation(TopologyMutation::AddIoMatrix { zone: zone.to_string(), is_input: false, name: format!("out_{}", outputs.len() + 1) });
    }
}

#[inline]
fn draw_pin(painter: &egui::Painter, pos: Pos2, r: f32, color: Color32) {
    painter.circle_filled(pos, r, color);
    painter.circle_stroke(pos, r, Stroke::new(1.0, Color32::BLACK));
}
