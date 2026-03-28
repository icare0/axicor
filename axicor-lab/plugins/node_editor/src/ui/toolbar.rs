// ui/toolbar.rs
use bevy_egui::egui::{self, Color32, Rect};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};

pub fn render_canvas_context_menu(
    ui: &mut egui::Ui,
    rect: Rect,
    state: &mut NodeGraphUiState,
    _graph: &BrainTopologyGraph,
    send_mutation: &mut impl FnMut(TopologyMutation),
    _send_save: &mut impl FnMut(),
    _send_bake: &mut impl FnMut(),
) {
    let is_model_level = state.level == crate::domain::EditorLevel::Model;
    let title = if is_model_level { "🏢 Create Department" } else { "🧩 Create Zone" };
    
    ui.label(egui::RichText::new(title).strong().color(Color32::LIGHT_BLUE));
    let resp = ui.text_edit_singleline(&mut state.new_node_buffer);
    resp.request_focus();
    
    ui.add_space(4.0);
    if ui.button("➕ Add Node").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let name = state.new_node_buffer.trim().to_string();
        if !name.is_empty() {
            if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
                let local = ((mouse.to_vec2() - rect.min.to_vec2() - state.pan) / state.zoom).to_pos2();
                
                // DOD FIX: Теперь используем send_mutation вместо прямого пуша в graph
                send_mutation(TopologyMutation::AddZone { name, pos: local });
            }
            state.new_node_buffer.clear();
            ui.close_menu();
            }
    }
}
