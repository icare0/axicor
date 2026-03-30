use bevy_egui::egui::{self, Color32};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState};

pub fn draw_model_crumb(
    ui: &mut egui::Ui,
    graph: &BrainTopologyGraph,
    _state: &NodeGraphUiState,
    mut send_open: impl FnMut(std::path::PathBuf),
) {
    let active_proj = graph.active_project.clone().unwrap_or_else(|| "No Project".to_string());
    
    // Модель всегда активна как корень
    if ui.button(egui::RichText::new(&active_proj).strong().color(Color32::WHITE)).clicked() {
        let path = std::path::PathBuf::from("Genesis-Models").join(&active_proj).join("simulation.toml");
        send_open(path);
    }
}
