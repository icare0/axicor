// ui/mod.rs
pub mod canvas;
pub mod node;
pub mod breadcrumb;
pub mod connections;
pub mod toolbar;
pub mod inspector;

use bevy_egui::egui::{self, Rect};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};
use self::breadcrumb::draw_breadcrumbs;
use self::connections::draw_all_connections;
use self::node::{calc_all_layouts, draw_all_nodes};
use self::toolbar::render_canvas_context_menu;
use self::inspector::draw_inspector_panel;

pub fn render_editor_ui(
    ui: &mut egui::Ui,
    window_rect: Rect,
    graph: &mut BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    mut _send_mutation: impl FnMut(TopologyMutation),
    mut _send_save: impl FnMut(),
    mut _send_compile: impl FnMut(),
    mut _send_bake: impl FnMut(),
    mut _send_open: impl FnMut(std::path::PathBuf),
) {
    let mut send_mutation = _send_mutation;
    let mut send_save = _send_save;
    let mut send_compile = _send_compile;
    let mut send_bake = _send_bake;
    let mut send_open = _send_open;

    // 1. Хедер навигации (Breadcrumbs)
    let header_height = 28.0;
    let header_rect = Rect::from_min_size(window_rect.min, egui::vec2(window_rect.width(), header_height));
    let content_rect = Rect::from_min_max(
        egui::pos2(window_rect.min.x, window_rect.min.y + header_height), 
        window_rect.max
    );

    ui.painter().rect_filled(header_rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
    ui.painter().line_segment(
        [header_rect.left_bottom(), header_rect.right_bottom()], 
        egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50))
    );

    let mut header_ui = ui.child_ui(header_rect, egui::Layout::left_to_right(egui::Align::Center));
    header_ui.add_space(layout_api::SYS_UI_SAFE_ZONE); // DOD FIX: Унифицированный отступ под DND-якорь

    draw_breadcrumbs(&mut header_ui, graph, state, &mut send_open);

    // DOD FIX: Строгое разделение 3 кнопок (Save, Compile, Bake)
    header_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
        ui.add_space(12.0); // Отступ справа
        if ui.button("🔥 Bake").clicked() { send_bake(); }
        if ui.button("⚙ Compile").clicked() { send_compile(); }
        if ui.button("💾 Save").clicked() { send_save(); }
    });

    // DOD FIX: Разделение области на Canvas и Inspector
    let mut inspector_width = 300.0;
    // DOD FIX: Защита от переполнения (Rect Crash). 
    // Инспектор не может занимать больше половины экрана, чтобы не схлопнуть канвас.
    if content_rect.width() < 600.0 {
        inspector_width = content_rect.width() / 2.0;
    }

    let mut canvas_rect = content_rect;
    let mut inspector_rect = content_rect;

    // Отрисовка канваса (всегда есть)
    if state.selected_node.is_some() {
        canvas_rect.max.x -= inspector_width;
        inspector_rect.min.x = canvas_rect.max.x;
        // Отрисовка инспектора
        draw_inspector_panel(ui, inspector_rect, graph, state, &mut send_mutation);
    }

    // 2. Канвас: Ввод и Трансформы
    ui.allocate_ui_at_rect(canvas_rect, |ui| {
        ui.set_clip_rect(canvas_rect);
        
        // PASS 0: INPUT & CONTEXT MENU
        let (transform, interact_resp) = canvas::handle_input(ui, canvas_rect, state);
        let painter = ui.painter_at(canvas_rect);

        // DOD FIX: Сброс выделения при клике по фону
        if interact_resp.clicked() {
            state.selected_node = None;
        }

        // DOD FIX: Удаление выделенной ноды по клавише Delete
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(selected) = state.selected_node.take() {
                graph.zones.retain(|z| z != &selected);
                graph.connections.retain(|(f, _, t, _)| f != &selected && t != &selected);
                graph.node_inputs.remove(&selected);
                graph.node_outputs.remove(&selected);
                state.node_positions.remove(&selected);
            }
        }

        canvas::draw_background(&painter, canvas_rect, &transform);
        
        interact_resp.context_menu(|ui| {
            render_canvas_context_menu(ui, canvas_rect, state, graph, &mut send_mutation, &mut send_save, &mut send_bake);
        });

        // PASS 1: CALC
        let layouts = calc_all_layouts(graph, state, &transform);

        // PASS 2: BACKGROUND (Connections)
        draw_all_connections(&painter, ui, graph, &layouts, state);

        // PASS 3: FOREGROUND (Nodes)
        draw_all_nodes(&painter, ui, graph, &layouts, state);

        // PASS 4: POST (Commit mutations)
        if let Some((src, src_p, dst, dst_p)) = state.pending_connection.take() {
             // DOD FIX: Сохраняем в кэш UI для немедленного фидбека (позже заменим на Command Queue)
             graph.connections.push((src, src_p, dst, dst_p));
             graph.is_dirty = true;
        }

        // DOD FIX: Липкий Drag & Drop для связей
        if let Some((src_zone, src_port, start_pos)) = state.dragging_pin.clone() {
            let pointer = &ui.input(|i| i.pointer.clone());
            
            if pointer.secondary_clicked() || pointer.secondary_released() {
                state.dragging_pin = None; // Отмена по ПКМ
            } else if pointer.primary_released() || pointer.primary_clicked() {
                if let Some(mouse_pos) = pointer.hover_pos() {
                    // Создаем ноду, только если мышь сдвинулась от розетки (защита от мгновенного дропа при клике)
                    if mouse_pos.distance(start_pos) > 20.0 && state.pending_connection.is_none() {
                        let local_pos = transform.to_local(mouse_pos);
                        let new_zone_name = format!("Zone_{}", graph.zones.len() + 1);
                        graph.zones.push(new_zone_name.clone());
                        
                        graph.node_inputs.insert(new_zone_name.clone(), vec!["in".to_string()]);
                        graph.node_outputs.insert(new_zone_name.clone(), vec!["out".to_string()]);
                        state.node_positions.insert(new_zone_name.clone(), local_pos);
                        
                        graph.connections.push((src_zone, src_port, new_zone_name.clone(), "in".to_string()));
                        graph.is_dirty = true; // ДОБАВЛЕНО
                        send_mutation(TopologyMutation::AddZone { name: new_zone_name, pos: local_pos });
                        
                        state.dragging_pin = None;
                    }
                }
            }
        }
    });
}
