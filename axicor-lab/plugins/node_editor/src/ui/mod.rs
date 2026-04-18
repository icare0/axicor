// ui/mod.rs
pub mod canvas;
pub mod node;
pub mod breadcrumb;
pub mod connections;
pub mod modals;

use bevy_egui::egui::{self, Rect};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};
use self::breadcrumb::draw_breadcrumbs;
use self::connections::draw_all_connections;
use self::node::{calc_all_layouts, draw_all_nodes};

pub fn render_editor_ui(
    ui: &mut egui::Ui,
    window_rect: Rect,
    graph: &mut BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    mut send_mutation: impl FnMut(TopologyMutation),
    mut send_save: impl FnMut(),
    mut send_compile: impl FnMut(),
    mut send_bake: impl FnMut(),
    mut send_open: impl FnMut(std::path::PathBuf),
    mut send_context_menu: impl FnMut(layout_api::OpenContextMenuEvent),
    target_window: bevy::prelude::Entity,
    _rtt_texture_id: Option<bevy_egui::egui::TextureId>,
) {
    // [DOD FIX]         
    let (content_rect, _) = layout_api::draw_unified_header(ui, window_rect, "");
    
    //      ( SYS_UI_SAFE_ZONE  DND )
    let mut header_rect = window_rect;
    header_rect.set_height(28.0); //    layout-api
    header_rect.min.x += layout_api::SYS_UI_SAFE_ZONE;

    let mut header_ui = ui.child_ui(header_rect, egui::Layout::left_to_right(egui::Align::Center));
    draw_breadcrumbs(&mut header_ui, graph, state, &mut send_open);

    header_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
        ui.add_space(12.0);
        if ui.button(" Bake").clicked() { send_bake(); }
        if ui.button(" Compile").clicked() { send_compile(); }
        if ui.button(" Save").clicked() { send_save(); }
    });

    let canvas_rect = content_rect; //  ,   100% 

    // 1.      (,    )
    let (transform, response) = crate::ui::canvas::handle_input(ui, canvas_rect, state);
    
    // [FIX]  painter ,     (borrow checker)
    let painter = ui.painter().clone();
    crate::ui::canvas::draw_background(&painter, canvas_rect, &transform);

    // 2.   
    let active_path = graph.active_path.clone();
    if let Some(path) = &active_path {
        if let Some(session) = graph.sessions.get_mut(path) {
            //     - ( /  / )
            let layouts = calc_all_layouts(session, state, &transform);
            draw_all_connections(&painter, ui, session, &layouts, state, &mut send_mutation);
            draw_all_nodes(&painter, ui, session, &layouts, state, &mut send_mutation, &mut send_context_menu, &mut send_save, target_window);
        }
    }

    // 4.   () -    
    // [DOD FIX]      -    .
    if ui.input(|i| i.pointer.any_released()) {
        if let Some((src_zone, src_port, pin_pos, _)) = state.dragging_pin.take() {
            if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
                //       ( )
                if (mouse_pos - pin_pos).length() > 20.0 {
                    let local_pos = transform.to_local(mouse_pos);
                    let prefix = if state.level == crate::domain::EditorLevel::Model { "Zone_" } else { "Shard_" };
                    let new_zone_name = format!("{}{}", prefix, bevy_egui::egui::Id::new(local_pos.x.to_bits()).value() % 1000);

                    // 1.      
                    send_mutation(TopologyMutation::Create(crate::domain::CreateTarget::Zone { 
                        name: new_zone_name.clone(), 
                        pos: local_pos 
                    }, None));

                    // 2.      
                    send_mutation(TopologyMutation::Create(crate::domain::CreateTarget::Connection {
                        from: src_zone,
                        from_port: src_port,
                        to: new_zone_name,
                        to_port: "in".to_string(),
                        voxel_z: None,
                    }, None));
                }
            }
        }
        state.dragging_pin = None;
    }

    if response.double_clicked() {
        if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
            if mouse_pos.x > 20.0 && state.pending_connection.is_none() {
                let local_pos = transform.to_local(mouse_pos);
                // DOD FIX: .to_bits()   -  ID
                let new_zone_name = format!("Zone_{}", bevy_egui::egui::Id::new(local_pos.x.to_bits()).value() % 1000);
                send_mutation(TopologyMutation::Create(crate::domain::CreateTarget::Zone { name: new_zone_name, pos: local_pos }, None));
                state.dragging_pin = None;
            }
        }
    }

    if response.secondary_clicked() {
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            // [DOD FIX]         
            let local_pos = transform.to_local(pos);
            send_context_menu(layout_api::OpenContextMenuEvent {
                target_window,
                position: pos,
                actions: vec![
                    layout_api::MenuAction {
                        action_id: format!("node_editor.add_node|{}|{}", local_pos.x, local_pos.y),
                        label: " Add Shard".into(),
                    },
                    layout_api::MenuAction {
                        action_id: format!("node_editor.add_env_rx|{}|{}", local_pos.x, local_pos.y),
                        label: " Add Sensor (EnvRX)".into(),
                    },
                    layout_api::MenuAction {
                        action_id: format!("node_editor.add_env_tx|{}|{}", local_pos.x, local_pos.y),
                        label: " Add Motor (EnvTX)".into(),
                    },
                    layout_api::MenuAction {
                        action_id: "node_editor.clear_graph".into(),
                        label: " Clear Graph".into(),
                    }
                ],
            });
        }
    }
}
